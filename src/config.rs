use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mqtt: MqttConfig,
    #[serde(default)]
    pub nats: NatsConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

pub const CONFIG_BACKUP_LIMIT: usize = 5;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub active_server: String,
    #[serde(default)]
    pub servers: Vec<MqttServerConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NatsConfig {
    pub active_server: String,
    #[serde(default)]
    pub servers: Vec<NatsServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttServerConfig {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub use_tls: bool,
    /// Path to custom CA certificate (PEM format)
    pub ca_cert: Option<String>,
    /// Path to client certificate for mTLS (PEM format)
    pub client_cert: Option<String>,
    /// Path to client private key for mTLS (PEM format)
    pub client_key: Option<String>,
    /// Skip TLS certificate verification (insecure, for testing only)
    #[serde(default)]
    pub tls_insecure: bool,
    pub client_id: String,
    /// If true, use client_id exactly as specified (no auto-generated suffix)
    /// If false and client_id is empty, generates "mqtop-{timestamp}"
    /// If false and client_id is set, appends "-{timestamp}" for reconnect safety
    #[serde(default)]
    pub use_exact_client_id: bool,
    /// Username for MQTT auth (defaults to client_id if not set)
    pub username: Option<String>,
    /// Token for authentication (goes in password field)
    pub token: Option<String>,
    /// Authentication mode
    #[serde(default)]
    pub auth_mode: MqttAuthMode,
    /// Identity ID for JWT auth-callout
    pub identity_id: Option<String>,
    /// Path to ES256 private key PEM file
    pub private_key_path: Option<String>,
    #[serde(default = "default_subscribe_topic")]
    pub subscribe_topic: String,
    /// QoS level for subscriptions (0, 1, or 2)
    #[serde(default = "default_qos")]
    pub subscribe_qos: u8,
    #[serde(default = "default_keep_alive")]
    pub keep_alive_secs: u64,
    /// MQTT protocol version (3 = 3.1.1, 5 = 5.0)
    #[serde(default = "default_mqtt_version")]
    pub mqtt_version: u8,
    /// Clean session - if true, broker discards previous session state
    #[serde(default = "default_clean_session")]
    pub clean_session: bool,
    /// Last Will and Testament topic (optional)
    pub lwt_topic: Option<String>,
    /// Last Will and Testament payload (optional)
    pub lwt_payload: Option<String>,
    /// Last Will QoS (0, 1, or 2)
    #[serde(default)]
    pub lwt_qos: u8,
    /// Last Will retain flag
    #[serde(default)]
    pub lwt_retain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn default_nats_port() -> u16 {
    4222
}

fn default_subscribe_topic() -> String {
    "#".to_string()
}

fn default_nats_subscribe_subject() -> String {
    ">".to_string()
}

fn default_keep_alive() -> u64 {
    30
}

fn default_qos() -> u8 {
    1
}

fn default_mqtt_version() -> u8 {
    3 // 3 = MQTT 3.1.1, 5 = MQTT 5.0
}

fn default_clean_session() -> bool {
    true
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
    /// Uses ~/.config explicitly for cross-platform consistency
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("mqtop")
    }

    /// Get the default config file path (~/.config/mqtop/config.toml)
    pub fn default_path() -> PathBuf {
        Self::default_dir().join("config.toml")
    }

    /// Get the config backup directory path (<config-dir>/backups/)
    pub fn backup_dir_for(path: &Path) -> PathBuf {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups")
    }

    /// Find config file using fallback chain:
    /// 1. If explicit path provided, use it
    /// 2. If ./config.toml exists in current directory, use it
    /// 3. Otherwise use ~/.config/mqtop/config.toml
    pub fn find_config_path(explicit_path: Option<&Path>) -> PathBuf {
        if let Some(path) = explicit_path {
            return path.to_path_buf();
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

        let config: Config =
            toml::from_str(&contents).with_context(|| "Failed to parse config file")?;

        config.validate()?;
        Ok(config)
    }

    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.validate()?;

        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let contents =
            toml::to_string_pretty(self).with_context(|| "Failed to serialize config")?;
        std::fs::write(path, contents)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;
        Ok(())
    }

    pub fn save_with_backup<P: AsRef<Path>>(&self, path: P, retention: usize) -> Result<()> {
        let path = path.as_ref();
        if path.exists() {
            Self::create_backup(path)?;
        }
        self.save_to(path)?;
        Self::prune_backups(path, retention)?;
        Ok(())
    }

    pub fn backup_existing<P: AsRef<Path>>(path: P) -> Result<Option<PathBuf>> {
        let path = path.as_ref();
        if path.exists() {
            return Ok(Some(Self::create_backup(path)?));
        }
        Ok(None)
    }

    pub fn list_backups<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
        let dir = Self::backup_dir_for(path.as_ref());
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .with_context(|| format!("Failed to read backup directory: {:?}", dir))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .collect();

        entries.sort_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
        entries.reverse();

        Ok(entries.into_iter().map(|entry| entry.path()).collect())
    }

    pub fn rollback_backup<P: AsRef<Path>>(path: P, index: usize, retention: usize) -> Result<()> {
        if index == 0 {
            bail!("Backup index must start at 1");
        }

        let path = path.as_ref();
        let backups = Self::list_backups(path)?;
        let backup = backups
            .get(index - 1)
            .with_context(|| "Backup index out of range")?;

        if path.exists() {
            Self::create_backup(path)?;
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        std::fs::copy(backup, path)
            .with_context(|| format!("Failed to restore backup: {:?}", backup))?;

        Self::prune_backups(path, retention)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_mqtt()?;
        self.validate_nats()?;
        Ok(())
    }

    fn validate_mqtt(&self) -> Result<()> {
        // Allow empty server list (user will configure via UI)
        if self.mqtt.servers.is_empty() {
            return Ok(());
        }

        // If there are servers, active_server must be valid (unless empty)
        if !self.mqtt.active_server.trim().is_empty() && self.mqtt.active_server().is_none() {
            bail!("Active MQTT server not found in server list");
        }

        let mut names = std::collections::HashSet::new();
        for server in &self.mqtt.servers {
            if server.name.trim().is_empty() {
                bail!("MQTT server name cannot be empty");
            }
            if server.host.trim().is_empty() {
                bail!("MQTT server host cannot be empty");
            }
            if !names.insert(server.name.clone()) {
                bail!("Duplicate MQTT server name: {}", server.name);
            }
            // client_id is required only when use_exact_client_id is true (no suffix)
            // otherwise it will be auto-generated with timestamp suffix
            if server.use_exact_client_id && server.client_id.trim().is_empty() {
                bail!(
                    "MQTT client_id cannot be empty when ID Suffix is 'none' (server: {})",
                    server.name
                );
            }
            if server.auth_mode == MqttAuthMode::JwtAuthCallout {
                if server.identity_id.as_ref().is_none_or(|s| s.trim().is_empty()) {
                    bail!("MQTT server '{}': JwtAuthCallout requires identity_id", server.name);
                }
                if server.private_key_path.as_ref().is_none_or(|s| s.trim().is_empty()) {
                    bail!("MQTT server '{}': JwtAuthCallout requires private_key_path", server.name);
                }
            }
        }
        Ok(())
    }

    fn validate_nats(&self) -> Result<()> {
        // Allow empty server list (user will configure via UI)
        if self.nats.servers.is_empty() {
            return Ok(());
        }

        // If there are servers, active_server must be valid (unless empty)
        if !self.nats.active_server.trim().is_empty() && self.nats.active_server().is_none() {
            bail!("Active NATS server not found in server list");
        }

        let mut names = std::collections::HashSet::new();
        for server in &self.nats.servers {
            if server.name.trim().is_empty() {
                bail!("NATS server name cannot be empty");
            }
            if server.host.trim().is_empty() {
                bail!("NATS server host cannot be empty");
            }
            if !names.insert(server.name.clone()) {
                bail!("Duplicate NATS server name: {}", server.name);
            }
            if server.subscribe_subject.trim().is_empty() {
                bail!("NATS subscribe_subject cannot be empty (server: {})", server.name);
            }
            if server.auth_mode == NatsAuthMode::JwtAuthCallout {
                if server.identity_id.as_ref().is_none_or(|s| s.trim().is_empty()) {
                    bail!("NATS server '{}': JwtAuthCallout requires identity_id", server.name);
                }
                if server.private_key_path.as_ref().is_none_or(|s| s.trim().is_empty()) {
                    bail!("NATS server '{}': JwtAuthCallout requires private_key_path", server.name);
                }
            }
        }
        Ok(())
    }

    fn create_backup(path: &Path) -> Result<PathBuf> {
        let backup_dir = Self::backup_dir_for(path);
        std::fs::create_dir_all(&backup_dir)
            .with_context(|| format!("Failed to create backup directory: {:?}", backup_dir))?;

        let timestamp = chrono::Local::now().timestamp_millis();
        let filename = format!("config-{}.toml", timestamp);
        let backup_path = backup_dir.join(filename);

        std::fs::copy(path, &backup_path)
            .with_context(|| format!("Failed to create backup at {:?}", backup_path))?;

        Ok(backup_path)
    }

    fn prune_backups(path: &Path, retention: usize) -> Result<()> {
        let dir = Self::backup_dir_for(path);
        if !dir.exists() {
            return Ok(());
        }

        let backups = Self::list_backups(path)?;
        if backups.len() <= retention {
            return Ok(());
        }

        for backup in backups.iter().skip(retention) {
            let _ = std::fs::remove_file(backup);
        }

        Ok(())
    }
}

impl MqttConfig {
    pub fn active_index(&self) -> Option<usize> {
        self.servers
            .iter()
            .position(|server| server.name == self.active_server)
    }

    pub fn active_server(&self) -> Option<&MqttServerConfig> {
        self.active_index().and_then(|idx| self.servers.get(idx))
    }

    pub fn active_server_mut(&mut self) -> Option<&mut MqttServerConfig> {
        let idx = self.active_index()?;
        self.servers.get_mut(idx)
    }
}

/// Authentication mode for NATS connections.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatsAuthMode {
    /// Token or user/pass authentication (existing behavior)
    #[default]
    Basic,
    /// ES256 JWT auth-callout (Nova Core)
    JwtAuthCallout,
}

/// Authentication mode for MQTT connections.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MqttAuthMode {
    /// Password/token authentication (existing behavior)
    #[default]
    Basic,
    /// ES256 JWT auth-callout (Nova Core)
    JwtAuthCallout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsServerConfig {
    pub name: String,
    pub host: String,
    #[serde(default = "default_nats_port")]
    pub port: u16,
    #[serde(default)]
    pub use_tls: bool,
    /// Path to custom CA certificate (PEM format)
    pub ca_cert: Option<String>,
    /// Skip TLS certificate verification (insecure, for testing only)
    #[serde(default)]
    pub tls_insecure: bool,
    /// Username for NATS auth (optional)
    pub username: Option<String>,
    /// Token for NATS auth (optional; also used as password for user/pass setups)
    pub token: Option<String>,
    /// Optional NATS creds file (JWT/NKey)
    pub creds_file: Option<String>,
    /// Authentication mode
    #[serde(default)]
    pub auth_mode: NatsAuthMode,
    /// Identity ID for JWT auth-callout (used as NATS username and JWT device field)
    pub identity_id: Option<String>,
    /// Path to ES256 private key PEM file
    pub private_key_path: Option<String>,
    #[serde(default = "default_nats_subscribe_subject")]
    pub subscribe_subject: String,
}

impl NatsConfig {
    pub fn active_index(&self) -> Option<usize> {
        self.servers
            .iter()
            .position(|server| server.name == self.active_server)
    }

    pub fn active_server(&self) -> Option<&NatsServerConfig> {
        self.active_index().and_then(|idx| self.servers.get(idx))
    }
}

impl MqttServerConfig {
    /// Get the username, defaulting to client_id if not set
    pub fn get_username(&self) -> &str {
        self.username.as_deref().unwrap_or(&self.client_id)
    }

    /// Get the token, returning empty string if none set
    pub fn get_token(&self) -> &str {
        self.token.as_deref().unwrap_or("")
    }

    /// Create a preset for Nova Core Testnet (MQTT)
    #[allow(dead_code)]
    pub fn nova_core_testnet() -> Self {
        Self {
            name: "Nova Core Testnet MQTT".to_string(),
            host: "novacore-testnet.sourceful.dev".to_string(),
            port: 8883,
            use_tls: true,
            ca_cert: None,
            client_cert: None,
            client_key: None,
            tls_insecure: false,
            client_id: String::new(),
            use_exact_client_id: false,
            username: None,
            token: None,
            auth_mode: MqttAuthMode::JwtAuthCallout,
            identity_id: None,
            private_key_path: None,
            subscribe_topic: "#".to_string(),
            subscribe_qos: 1,
            keep_alive_secs: 30,
            mqtt_version: 3,
            clean_session: true,
            lwt_topic: None,
            lwt_payload: None,
            lwt_qos: 0,
            lwt_retain: false,
        }
    }

    /// Create a preset for Nova Core Mainnet (MQTT)
    #[allow(dead_code)]
    pub fn nova_core_mainnet() -> Self {
        let mut preset = Self::nova_core_testnet();
        preset.name = "Nova Core Mainnet MQTT".to_string();
        preset.host = "novacore-mainnet.sourceful.dev".to_string();
        preset
    }
}

impl NatsServerConfig {
    pub fn get_username(&self) -> &str {
        self.username.as_deref().unwrap_or("")
    }

    pub fn get_token(&self) -> &str {
        self.token.as_deref().unwrap_or("")
    }

    /// Create a preset for Nova Core Testnet (NATS)
    #[allow(dead_code)]
    pub fn nova_core_testnet() -> Self {
        Self {
            name: "Nova Core Testnet".to_string(),
            host: "novacore-testnet.sourceful.dev".to_string(),
            port: 4443,
            use_tls: true,
            ca_cert: None,
            tls_insecure: false,
            username: None,
            token: None,
            creds_file: None,
            auth_mode: NatsAuthMode::JwtAuthCallout,
            identity_id: None,
            private_key_path: None,
            subscribe_subject: ">".to_string(),
        }
    }

    /// Create a preset for Nova Core Mainnet (NATS)
    #[allow(dead_code)]
    pub fn nova_core_mainnet() -> Self {
        let mut preset = Self::nova_core_testnet();
        preset.name = "Nova Core Mainnet".to_string();
        preset.host = "novacore-mainnet.sourceful.dev".to_string();
        preset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backwards_compatible_nats_config() {
        // Config without new fields should still parse
        let toml_str = r#"
            name = "test"
            host = "localhost"
            port = 4222
            subscribe_subject = ">"
        "#;
        let config: NatsServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.auth_mode, NatsAuthMode::Basic);
        assert!(config.identity_id.is_none());
        assert!(config.private_key_path.is_none());
    }

    #[test]
    fn test_jwt_auth_callout_config() {
        let toml_str = r#"
            name = "nova"
            host = "novacore-testnet.sourceful.dev"
            port = 4443
            use_tls = true
            auth_mode = "jwt_auth_callout"
            identity_id = "int-my-service"
            private_key_path = "~/.config/mqtop/keys/testnet.pem"
            subscribe_subject = ">"
        "#;
        let config: NatsServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.auth_mode, NatsAuthMode::JwtAuthCallout);
        assert_eq!(config.identity_id.as_deref(), Some("int-my-service"));
    }

    #[test]
    fn test_nova_core_preset_roundtrip() {
        let preset = NatsServerConfig::nova_core_testnet();
        let toml_str = toml::to_string_pretty(&preset).unwrap();
        let parsed: NatsServerConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.host, "novacore-testnet.sourceful.dev");
        assert_eq!(parsed.port, 4443);
        assert_eq!(parsed.auth_mode, NatsAuthMode::JwtAuthCallout);
    }
}
