#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Tracks numeric metrics from JSON payloads over time
#[derive(Debug)]
pub struct MetricTracker {
    /// Tracked metrics by label
    metrics: HashMap<String, TrackedMetric>,
    /// Max data points to keep per metric
    max_points: usize,
}

#[derive(Debug)]
pub struct TrackedMetric {
    /// Display label
    pub label: String,
    /// Topic pattern to match
    pub topic_pattern: String,
    /// JSON field path (e.g., "W" or "data.power")
    pub field_path: String,
    /// Data points (timestamp, value)
    pub data: VecDeque<(Instant, f64)>,
    /// Running stats
    pub min: f64,
    pub max: f64,
    pub sum: f64,
    pub count: u64,
}

impl TrackedMetric {
    pub fn new(label: String, topic_pattern: String, field_path: String) -> Self {
        Self {
            label,
            topic_pattern,
            field_path,
            data: VecDeque::new(),
            min: f64::MAX,
            max: f64::MIN,
            sum: 0.0,
            count: 0,
        }
    }

    pub fn record(&mut self, value: f64, max_points: usize) {
        self.data.push_back((Instant::now(), value));
        while self.data.len() > max_points {
            self.data.pop_front();
        }

        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.count += 1;
    }

    pub fn avg(&self) -> f64 {
        if self.count > 0 {
            self.sum / self.count as f64
        } else {
            0.0
        }
    }

    pub fn latest(&self) -> Option<f64> {
        self.data.back().map(|(_, v)| *v)
    }

    /// Generate sparkline data (normalized 0-1)
    pub fn sparkline_data(&self, width: usize) -> Vec<f64> {
        if self.data.is_empty() || self.max <= self.min {
            return vec![0.0; width.min(self.data.len()).max(1)];
        }

        let range = self.max - self.min;
        let step = if self.data.len() <= width {
            1
        } else {
            self.data.len() / width
        };

        self.data
            .iter()
            .step_by(step.max(1))
            .take(width)
            .map(|(_, v)| (v - self.min) / range)
            .collect()
    }
}

impl MetricTracker {
    pub fn new(max_points: usize) -> Self {
        Self {
            metrics: HashMap::new(),
            max_points,
        }
    }

    /// Add a new metric to track
    pub fn track(&mut self, label: String, topic_pattern: String, field_path: String) {
        self.metrics.insert(
            label.clone(),
            TrackedMetric::new(label, topic_pattern, field_path),
        );
    }

    /// Stop tracking a metric
    pub fn untrack(&mut self, label: &str) {
        self.metrics.remove(label);
    }

    /// Process a message and update any matching metrics
    pub fn process_message(&mut self, topic: &str, payload: &[u8]) {
        // Try to parse as JSON
        let json: serde_json::Value = match serde_json::from_slice(payload) {
            Ok(v) => v,
            Err(_) => return,
        };

        for metric in self.metrics.values_mut() {
            // Check if topic matches pattern
            if !topic_matches(&metric.topic_pattern, topic) {
                continue;
            }

            // Extract value from JSON
            if let Some(value) = extract_numeric(&json, &metric.field_path) {
                metric.record(value, self.max_points);
            }
        }
    }

    /// Get all tracked metrics
    pub fn get_metrics(&self) -> Vec<&TrackedMetric> {
        self.metrics.values().collect()
    }

    /// Get a specific metric
    pub fn get_metric(&self, label: &str) -> Option<&TrackedMetric> {
        self.metrics.get(label)
    }

    /// Check if any metrics are being tracked
    pub fn has_metrics(&self) -> bool {
        !self.metrics.is_empty()
    }
}

/// Check if a topic matches a pattern (supports + and # wildcards)
pub fn topic_matches(pattern: &str, topic: &str) -> bool {
    if pattern == "#" {
        return true;
    }

    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let topic_parts: Vec<&str> = topic.split('/').collect();

    let mut pi = 0;
    let mut ti = 0;

    while pi < pattern_parts.len() && ti < topic_parts.len() {
        match pattern_parts[pi] {
            "#" => return true, // # matches everything after
            "+" => {
                // + matches single level
                pi += 1;
                ti += 1;
            }
            p if p == topic_parts[ti] => {
                pi += 1;
                ti += 1;
            }
            _ => return false,
        }
    }

    pi == pattern_parts.len() && ti == topic_parts.len()
}

/// Extract a numeric value from JSON using a field path
fn extract_numeric(json: &serde_json::Value, path: &str) -> Option<f64> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;

    for part in parts {
        current = current.get(part)?;
    }

    match current {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Get all numeric field paths from a JSON value
pub fn get_numeric_fields(json: &serde_json::Value) -> Vec<(String, f64)> {
    let mut fields = Vec::new();
    collect_numeric_fields(json, "", &mut fields);
    fields
}

fn collect_numeric_fields(json: &serde_json::Value, prefix: &str, fields: &mut Vec<(String, f64)>) {
    match json {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                collect_numeric_fields(value, &path, fields);
            }
        }
        serde_json::Value::Number(n) => {
            if let Some(v) = n.as_f64() {
                fields.push((prefix.to_string(), v));
            }
        }
        serde_json::Value::String(s) => {
            if let Ok(v) = s.parse::<f64>() {
                fields.push((prefix.to_string(), v));
            }
        }
        _ => {}
    }
}

/// Render a sparkline from normalized data (0-1)
pub fn render_sparkline(data: &[f64], width: usize) -> String {
    const CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    if data.is_empty() {
        return "─".repeat(width);
    }

    let mut result = String::new();
    let step = if data.len() <= width {
        1
    } else {
        data.len() / width
    };

    for &v in data.iter().step_by(step.max(1)).take(width) {
        let idx = ((v.clamp(0.0, 1.0) * 7.0).round() as usize).min(7);
        result.push(CHARS[idx]);
    }

    // Pad if needed
    while result.chars().count() < width {
        result.push('─');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_matches() {
        assert!(topic_matches("#", "any/topic/here"));
        assert!(topic_matches("telemetry/#", "telemetry/device/sensor"));
        assert!(topic_matches(
            "telemetry/+/sensor",
            "telemetry/device1/sensor"
        ));
        assert!(!topic_matches(
            "telemetry/+/sensor",
            "telemetry/device1/other"
        ));
        assert!(topic_matches("exact/match", "exact/match"));
        assert!(!topic_matches("exact/match", "exact/other"));
    }

    #[test]
    fn test_extract_numeric() {
        let json: serde_json::Value = serde_json::json!({
            "W": 1500,
            "data": {
                "power": 1234.5
            },
            "string_num": "42.5"
        });

        assert_eq!(extract_numeric(&json, "W"), Some(1500.0));
        assert_eq!(extract_numeric(&json, "data.power"), Some(1234.5));
        assert_eq!(extract_numeric(&json, "string_num"), Some(42.5));
        assert_eq!(extract_numeric(&json, "nonexistent"), None);
    }

    #[test]
    fn test_get_numeric_fields() {
        let json: serde_json::Value = serde_json::json!({
            "W": 1500,
            "V": 230.5,
            "type": "meter",
            "data": {
                "power": 1234
            }
        });

        let fields = get_numeric_fields(&json);
        assert!(fields.iter().any(|(k, _)| k == "W"));
        assert!(fields.iter().any(|(k, _)| k == "V"));
        assert!(fields.iter().any(|(k, _)| k == "data.power"));
        assert!(!fields.iter().any(|(k, _)| k == "type"));
    }

    #[test]
    fn test_sparkline() {
        let data = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let sparkline = render_sparkline(&data, 5);
        assert_eq!(sparkline.chars().count(), 5);
    }

    #[test]
    fn test_metric_tracking() {
        let mut tracker = MetricTracker::new(100);
        tracker.track(
            "Power".to_string(),
            "telemetry/#".to_string(),
            "W".to_string(),
        );

        let payload = br#"{"W": 1500, "V": 230}"#;
        tracker.process_message("telemetry/device1/meter", payload);

        let metric = tracker.get_metric("Power").unwrap();
        assert_eq!(metric.latest(), Some(1500.0));
        assert_eq!(metric.count, 1);
    }
}
