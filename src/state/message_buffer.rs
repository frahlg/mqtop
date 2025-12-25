use std::collections::{HashMap, VecDeque};

use crate::mqtt::MqttMessage;

/// A bounded ring buffer that stores the last N messages per topic.
/// Prevents memory exhaustion under high message rates.
#[derive(Debug)]
pub struct MessageBuffer {
    /// Messages per topic
    buffers: HashMap<String, VecDeque<MqttMessage>>,
    /// Maximum messages to keep per topic
    max_per_topic: usize,
    /// Total messages currently stored
    total_stored: usize,
}

impl MessageBuffer {
    pub fn new(max_per_topic: usize) -> Self {
        Self {
            buffers: HashMap::new(),
            max_per_topic,
            total_stored: 0,
        }
    }

    /// Add a message to the buffer
    pub fn push(&mut self, message: MqttMessage) {
        let topic = message.topic.clone();
        let buffer = self.buffers.entry(topic).or_insert_with(VecDeque::new);

        // Remove oldest if at capacity
        if buffer.len() >= self.max_per_topic {
            buffer.pop_front();
            self.total_stored = self.total_stored.saturating_sub(1);
        }

        buffer.push_back(message);
        self.total_stored += 1;
    }

    /// Get messages for a specific topic (newest first)
    pub fn get_messages(&self, topic: &str) -> Vec<&MqttMessage> {
        self.buffers
            .get(topic)
            .map(|buf| buf.iter().rev().collect())
            .unwrap_or_default()
    }

    /// Get the most recent message for a topic
    pub fn get_latest(&self, topic: &str) -> Option<&MqttMessage> {
        self.buffers.get(topic)?.back()
    }

    /// Get message count for a topic
    pub fn count_for_topic(&self, topic: &str) -> usize {
        self.buffers.get(topic).map(|b| b.len()).unwrap_or(0)
    }

    /// Get total messages stored across all topics
    pub fn total_stored(&self) -> usize {
        self.total_stored
    }

    /// Get number of topics with messages
    pub fn topic_count(&self) -> usize {
        self.buffers.len()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.buffers.clear();
        self.total_stored = 0;
    }

    /// Clear messages for a specific topic
    pub fn clear_topic(&mut self, topic: &str) {
        if let Some(buffer) = self.buffers.remove(topic) {
            self.total_stored = self.total_stored.saturating_sub(buffer.len());
        }
    }

    /// Get all recent messages across all topics (newest first, limited)
    pub fn get_recent_all(&self, limit: usize) -> Vec<&MqttMessage> {
        let mut all_messages: Vec<_> = self.buffers
            .values()
            .flat_map(|buf| buf.iter())
            .collect();

        // Sort by timestamp descending
        all_messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        all_messages.into_iter().take(limit).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(topic: &str, payload: &str) -> MqttMessage {
        MqttMessage::new(
            topic.to_string(),
            payload.as_bytes().to_vec(),
            0,
            false,
        )
    }

    #[test]
    fn test_push_and_get() {
        let mut buffer = MessageBuffer::new(10);

        buffer.push(make_message("test/topic", "message1"));
        buffer.push(make_message("test/topic", "message2"));

        let messages = buffer.get_messages("test/topic");
        assert_eq!(messages.len(), 2);

        // Newest first
        assert_eq!(messages[0].payload_str().unwrap(), "message2");
        assert_eq!(messages[1].payload_str().unwrap(), "message1");
    }

    #[test]
    fn test_ring_buffer_behavior() {
        let mut buffer = MessageBuffer::new(3); // Only keep 3 messages

        buffer.push(make_message("topic", "msg1"));
        buffer.push(make_message("topic", "msg2"));
        buffer.push(make_message("topic", "msg3"));
        buffer.push(make_message("topic", "msg4")); // Should evict msg1

        let messages = buffer.get_messages("topic");
        assert_eq!(messages.len(), 3);

        // msg1 should be gone, newest first
        assert_eq!(messages[0].payload_str().unwrap(), "msg4");
        assert_eq!(messages[2].payload_str().unwrap(), "msg2");
    }

    #[test]
    fn test_multiple_topics() {
        let mut buffer = MessageBuffer::new(5);

        buffer.push(make_message("topic/a", "a1"));
        buffer.push(make_message("topic/b", "b1"));
        buffer.push(make_message("topic/a", "a2"));

        assert_eq!(buffer.count_for_topic("topic/a"), 2);
        assert_eq!(buffer.count_for_topic("topic/b"), 1);
        assert_eq!(buffer.topic_count(), 2);
        assert_eq!(buffer.total_stored(), 3);
    }

    #[test]
    fn test_get_latest() {
        let mut buffer = MessageBuffer::new(10);

        buffer.push(make_message("topic", "first"));
        buffer.push(make_message("topic", "second"));
        buffer.push(make_message("topic", "latest"));

        let latest = buffer.get_latest("topic").unwrap();
        assert_eq!(latest.payload_str().unwrap(), "latest");
    }

    #[test]
    fn test_clear() {
        let mut buffer = MessageBuffer::new(10);

        buffer.push(make_message("a", "1"));
        buffer.push(make_message("b", "2"));

        buffer.clear();

        assert_eq!(buffer.total_stored(), 0);
        assert_eq!(buffer.topic_count(), 0);
    }
}
