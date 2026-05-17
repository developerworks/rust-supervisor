//! Replay protection (C4).
//!
//! Maintains a sliding window of recently-seen request identifiers.
//! A request_id appearing twice within the configured window (size + TTL)
//! is rejected as a replay attack.

use crate::config::ipc_security::ReplayProtectionConfig;
use crate::dashboard::error::DashboardError;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Sliding window that tracks seen request_ids with expiry.
pub struct ReplayWindow {
    /// Map from request_id to insertion timestamp.
    entries: HashMap<String, Instant>,
    /// Maximum number of entries before evicting oldest.
    max_size: usize,
    /// Time-to-live for each entry.
    ttl: Duration,
}

impl ReplayWindow {
    /// Creates a new replay window.
    ///
    /// # Arguments
    ///
    /// - `max_size`: Maximum entries (oldest evicted when full).
    /// - `ttl`: Entry time-to-live.
    ///
    /// # Returns
    ///
    /// Returns an empty [`ReplayWindow`].
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            entries: HashMap::with_capacity(max_size.min(64)),
            max_size,
            ttl,
        }
    }

    /// Creates a replay window from configuration.
    ///
    /// # Arguments
    ///
    /// - `config`: Replay protection configuration.
    ///
    /// # Returns
    ///
    /// Returns a configured [`ReplayWindow`].
    pub fn from_config(config: &ReplayProtectionConfig) -> Self {
        Self::new(config.window_size, Duration::from_secs(config.ttl_seconds))
    }

    /// Checks whether a request_id is a replay and records it if not.
    ///
    /// # Arguments
    ///
    /// - `request_id`: The request identifier to check.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` for a first submission, or `Err(DashboardError)`
    /// with code `replay_detected` for a replay.
    pub fn check_and_record(&mut self, request_id: &str) -> Result<(), DashboardError> {
        self.purge_expired();

        if self.entries.contains_key(request_id) {
            return Err(DashboardError::replay_detected(request_id));
        }

        // Evict oldest if at capacity
        if self.entries.len() >= self.max_size
            && let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, t)| **t)
                .map(|(k, _)| k.clone())
        {
            self.entries.remove(&oldest_key);
        }

        self.entries.insert(request_id.to_string(), Instant::now());
        Ok(())
    }

    /// Returns true if request_id is already in the window (replay).
    ///
    /// Does not record the entry — use [`check_and_record`] for atomic
    /// check-and-record.
    pub fn is_replay(&self, request_id: &str) -> bool {
        self.entries.contains_key(request_id)
    }

    /// Records a request_id unconditionally (for idempotency cache use).
    pub fn record(&mut self, request_id: String) {
        if self.entries.len() >= self.max_size
            && let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, t)| **t)
                .map(|(k, _)| k.clone())
        {
            self.entries.remove(&oldest_key);
        }
        self.entries.insert(request_id, Instant::now());
    }

    /// Purges expired entries.
    pub fn purge_expired(&mut self) {
        let now = Instant::now();
        self.entries
            .retain(|_, inserted| now.duration_since(*inserted) < self.ttl);
    }

    /// Returns the current number of entries in the window.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the window is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
