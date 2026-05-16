//! Failure window tracking for sliding accumulation.
//!
//! This module implements `FailureWindow` that supports two modes:
//! - `time_sliding`: Accumulates failures within a fixed time window (e.g., last 60 seconds)
//! - `count_sliding`: Accumulates the most recent N failures (e.g., last 10 exits)
//!
//! The accumulated results are written to `MeltdownScopeState.quota_counters`
//! for the `evaluate budget` stage to read.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Window mode configuration for failure accumulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowMode {
    /// Time-based sliding window with fixed duration.
    TimeSliding {
        /// Window width in seconds.
        window_secs: u64,
    },
    /// Count-based sliding window with fixed failure count.
    CountSliding {
        /// Maximum number of failures to retain.
        max_count: usize,
    },
}

impl Default for WindowMode {
    /// Creates a default time-sliding window with 60-second width.
    fn default() -> Self {
        Self::TimeSliding { window_secs: 60 }
    }
}

/// Configuration for failure window behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureWindowConfig {
    /// Window mode selection and parameters.
    pub mode: WindowMode,
    /// Threshold at which the window is considered exhausted.
    pub threshold: usize,
}

impl FailureWindowConfig {
    /// Creates a time-sliding failure window configuration.
    ///
    /// # Arguments
    ///
    /// - `window_secs`: Window width in seconds.
    /// - `threshold`: Failure count threshold.
    ///
    /// # Returns
    ///
    /// Returns a [`FailureWindowConfig`] with time-sliding mode.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = rust_supervisor::policy::failure_window::FailureWindowConfig::time_sliding(60, 5);
    /// assert_eq!(config.threshold, 5);
    /// ```
    pub fn time_sliding(window_secs: u64, threshold: usize) -> Self {
        Self {
            mode: WindowMode::TimeSliding { window_secs },
            threshold,
        }
    }

    /// Creates a count-sliding failure window configuration.
    ///
    /// # Arguments
    ///
    /// - `max_count`: Maximum number of failures to retain.
    /// - `threshold`: Failure count threshold.
    ///
    /// # Returns
    ///
    /// Returns a [`FailureWindowConfig`] with count-sliding mode.
    pub fn count_sliding(max_count: usize, threshold: usize) -> Self {
        Self {
            mode: WindowMode::CountSliding { max_count },
            threshold,
        }
    }
}

/// State of the failure window after recording a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureWindowState {
    /// Current number of failures in the window.
    pub current_count: usize,
    /// Whether the threshold has been reached or exceeded.
    pub threshold_reached: bool,
    /// Oldest timestamp in the window (for time-sliding mode).
    pub oldest_timestamp: Option<Instant>,
}

/// Mutable failure window tracker supporting time and count sliding modes.
#[derive(Debug, Clone)]
pub struct FailureWindow {
    /// Configuration that defines window behavior.
    pub config: FailureWindowConfig,
    /// Timestamps of recorded failures.
    failures: VecDeque<Instant>,
    /// Latest failure timestamp for cleanup logic.
    last_failure: Option<Instant>,
}

impl FailureWindow {
    /// Creates a new failure window with the given configuration.
    ///
    /// # Arguments
    ///
    /// - `config`: Window configuration defining mode and thresholds.
    ///
    /// # Returns
    ///
    /// Returns a [`FailureWindow`] with no recorded failures.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::policy::failure_window::{FailureWindow, FailureWindowConfig};
    ///
    /// let config = FailureWindowConfig::time_sliding(60, 5);
    /// let window = FailureWindow::new(config);
    /// assert_eq!(window.current_state().current_count, 0);
    /// ```
    pub fn new(config: FailureWindowConfig) -> Self {
        Self {
            config,
            failures: VecDeque::new(),
            last_failure: None,
        }
    }

    /// Records a failure into the window.
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic time supplied by the runtime or test.
    ///
    /// # Returns
    ///
    /// Returns the updated [`FailureWindowState`] after pruning and recording.
    pub fn record_failure(&mut self, now: Instant) -> FailureWindowState {
        self.prune(now);
        self.failures.push_back(now);
        self.last_failure = Some(now);

        // For count-sliding mode, enforce max_count limit
        if let WindowMode::CountSliding { max_count } = self.config.mode {
            while self.failures.len() > max_count {
                self.failures.pop_front();
            }
        }

        self.current_state()
    }

    /// Clears all recorded failures.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function returns nothing.
    pub fn clear(&mut self) {
        self.failures.clear();
        self.last_failure = None;
    }

    /// Returns the current state of the failure window.
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic time for time-sliding calculations.
    ///
    /// # Returns
    ///
    /// Returns a [`FailureWindowState`] with current metrics.
    pub fn current_state_at(&self, now: Instant) -> FailureWindowState {
        // Create a temporary copy to prune without mutating
        let mut temp_failures = self.failures.clone();
        if let WindowMode::TimeSliding { window_secs } = self.config.mode {
            let window = Duration::from_secs(window_secs);
            while temp_failures
                .front()
                .is_some_and(|ts| now.duration_since(*ts) > window)
            {
                temp_failures.pop_front();
            }
        }

        let current_count = temp_failures.len();
        let threshold_reached = current_count >= self.config.threshold;
        let oldest_timestamp = temp_failures.front().copied();

        FailureWindowState {
            current_count,
            threshold_reached,
            oldest_timestamp,
        }
    }

    /// Returns the current state without time-based pruning.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`FailureWindowState`] with raw current metrics.
    pub fn current_state(&self) -> FailureWindowState {
        let current_count = self.failures.len();
        let threshold_reached = current_count >= self.config.threshold;
        let oldest_timestamp = self.failures.front().copied();

        FailureWindowState {
            current_count,
            threshold_reached,
            oldest_timestamp,
        }
    }

    /// Removes expired entries based on window mode.
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic time.
    ///
    /// # Returns
    ///
    /// This function returns nothing.
    fn prune(&mut self, now: Instant) {
        if let WindowMode::TimeSliding { window_secs } = self.config.mode {
            let window = Duration::from_secs(window_secs);
            while self
                .failures
                .front()
                .is_some_and(|ts| now.duration_since(*ts) > window)
            {
                self.failures.pop_front();
            }
        }
        // Count-sliding mode does not prune by time
    }

    /// Returns the number of failures currently in the window.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the current failure count.
    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_sliding_window_expiration() {
        let config = FailureWindowConfig::time_sliding(10, 3);
        let mut window = FailureWindow::new(config);

        let base = Instant::now();
        window.record_failure(base);
        window.record_failure(base + Duration::from_secs(5));

        // Both failures still in window
        let state = window.current_state_at(base + Duration::from_secs(8));
        assert_eq!(state.current_count, 2);
        assert!(!state.threshold_reached);

        // First failure should expire after 10 seconds
        let state = window.current_state_at(base + Duration::from_secs(11));
        assert_eq!(state.current_count, 1);
    }

    #[test]
    fn test_count_sliding_window_limit() {
        let config = FailureWindowConfig::count_sliding(3, 5);
        let mut window = FailureWindow::new(config);

        let base = Instant::now();
        window.record_failure(base);
        window.record_failure(base + Duration::from_secs(1));
        window.record_failure(base + Duration::from_secs(2));
        window.record_failure(base + Duration::from_secs(3));

        // Should only retain last 3 failures
        assert_eq!(window.failure_count(), 3);
    }

    #[test]
    fn test_threshold_detection() {
        let config = FailureWindowConfig::time_sliding(60, 3);
        let mut window = FailureWindow::new(config);

        let base = Instant::now();
        window.record_failure(base);
        window.record_failure(base + Duration::from_secs(1));

        let state = window.current_state();
        assert!(!state.threshold_reached);

        window.record_failure(base + Duration::from_secs(2));
        let state = window.current_state();
        assert!(state.threshold_reached);
    }

    #[test]
    fn test_default_config() {
        let config = WindowMode::default();
        match config {
            WindowMode::TimeSliding { window_secs } => {
                assert_eq!(window_secs, 60);
            }
            _ => panic!("Default should be TimeSliding"),
        }
    }
}
