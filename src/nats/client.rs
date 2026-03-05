#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Notify, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::config::NatsServerConfig;
use crate::mqtt::resilience::{BackoffStrategy, ConnectionHealth};
use crate::mqtt::{ConnectionState, MqttEvent, MqttMessage};

trait AsyncReadWrite: AsyncRead + AsyncWrite {}
impl<T: AsyncRead + AsyncWrite + ?Sized> AsyncReadWrite for T {}

enum Command {
    Publish {
        subject: String,
        payload: Vec<u8>,
        resp: oneshot::Sender<Result<()>>,
    },
    Pong,
    Disconnect {
        resp: oneshot::Sender<Result<()>>,
    },
}

pub struct NatsClient {
    cmd_tx: Arc<RwLock<mpsc::UnboundedSender<Command>>>,
    shutdown: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
}

impl NatsClient {
    pub async fn connect(
        config: NatsServerConfig,
        event_tx: mpsc::UnboundedSender<MqttEvent>,
    ) -> Result<Self> {
        let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Connecting));

        // Built-in client supports token and user/pass auth. creds_file is intentionally
        // not supported without bringing in extra crypto deps.
        if let Some(creds_file) = config
            .creds_file
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            bail!(
                "NATS creds_file is not supported by the built-in client (creds_file={})",
                creds_file
            );
        }

        // First connection attempt — synchronous so caller gets immediate error if unreachable.
        let stream = connect_stream(&config).await?;
        let (reader, writer) = perform_handshake(stream, &config).await?;

        info!(
            "NATS connected, subscribed to {}",
            config.subscribe_subject.trim()
        );
        let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Connected));

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
        let cmd_tx_shared = Arc::new(RwLock::new(cmd_tx.clone()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_notify = Arc::new(Notify::new());

        let read_handle = spawn_read_loop(reader, event_tx.clone(), cmd_tx);
        let write_handle = spawn_write_loop(writer, cmd_rx);

        // Spawn supervisor for reconnection.
        {
            let cmd_tx_shared = Arc::clone(&cmd_tx_shared);
            let shutdown = Arc::clone(&shutdown);
            let shutdown_notify = Arc::clone(&shutdown_notify);
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                supervisor_loop(
                    config,
                    event_tx,
                    cmd_tx_shared,
                    read_handle,
                    write_handle,
                    shutdown,
                    shutdown_notify,
                )
                .await;
            });
        }

        Ok(Self {
            cmd_tx: cmd_tx_shared,
            shutdown,
            shutdown_notify,
        })
    }

    pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<()> {
        if subject.trim().is_empty() {
            return Err(anyhow!("Subject cannot be empty"));
        }

        let (resp_tx, resp_rx) = oneshot::channel::<Result<()>>();
        self.cmd_tx
            .read()
            .await
            .send(Command::Publish {
                subject: subject.trim().to_string(),
                payload: payload.to_vec(),
                resp: resp_tx,
            })
            .map_err(|_| anyhow!("NATS connection is closed"))?;

        resp_rx
            .await
            .map_err(|_| anyhow!("NATS publish canceled"))?
    }

    pub async fn disconnect(&self) -> Result<()> {
        self.shutdown.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();

        let (resp_tx, resp_rx) = oneshot::channel::<Result<()>>();
        self.cmd_tx
            .read()
            .await
            .send(Command::Disconnect { resp: resp_tx })
            .map_err(|_| anyhow!("NATS connection is closed"))?;

        resp_rx
            .await
            .map_err(|_| anyhow!("NATS disconnect canceled"))?
    }
}

// ---------------------------------------------------------------------------
// Supervisor — monitors read/write tasks, reconnects on failure
// ---------------------------------------------------------------------------

async fn supervisor_loop(
    config: NatsServerConfig,
    event_tx: mpsc::UnboundedSender<MqttEvent>,
    cmd_tx_shared: Arc<RwLock<mpsc::UnboundedSender<Command>>>,
    mut read_handle: JoinHandle<()>,
    mut write_handle: JoinHandle<()>,
    shutdown: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
) {
    let mut health = ConnectionHealth::new(BackoffStrategy::default());
    health.record_success(); // First connection already succeeded.

    loop {
        // Wait for either task to exit or shutdown signal.
        tokio::select! {
            _ = &mut read_handle => {
                write_handle.abort();
            }
            _ = &mut write_handle => {
                read_handle.abort();
            }
            _ = shutdown_notify.notified(), if !shutdown.load(Ordering::SeqCst) => {
                // Shutdown requested while tasks were still running.
                // Let write_loop process the Disconnect command.
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    &mut write_handle,
                ).await;
                read_handle.abort();
                let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Disconnected));
                return;
            }
        }

        // If shutdown was requested, don't reconnect.
        if shutdown.load(Ordering::SeqCst) {
            let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Disconnected));
            return;
        }

        // Connection lost — attempt reconnection.
        health.record_failure("Connection lost".to_string());
        let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Reconnecting));

        if !health.should_reconnect() {
            error!("NATS max reconnection attempts reached, giving up");
            let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Disconnected));
            return;
        }

        // Backoff sleep (interruptible by shutdown).
        if let Some(delay) = health.next_reconnect_delay() {
            warn!(
                "NATS reconnecting in {:?} (attempt {}, total reconnects: {})",
                delay,
                health.failure_count(),
                health.total_reconnects()
            );
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = shutdown_notify.notified() => {
                    let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Disconnected));
                    return;
                }
            }
        }

        if shutdown.load(Ordering::SeqCst) {
            let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Disconnected));
            return;
        }

        // Attempt reconnection.
        match try_reconnect(&config, &event_tx, &cmd_tx_shared).await {
            Ok((rh, wh)) => {
                health.record_success();
                read_handle = rh;
                write_handle = wh;
                // Continue loop — monitor the new tasks.
            }
            Err(err) => {
                error!("NATS reconnection failed: {:?}", err);
                let _ = event_tx.send(MqttEvent::Error(format!("Reconnect failed: {}", err)));
                // Continue loop — will sleep and retry.
            }
        }
    }
}

async fn try_reconnect(
    config: &NatsServerConfig,
    event_tx: &mpsc::UnboundedSender<MqttEvent>,
    cmd_tx_shared: &Arc<RwLock<mpsc::UnboundedSender<Command>>>,
) -> Result<(JoinHandle<()>, JoinHandle<()>)> {
    let stream = connect_stream(config).await?;
    let (reader, writer) = perform_handshake(stream, config).await?;

    info!(
        "NATS reconnected, subscribed to {}",
        config.subscribe_subject.trim()
    );
    let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Connected));

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();

    // Swap the shared sender so new publish() calls use the fresh channel.
    *cmd_tx_shared.write().await = cmd_tx.clone();

    let read_handle = spawn_read_loop(reader, event_tx.clone(), cmd_tx);
    let write_handle = spawn_write_loop(writer, cmd_rx);

    Ok((read_handle, write_handle))
}

// ---------------------------------------------------------------------------
// Connection + handshake
// ---------------------------------------------------------------------------

async fn connect_stream(
    config: &NatsServerConfig,
) -> Result<Box<dyn AsyncReadWrite + Unpin + Send>> {
    let addr = format!("{}:{}", config.host, config.port);
    let tcp = TcpStream::connect(addr)
        .await
        .with_context(|| format!("Failed to connect to {}:{}", config.host, config.port))?;
    let _ = tcp.set_nodelay(true);

    if !config.use_tls {
        return Ok(Box::new(tcp));
    }

    let tls = connect_tls(tcp, config).await?;
    Ok(Box::new(tls))
}

/// Perform the NATS handshake: wait for INFO, send CONNECT + SUB + PING.
/// Returns the reader/writer halves ready for the read/write loops.
async fn perform_handshake(
    stream: Box<dyn AsyncReadWrite + Unpin + Send>,
    config: &NatsServerConfig,
) -> Result<(
    BufReader<tokio::io::ReadHalf<Box<dyn AsyncReadWrite + Unpin + Send>>>,
    tokio::io::WriteHalf<Box<dyn AsyncReadWrite + Unpin + Send>>,
)> {
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);

    // Wait for INFO (server greeting).
    loop {
        let line = read_op_line(&mut reader).await?;
        if line.starts_with("INFO ") {
            debug!("NATS server: {}", line);
            break;
        }
        if line == "PING" {
            write_half.write_all(b"PONG\r\n").await?;
            continue;
        }
        if line.starts_with("-ERR") {
            bail!("NATS server error during handshake: {}", line);
        }
        // Ignore +OK, empty lines, etc.
    }

    // CONNECT
    let connect = build_connect_payload(config);
    write_half
        .write_all(format!("CONNECT {}\r\n", connect).as_bytes())
        .await?;

    // SUB
    let subject = config.subscribe_subject.trim();
    if subject.is_empty() {
        bail!("NATS subscribe_subject cannot be empty");
    }
    write_half
        .write_all(format!("SUB {} 1\r\n", subject).as_bytes())
        .await?;

    // Force the server to respond quickly if the connection is unhealthy.
    write_half.write_all(b"PING\r\n").await?;
    write_half.flush().await?;

    Ok((reader, write_half))
}

async fn connect_tls(
    tcp: TcpStream,
    config: &NatsServerConfig,
) -> Result<impl AsyncRead + AsyncWrite + Unpin + Send> {
    use rumqttc::tokio_rustls::rustls::{self, ClientConfig, RootCertStore};
    use rumqttc::tokio_rustls::TlsConnector;

    // Build root certificate store.
    let mut root_store = RootCertStore::empty();

    if let Some(ca_path) = &config.ca_cert {
        // Load custom CA certificate
        let ca_file = std::fs::File::open(ca_path)
            .with_context(|| format!("Failed to open CA certificate: {}", ca_path))?;
        let mut reader = std::io::BufReader::new(ca_file);
        let ca_certs = rustls_pemfile::certs(&mut reader)
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("Failed to parse CA certificate: {}", ca_path))?;
        for cert in ca_certs {
            root_store
                .add(cert)
                .with_context(|| "Failed to add CA certificate to store")?;
        }
        info!("Loaded custom CA certificate from: {}", ca_path);
    } else {
        // Use system certificates
        let native_certs =
            rustls_native_certs::load_native_certs().context("Failed to load native certificates")?;
        for cert in native_certs {
            root_store.add(cert).ok(); // Ignore individual cert errors
        }
    }

    let builder = ClientConfig::builder().with_root_certificates(root_store);
    let client_config = builder.with_no_client_auth();

    let client_config = if config.tls_insecure {
        tracing::warn!("TLS certificate verification disabled for NATS - INSECURE!");
        ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(InsecureCertVerifier))
            .with_no_client_auth()
    } else {
        client_config
    };

    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name = rustls::pki_types::ServerName::try_from(config.host.clone())
        .context("Invalid TLS server name")?;
    let tls = connector
        .connect(server_name, tcp)
        .await
        .context("TLS handshake failed")?;
    Ok(tls)
}

/// Certificate verifier that accepts any certificate (INSECURE - for testing only)
#[derive(Debug)]
struct InsecureCertVerifier;

impl rumqttc::tokio_rustls::rustls::client::danger::ServerCertVerifier for InsecureCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rumqttc::tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rumqttc::tokio_rustls::rustls::pki_types::CertificateDer<'_>],
        _server_name: &rumqttc::tokio_rustls::rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rumqttc::tokio_rustls::rustls::pki_types::UnixTime,
    ) -> Result<
        rumqttc::tokio_rustls::rustls::client::danger::ServerCertVerified,
        rumqttc::tokio_rustls::rustls::Error,
    > {
        Ok(rumqttc::tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rumqttc::tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &rumqttc::tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        rumqttc::tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        rumqttc::tokio_rustls::rustls::Error,
    > {
        Ok(rumqttc::tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rumqttc::tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        _dss: &rumqttc::tokio_rustls::rustls::DigitallySignedStruct,
    ) -> Result<
        rumqttc::tokio_rustls::rustls::client::danger::HandshakeSignatureValid,
        rumqttc::tokio_rustls::rustls::Error,
    > {
        Ok(rumqttc::tokio_rustls::rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rumqttc::tokio_rustls::rustls::SignatureScheme> {
        use rumqttc::tokio_rustls::rustls::SignatureScheme;
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
        ]
    }
}

// ---------------------------------------------------------------------------
// Read / write loops
// ---------------------------------------------------------------------------

fn spawn_read_loop(
    reader: BufReader<tokio::io::ReadHalf<Box<dyn AsyncReadWrite + Unpin + Send>>>,
    event_tx: mpsc::UnboundedSender<MqttEvent>,
    cmd_tx: mpsc::UnboundedSender<Command>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(err) = read_loop(reader, event_tx, cmd_tx).await {
            error!("NATS read loop stopped: {:?}", err);
        }
    })
}

fn spawn_write_loop(
    writer: tokio::io::WriteHalf<Box<dyn AsyncReadWrite + Unpin + Send>>,
    cmd_rx: mpsc::UnboundedReceiver<Command>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(err) = write_loop(writer, cmd_rx).await {
            error!("NATS write loop stopped: {:?}", err);
        }
    })
}

fn build_connect_payload(config: &NatsServerConfig) -> String {
    let mut obj = serde_json::json!({
        "verbose": false,
        "pedantic": false,
        "lang": "rust",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": 1,
        "echo": true,
        "name": format!("mqtop:{}", config.name),
    });

    if let Some(map) = obj.as_object_mut() {
        let user = config.get_username().trim();
        let token = config.get_token().trim();
        if !user.is_empty() {
            map.insert(
                "user".to_string(),
                serde_json::Value::String(user.to_string()),
            );
            map.insert(
                "pass".to_string(),
                serde_json::Value::String(token.to_string()),
            );
        } else if !token.is_empty() {
            map.insert(
                "auth_token".to_string(),
                serde_json::Value::String(token.to_string()),
            );
        }
    }

    serde_json::to_string(&obj).unwrap_or_else(|_| "{}".to_string())
}

async fn read_op_line<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<String> {
    let mut buf = Vec::new();
    let n = reader.read_until(b'\n', &mut buf).await?;
    if n == 0 {
        bail!("NATS connection closed");
    }
    Ok(String::from_utf8_lossy(&buf)
        .trim_end_matches(['\r', '\n'])
        .to_string())
}

async fn read_loop<R: AsyncRead + Unpin>(
    mut reader: BufReader<R>,
    event_tx: mpsc::UnboundedSender<MqttEvent>,
    cmd_tx: mpsc::UnboundedSender<Command>,
) -> Result<()> {
    loop {
        let line = read_op_line(&mut reader).await?;

        if line == "PING" {
            let _ = cmd_tx.send(Command::Pong);
            continue;
        }
        if line == "PONG" || line == "+OK" || line.is_empty() {
            continue;
        }
        if line.starts_with("INFO ") {
            continue;
        }
        if line.starts_with("-ERR") {
            return Err(anyhow!("NATS server error: {}", line));
        }

        if line.starts_with("MSG ") {
            let (subject, size) = parse_msg_header(&line)
                .with_context(|| format!("Invalid NATS MSG header: {}", line))?;

            let mut payload = vec![0u8; size];
            reader.read_exact(&mut payload).await?;
            // Trailing CRLF
            let mut crlf = [0u8; 2];
            reader.read_exact(&mut crlf).await?;

            let msg = MqttMessage::new(subject, payload, 0, false);
            let _ = event_tx.send(MqttEvent::Message(msg));
            continue;
        }

        if line.starts_with("HMSG ") {
            let (subject, hdr_len, total_len) = parse_hmsg_header(&line)
                .with_context(|| format!("Invalid NATS HMSG header: {}", line))?;

            let mut buf = vec![0u8; total_len];
            reader.read_exact(&mut buf).await?;
            let mut crlf = [0u8; 2];
            reader.read_exact(&mut crlf).await?;

            let payload = if hdr_len <= buf.len() {
                buf[hdr_len..].to_vec()
            } else {
                Vec::new()
            };
            let msg = MqttMessage::new(subject, payload, 0, false);
            let _ = event_tx.send(MqttEvent::Message(msg));
            continue;
        }

        debug!("NATS op: {}", line);
    }
}

async fn write_loop<W: AsyncWrite + Unpin>(
    mut writer: W,
    mut cmd_rx: mpsc::UnboundedReceiver<Command>,
) -> Result<()> {
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            Command::Publish {
                subject,
                payload,
                resp,
            } => {
                let result = write_pub(&mut writer, &subject, &payload).await;
                let _ = resp.send(result.as_ref().map(|_| ()).map_err(|e| anyhow!("{}", e)));
                result?;
            }
            Command::Pong => {
                writer.write_all(b"PONG\r\n").await?;
            }
            Command::Disconnect { resp } => {
                let result = writer.shutdown().await.map_err(|e| e.into());
                let _ = resp.send(result);
                break;
            }
        }
    }
    Ok(())
}

async fn write_pub<W: AsyncWrite + Unpin>(
    writer: &mut W,
    subject: &str,
    payload: &[u8],
) -> Result<()> {
    let header = format!("PUB {} {}\r\n", subject, payload.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(payload).await?;
    writer.write_all(b"\r\n").await?;
    writer.flush().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Protocol parsing
// ---------------------------------------------------------------------------

fn parse_msg_header(line: &str) -> Result<(String, usize)> {
    // MSG <subject> <sid> [reply-to] <size>
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 4 && parts.len() != 5 {
        bail!("expected 4 or 5 tokens");
    }
    let subject = parts[1].to_string();
    let size_str = parts[parts.len() - 1];
    let size: usize = size_str.parse().context("size is not a number")?;
    Ok((subject, size))
}

fn parse_hmsg_header(line: &str) -> Result<(String, usize, usize)> {
    // HMSG <subject> <sid> [reply-to] <hdr_len> <total_len>
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 5 && parts.len() != 6 {
        bail!("expected 5 or 6 tokens");
    }
    let subject = parts[1].to_string();
    let hdr_len_str = parts[parts.len() - 2];
    let total_len_str = parts[parts.len() - 1];
    let hdr_len: usize = hdr_len_str.parse().context("hdr_len is not a number")?;
    let total_len: usize = total_len_str.parse().context("total_len is not a number")?;
    Ok((subject, hdr_len, total_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_msg_header ---

    #[test]
    fn parse_msg_basic() {
        let (subject, size) = parse_msg_header("MSG foo.bar 1 11").unwrap();
        assert_eq!(subject, "foo.bar");
        assert_eq!(size, 11);
    }

    #[test]
    fn parse_msg_with_reply_to() {
        let (subject, size) = parse_msg_header("MSG foo.bar 1 _INBOX.abc 5").unwrap();
        assert_eq!(subject, "foo.bar");
        assert_eq!(size, 5);
    }

    #[test]
    fn parse_msg_zero_size() {
        let (subject, size) = parse_msg_header("MSG topic 1 0").unwrap();
        assert_eq!(subject, "topic");
        assert_eq!(size, 0);
    }

    #[test]
    fn parse_msg_too_few_tokens() {
        assert!(parse_msg_header("MSG foo").is_err());
    }

    #[test]
    fn parse_msg_non_numeric_size() {
        assert!(parse_msg_header("MSG foo 1 abc").is_err());
    }

    // --- parse_hmsg_header ---

    #[test]
    fn parse_hmsg_basic() {
        let (subject, hdr, total) = parse_hmsg_header("HMSG foo.bar 1 22 33").unwrap();
        assert_eq!(subject, "foo.bar");
        assert_eq!(hdr, 22);
        assert_eq!(total, 33);
    }

    #[test]
    fn parse_hmsg_with_reply_to() {
        let (subject, hdr, total) =
            parse_hmsg_header("HMSG foo.bar 1 _INBOX.xyz 10 50").unwrap();
        assert_eq!(subject, "foo.bar");
        assert_eq!(hdr, 10);
        assert_eq!(total, 50);
    }

    #[test]
    fn parse_hmsg_too_few_tokens() {
        assert!(parse_hmsg_header("HMSG foo 1").is_err());
    }

    #[test]
    fn parse_hmsg_non_numeric() {
        assert!(parse_hmsg_header("HMSG foo 1 abc def").is_err());
    }

    // --- build_connect_payload ---

    fn test_config(name: &str) -> NatsServerConfig {
        NatsServerConfig {
            name: name.to_string(),
            host: "localhost".to_string(),
            port: 4222,
            use_tls: false,
            ca_cert: None,
            tls_insecure: false,
            username: None,
            token: None,
            creds_file: None,
            subscribe_subject: ">".to_string(),
        }
    }

    #[test]
    fn build_connect_no_auth() {
        let cfg = test_config("test");
        let payload = build_connect_payload(&cfg);
        let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["name"], "mqtop:test");
        assert_eq!(v["verbose"], false);
        assert!(v.get("user").is_none());
        assert!(v.get("auth_token").is_none());
    }

    #[test]
    fn build_connect_user_pass() {
        let mut cfg = test_config("test");
        cfg.username = Some("alice".to_string());
        cfg.token = Some("secret".to_string());
        let payload = build_connect_payload(&cfg);
        let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["user"], "alice");
        assert_eq!(v["pass"], "secret");
        assert!(v.get("auth_token").is_none());
    }

    #[test]
    fn build_connect_token_only() {
        let mut cfg = test_config("test");
        cfg.token = Some("my-token".to_string());
        let payload = build_connect_payload(&cfg);
        let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["auth_token"], "my-token");
        assert!(v.get("user").is_none());
    }

    // --- read_loop async tests ---

    #[tokio::test]
    async fn read_loop_handles_ping() {
        let input = b"PING\r\n";
        let reader = BufReader::new(&input[..]);
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();

        // read_loop will fail after PING because stream ends, but Pong command should be sent
        let _ = read_loop(reader, event_tx, cmd_tx).await;

        match cmd_rx.try_recv() {
            Ok(Command::Pong) => {} // expected
            other => panic!("Expected Pong command, got {:?}", other.is_ok()),
        }
    }

    #[tokio::test]
    async fn read_loop_handles_msg() {
        let input = b"MSG sensors.temp 1 5\r\nhello\r\n";
        let reader = BufReader::new(&input[..]);
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let (cmd_tx, _cmd_rx) = mpsc::unbounded_channel();

        let _ = read_loop(reader, event_tx, cmd_tx).await;

        // Should have received a Message event
        let mut found_msg = false;
        while let Ok(ev) = event_rx.try_recv() {
            if let MqttEvent::Message(msg) = ev {
                assert_eq!(msg.topic, "sensors.temp");
                assert_eq!(msg.payload, b"hello");
                found_msg = true;
            }
        }
        assert!(found_msg, "Expected a Message event");
    }
}
