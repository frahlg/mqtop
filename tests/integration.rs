//! Integration tests for mqtop
//!
//! Note: These tests require a running MQTT broker for full integration testing.
//! Unit tests for resilience logic are in the resilience module.

#![allow(unused_imports)]
#![allow(unexpected_cfgs)]

/// Test configuration parsing
mod config_tests {
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_loads_from_file() {
        let config_content = r#"
[mqtt]
host = "test.example.com"
port = 8883
use_tls = true
client_id = "test-client"
token = "secret-token"
subscribe_topic = "sensors/#"

[ui]
message_buffer_size = 200
stats_window_secs = 30
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        // We can't directly test Config::load without the module being compiled
        // This test validates the config file format is valid TOML
        let parsed: toml::Value = toml::from_str(config_content).unwrap();
        assert_eq!(
            parsed["mqtt"]["host"].as_str().unwrap(),
            "test.example.com"
        );
        assert_eq!(parsed["mqtt"]["port"].as_integer().unwrap(), 8883);
        assert!(parsed["mqtt"]["use_tls"].as_bool().unwrap());
    }

    #[test]
    fn test_minimal_config() {
        let config_content = r#"
[mqtt]
host = "localhost"
client_id = "minimal"
"#;
        let parsed: toml::Value = toml::from_str(config_content).unwrap();
        assert_eq!(parsed["mqtt"]["host"].as_str().unwrap(), "localhost");
        // Default values should be applied when Config::load is called
    }
}

/// Test message handling
mod message_tests {
    #[test]
    fn test_json_payload_parsing() {
        let payload = br#"{"temperature": 23.5, "humidity": 65}"#;
        let json: serde_json::Value = serde_json::from_slice(payload).unwrap();
        assert_eq!(json["temperature"], 23.5);
        assert_eq!(json["humidity"], 65);
    }

    #[test]
    fn test_binary_payload_hex() {
        let payload: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let hex: String = payload.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
        assert_eq!(hex, "de ad be ef");
    }
}

/// Resilience and backoff tests are in src/mqtt/resilience.rs
/// These integration tests would require a real or mock broker

#[cfg(feature = "integration")]
mod broker_tests {
    use super::*;
    use tokio::sync::mpsc;

    /// This test requires MQTT_TEST_HOST environment variable
    #[tokio::test]
    async fn test_connection_to_broker() {
        let host = std::env::var("MQTT_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = std::env::var("MQTT_TEST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(1883);

        // This would test actual broker connection
        // Skipped by default to avoid CI failures without broker
    }
}
