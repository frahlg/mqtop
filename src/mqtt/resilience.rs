#![allow(dead_code)]

use std::time::Duration;

/// Backoff strategy for reconnection attempts
#[derive(Debug, Clone)]
pub struct BackoffStrategy {
    /// Base delay in milliseconds
    base_delay_ms: u64,
    /// Maximum delay cap
    max_delay: Duration,
    /// Maximum number of attempts before giving up (None = infinite)
    max_attempts: Option<u32>,
    /// Jitter factor (0.0 to 1.0) to randomize delays
    jitter_factor: f64,
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self {
            base_delay_ms: 5000, // Start at 5 seconds instead of 100ms
            max_delay: Duration::from_secs(60),
            max_attempts: None, // Never give up
            jitter_factor: 0.1,
        }
    }
}

impl BackoffStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base_delay(mut self, delay: Duration) -> Self {
        self.base_delay_ms = delay.as_millis() as u64;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = Some(attempts);
        self
    }

    pub fn with_jitter(mut self, factor: f64) -> Self {
        self.jitter_factor = factor.clamp(0.0, 1.0);
        self
    }

    /// Calculate the delay for a given attempt number (1-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Option<Duration> {
        // Check if we've exceeded max attempts
        if let Some(max) = self.max_attempts {
            if attempt > max {
                return None;
            }
        }

        // Calculate exponential backoff: base * 2^(attempt-1)
        let exponent = (attempt.saturating_sub(1)).min(20); // Cap to prevent overflow
        let delay_ms = self.base_delay_ms.saturating_mul(2u64.pow(exponent));
        let delay = Duration::from_millis(delay_ms).min(self.max_delay);

        // Apply jitter
        let jitter_range = (delay.as_millis() as f64 * self.jitter_factor) as u64;
        let jitter = if jitter_range > 0 {
            // Simple deterministic "jitter" based on attempt number for reproducibility
            // In production, you'd use actual randomness
            (attempt as u64 * 17) % jitter_range
        } else {
            0
        };

        let final_delay =
            Duration::from_millis(delay.as_millis() as u64 + jitter).min(self.max_delay);

        Some(final_delay)
    }

    /// Check if we should continue trying after given number of attempts
    pub fn should_continue(&self, attempts: u32) -> bool {
        match self.max_attempts {
            Some(max) => attempts < max,
            None => true,
        }
    }
}

/// Tracks connection health and manages reconnection state
#[derive(Debug)]
pub struct ConnectionHealth {
    /// Current number of consecutive failures
    consecutive_failures: u32,
    /// Total number of successful connections
    total_connections: u64,
    /// Total number of reconnection attempts
    total_reconnects: u64,
    /// Backoff strategy
    backoff: BackoffStrategy,
    /// Last error message
    last_error: Option<String>,
}

impl ConnectionHealth {
    pub fn new(backoff: BackoffStrategy) -> Self {
        Self {
            consecutive_failures: 0,
            total_connections: 0,
            total_reconnects: 0,
            backoff,
            last_error: None,
        }
    }

    /// Record a successful connection
    pub fn record_success(&mut self) {
        if self.consecutive_failures > 0 {
            self.total_reconnects += 1;
        }
        self.consecutive_failures = 0;
        self.total_connections += 1;
        self.last_error = None;
    }

    /// Record a connection failure
    pub fn record_failure(&mut self, error: String) {
        self.consecutive_failures += 1;
        self.last_error = Some(error);
    }

    /// Get the delay before the next reconnection attempt
    pub fn next_reconnect_delay(&self) -> Option<Duration> {
        self.backoff.delay_for_attempt(self.consecutive_failures)
    }

    /// Check if we should continue trying to reconnect
    pub fn should_reconnect(&self) -> bool {
        self.backoff.should_continue(self.consecutive_failures)
    }

    /// Get current failure count
    pub fn failure_count(&self) -> u32 {
        self.consecutive_failures
    }

    /// Get total successful connections
    pub fn total_connections(&self) -> u64 {
        self.total_connections
    }

    /// Get total reconnection attempts
    pub fn total_reconnects(&self) -> u64 {
        self.total_reconnects
    }

    /// Get the last error message
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Check if currently healthy (no consecutive failures)
    pub fn is_healthy(&self) -> bool {
        self.consecutive_failures == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_exponential_growth() {
        let backoff = BackoffStrategy::new()
            .with_base_delay(Duration::from_millis(100))
            .with_max_delay(Duration::from_secs(60))
            .with_jitter(0.0); // No jitter for predictable tests

        // First attempt: 100ms
        assert_eq!(
            backoff.delay_for_attempt(1),
            Some(Duration::from_millis(100))
        );

        // Second attempt: 200ms
        assert_eq!(
            backoff.delay_for_attempt(2),
            Some(Duration::from_millis(200))
        );

        // Third attempt: 400ms
        assert_eq!(
            backoff.delay_for_attempt(3),
            Some(Duration::from_millis(400))
        );

        // Fourth attempt: 800ms
        assert_eq!(
            backoff.delay_for_attempt(4),
            Some(Duration::from_millis(800))
        );
    }

    #[test]
    fn test_backoff_respects_max_delay() {
        let backoff = BackoffStrategy::new()
            .with_base_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(10))
            .with_jitter(0.0);

        // After many attempts, should cap at max_delay
        let delay = backoff.delay_for_attempt(20).unwrap();
        assert_eq!(delay, Duration::from_secs(10));
    }

    #[test]
    fn test_backoff_max_attempts() {
        let backoff = BackoffStrategy::new().with_max_attempts(3);

        assert!(backoff.should_continue(0));
        assert!(backoff.should_continue(1));
        assert!(backoff.should_continue(2));
        assert!(!backoff.should_continue(3));

        // Delay should return None after max attempts
        assert!(backoff.delay_for_attempt(1).is_some());
        assert!(backoff.delay_for_attempt(3).is_some());
        assert!(backoff.delay_for_attempt(4).is_none());
    }

    #[test]
    fn test_backoff_infinite_attempts() {
        let backoff = BackoffStrategy::new(); // Default is infinite

        assert!(backoff.should_continue(1000));
        assert!(backoff.delay_for_attempt(1000).is_some());
    }

    #[test]
    fn test_connection_health_success_resets_failures() {
        let mut health = ConnectionHealth::new(BackoffStrategy::default());

        // Simulate failures
        health.record_failure("error 1".to_string());
        health.record_failure("error 2".to_string());
        assert_eq!(health.failure_count(), 2);
        assert!(!health.is_healthy());

        // Success should reset
        health.record_success();
        assert_eq!(health.failure_count(), 0);
        assert!(health.is_healthy());
        assert!(health.last_error().is_none());
    }

    #[test]
    fn test_connection_health_tracks_totals() {
        let mut health = ConnectionHealth::new(BackoffStrategy::default());

        // First connection
        health.record_success();
        assert_eq!(health.total_connections(), 1);
        assert_eq!(health.total_reconnects(), 0);

        // Simulate disconnect and reconnect
        health.record_failure("disconnect".to_string());
        health.record_success();
        assert_eq!(health.total_connections(), 2);
        assert_eq!(health.total_reconnects(), 1);

        // Another disconnect and reconnect
        health.record_failure("disconnect".to_string());
        health.record_failure("still down".to_string());
        health.record_success();
        assert_eq!(health.total_connections(), 3);
        assert_eq!(health.total_reconnects(), 2);
    }

    #[test]
    fn test_connection_health_delay_progression() {
        let health = ConnectionHealth::new(
            BackoffStrategy::new()
                .with_base_delay(Duration::from_millis(100))
                .with_jitter(0.0),
        );

        // With 0 failures, no delay needed
        let mut health = health;
        assert!(health.is_healthy());

        // Record failures and check increasing delays
        health.record_failure("e1".to_string());
        let d1 = health.next_reconnect_delay().unwrap();

        health.record_failure("e2".to_string());
        let d2 = health.next_reconnect_delay().unwrap();

        health.record_failure("e3".to_string());
        let d3 = health.next_reconnect_delay().unwrap();

        assert!(d2 > d1, "Delay should increase: {:?} > {:?}", d2, d1);
        assert!(d3 > d2, "Delay should increase: {:?} > {:?}", d3, d2);
    }

    #[test]
    fn test_connection_health_should_reconnect_with_limit() {
        let mut health = ConnectionHealth::new(BackoffStrategy::new().with_max_attempts(2));

        assert!(health.should_reconnect());

        health.record_failure("e1".to_string());
        assert!(health.should_reconnect());

        health.record_failure("e2".to_string());
        assert!(!health.should_reconnect()); // Exceeded limit
    }
}
