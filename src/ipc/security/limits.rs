//! Request size limit (C5) and rate limit (C6).
//!
//! C5: Rejects requests exceeding `max_bytes` before JSON deserialization.
//! C6: Token bucket rate limiter, per connection, with configurable
//!     refill rate and burst capacity.

use crate::config::ipc_security::{RateLimitConfig, RequestSizeLimitConfig};
use crate::dashboard::error::DashboardError;
use std::time::Instant;

// ---------------------------------------------------------------------------
// C5: Request size limit
// ---------------------------------------------------------------------------

/// Checks request body size against the configured limit (C5).
///
/// # Arguments
///
/// - `actual_bytes`: Raw byte length of the request body.
/// - `config`: Size limit configuration.
///
/// # Returns
///
/// Returns `Ok(())` when within limit, or `Err(DashboardError)` with
/// code `request_too_large`.
pub fn check_request_size(
    actual_bytes: usize,
    config: &RequestSizeLimitConfig,
) -> Result<(), DashboardError> {
    if !config.enabled {
        return Ok(());
    }
    if actual_bytes > config.max_bytes {
        return Err(DashboardError::request_too_large(
            actual_bytes,
            config.max_bytes,
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// C6: Token bucket rate limiter
// ---------------------------------------------------------------------------

/// Token bucket rate limiter.
///
/// Maintains a token pool that refills at a constant rate and depletes by
/// one token per request. Burst capacity limits the maximum token
/// accumulation.
pub struct TokenBucket {
    /// Current token count.
    tokens: f64,
    /// Maximum token capacity.
    max_tokens: f64,
    /// Token refill rate in tokens per second.
    refill_rate: f64,
    /// Timestamp of last refill.
    last_refill: Instant,
}

impl TokenBucket {
    /// Creates a new token bucket.
    ///
    /// # Arguments
    ///
    /// - `refill_rate`: Tokens added per second.
    /// - `burst_capacity`: Maximum tokens the bucket can hold.
    ///
    /// # Returns
    ///
    /// Returns a fully filled [`TokenBucket`].
    pub fn new(refill_rate: f64, burst_capacity: u32) -> Self {
        let max_tokens = burst_capacity as f64;
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Creates a token bucket from rate limit configuration.
    ///
    /// # Arguments
    ///
    /// - `config`: Rate limit configuration.
    ///
    /// # Returns
    ///
    /// Returns a configured [`TokenBucket`].
    pub fn from_config(config: &RateLimitConfig) -> Self {
        Self::new(config.refill_rate, config.burst_capacity)
    }

    /// Refills tokens based on elapsed time since last refill.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Attempts to consume one token.
    ///
    /// # Returns
    ///
    /// Returns `true` if a token was available and consumed, `false` if
    /// the bucket is empty.
    pub fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Checks the rate limit and returns an error if exceeded (C6).
    ///
    /// # Arguments
    ///
    /// - `config`: Rate limit configuration (for enabled check).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when allowed, or `Err(DashboardError)` with
    /// code `rate_limit_exceeded`.
    pub fn check_rate_limit(&mut self, config: &RateLimitConfig) -> Result<(), DashboardError> {
        if !config.enabled {
            return Ok(());
        }
        if !self.try_consume() {
            return Err(DashboardError::rate_limit_exceeded());
        }
        Ok(())
    }
}
