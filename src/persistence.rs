#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// User data that persists across sessions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserData {
    /// Starred/favorite topics
    #[serde(default)]
    pub starred_topics: HashSet<String>,

    /// Starred device IDs
    #[serde(default)]
    pub starred_devices: HashSet<String>,

    /// Last selected topic (for restoring state)
    #[serde(default)]
    pub last_topic: Option<String>,

    /// Tracked metrics (topic -> field name)
    #[serde(default)]
    pub tracked_metrics: Vec<TrackedMetric>,
}

/// A metric being tracked for stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedMetric {
    pub topic_pattern: String,
    pub field_path: String,
    pub label: String,
}

impl UserData {
    /// Get the default data file path
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mqtop")
            .join("userdata.json")
    }

    /// Load user data from file, or return default if not found
    pub fn load() -> Self {
        Self::load_from(Self::default_path()).unwrap_or_default()
    }

    /// Load from a specific path
    pub fn load_from(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read user data from {:?}", path))?;

        serde_json::from_str(&contents)
            .with_context(|| "Failed to parse user data")
    }

    /// Save user data to file
    pub fn save(&self) -> Result<()> {
        self.save_to(Self::default_path())
    }

    /// Save to a specific path
    pub fn save_to(&self, path: PathBuf) -> Result<()> {
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        let contents = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize user data")?;

        std::fs::write(&path, contents)
            .with_context(|| format!("Failed to write user data to {:?}", path))?;

        Ok(())
    }

    /// Toggle star status for a topic
    pub fn toggle_star(&mut self, topic: &str) -> bool {
        if self.starred_topics.contains(topic) {
            self.starred_topics.remove(topic);
            false
        } else {
            self.starred_topics.insert(topic.to_string());
            true
        }
    }

    /// Check if a topic is starred
    pub fn is_starred(&self, topic: &str) -> bool {
        self.starred_topics.contains(topic)
    }

    /// Toggle star for a device
    pub fn toggle_device_star(&mut self, device_id: &str) -> bool {
        if self.starred_devices.contains(device_id) {
            self.starred_devices.remove(device_id);
            false
        } else {
            self.starred_devices.insert(device_id.to_string());
            true
        }
    }

    /// Check if a device is starred
    pub fn is_device_starred(&self, device_id: &str) -> bool {
        self.starred_devices.contains(device_id)
    }

    /// Add a tracked metric
    pub fn add_tracked_metric(&mut self, topic_pattern: String, field_path: String, label: String) {
        // Remove existing with same label
        self.tracked_metrics.retain(|m| m.label != label);
        self.tracked_metrics.push(TrackedMetric {
            topic_pattern,
            field_path,
            label,
        });
    }

    /// Remove a tracked metric by label
    pub fn remove_tracked_metric(&mut self, label: &str) {
        self.tracked_metrics.retain(|m| m.label != label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_toggle_star() {
        let mut data = UserData::default();

        assert!(!data.is_starred("test/topic"));

        let starred = data.toggle_star("test/topic");
        assert!(starred);
        assert!(data.is_starred("test/topic"));

        let starred = data.toggle_star("test/topic");
        assert!(!starred);
        assert!(!data.is_starred("test/topic"));
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_userdata.json");

        let mut data = UserData::default();
        data.toggle_star("topic1");
        data.toggle_star("topic2");
        data.last_topic = Some("topic1".to_string());

        data.save_to(path.clone()).unwrap();

        let loaded = UserData::load_from(path).unwrap();
        assert!(loaded.is_starred("topic1"));
        assert!(loaded.is_starred("topic2"));
        assert_eq!(loaded.last_topic, Some("topic1".to_string()));
    }
}
