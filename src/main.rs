mod app;
mod config;
mod mqtt;
mod persistence;
mod state;
mod ui;

use std::io;
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
use config::Config;
use mqtt::{MqttClient, MqttEvent};

#[derive(Parser, Debug)]
#[command(name = "mqtop")]
#[command(author = "Sourceful Energy")]
#[command(version)]
#[command(about = "mqtop - High-performance MQTT explorer TUI by Sourceful Energy", long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

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

    /// Topic to subscribe to (overrides config)
    #[arg(short, long)]
    topic: Option<String>,

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

    // Load config
    let mut config = if args.config.exists() {
        Config::load(&args.config)
            .with_context(|| format!("Failed to load config from {:?}", args.config))?
    } else {
        // Create default config if file doesn't exist
        eprintln!("Config file not found at {:?}, using defaults", args.config);
        eprintln!("Create a config.toml or provide --host and --client-id");

        // Check for minimum required args
        if args.host.is_none() || args.client_id.is_none() {
            eprintln!("\nUsage: mqtop --host <mqtt-host> --client-id <client-id>");
            eprintln!("\nOr create a config.toml:");
            eprintln!("\n[mqtt]");
            eprintln!("host = \"your-mqtt-broker.com\"");
            eprintln!("port = 8883");
            eprintln!("use_tls = true");
            eprintln!("client_id = \"your-client-id\"");
            eprintln!("token = \"your-token\"");
            eprintln!("subscribe_topic = \"#\"");
            std::process::exit(1);
        }

        Config {
            mqtt: config::MqttConfig {
                host: args.host.clone().unwrap_or_default(),
                port: args.port.unwrap_or(1883),
                use_tls: args.tls,
                client_id: args.client_id.clone().unwrap_or_default(),
                username: args.username.clone(),
                token: std::env::var("MQTT_TOKEN").ok(),
                subscribe_topic: args.topic.clone().unwrap_or_else(|| "#".to_string()),
                keep_alive_secs: 30,
            },
            ui: config::UiConfig::default(),
        }
    };

    // Override config with CLI args
    if let Some(host) = args.host {
        config.mqtt.host = host;
    }
    if let Some(port) = args.port {
        config.mqtt.port = port;
    }
    if let Some(client_id) = args.client_id {
        config.mqtt.client_id = client_id;
    }
    if let Some(username) = args.username {
        config.mqtt.username = Some(username);
    }
    if let Some(topic) = args.topic {
        config.mqtt.subscribe_topic = topic;
    }
    if args.tls {
        config.mqtt.use_tls = true;
    }

    info!("Starting mqtop");
    info!("Connecting to {}:{}", config.mqtt.host, config.mqtt.port);

    // Run the TUI application
    run_app(config).await
}

async fn run_app(config: Config) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let tick_rate = Duration::from_millis(config.ui.tick_rate_ms);
    let mut app = App::new(config.clone());

    // Create channel for MQTT events
    let (mqtt_tx, mut mqtt_rx) = mpsc::unbounded_channel::<MqttEvent>();

    // Start MQTT client
    let _mqtt_client = MqttClient::connect(config.mqtt.clone(), mqtt_tx)
        .await
        .context("Failed to connect to MQTT broker")?;

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

        // Check for quit
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
