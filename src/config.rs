use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mqtt: MqttConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

/// Parse color string to ratatui Color
pub fn parse_color(color: &str) -> ratatui::style::Color {
    use ratatui::style::Color;
    match color.to_lowercase().as_str() {
        "red" => Color::Red,
        "green" => Color::Green,
        "blue" => Color::Blue,
        "yellow" => Color::Yellow,
        "cyan" => Color::Cyan,
        "magenta" => Color::Magenta,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "light_red" | "lightred" => Color::LightRed,
        "light_green" | "lightgreen" => Color::LightGreen,
        "light_blue" | "lightblue" => Color::LightBlue,
        "light_yellow" | "lightyellow" => Color::LightYellow,
        "light_cyan" | "lightcyan" => Color::LightCyan,
        "light_magenta" | "lightmagenta" => Color::LightMagenta,
        _ => Color::White,
    }
}

/// Topic color rule for highlighting topics in the tree view
#[derive(Debug, Clone, Deserialize)]
pub struct TopicColorRule {
    /// Pattern to match (case-insensitive, matches segment or path)
    pub pattern: String,
    /// Color name: red, green, blue, yellow, cyan, magenta, white, gray,
    /// light_red, light_green, light_blue, light_yellow, light_cyan, light_magenta
    pub color: String,
}

impl TopicColorRule {
    /// Check if this rule matches a topic segment or path
    pub fn matches(&self, segment: &str, full_path: &str) -> bool {
        let pattern = self.pattern.to_lowercase();
        let segment_lower = segment.to_lowercase();
        let path_lower = full_path.to_lowercase();

        segment_lower == pattern
            || path_lower.starts_with(&pattern)
            || path_lower.contains(&format!("/{}/", pattern))
    }

    /// Parse color string to ratatui Color
    pub fn to_color(&self) -> ratatui::style::Color {
        parse_color(&self.color)
    }
}

/// Topic category for counting in stats panel
#[derive(Debug, Clone, Deserialize)]
pub struct TopicCategory {
    /// Display label in stats panel
    pub label: String,
    /// Pattern to match (case-insensitive)
    pub pattern: String,
    /// Color for the count display
    pub color: String,
}

impl TopicCategory {
    /// Check if a topic path matches this category
    pub fn matches(&self, full_path: &str) -> bool {
        let pattern = self.pattern.to_lowercase();
        let path_lower = full_path.to_lowercase();

        path_lower.starts_with(&pattern)
            || path_lower.contains(&format!("/{}/", pattern))
            || path_lower.contains(&pattern)
    }

    /// Parse color string to ratatui Color
    pub fn to_color(&self) -> ratatui::style::Color {
        parse_color(&self.color)
    }
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
    /// Custom topic color rules for highlighting in tree view
    #[serde(default)]
    pub topic_colors: Vec<TopicColorRule>,
    /// Topic categories for counting in stats panel
    #[serde(default)]
    pub topic_categories: Vec<TopicCategory>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            message_buffer_size: default_message_buffer_size(),
            stats_window_secs: default_stats_window(),
            tick_rate_ms: default_tick_rate(),
            topic_colors: Vec::new(),
            topic_categories: Vec::new(),
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
    /// Get the default config directory path (~/.config/mqtop/)
    pub fn default_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mqtop")
    }

    /// Get the default config file path (~/.config/mqtop/config.toml)
    pub fn default_path() -> PathBuf {
        Self::default_dir().join("config.toml")
    }

    /// Find config file using fallback chain:
    /// 1. If explicit path provided and exists, use it
    /// 2. If ./config.toml exists in current directory, use it
    /// 3. Otherwise use ~/.config/mqtop/config.toml
    pub fn find_config_path(explicit_path: Option<&Path>) -> PathBuf {
        // 1. Explicit path takes priority
        if let Some(path) = explicit_path {
            if path.exists() {
                return path.to_path_buf();
            }
        }

        // 2. Local config.toml in current directory
        let local_config = PathBuf::from("config.toml");
        if local_config.exists() {
            return local_config;
        }

        // 3. Default to ~/.config/mqtop/config.toml
        Self::default_path()
    }

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
