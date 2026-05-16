//! Backoff timing for restart scheduling.
//!
//! This module owns exponential backoff calculation and deterministic jitter
//! support. It does not sleep or spawn tasks.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Jitter source used by backoff calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JitterMode {
    /// Adds no jitter and returns the exponential delay unchanged.
    Disabled,
    /// Adds deterministic jitter derived from this seed.
    Deterministic {
        /// Stable seed used by tests and reproducible simulations.
        seed: u64,
    },
    /// Full jitter mode with uniform random sampling between zero and upper bound.
    FullJitter {
        /// Stable seed used for full jitter calculation.
        seed: u64,
    },
    /// Decorrelated jitter mode that depends on previous wait duration.
    DecorrelatedJitter {
        /// Stable seed used for decorrelated jitter calculation.
        seed: u64,
    },
}

/// Exponential backoff configuration for restart start_counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackoffPolicy {
    /// Initial delay for the first restart child_start_count.
    pub initial: Duration,
    /// Maximum delay allowed after exponential growth and jitter.
    pub max: Duration,
    /// Jitter percentage in the inclusive range from zero to one hundred.
    pub jitter_percent: u8,
    /// Stable runtime duration after which child_start_count counters may be reset.
    pub reset_after: Duration,
    /// Jitter mode used by the calculation.
    pub jitter_mode: JitterMode,
}

impl BackoffPolicy {
    /// Creates an exponential backoff policy.
    ///
    /// # Arguments
    ///
    /// - `initial`: First restart delay.
    /// - `max`: Maximum restart delay.
    /// - `jitter_percent`: Jitter percentage capped at one hundred.
    /// - `reset_after`: Runtime duration after which counters may reset.
    ///
    /// # Returns
    ///
    /// Returns a [`BackoffPolicy`] with jitter disabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// let policy = rust_supervisor::policy::backoff::BackoffPolicy::new(
    ///     Duration::from_millis(10),
    ///     Duration::from_millis(100),
    ///     0,
    ///     Duration::from_secs(1),
    /// );
    /// assert_eq!(policy.delay_for_child_start_count(1), Duration::from_millis(10));
    /// ```
    pub fn new(
        initial: Duration,
        max: Duration,
        jitter_percent: u8,
        reset_after: Duration,
    ) -> Self {
        Self {
            initial,
            max,
            jitter_percent: jitter_percent.min(100),
            reset_after,
            jitter_mode: JitterMode::Disabled,
        }
    }

    /// Returns this policy with deterministic jitter enabled.
    ///
    /// # Arguments
    ///
    /// - `seed`: Stable seed used to derive jitter.
    ///
    /// # Returns
    ///
    /// Returns a new [`BackoffPolicy`] that keeps the same timing bounds.
    pub fn with_deterministic_jitter(mut self, seed: u64) -> Self {
        self.jitter_mode = JitterMode::Deterministic { seed };
        self
    }

    /// Calculates a restart delay for a one-based child_start_count number.
    ///
    /// # Arguments
    ///
    /// - `child_start_count`: One-based restart child_start_count. Zero is treated as one.
    ///
    /// # Returns
    ///
    /// Returns a delay capped by [`BackoffPolicy::max`].
    pub fn delay_for_child_start_count(&self, child_start_count: u64) -> Duration {
        let exponential = self.exponential_delay(child_start_count.max(1));
        self.apply_jitter(exponential).min(self.max)
    }

    /// Reports whether a stable runtime duration should reset counters.
    ///
    /// # Arguments
    ///
    /// - `stable_for`: Duration for which the child has run without failure.
    ///
    /// # Returns
    ///
    /// Returns `true` when `stable_for` reaches [`BackoffPolicy::reset_after`].
    pub fn should_reset(&self, stable_for: Duration) -> bool {
        stable_for >= self.reset_after
    }

    /// Computes the unclamped exponential delay.
    ///
    /// # Arguments
    ///
    /// - `child_start_count`: One-based restart child_start_count.
    ///
    /// # Returns
    ///
    /// Returns the exponential delay before jitter is applied.
    fn exponential_delay(&self, child_start_count: u64) -> Duration {
        let shift = child_start_count.saturating_sub(1).min(32);
        let multiplier = 1_u128 << shift;
        let millis = self.initial.as_millis().saturating_mul(multiplier);
        duration_from_millis(millis).min(self.max)
    }

    /// Applies bounded jitter to a base delay.
    ///
    /// # Arguments
    ///
    /// - `base`: Delay before jitter.
    ///
    /// # Returns
    ///
    /// Returns a jittered delay that never exceeds the configured maximum.
    fn apply_jitter(&self, base: Duration) -> Duration {
        if self.jitter_percent == 0 {
            return base;
        }

        match self.jitter_mode {
            JitterMode::Disabled => base,
            JitterMode::Deterministic { seed } => {
                let jitter = deterministic_jitter(base, self.jitter_percent, seed);
                base.saturating_add(jitter)
            }
            JitterMode::FullJitter { seed } => calculate_full_jitter(base, self.max, seed),
            JitterMode::DecorrelatedJitter { seed } => {
                calculate_decorrelated_jitter(base, self.initial, self.max, seed)
            }
        }
    }
}

/// Converts milliseconds into a duration without overflowing.
///
/// # Arguments
///
/// - `millis`: Millisecond count held in a wide integer.
///
/// # Returns
///
/// Returns a [`Duration`] capped at `u64::MAX` milliseconds.
fn duration_from_millis(millis: u128) -> Duration {
    Duration::from_millis(millis.min(u64::MAX as u128) as u64)
}

/// Derives deterministic positive jitter.
///
/// # Arguments
///
/// - `base`: Base delay.
/// - `percent`: Jitter percentage.
/// - `seed`: Stable seed.
///
/// # Returns
///
/// Returns a jitter duration between zero and the configured percentage.
fn deterministic_jitter(base: Duration, percent: u8, seed: u64) -> Duration {
    let max_jitter = base.as_millis().saturating_mul(percent as u128) / 100;
    if max_jitter == 0 {
        return Duration::ZERO;
    }

    let mixed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    duration_from_millis((mixed as u128) % (max_jitter + 1))
}

/// Calculates full jitter with uniform random sampling.
///
/// Full jitter uniformly samples between zero and min(base_delay, max_delay)
/// to prevent thundering herd problems in distributed systems.
///
/// # Arguments
///
/// - `base`: Base exponential delay before jitter.
/// - `max`: Maximum allowed delay cap.
/// - `seed`: Stable seed for reproducible random sampling.
///
/// # Returns
///
/// Returns a jittered duration uniformly distributed between zero and upper bound.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use rust_supervisor::policy::backoff::calculate_full_jitter;
///
/// let delay = calculate_full_jitter(
///     Duration::from_millis(100),
///     Duration::from_millis(1000),
///     42,
/// );
/// assert!(delay <= Duration::from_millis(100));
/// ```
pub fn calculate_full_jitter(base: Duration, max: Duration, seed: u64) -> Duration {
    let upper_bound = std::cmp::min(base, max);
    let upper_millis = upper_bound.as_millis();
    if upper_millis == 0 {
        return Duration::ZERO;
    }

    // Use simple LCG (Linear Congruential Generator) for deterministic randomness
    let lcg_next = |state: &mut u64| -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        *state
    };

    let mut rng_state = seed;
    let random_value = lcg_next(&mut rng_state);
    let jitter_millis = (random_value as u128) % (upper_millis + 1);
    duration_from_millis(jitter_millis)
}

/// Calculates decorrelated jitter that depends on previous wait duration.
///
/// Decorrelated jitter uses the formula: sleep = min(cap, random(base, sleep * 3))
/// This prevents correlation between retry attempts while maintaining bounded delays.
///
/// # Arguments
///
/// - `base`: Initial base delay for first retry.
/// - `initial`: Minimum delay floor.
/// - `max`: Maximum delay cap.
/// - `seed`: Stable seed for reproducible random sampling.
///
/// # Returns
///
/// Returns a decorrelated jittered duration. For first call, returns value
/// between initial and min(base * 3, max). Subsequent calls should pass
/// previous result as new base for decorrelation.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use rust_supervisor::policy::backoff::calculate_decorrelated_jitter;
///
/// let delay = calculate_decorrelated_jitter(
///     Duration::from_millis(100),
///     Duration::from_millis(10),
///     Duration::from_millis(1000),
///     42,
/// );
/// assert!(delay >= Duration::from_millis(10));
/// assert!(delay <= Duration::from_millis(1000));
/// ```
pub fn calculate_decorrelated_jitter(
    base: Duration,
    initial: Duration,
    max: Duration,
    seed: u64,
) -> Duration {
    // Formula: sleep = min(cap, random(base, sleep * 3))
    // For first call, use initial as lower bound and min(base * 3, max) as upper bound
    let lower = initial.as_millis();
    let upper_candidate = base.as_millis().saturating_mul(3);
    let upper = std::cmp::min(upper_candidate, max.as_millis());

    if upper <= lower {
        return duration_from_millis(lower);
    }

    // Use simple LCG for deterministic randomness
    let lcg_next = |state: &mut u64| -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        *state
    };

    let mut rng_state = seed;
    let random_value = lcg_next(&mut rng_state);
    let range = upper - lower;
    let jitter_millis = lower + ((random_value as u128) % (range + 1));
    duration_from_millis(jitter_millis)
}

/// Cold start budget tracker for limiting restarts during initial startup.
///
/// Tracks restart attempts within a time window after supervisor or child startup.
/// When the budget is exhausted, tighter protection policies are applied to prevent
/// resource exhaustion during the critical cold start period.
#[derive(Debug, Clone)]
pub struct ColdStartBudget {
    /// Time window in seconds during which cold start budget applies.
    pub window_secs: u64,
    /// Maximum number of restarts allowed within the cold start window.
    pub max_restarts: u32,
    /// Current restart count within the window.
    pub restart_count: u32,
    /// Supervisor or child start time (Unix epoch seconds).
    pub start_time_secs: u64,
}

impl ColdStartBudget {
    /// Creates a new cold start budget tracker.
    ///
    /// # Arguments
    ///
    /// - `window_secs`: Time window in seconds for cold start period.
    /// - `max_restarts`: Maximum restarts allowed within the window.
    /// - `start_time_secs`: Start time as Unix epoch seconds.
    ///
    /// # Returns
    ///
    /// Returns a new [`ColdStartBudget`] with zero restart count.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::policy::backoff::ColdStartBudget;
    ///
    /// let budget = ColdStartBudget::new(300, 5, 1000);
    /// assert_eq!(budget.get_restart_count(), 0);
    /// assert!(!budget.is_exhausted(1000));
    /// ```
    pub fn new(window_secs: u64, max_restarts: u32, start_time_secs: u64) -> Self {
        Self {
            window_secs,
            max_restarts,
            restart_count: 0,
            start_time_secs,
        }
    }

    /// Records a restart attempt and checks if budget is exhausted.
    ///
    /// # Arguments
    ///
    /// - `current_time_secs`: Current time as Unix epoch seconds.
    ///
    /// # Returns
    ///
    /// Returns `true` if the cold start budget has been exhausted, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::policy::backoff::ColdStartBudget;
    ///
    /// let mut budget = ColdStartBudget::new(300, 2, 1000);
    /// assert!(!budget.record_restart(1010)); // First restart
    /// assert!(!budget.record_restart(1020)); // Second restart
    /// assert!(budget.record_restart(1030));  // Third restart exhausts budget
    /// ```
    pub fn record_restart(&mut self, current_time_secs: u64) -> bool {
        // Check if we're still within the cold start window
        let elapsed = current_time_secs.saturating_sub(self.start_time_secs);
        if elapsed > self.window_secs {
            // Window expired, reset budget
            self.restart_count = 1;
            return false;
        }

        self.restart_count += 1;
        self.restart_count > self.max_restarts
    }

    /// Checks if the cold start budget is currently exhausted.
    ///
    /// # Arguments
    ///
    /// - `current_time_secs`: Current time as Unix epoch seconds.
    ///
    /// # Returns
    ///
    /// Returns `true` if restart count exceeds limit within active window.
    pub fn is_exhausted(&self, current_time_secs: u64) -> bool {
        let elapsed = current_time_secs.saturating_sub(self.start_time_secs);
        if elapsed > self.window_secs {
            return false; // Window expired
        }
        self.restart_count >= self.max_restarts
    }

    /// Returns the current restart count within the cold start window.
    ///
    /// # Returns
    ///
    /// Returns the number of restarts recorded in the current window.
    pub fn get_restart_count(&self) -> u32 {
        self.restart_count
    }

    /// Checks if the cold start window is still active.
    ///
    /// # Arguments
    ///
    /// - `current_time_secs`: Current time as Unix epoch seconds.
    ///
    /// # Returns
    ///
    /// Returns `true` if within the cold start time window.
    pub fn is_window_active(&self, current_time_secs: u64) -> bool {
        let elapsed = current_time_secs.saturating_sub(self.start_time_secs);
        elapsed <= self.window_secs
    }
}

/// Hot loop detector for identifying rapid crash-restart cycles.
///
/// Detects when a child crashes and restarts too frequently within a sliding
/// time window, indicating a potential hot loop condition that requires
/// protective intervention.
#[derive(Debug, Clone)]
pub struct HotLoopDetector {
    /// Sliding time window in seconds for detecting hot loops.
    pub window_secs: u64,
    /// Minimum number of restarts within window to trigger detection.
    pub min_restarts: u32,
    /// Timestamps of recent crashes (Unix epoch seconds).
    pub crash_times: Vec<u64>,
}

impl HotLoopDetector {
    /// Creates a new hot loop detector.
    ///
    /// # Arguments
    ///
    /// - `window_secs`: Sliding time window in seconds.
    /// - `min_restarts`: Minimum restarts within window to trigger detection.
    ///
    /// # Returns
    ///
    /// Returns a new [`HotLoopDetector`] with empty crash history.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::policy::backoff::HotLoopDetector;
    ///
    /// let detector = HotLoopDetector::new(60, 5);
    /// assert!(!detector.is_hot_loop_detected(1000));
    /// ```
    pub fn new(window_secs: u64, min_restarts: u32) -> Self {
        Self {
            window_secs,
            min_restarts,
            crash_times: Vec::new(),
        }
    }

    /// Records a crash event and checks if hot loop is detected.
    ///
    /// # Arguments
    ///
    /// - `crash_time_secs`: Crash timestamp as Unix epoch seconds.
    ///
    /// # Returns
    ///
    /// Returns `true` if hot loop condition is detected, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::policy::backoff::HotLoopDetector;
    ///
    /// let mut detector = HotLoopDetector::new(60, 3);
    /// detector.record_crash(1000);
    /// detector.record_crash(1010);
    /// detector.record_crash(1020);
    /// assert!(detector.is_hot_loop_detected(1020)); // 3 crashes in 20 seconds
    /// ```
    pub fn record_crash(&mut self, crash_time_secs: u64) -> bool {
        // Add new crash timestamp
        self.crash_times.push(crash_time_secs);

        // Remove timestamps outside the sliding window
        let cutoff = crash_time_secs.saturating_sub(self.window_secs);
        self.crash_times.retain(|&t| t > cutoff);

        // Check if we've exceeded the threshold
        self.is_hot_loop_detected(crash_time_secs)
    }

    /// Checks if hot loop condition is currently detected.
    ///
    /// # Arguments
    ///
    /// - `current_time_secs`: Current time as Unix epoch seconds.
    ///
    /// # Returns
    ///
    /// Returns `true` if crash count within window meets or exceeds threshold.
    pub fn is_hot_loop_detected(&self, current_time_secs: u64) -> bool {
        let cutoff = current_time_secs.saturating_sub(self.window_secs);
        let crashes_in_window = self.crash_times.iter().filter(|&&t| t > cutoff).count();
        crashes_in_window >= self.min_restarts as usize
    }

    /// Returns the number of crashes within the current sliding window.
    ///
    /// # Arguments
    ///
    /// - `current_time_secs`: Current time as Unix epoch seconds.
    ///
    /// # Returns
    ///
    /// Returns the count of crashes within the active window.
    pub fn get_crash_count_in_window(&self, current_time_secs: u64) -> usize {
        let cutoff = current_time_secs.saturating_sub(self.window_secs);
        self.crash_times.iter().filter(|&&t| t > cutoff).count()
    }

    /// Clears the crash history, typically called after successful stable runtime.
    pub fn clear_history(&mut self) {
        self.crash_times.clear();
    }
}

#[cfg(test)]
mod backoff_extended_tests {
    use crate::policy::backoff::{
        ColdStartBudget, HotLoopDetector, calculate_decorrelated_jitter, calculate_full_jitter,
    };
    use std::time::Duration;

    /// Tests that cold start budget correctly tracks restarts within window and enforces limit.
    #[test]
    fn test_cold_start_budget_basic_tracking() {
        let mut budget = ColdStartBudget::new(300, 3, 1000);

        // Within window, under limit
        assert!(!budget.record_restart(1010));
        assert!(!budget.record_restart(1020));
        assert!(!budget.record_restart(1030));

        // Exceeds limit
        assert!(budget.record_restart(1040));
    }

    /// Tests that cold start budget resets after window expiry.
    #[test]
    fn test_cold_start_window_expiry() {
        let mut budget = ColdStartBudget::new(300, 2, 1000);

        // Fill budget within window
        budget.record_restart(1010);
        budget.record_restart(1020);

        // After window expires, budget resets
        assert!(!budget.record_restart(1400)); // Outside 300s window
        assert_eq!(budget.get_restart_count(), 1);
    }

    /// Tests that hot loop detector triggers when crash count reaches threshold in window.
    #[test]
    fn test_hot_loop_detection_basic() {
        let mut detector = HotLoopDetector::new(60, 3);

        detector.record_crash(1000);
        detector.record_crash(1010);
        assert!(!detector.is_hot_loop_detected(1010)); // Only 2 crashes

        detector.record_crash(1020);
        assert!(detector.is_hot_loop_detected(1020)); // 3 crashes in window
    }

    /// Tests that hot loop sliding window correctly expires old crashes.
    #[test]
    fn test_hot_loop_sliding_window() {
        let mut detector = HotLoopDetector::new(60, 3);

        detector.record_crash(1000);
        detector.record_crash(1010);
        detector.record_crash(1020);
        assert!(detector.is_hot_loop_detected(1020));

        // After window slides past first crashes
        assert!(!detector.is_hot_loop_detected(1070)); // Only 1 crash in last 60s
    }

    /// Tests that full jitter calculation respects base delay upper bound.
    #[test]
    fn test_full_jitter_bounds() {
        let delay =
            calculate_full_jitter(Duration::from_millis(100), Duration::from_millis(1000), 42);
        assert!(delay <= Duration::from_millis(100)); // Capped by base
    }

    /// Tests that decorrelated jitter calculation stays within initial and max bounds.
    #[test]
    fn test_decorrelated_jitter_bounds() {
        let delay = calculate_decorrelated_jitter(
            Duration::from_millis(100),
            Duration::from_millis(10),
            Duration::from_millis(1000),
            42,
        );
        assert!(delay >= Duration::from_millis(10)); // At least initial
        assert!(delay <= Duration::from_millis(1000)); // At most max
    }
}
