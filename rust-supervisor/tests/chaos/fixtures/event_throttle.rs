//! Event subscriber throttle fixture.
//!
//! Provides `FixtureEventThrottle` that simulates a slow event
//! subscriber by introducing an artificial delay per event.

use std::time::Duration;

/// A fixture that throttles event processing.
#[derive(Debug, Clone)]
pub struct FixtureEventThrottle {
    /// Artificial delay per event in milliseconds.
    pub slow_consumer_ms: u64,
}

impl Default for FixtureEventThrottle {
    fn default() -> Self {
        Self {
            slow_consumer_ms: 100,
        }
    }
}

impl FixtureEventThrottle {
    /// Creates a new throttle with the given per-event delay.
    pub fn new(slow_consumer_ms: u64) -> Self {
        Self { slow_consumer_ms }
    }

    /// Sets the per-event delay in milliseconds.
    #[allow(dead_code)]
    pub fn with_slow_consumer_ms(mut self, ms: u64) -> Self {
        self.slow_consumer_ms = ms;
        self
    }

    /// Simulates processing one event with the configured delay.
    pub async fn process_event(&self) {
        if self.slow_consumer_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.slow_consumer_ms)).await;
        }
    }
}
