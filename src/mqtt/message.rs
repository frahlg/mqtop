use chrono::{DateTime, Utc};

/// Represents a received MQTT message
#[derive(Debug, Clone)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: u8,
    pub retain: bool,
    pub timestamp: DateTime<Utc>,
    original_payload_size: usize,
}

impl MqttMessage {
    pub fn new(topic: String, payload: Vec<u8>, qos: u8, retain: bool) -> Self {
        let original_payload_size = payload.len();
        Self {
            topic,
            payload,
            qos,
            retain,
            timestamp: Utc::now(),
            original_payload_size,
        }
    }

    pub fn new_with_payload_limit(
        topic: String,
        payload: &[u8],
        qos: u8,
        retain: bool,
        max_payload_size: usize,
    ) -> Self {
        let original_payload_size = payload.len();
        if original_payload_size <= max_payload_size {
            return Self::new(topic, payload.to_vec(), qos, retain);
        }

        let mut stored_size = original_payload_size.min(max_payload_size);

        if stored_size < original_payload_size {
            if let Ok(text) = std::str::from_utf8(payload) {
                while !text.is_char_boundary(stored_size) {
                    stored_size -= 1;
                }
            }
        }

        Self {
            topic,
            payload: payload[..stored_size].to_vec(),
            qos,
            retain,
            timestamp: Utc::now(),
            original_payload_size,
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

    /// Get the original payload size in bytes, before truncation.
    pub fn payload_size(&self) -> usize {
        self.original_payload_size
    }

    /// Get the number of payload bytes retained for display.
    pub fn stored_payload_size(&self) -> usize {
        self.payload.len()
    }

    /// Whether some payload bytes were omitted due to the configured size limit.
    pub fn is_payload_truncated(&self) -> bool {
        self.stored_payload_size() < self.original_payload_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_is_truncated_to_limit_and_original_size_is_preserved() {
        let message =
            MqttMessage::new_with_payload_limit("test".to_string(), b"abcdefghij", 0, false, 4);

        assert_eq!(message.payload, b"abcd");
        assert_eq!(message.payload_size(), 10);
        assert_eq!(message.stored_payload_size(), 4);
        assert!(message.is_payload_truncated());
    }

    #[test]
    fn utf8_payload_is_not_cut_inside_a_character() {
        let message =
            MqttMessage::new_with_payload_limit("test".to_string(), "a€b".as_bytes(), 0, false, 3);

        assert_eq!(message.payload_str(), Some("a"));
        assert_eq!(message.stored_payload_size(), 1);
        assert!(message.is_payload_truncated());
    }
}
