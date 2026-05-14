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
