#![allow(dead_code)]

use anyhow::{Context, Result};
use rumqttc::tokio_rustls::rustls::{self, ClientConfig, RootCertStore};
use rumqttc::{AsyncClient, Event, LastWill, MqttOptions, Packet, QoS, TlsConfiguration, Transport};
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::config::MqttServerConfig;
use crate::mqtt::message::MqttMessage;
use crate::mqtt::resilience::{BackoffStrategy, ConnectionHealth};

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Events from the MQTT client
#[derive(Debug)]
pub enum MqttEvent {
    Message(MqttMessage),
    StateChange(ConnectionState),
    Error(String),
}

pub struct MqttClient {
    client: AsyncClient,
    config: Arc<MqttServerConfig>,
    health: Arc<RwLock<ConnectionHealth>>,
}

impl MqttClient {
    /// Create a new MQTT client and start the event loop
    pub async fn connect(
        config: MqttServerConfig,
        event_tx: mpsc::UnboundedSender<MqttEvent>,
    ) -> Result<Self> {
        Self::connect_with_backoff(config, event_tx, BackoffStrategy::default()).await
    }

    /// Create a new MQTT client with custom backoff strategy
    pub async fn connect_with_backoff(
        config: MqttServerConfig,
        event_tx: mpsc::UnboundedSender<MqttEvent>,
        backoff: BackoffStrategy,
    ) -> Result<Self> {
        let config = Arc::new(config);
        let health = Arc::new(RwLock::new(ConnectionHealth::new(backoff)));

        // Build MQTT options with client_id based on configuration
        let unique_client_id =
            Self::generate_client_id(&config.client_id, config.use_exact_client_id);
        info!("Connecting with client_id: {}", unique_client_id);
        let mut mqttoptions = MqttOptions::new(&unique_client_id, &config.host, config.port);

        // Set authentication: username (defaults to client_id), password = token
        mqttoptions.set_credentials(config.get_username(), config.get_token());
        mqttoptions.set_keep_alive(Duration::from_secs(config.keep_alive_secs));

        // Set clean session from config
        mqttoptions.set_clean_session(config.clean_session);

        // Configure Last Will and Testament if provided
        if let Some(lwt_topic) = &config.lwt_topic {
            if !lwt_topic.is_empty() {
                let lwt_payload = config.lwt_payload.clone().unwrap_or_default();
                let lwt_qos = match config.lwt_qos {
                    0 => QoS::AtMostOnce,
                    2 => QoS::ExactlyOnce,
                    _ => QoS::AtLeastOnce,
                };
                let last_will = LastWill::new(lwt_topic, lwt_payload, lwt_qos, config.lwt_retain);
                mqttoptions.set_last_will(last_will);
                info!("Configured LWT on topic: {}", lwt_topic);
            }
        }

        // Configure TLS if enabled
        if config.use_tls {
            let transport = Self::build_tls_transport(&config)?;
            mqttoptions.set_transport(transport);
        }

        // Increase capacity for high-throughput scenarios
        mqttoptions.set_inflight(100);

        // Increase max packet size for large payloads (default is 10KB, set to 1MB)
        mqttoptions.set_max_packet_size(1024 * 1024, 1024 * 1024);

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 1000);

        let health_clone = Arc::clone(&health);
        let event_tx_clone = event_tx.clone();
        let subscribe_topic = config.subscribe_topic.clone();
        let subscribe_qos = match config.subscribe_qos {
            0 => QoS::AtMostOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtLeastOnce,
        };
        let client_clone = client.clone();
        let use_exact_client_id = config.use_exact_client_id;
        let keep_alive_secs = config.keep_alive_secs;

        // Spawn the event loop handler
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(notification) => {
                        match notification {
                            Event::Incoming(Packet::Publish(publish)) => {
                                let msg = MqttMessage::new(
                                    publish.topic.to_string(),
                                    publish.payload.to_vec(),
                                    publish.qos as u8,
                                    publish.retain,
                                );
                                let _ = event_tx_clone.send(MqttEvent::Message(msg));
                            }
                            Event::Incoming(Packet::ConnAck(connack)) => {
                                info!("Connected to MQTT broker: {:?}", connack);
                                health_clone.write().await.record_success();
                                let _ = event_tx_clone
                                    .send(MqttEvent::StateChange(ConnectionState::Connected));

                                // Subscribe after connection is established
                                info!("Subscribing to: {} with QoS {:?}", subscribe_topic, subscribe_qos);
                                if let Err(e) = client_clone
                                    .subscribe(&subscribe_topic, subscribe_qos)
                                    .await
                                {
                                    error!("Failed to subscribe: {:?}", e);
                                    let _ = event_tx_clone.send(MqttEvent::Error(format!(
                                        "Subscribe failed: {:?}",
                                        e
                                    )));
                                }
                            }
                            Event::Incoming(Packet::SubAck(suback)) => {
                                info!("Subscription acknowledged: {:?}", suback);
                            }
                            Event::Incoming(Packet::PingResp) => {
                                debug!("Ping response received");
                            }
                            Event::Outgoing(_) => {
                                // Outgoing events, usually not interesting
                            }
                            other => {
                                debug!("MQTT event: {:?}", other);
                            }
                        }
                    }
                    Err(e) => {
                        let error_str = format!("{:?}", e);
                        error!("MQTT connection error: {}", error_str);

                        let mut health = health_clone.write().await;
                        health.record_failure(error_str.clone());

                        let _ = event_tx_clone
                            .send(MqttEvent::StateChange(ConnectionState::Reconnecting));
                        let _ = event_tx_clone.send(MqttEvent::Error(error_str));

                        // Check if we should continue reconnecting
                        if !health.should_reconnect() {
                            error!("Max reconnection attempts reached, giving up");
                            let _ = event_tx_clone
                                .send(MqttEvent::StateChange(ConnectionState::Disconnected));
                            break;
                        }

                        // Get backoff delay
                        if let Some(mut delay) = health.next_reconnect_delay() {
                            // When using exact client ID, ensure minimum delay to let broker
                            // clean up old session and avoid session takeover kick loops
                            if use_exact_client_id {
                                let min_delay = Duration::from_secs(keep_alive_secs + 2);
                                if delay < min_delay {
                                    delay = min_delay;
                                    info!(
                                        "Using extended delay for exact client ID (keep_alive + 2s)"
                                    );
                                }
                            }
                            warn!(
                                "Reconnecting in {:?} (attempt {}, total reconnects: {})",
                                delay,
                                health.failure_count(),
                                health.total_reconnects()
                            );
                            drop(health); // Release lock before sleeping
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }
        });

        let mqtt_client = Self {
            client,
            config,
            health,
        };

        let _ = event_tx.send(MqttEvent::StateChange(ConnectionState::Connecting));

        Ok(mqtt_client)
    }

    /// Subscribe to the configured topic pattern
    pub async fn subscribe(&self) -> Result<()> {
        info!("Subscribing to: {}", self.config.subscribe_topic);
        self.client
            .subscribe(&self.config.subscribe_topic, QoS::AtLeastOnce)
            .await?;
        Ok(())
    }

    /// Subscribe to a specific topic
    pub async fn subscribe_topic(&self, topic: &str) -> Result<()> {
        info!("Subscribing to: {}", topic);
        self.client.subscribe(topic, QoS::AtLeastOnce).await?;
        Ok(())
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe(&self, topic: &str) -> Result<()> {
        info!("Unsubscribing from: {}", topic);
        self.client.unsubscribe(topic).await?;
        Ok(())
    }

    /// Publish a message
    pub async fn publish(&self, topic: &str, payload: &[u8], qos: QoS, retain: bool) -> Result<()> {
        self.client.publish(topic, qos, retain, payload).await?;
        Ok(())
    }

    /// Disconnect from the broker
    pub async fn disconnect(&self) -> Result<()> {
        self.client.disconnect().await?;
        Ok(())
    }

    /// Check if the connection is healthy
    pub async fn is_healthy(&self) -> bool {
        self.health.read().await.is_healthy()
    }

    /// Get connection health statistics
    pub async fn health_stats(&self) -> (u64, u64, u32) {
        let health = self.health.read().await;
        (
            health.total_connections(),
            health.total_reconnects(),
            health.failure_count(),
        )
    }

    /// Get the last error message if any
    pub async fn last_error(&self) -> Option<String> {
        self.health.read().await.last_error().map(|s| s.to_string())
    }

    /// Build TLS transport based on configuration
    fn build_tls_transport(config: &MqttServerConfig) -> Result<Transport> {
        use rustls_pemfile::{certs, private_key};
        use std::io::BufReader;

        // Build root certificate store
        let mut root_store = RootCertStore::empty();

        if let Some(ca_path) = &config.ca_cert {
            // Load custom CA certificate
            let ca_file = std::fs::File::open(ca_path)
                .with_context(|| format!("Failed to open CA certificate: {}", ca_path))?;
            let mut reader = BufReader::new(ca_file);
            let ca_certs = certs(&mut reader).collect::<Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse CA certificate: {}", ca_path))?;
            for cert in ca_certs {
                root_store.add(cert)
                    .with_context(|| "Failed to add CA certificate to store")?;
            }
            info!("Loaded custom CA certificate from: {}", ca_path);
        } else {
            // Use system certificates
            let native_certs = rustls_native_certs::load_native_certs()
                .context("Failed to load native certificates")?;
            for cert in native_certs {
                root_store.add(cert).ok(); // Ignore individual cert errors
            }
        }

        // Build client config
        let builder = ClientConfig::builder().with_root_certificates(root_store);

        let client_config = if let (Some(cert_path), Some(key_path)) = (&config.client_cert, &config.client_key) {
            // Load client certificate for mTLS
            let cert_file = std::fs::File::open(cert_path)
                .with_context(|| format!("Failed to open client certificate: {}", cert_path))?;
            let mut cert_reader = BufReader::new(cert_file);
            let client_certs = certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse client certificate: {}", cert_path))?;

            let key_file = std::fs::File::open(key_path)
                .with_context(|| format!("Failed to open client key: {}", key_path))?;
            let mut key_reader = BufReader::new(key_file);
            let client_key = private_key(&mut key_reader)
                .with_context(|| format!("Failed to read client key: {}", key_path))?
                .ok_or_else(|| anyhow::anyhow!("No private key found in: {}", key_path))?;

            info!("Loaded client certificate for mTLS from: {}", cert_path);
            builder.with_client_auth_cert(client_certs, client_key)
                .context("Failed to configure client authentication")?
        } else {
            builder.with_no_client_auth()
        };

        // Handle insecure mode (skip certificate verification)
        let client_config = if config.tls_insecure {
            warn!("TLS certificate verification disabled - INSECURE!");
            // Create a new config that skips verification
            let mut dangerous_config = ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(InsecureCertVerifier))
                .with_no_client_auth();
            dangerous_config.alpn_protocols = client_config.alpn_protocols.clone();
            dangerous_config
        } else {
            client_config
        };

        Ok(Transport::tls_with_config(TlsConfiguration::Rustls(Arc::new(client_config))))
    }

    /// Generate a client_id for MQTT connection
    /// - If use_exact_client_id is true: use client_id exactly as specified
    /// - If use_exact_client_id is false and client_id is empty: generate "mqtop-{timestamp}"
    /// - If use_exact_client_id is false and client_id is set: append "-{timestamp}" for reconnect safety
    fn generate_client_id(configured_id: &str, use_exact: bool) -> String {
        if use_exact {
            // User wants exact client_id (for auth purposes or persistent sessions)
            return configured_id.to_string();
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() % 100000)
            .unwrap_or(0);

        if configured_id.trim().is_empty() {
            format!("mqtop-{}", timestamp)
        } else {
            // Append timestamp suffix for reconnect safety
            format!("{}-{}", configured_id, timestamp)
        }
    }
}

/// Certificate verifier that accepts any certificate (INSECURE - for testing only)
#[derive(Debug)]
struct InsecureCertVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
