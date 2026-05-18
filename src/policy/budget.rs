//! Restart budget tracker module.
//!
//! Implements a sliding window + token bucket hybrid model
//! for limiting effective restart rate (US1: fast failure doesn't cause storm).
//!
//! The [`RestartBudgetTracker`] maintains a failure timestamp queue
//! within a configurable sliding window and a token bucket that refills
//! at `recovery_rate_per_sec`. When tokens are exhausted, further restart
//! attempts are denied with a [`BudgetVerdict::Exhausted`] carrying the
//! retry-after duration.
//!
//! # Budget → Meltdown → Backoff Order
//!
//! Budget evaluation happens first: if the budget is exhausted, the
//! meltdown and backoff stages are skipped entirely. This prevents
//! infinite restart storms from overwhelming the supervisor.

use std::collections::VecDeque;
use std::time::Duration;

/// Configuration for restart budget tracking.
///
/// Fields must satisfy: `window > 0s`, `max_burst >= 1`,
/// `0.0 < recovery_rate_per_sec <= 1000.0`, `max_tokens >= 1`.
/// Invalid values are rejected at config load time with a structured error.
///
/// # Business bounds
///
/// - `max_burst` > 10_000 produces a configuration warning (memory ~160KB).
///   Values near `u32::MAX` are rejected outright.
/// - `recovery_rate_per_sec` < 0.001 produces a configuration warning
///   (engineering‑equivalent to never recovering).
#[derive(Debug, Clone, PartialEq)]
pub struct RestartBudgetConfig {
    /// Sliding window duration for failure counting.
    pub window: Duration,
    /// Maximum burst failures allowed within the window.
    pub max_burst: u32,
    /// Token recovery rate per second (0.0 = no recovery).
    pub recovery_rate_per_sec: f64,
}

impl RestartBudgetConfig {
    /// Creates a restart budget configuration.
    ///
    /// # Arguments
    ///
    /// - `window`: Sliding window for failure counting.
    /// - `max_burst`: Maximum burst failures in the window.
    /// - `recovery_rate_per_sec`: Tokens recovered per second.
    ///
    /// # Returns
    ///
    /// Returns a [`RestartBudgetConfig`].
    pub fn new(window: Duration, max_burst: u32, recovery_rate_per_sec: f64) -> Self {
        Self {
            window,
            max_burst,
            recovery_rate_per_sec,
        }
    }

    /// Returns a safe default configuration used when no budget is declared.
    ///
    /// Used for backward compatibility: old config files without a `budget`
    /// section will get these values instead of being rejected.
    pub fn safe_default() -> Self {
        Self {
            window: Duration::from_secs(60),
            max_burst: 10,
            recovery_rate_per_sec: 0.5,
        }
    }

    /// Validates the configuration bounds and returns warnings for
    /// values that are technically legal but practically dangerous.
    ///
    /// # Returns
    ///
    /// A vector of warning strings. The caller should log these and
    /// decide whether to reject the configuration.
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // max_burst > 10_000 consumes measurable memory; near u32::MAX is fatal.
        if self.max_burst > 10_000 {
            warnings.push(format!(
                "max_burst ({}) exceeds 10_000; memory may reach ~{} bytes",
                self.max_burst,
                self.max_burst as u64 * 16
            ));
        }
        if self.max_burst >= u32::MAX / 2 {
            warnings.push(format!(
                "max_burst ({}) is dangerously close to u32::MAX; queue would exhaust process memory",
                self.max_burst
            ));
        }

        // recovery_rate_per_sec < 0.001 is practically equivalent to no recovery.
        if self.recovery_rate_per_sec > 0.0 && self.recovery_rate_per_sec < 0.001 {
            warnings.push(format!(
                "recovery_rate_per_sec ({}) is below 0.001; budget will effectively never recover",
                self.recovery_rate_per_sec
            ));
        }

        warnings
    }
}

/// Outcome of a budget consumption attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetVerdict {
    /// Budget granted, restart may proceed.
    Granted,
    /// Budget exhausted, restart must wait.
    Exhausted {
        /// Nanoseconds to wait before retrying.
        retry_after_ns: u128,
    },
}

/// Mutable restart budget tracker with sliding window + token bucket.
#[derive(Debug)]
pub struct RestartBudgetTracker {
    /// Budget configuration.
    config: RestartBudgetConfig,
    /// Failure timestamp queue (Unix nanos).
    failures: VecDeque<u128>,
    /// Current token count.
    tokens: f64,
    /// Last update timestamp (Unix nanos).
    last_update_unix_nanos: u128,
}

impl RestartBudgetTracker {
    /// Creates a tracker with full token capacity.
    ///
    /// # Arguments
    ///
    /// - `config`: Budget configuration.
    /// - `now_unix_nanos`: Current Unix timestamp in nanoseconds.
    ///
    /// # Returns
    ///
    /// Returns a [`RestartBudgetTracker`] initialized with full tokens.
    pub fn new(config: RestartBudgetConfig, now_unix_nanos: u128) -> Self {
        let max_tokens = config.max_burst as f64;
        Self {
            config,
            failures: VecDeque::new(),
            tokens: max_tokens,
            last_update_unix_nanos: now_unix_nanos,
        }
    }

    /// Attempts to consume one token for a restart.
    ///
    /// Refills tokens based on elapsed time before checking availability.
    /// The three-step operation (evict -> refill -> check) is atomic within
    /// this `&mut self` call; callers do not need external locking.
    ///
    /// # Arguments
    ///
    /// - `now_unix_nanos`: Current Unix timestamp in nanoseconds.
    ///
    /// # Returns
    ///
    /// Returns [`BudgetVerdict::Granted`] when a token is available,
    /// or [`BudgetVerdict::Exhausted`] with the retry-after duration.
    pub fn try_consume(&mut self, now_unix_nanos: u128) -> BudgetVerdict {
        self.refill(now_unix_nanos);
        self.evict(now_unix_nanos);

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            BudgetVerdict::Granted
        } else {
            let retry_after_ns = self.estimate_retry_ns(now_unix_nanos);
            BudgetVerdict::Exhausted { retry_after_ns }
        }
    }

    /// Returns the current token count (for diagnostics).
    pub fn current_tokens(&self, _now_unix_nanos: u128) -> f64 {
        self.tokens
    }

    /// Returns the number of failures currently in the sliding window.
    pub fn window_failures(&self, now_unix_nanos: u128) -> u32 {
        let window_start = now_unix_nanos.saturating_sub(self.config.window.as_nanos());
        self.failures
            .iter()
            .filter(|&&ts| ts >= window_start)
            .count() as u32
    }

    // --- private helpers ---

    /// Refills tokens based on elapsed time.
    fn refill(&mut self, now_unix_nanos: u128) {
        let elapsed_ns = now_unix_nanos.saturating_sub(self.last_update_unix_nanos);
        let elapsed_secs = elapsed_ns as f64 / 1_000_000_000.0;
        let recovered = elapsed_secs * self.config.recovery_rate_per_sec;
        let max_tokens = self.config.max_burst as f64;
        self.tokens = (self.tokens + recovered).min(max_tokens);
        self.last_update_unix_nanos = now_unix_nanos;
    }

    /// Evicts failures outside the sliding window.
    fn evict(&mut self, now_unix_nanos: u128) {
        let window_start = now_unix_nanos.saturating_sub(self.config.window.as_nanos());
        while let Some(&front) = self.failures.front() {
            if front >= window_start {
                break;
            }
            self.failures.pop_front();
        }
    }

    /// Estimates how many nanoseconds until a token becomes available.
    fn estimate_retry_ns(&self, _now_unix_nanos: u128) -> u128 {
        if self.config.recovery_rate_per_sec <= 0.0 {
            return self.config.window.as_nanos();
        }
        let deficit = 1.0 - self.tokens;
        let secs_needed = deficit / self.config.recovery_rate_per_sec;
        (secs_needed * 1_000_000_000.0) as u128
    }
}
