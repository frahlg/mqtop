use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Tracks message latency and inter-arrival times
#[derive(Debug)]
pub struct LatencyTracker {
    /// Recent inter-arrival times (time between messages)
    inter_arrival_times: VecDeque<Duration>,
    /// Message timestamps vs receive time (if timestamp in payload)
    payload_latencies: VecDeque<Duration>,
    /// Last message receive time
    last_message_time: Option<Instant>,
    /// Max samples to keep
    max_samples: usize,
    /// Running stats for inter-arrival
    pub min_inter_arrival: Duration,
    pub max_inter_arrival: Duration,
    pub total_inter_arrival: Duration,
    pub inter_arrival_count: u64,
    /// Running stats for payload latency
    pub min_payload_latency: Option<Duration>,
    pub max_payload_latency: Option<Duration>,
    pub total_payload_latency: Duration,
    pub payload_latency_count: u64,
}

impl LatencyTracker {
    pub fn new(max_samples: usize) -> Self {
        Self {
            inter_arrival_times: VecDeque::with_capacity(max_samples),
            payload_latencies: VecDeque::with_capacity(max_samples),
            last_message_time: None,
            max_samples,
            min_inter_arrival: Duration::MAX,
            max_inter_arrival: Duration::ZERO,
            total_inter_arrival: Duration::ZERO,
            inter_arrival_count: 0,
            min_payload_latency: None,
            max_payload_latency: None,
            total_payload_latency: Duration::ZERO,
            payload_latency_count: 0,
        }
    }

    /// Record a message arrival
    pub fn record_message(&mut self, payload: &[u8]) {
        let now = Instant::now();

        // Calculate inter-arrival time
        if let Some(last) = self.last_message_time {
            let inter_arrival = now.duration_since(last);

            // Update running stats
            self.min_inter_arrival = self.min_inter_arrival.min(inter_arrival);
            self.max_inter_arrival = self.max_inter_arrival.max(inter_arrival);
            self.total_inter_arrival += inter_arrival;
            self.inter_arrival_count += 1;

            // Store sample
            if self.inter_arrival_times.len() >= self.max_samples {
                self.inter_arrival_times.pop_front();
            }
            self.inter_arrival_times.push_back(inter_arrival);
        }

        self.last_message_time = Some(now);

        // Try to extract timestamp from payload and calculate latency
        if let Some(latency) = self.extract_payload_latency(payload) {
            // Update running stats
            self.min_payload_latency = Some(
                self.min_payload_latency.map_or(latency, |m| m.min(latency))
            );
            self.max_payload_latency = Some(
                self.max_payload_latency.map_or(latency, |m| m.max(latency))
            );
            self.total_payload_latency += latency;
            self.payload_latency_count += 1;

            // Store sample
            if self.payload_latencies.len() >= self.max_samples {
                self.payload_latencies.pop_front();
            }
            self.payload_latencies.push_back(latency);
        }
    }

    /// Try to extract a timestamp from JSON payload and calculate latency
    fn extract_payload_latency(&self, payload: &[u8]) -> Option<Duration> {
        let json: serde_json::Value = serde_json::from_slice(payload).ok()?;

        // Try common timestamp field names
        let timestamp = json.get("timestamp")
            .or_else(|| json.get("ts"))
            .or_else(|| json.get("time"))
            .or_else(|| json.get("t"))?;

        let ts_millis = match timestamp {
            serde_json::Value::Number(n) => {
                let ts = n.as_i64()?;
                // Handle both seconds and milliseconds
                if ts > 1_000_000_000_000 {
                    ts // Already milliseconds
                } else {
                    ts * 1000 // Convert seconds to milliseconds
                }
            }
            serde_json::Value::String(s) => {
                // Try to parse ISO 8601 or epoch
                s.parse::<i64>().ok()?
            }
            _ => return None,
        };

        let now_millis = chrono::Utc::now().timestamp_millis();
        let latency_millis = now_millis - ts_millis;

        // Only accept reasonable latencies (0 to 1 hour)
        if latency_millis >= 0 && latency_millis < 3_600_000 {
            Some(Duration::from_millis(latency_millis as u64))
        } else {
            None
        }
    }

    /// Get average inter-arrival time
    pub fn avg_inter_arrival(&self) -> Option<Duration> {
        if self.inter_arrival_count > 0 {
            Some(self.total_inter_arrival / self.inter_arrival_count as u32)
        } else {
            None
        }
    }

    /// Get average payload latency
    pub fn avg_payload_latency(&self) -> Option<Duration> {
        if self.payload_latency_count > 0 {
            Some(self.total_payload_latency / self.payload_latency_count as u32)
        } else {
            None
        }
    }

    /// Get recent inter-arrival times for sparkline
    pub fn recent_inter_arrivals(&self) -> &VecDeque<Duration> {
        &self.inter_arrival_times
    }

    /// Get recent payload latencies for sparkline
    pub fn recent_payload_latencies(&self) -> &VecDeque<Duration> {
        &self.payload_latencies
    }

    /// Format duration for display
    pub fn format_duration(d: Duration) -> String {
        let millis = d.as_millis();
        if millis < 1000 {
            format!("{}ms", millis)
        } else if millis < 60_000 {
            format!("{:.1}s", millis as f64 / 1000.0)
        } else {
            format!("{:.1}m", millis as f64 / 60_000.0)
        }
    }

    /// Check if latency is concerning (> 5 seconds)
    pub fn has_high_latency(&self) -> bool {
        self.max_payload_latency
            .map(|l| l > Duration::from_secs(5))
            .unwrap_or(false)
    }

    /// Get jitter (variation in inter-arrival time)
    pub fn jitter(&self) -> Option<Duration> {
        if self.inter_arrival_times.len() < 2 {
            return None;
        }

        let avg = self.avg_inter_arrival()?;
        let variance: f64 = self.inter_arrival_times
            .iter()
            .map(|d| {
                let diff = if *d > avg {
                    d.as_secs_f64() - avg.as_secs_f64()
                } else {
                    avg.as_secs_f64() - d.as_secs_f64()
                };
                diff * diff
            })
            .sum::<f64>() / self.inter_arrival_times.len() as f64;

        Some(Duration::from_secs_f64(variance.sqrt()))
    }
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_inter_arrival_tracking() {
        let mut tracker = LatencyTracker::new(10);

        tracker.record_message(b"{}");
        sleep(Duration::from_millis(10));
        tracker.record_message(b"{}");
        sleep(Duration::from_millis(10));
        tracker.record_message(b"{}");

        assert_eq!(tracker.inter_arrival_count, 2);
        assert!(tracker.avg_inter_arrival().is_some());
    }

    #[test]
    fn test_payload_latency() {
        let mut tracker = LatencyTracker::new(10);

        // Create a payload with recent timestamp
        let now_millis = chrono::Utc::now().timestamp_millis();
        let payload = format!(r#"{{"timestamp": {}}}"#, now_millis - 100);

        tracker.record_message(payload.as_bytes());

        assert_eq!(tracker.payload_latency_count, 1);
        let latency = tracker.avg_payload_latency().unwrap();
        // Should be around 100ms (with some tolerance for test execution)
        assert!(latency.as_millis() >= 50 && latency.as_millis() < 500);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(LatencyTracker::format_duration(Duration::from_millis(50)), "50ms");
        assert_eq!(LatencyTracker::format_duration(Duration::from_millis(1500)), "1.5s");
        assert_eq!(LatencyTracker::format_duration(Duration::from_secs(90)), "1.5m");
    }
}
