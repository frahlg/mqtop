use chrono::{DateTime, Utc};

/// Represents a received MQTT message
#[derive(Debug, Clone)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: u8,
    pub retain: bool,
    pub timestamp: DateTime<Utc>,
}

impl MqttMessage {
    pub fn new(topic: String, payload: Vec<u8>, qos: u8, retain: bool) -> Self {
        Self {
            topic,
            payload,
            qos,
            retain,
            timestamp: Utc::now(),
        }
    }

    /// Try to parse payload as UTF-8 string
    pub fn payload_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.payload).ok()
    }

    /// Try to parse payload as JSON and pretty-print it
    pub fn payload_json_pretty(&self) -> Option<String> {
        let s = self.payload_str()?;
        let value: serde_json::Value = serde_json::from_str(s).ok()?;
        serde_json::to_string_pretty(&value).ok()
    }

    /// Get payload as hex string
    pub fn payload_hex(&self) -> String {
        self.payload
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Get payload size in bytes
    pub fn payload_size(&self) -> usize {
        self.payload.len()
    }
}
