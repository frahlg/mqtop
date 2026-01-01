use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Tracks message statistics with rolling window calculations
#[derive(Debug)]
pub struct Stats {
    /// Window size for rate calculations
    window: Duration,
    /// Timestamps of recent messages (for rate calculation)
    message_times: VecDeque<Instant>,
    /// Byte sizes of recent messages
    message_sizes: VecDeque<usize>,
    /// Total messages received (all time)
    total_messages: u64,
    /// Total bytes received (all time)
    total_bytes: u64,
    /// Start time for uptime calculation
    start_time: Instant,
}

impl Stats {
    pub fn new(window_secs: u64) -> Self {
        Self {
            window: Duration::from_secs(window_secs),
            message_times: VecDeque::new(),
            message_sizes: VecDeque::new(),
            total_messages: 0,
            total_bytes: 0,
            start_time: Instant::now(),
        }
    }

    /// Record a new message
    pub fn record_message(&mut self, payload_size: usize) {
        let now = Instant::now();

        self.message_times.push_back(now);
        self.message_sizes.push_back(payload_size);
        self.total_messages += 1;
        self.total_bytes += payload_size as u64;

        // Prune old entries outside the window
        self.prune_old_entries(now);
    }

    fn prune_old_entries(&mut self, now: Instant) {
        let cutoff = now.checked_sub(self.window).unwrap_or(now);

        while let Some(&time) = self.message_times.front() {
            if time < cutoff {
                self.message_times.pop_front();
                self.message_sizes.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get messages per second (averaged over window)
    pub fn messages_per_second(&self) -> f64 {
        self.prune_and_calculate_rate()
    }

    fn prune_and_calculate_rate(&self) -> f64 {
        if self.message_times.is_empty() {
            return 0.0;
        }

        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or(now);

        let count = self.message_times
            .iter()
            .filter(|&&t| t >= cutoff)
            .count();

        count as f64 / self.window.as_secs_f64()
    }

    /// Get bytes per second (averaged over window)
    pub fn bytes_per_second(&self) -> f64 {
        if self.message_times.is_empty() {
            return 0.0;
        }

        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or(now);

        let bytes: usize = self.message_times
            .iter()
            .zip(self.message_sizes.iter())
            .filter(|(&t, _)| t >= cutoff)
            .map(|(_, &s)| s)
            .sum();

        bytes as f64 / self.window.as_secs_f64()
    }

    /// Get total messages received
    pub fn total_messages(&self) -> u64 {
        self.total_messages
    }

    /// Get total bytes received
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get formatted uptime string
    pub fn uptime_string(&self) -> String {
        let duration = self.uptime();
        let secs = duration.as_secs();

        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Format bytes in human-readable form
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Format rate in human-readable form
    pub fn format_rate(rate: f64) -> String {
        if rate >= 1000.0 {
            format!("{:.1}k/s", rate / 1000.0)
        } else if rate >= 1.0 {
            format!("{:.1}/s", rate)
        } else {
            format!("{:.2}/s", rate)
        }
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.message_times.clear();
        self.message_sizes.clear();
        self.total_messages = 0;
        self.total_bytes = 0;
        self.start_time = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_message() {
        let mut stats = Stats::new(10);

        stats.record_message(100);
        stats.record_message(200);

        assert_eq!(stats.total_messages(), 2);
        assert_eq!(stats.total_bytes(), 300);
    }

    #[test]
    fn test_rate_calculation() {
        let mut stats = Stats::new(1); // 1 second window

        // Record 10 messages
        for _ in 0..10 {
            stats.record_message(50);
        }

        // Rate should be approximately 10/s
        let rate = stats.messages_per_second();
        assert!(rate > 0.0, "Rate should be positive");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(Stats::format_bytes(500), "500 B");
        assert_eq!(Stats::format_bytes(1536), "1.50 KB");
        assert_eq!(Stats::format_bytes(1_572_864), "1.50 MB");
        assert_eq!(Stats::format_bytes(1_610_612_736), "1.50 GB");
    }

    #[test]
    fn test_format_rate() {
        assert_eq!(Stats::format_rate(0.5), "0.50/s");
        assert_eq!(Stats::format_rate(5.5), "5.5/s");
        assert_eq!(Stats::format_rate(1500.0), "1.5k/s");
    }

    #[test]
    fn test_uptime_string() {
        let stats = Stats::new(10);
        // Just verify it doesn't panic
        let uptime = stats.uptime_string();
        assert!(!uptime.is_empty());
    }

    #[test]
    fn test_reset() {
        let mut stats = Stats::new(10);

        stats.record_message(100);
        stats.record_message(100);

        assert_eq!(stats.total_messages(), 2);

        stats.reset();

        assert_eq!(stats.total_messages(), 0);
        assert_eq!(stats.total_bytes(), 0);
    }
}
