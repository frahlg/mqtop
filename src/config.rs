use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mqtt: MqttConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub use_tls: bool,
    pub client_id: String,
    /// Username for MQTT auth (defaults to client_id if not set)
    pub username: Option<String>,
    /// Token for authentication (goes in password field)
    /// Can also be set via MQTT_TOKEN env var
    pub token: Option<String>,
    #[serde(default = "default_subscribe_topic")]
    pub subscribe_topic: String,
    #[serde(default = "default_keep_alive")]
    pub keep_alive_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_message_buffer_size")]
    pub message_buffer_size: usize,
    #[serde(default = "default_stats_window")]
    pub stats_window_secs: u64,
    #[serde(default = "default_tick_rate")]
    pub tick_rate_ms: u64,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            message_buffer_size: default_message_buffer_size(),
            stats_window_secs: default_stats_window(),
            tick_rate_ms: default_tick_rate(),
        }
    }
}

fn default_port() -> u16 {
    1883
}

fn default_subscribe_topic() -> String {
    "#".to_string()
}

fn default_keep_alive() -> u64 {
    30
}

fn default_message_buffer_size() -> usize {
    100
}

fn default_stats_window() -> u64 {
    10
}

fn default_tick_rate() -> u64 {
    100
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let mut config: Config = toml::from_str(&contents)
            .with_context(|| "Failed to parse config file")?;

        // Override token from environment if not set in config
        if config.mqtt.token.is_none() {
            config.mqtt.token = std::env::var("MQTT_TOKEN").ok();
        }

        Ok(config)
    }
}

impl MqttConfig {
    /// Get the username, defaulting to client_id if not set
    pub fn get_username(&self) -> &str {
        self.username.as_deref().unwrap_or(&self.client_id)
    }

    /// Get the token, returning empty string if none set
    pub fn get_token(&self) -> &str {
        self.token.as_deref().unwrap_or("")
    }
}
