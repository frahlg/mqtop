mod app;
mod config;
mod mqtt;
mod persistence;
mod state;
mod ui;

use std::io::{self, stdin, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tokio::sync::mpsc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use app::App;
use config::{Config, MqttConfig, MqttServerConfig, CONFIG_BACKUP_LIMIT};
use mqtt::{MqttClient, MqttEvent};

const DEFAULT_WIZARD_PORT: u16 = 1883;
const DEFAULT_WIZARD_KEEP_ALIVE: u64 = 30;

fn list_backups(config_path: &PathBuf) -> Result<()> {
    let backups = Config::list_backups(config_path)?;
    if backups.is_empty() {
        println!("No backups found");
        return Ok(());
    }

    println!("Available backups (newest first):");
    for (index, backup) in backups.iter().enumerate() {
        println!("  {}: {}", index + 1, backup.display());
    }
    Ok(())
}

fn prompt_input(label: &str, default: Option<&str>) -> Result<String> {
    let mut input = String::new();
    match default {
        Some(default) => {
            print!("{} [{}]: ", label, default);
        }
        None => {
            print!("{}: ", label);
        }
    }
    io::stdout().flush()?;
    stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(default.unwrap_or("").to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn prompt_bool(label: &str, default: bool) -> Result<bool> {
    let default_hint = if default { "Y/n" } else { "y/N" };
    let value = prompt_input(&format!("{} ({})", label, default_hint), None)?;
    if value.trim().is_empty() {
        return Ok(default);
    }
    Ok(matches!(
        value.to_lowercase().as_str(),
        "y" | "yes" | "true" | "1"
    ))
}

fn run_config_wizard(config_path: &PathBuf) -> Result<Config> {
    println!("mqtop setup wizard");
    println!("Config path: {}", config_path.display());

    let name = prompt_input("Server name", Some("default"))?;
    let host = prompt_input("Host", None)?;
    let port_raw = prompt_input("Port", Some(&DEFAULT_WIZARD_PORT.to_string()))?;
    let port = port_raw.parse::<u16>().unwrap_or(DEFAULT_WIZARD_PORT);
    let use_tls = prompt_bool("Use TLS", false)?;
    let client_id = prompt_input("Client ID", None)?;
    let username = prompt_input("Username (optional)", Some(""))?;
    let token = prompt_input("Token (optional)", Some(""))?;
    let subscribe_topic = prompt_input("Subscribe topic", Some("#"))?;
    let keep_alive_raw = prompt_input(
        "Keep alive (secs)",
        Some(&DEFAULT_WIZARD_KEEP_ALIVE.to_string()),
    )?;
    let keep_alive_secs = keep_alive_raw
        .parse::<u64>()
        .unwrap_or(DEFAULT_WIZARD_KEEP_ALIVE);

    let server = MqttServerConfig {
        name: if name.trim().is_empty() {
            "default".to_string()
        } else {
            name.trim().to_string()
        },
        host: host.trim().to_string(),
        port,
        use_tls,
        client_id: client_id.trim().to_string(),
        username: if username.trim().is_empty() {
            None
        } else {
            Some(username.trim().to_string())
        },
        token: if token.trim().is_empty() {
            None
        } else {
            Some(token.trim().to_string())
        },
        subscribe_topic: if subscribe_topic.trim().is_empty() {
            "#".to_string()
        } else {
            subscribe_topic.trim().to_string()
        },
        keep_alive_secs,
    };

    let config = Config {
        mqtt: MqttConfig {
            active_server: server.name.clone(),
            servers: vec![server],
        },
        ui: config::UiConfig::default(),
    };

    config.save_with_backup(config_path, CONFIG_BACKUP_LIMIT)?;
    println!("Saved config to {}", config_path.display());
    Ok(config)
}

async fn connect_mqtt(app: &App, mqtt_tx: mpsc::UnboundedSender<MqttEvent>) -> Result<MqttClient> {
    let server = app
        .active_server()
        .context("Active MQTT server missing")?
        .clone();
    MqttClient::connect(server, mqtt_tx)
        .await
        .context("Failed to connect to MQTT broker")
}

#[derive(Parser, Debug)]
#[command(name = "mqtop")]
#[command(author = "Sourceful Energy")]
#[command(version)]
#[command(about = "mqtop - High-performance MQTT explorer TUI by Sourceful Energy", long_about = None)]
struct Args {
    /// Path to configuration file (default: ~/.config/mqtop/config.toml)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// MQTT broker host (overrides config)
    #[arg(long)]
    host: Option<String>,

    /// MQTT broker port (overrides config)
    #[arg(long)]
    port: Option<u16>,

    /// Client ID (overrides config)
    #[arg(long)]
    client_id: Option<String>,

    /// Username for MQTT auth (defaults to client_id if not specified)
    #[arg(short, long)]
    username: Option<String>,

    /// Token for MQTT auth (overrides config)
    #[arg(long)]
    token: Option<String>,

    /// Topic to subscribe to (overrides config)
    #[arg(short, long)]
    topic: Option<String>,

    /// Restore config from a backup index (1 = newest)
    #[arg(long)]
    rollback: Option<usize>,

    /// List available config backups
    #[arg(long)]
    list_backups: bool,

    /// Run interactive config wizard
    #[arg(long)]
    setup: bool,

    /// Enable debug logging to file
    #[arg(short, long)]
    debug: bool,

    /// Use TLS
    #[arg(long)]
    tls: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set up logging
    if args.debug {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::DEBUG)
            .with_writer(|| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("mqtop.log")
                    .expect("Failed to open log file")
            })
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .context("Failed to set tracing subscriber")?;
    }

    // Find and load config
    let config_path = Config::find_config_path(args.config.as_deref());

    if args.list_backups {
        list_backups(&config_path)?;
        return Ok(());
    }

    if let Some(index) = args.rollback {
        Config::rollback_backup(&config_path, index, CONFIG_BACKUP_LIMIT)?;
        eprintln!("Rolled back config using backup #{}", index);
        return Ok(());
    }

    let mut config = if config_path.exists() && !args.setup {
        match Config::load(&config_path) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Config load failed: {}", err);
                let _ = Config::backup_existing(&config_path);
                run_config_wizard(&config_path)?
            }
        }
    } else {
        run_config_wizard(&config_path)?
    };

    if config.mqtt.active_server().is_none() {
        eprintln!("Active server is not defined. Run with --setup to configure.");
        std::process::exit(1);
    }

    // Override config with CLI args (active server only)
    if let Some(server) = config.mqtt.active_server_mut() {
        if let Some(host) = args.host {
            server.host = host;
        }
        if let Some(port) = args.port {
            server.port = port;
        }
        if let Some(client_id) = args.client_id {
            server.client_id = client_id;
        }
        if let Some(username) = args.username {
            server.username = Some(username);
        }
        if let Some(token) = args.token {
            server.token = Some(token);
        }
        if let Some(topic) = args.topic {
            server.subscribe_topic = topic;
        }
        if args.tls {
            server.use_tls = true;
        }
    }

    config
        .save_with_backup(&config_path, CONFIG_BACKUP_LIMIT)
        .context("Failed to persist config")?;

    let active = config
        .mqtt
        .active_server()
        .context("Active MQTT server missing")?;
    info!("Starting mqtop");
    info!("Connecting to {}:{}", active.host, active.port);

    // Run the TUI application
    run_app(config, config_path).await
}

async fn run_app(config: Config, config_path: PathBuf) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let tick_rate = Duration::from_millis(config.ui.tick_rate_ms);
    let mut app = App::new(config.clone(), config_path);

    // Create channel for MQTT events
    let (mqtt_tx, mut mqtt_rx) = mpsc::unbounded_channel::<MqttEvent>();

    // Start MQTT client
    let mut mqtt_client = connect_mqtt(&app, mqtt_tx.clone()).await?;

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| ui::render(f, &app))?;

        // Handle events with timeout
        let timeout = tick_rate;

        // Check for MQTT events (non-blocking)
        while let Ok(event) = mqtt_rx.try_recv() {
            app.handle_mqtt_event(event);
        }

        // Check for terminal events
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code, key.modifiers);
                }
            }
        }

        if let Some(index) = app.pending_server_switch.take() {
            if let Err(err) = mqtt_client.disconnect().await {
                tracing::warn!("Failed to disconnect MQTT client: {:?}", err);
            }
            app.reset_for_server_switch(index)?;
            mqtt_client = connect_mqtt(&app, mqtt_tx.clone()).await?;
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    info!("mqtop exiting");
    Ok(())
}
