//! Heartbeat records and stale health detection.
//!
//! This module owns health timing policy. It only classifies heartbeat freshness
//! and leaves task control to the runtime.

use crate::id::types::ChildId;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime};

/// Health timing policy for a supervised child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthPolicy {
    /// Expected interval between healthy heartbeats.
    pub heartbeat_interval: Duration,
    /// Maximum accepted age for the latest heartbeat.
    pub stale_after: Duration,
}

impl HealthPolicy {
    /// Creates a health policy.
    ///
    /// # Arguments
    ///
    /// - `heartbeat_interval`: Expected interval between heartbeats.
    /// - `stale_after`: Maximum heartbeat age before stale detection.
    ///
    /// # Returns
    ///
    /// Returns a [`HealthPolicy`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// let policy = rust_supervisor::health::heartbeat::HealthPolicy::new(
    ///     Duration::from_secs(1),
    ///     Duration::from_secs(3),
    /// );
    /// assert_eq!(policy.heartbeat_interval, Duration::from_secs(1));
    /// ```
    pub fn new(heartbeat_interval: Duration, stale_after: Duration) -> Self {
        Self {
            heartbeat_interval,
            stale_after,
        }
    }

    /// Tests whether a heartbeat is stale at a monotonic time.
    ///
    /// # Arguments
    ///
    /// - `heartbeat`: Latest heartbeat record.
    /// - `now`: Current monotonic time.
    ///
    /// # Returns
    ///
    /// Returns `true` when `heartbeat` is older than [`HealthPolicy::stale_after`].
    pub fn is_stale(&self, heartbeat: &Heartbeat, now: Instant) -> bool {
        now.duration_since(heartbeat.monotonic_at) > self.stale_after
    }
}

/// Latest health signal emitted by a supervised child.
#[derive(Debug, Clone)]
pub struct Heartbeat {
    /// Child that emitted the heartbeat.
    pub child_id: ChildId,
    /// Monotonic timestamp used for stale detection.
    pub monotonic_at: Instant,
    /// Wall-clock timestamp used for audit and diagnostics.
    pub recorded_at: SystemTime,
    /// Optional low-cardinality health detail.
    pub detail: Option<String>,
}

impl Heartbeat {
    /// Creates a heartbeat at the supplied monotonic time.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child that emitted the heartbeat.
    /// - `monotonic_at`: Monotonic timestamp used for stale detection.
    /// - `detail`: Optional diagnostic detail.
    ///
    /// # Returns
    ///
    /// Returns a [`Heartbeat`] with the current wall-clock timestamp.
    pub fn new(child_id: ChildId, monotonic_at: Instant, detail: Option<String>) -> Self {
        Self {
            child_id,
            monotonic_at,
            recorded_at: SystemTime::now(),
            detail,
        }
    }

    /// Computes the heartbeat age at a monotonic time.
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic time.
    ///
    /// # Returns
    ///
    /// Returns the elapsed duration since this heartbeat.
    pub fn age_at(&self, now: Instant) -> Duration {
        now.duration_since(self.monotonic_at)
    }
}
