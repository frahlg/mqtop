use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks device health based on telemetry message frequency
#[derive(Debug)]
pub struct DeviceTracker {
    /// Known devices with their health info
    devices: HashMap<String, DeviceHealth>,
    /// Time window for rate calculation
    rate_window: Duration,
    /// Threshold for healthy status (messages per minute)
    healthy_threshold: f64,
    /// Threshold for warning status (messages per minute)
    warning_threshold: f64,
}

/// Health status of a device
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Receiving messages at expected rate
    Healthy,
    /// Receiving messages but below expected rate
    Warning,
    /// No recent messages
    Stale,
    /// Never received messages or just discovered
    Unknown,
}

/// Health information for a single device
#[derive(Debug, Clone)]
pub struct DeviceHealth {
    /// Device identifier (from topic path)
    pub device_id: String,
    /// Device type (meter, inverter, etc.)
    pub device_type: Option<String>,
    /// Total messages received
    pub message_count: u64,
    /// Last message timestamp
    pub last_seen: Instant,
    /// Messages in current rate window
    pub recent_messages: Vec<Instant>,
    /// Current health status
    pub status: HealthStatus,
    /// Last payload size
    pub last_payload_size: usize,
    /// Topics this device sends on
    pub topics: Vec<String>,
}

impl DeviceHealth {
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            device_type: None,
            message_count: 0,
            last_seen: Instant::now(),
            recent_messages: Vec::new(),
            status: HealthStatus::Unknown,
            last_payload_size: 0,
            topics: Vec::new(),
        }
    }

    /// Calculate messages per minute based on recent messages
    pub fn messages_per_minute(&self, window: Duration) -> f64 {
        let now = Instant::now();
        let cutoff = now.checked_sub(window).unwrap_or(now);
        let count = self.recent_messages.iter().filter(|t| **t > cutoff).count();

        let window_mins = window.as_secs_f64() / 60.0;
        if window_mins > 0.0 {
            count as f64 / window_mins
        } else {
            0.0
        }
    }

    /// Time since last message
    pub fn time_since_last(&self) -> Duration {
        self.last_seen.elapsed()
    }

    /// Format time since last message as string
    pub fn last_seen_string(&self) -> String {
        let elapsed = self.time_since_last();
        if elapsed.as_secs() < 60 {
            format!("{}s ago", elapsed.as_secs())
        } else if elapsed.as_secs() < 3600 {
            format!("{}m ago", elapsed.as_secs() / 60)
        } else {
            format!("{}h ago", elapsed.as_secs() / 3600)
        }
    }
}

impl DeviceTracker {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            rate_window: Duration::from_secs(60),
            healthy_threshold: 1.0,  // At least 1 msg/min
            warning_threshold: 0.1,  // At least 1 msg/10min
        }
    }

    /// Process a message and update device tracking
    pub fn process_message(&mut self, topic: &str, payload_size: usize) {
        // Extract device ID from topic
        // Pattern: telemetry/{device_id}/...
        if let Some(device_id) = extract_device_id(topic) {
            let device_type = extract_device_type(topic);
            let rate_window = self.rate_window;

            let device = self.devices.entry(device_id.clone()).or_insert_with(|| {
                DeviceHealth::new(device_id.clone())
            });

            device.message_count += 1;
            device.last_seen = Instant::now();
            device.last_payload_size = payload_size;
            device.recent_messages.push(Instant::now());

            // Set device type if found
            if device.device_type.is_none() {
                device.device_type = device_type;
            }

            // Track topic if not already tracked
            if !device.topics.contains(&topic.to_string()) {
                device.topics.push(topic.to_string());
            }

            // Prune old messages from rate window
            let cutoff = Instant::now().checked_sub(rate_window).unwrap_or(Instant::now());
            device.recent_messages.retain(|t| *t > cutoff);

            // Update status inline to avoid borrow issues
            let rate = device.messages_per_minute(rate_window);
            let stale_threshold = Duration::from_secs(300); // 5 minutes

            device.status = if device.time_since_last() > stale_threshold {
                HealthStatus::Stale
            } else if rate >= self.healthy_threshold {
                HealthStatus::Healthy
            } else if rate >= self.warning_threshold {
                HealthStatus::Warning
            } else if device.message_count > 0 {
                HealthStatus::Warning
            } else {
                HealthStatus::Unknown
            };
        }
    }

    /// Update health status for a device
    fn update_device_status(&mut self, device_id: &str) {
        if let Some(device) = self.devices.get_mut(device_id) {
            let rate = device.messages_per_minute(self.rate_window);
            let stale_threshold = Duration::from_secs(300); // 5 minutes

            device.status = if device.time_since_last() > stale_threshold {
                HealthStatus::Stale
            } else if rate >= self.healthy_threshold {
                HealthStatus::Healthy
            } else if rate >= self.warning_threshold {
                HealthStatus::Warning
            } else if device.message_count > 0 {
                HealthStatus::Warning
            } else {
                HealthStatus::Unknown
            };
        }
    }

    /// Get all devices sorted by last seen (most recent first)
    pub fn get_devices(&self) -> Vec<&DeviceHealth> {
        let mut devices: Vec<_> = self.devices.values().collect();
        devices.sort_by(|a, b| a.last_seen.cmp(&b.last_seen).reverse());
        devices
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Get count by health status
    pub fn count_by_status(&self) -> (usize, usize, usize, usize) {
        let mut healthy = 0;
        let mut warning = 0;
        let mut stale = 0;
        let mut unknown = 0;

        for device in self.devices.values() {
            match device.status {
                HealthStatus::Healthy => healthy += 1,
                HealthStatus::Warning => warning += 1,
                HealthStatus::Stale => stale += 1,
                HealthStatus::Unknown => unknown += 1,
            }
        }

        (healthy, warning, stale, unknown)
    }

    /// Update all device statuses (call periodically)
    pub fn update_all_statuses(&mut self) {
        let device_ids: Vec<String> = self.devices.keys().cloned().collect();
        for device_id in device_ids {
            self.update_device_status(&device_id);
        }
    }
}

impl Default for DeviceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract device ID from topic path
/// Pattern: telemetry/{device_id}/... or devices/{device_id}/...
fn extract_device_id(topic: &str) -> Option<String> {
    let parts: Vec<&str> = topic.split('/').collect();

    // telemetry/{device_id}/...
    if parts.len() >= 2 && parts[0] == "telemetry" {
        return Some(parts[1].to_string());
    }

    // devices/{device_id}/...
    if parts.len() >= 2 && parts[0] == "devices" {
        return Some(parts[1].to_string());
    }

    // sites/{site_id}/devices/{device_id}/...
    if parts.len() >= 4 && parts[0] == "sites" && parts[2] == "devices" {
        return Some(parts[3].to_string());
    }

    None
}

/// Extract device type from topic path
/// Pattern: telemetry/{device_id}/{type}/...
fn extract_device_type(topic: &str) -> Option<String> {
    let parts: Vec<&str> = topic.split('/').collect();

    // telemetry/{device_id}/{type}/...
    if parts.len() >= 3 && parts[0] == "telemetry" {
        return Some(parts[2].to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_device_id() {
        assert_eq!(
            extract_device_id("telemetry/zap-0000d8c467e385a0/meter/zap/json"),
            Some("zap-0000d8c467e385a0".to_string())
        );
        assert_eq!(
            extract_device_id("devices/dev123/status"),
            Some("dev123".to_string())
        );
        assert_eq!(
            extract_device_id("sites/site1/devices/dev456/telemetry"),
            Some("dev456".to_string())
        );
        assert_eq!(extract_device_id("random/topic"), None);
    }

    #[test]
    fn test_extract_device_type() {
        assert_eq!(
            extract_device_type("telemetry/zap-0000d8c467e385a0/meter/zap/json"),
            Some("meter".to_string())
        );
        assert_eq!(
            extract_device_type("telemetry/dev123/inverter/data"),
            Some("inverter".to_string())
        );
    }

    #[test]
    fn test_device_tracking() {
        let mut tracker = DeviceTracker::new();

        tracker.process_message("telemetry/device1/meter/zap/json", 100);
        tracker.process_message("telemetry/device1/meter/zap/json", 150);
        tracker.process_message("telemetry/device2/inverter/data", 200);

        assert_eq!(tracker.device_count(), 2);

        let devices = tracker.get_devices();
        assert_eq!(devices.len(), 2);

        // Find device1
        let device1 = devices.iter().find(|d| d.device_id == "device1").unwrap();
        assert_eq!(device1.message_count, 2);
        assert_eq!(device1.device_type, Some("meter".to_string()));
        assert_eq!(device1.last_payload_size, 150);
    }

    #[test]
    fn test_health_status() {
        let mut tracker = DeviceTracker::new();

        // Process messages to make device healthy
        for _ in 0..10 {
            tracker.process_message("telemetry/device1/meter/data", 100);
        }

        let devices = tracker.get_devices();
        let device = devices.iter().find(|d| d.device_id == "device1").unwrap();
        assert_eq!(device.status, HealthStatus::Healthy);
    }
}
