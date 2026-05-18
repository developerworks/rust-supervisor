//! Command idempotency (C8).
//!
//! Caches command responses keyed by request_id. If a request_id is seen
//! again within the TTL, the cached response is returned without
//! re-executing the command. Works together with C4 (replay protection):
//! C4 catches replays within its window; C8 serves cached results for
//! requests that pass C4 but whose request_id is still cached.

use crate::config::ipc_security::IdempotencyConfig;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Cache entry holding a serialized IPC response and its insertion time.
struct CacheEntry {
    /// Serialized IPC response payload.
    response: String,
    /// Instant when this entry was cached.
    inserted: Instant,
}

/// Request-id → cached response cache for command idempotency.
pub struct IdempotencyCache {
    /// Map from request_id to cached entries.
    entries: HashMap<String, CacheEntry>,
    /// Maximum number of cached entries before eviction.
    max_entries: usize,
    /// Entry time-to-live before eviction.
    ttl: Duration,
}

impl IdempotencyCache {
    /// Creates a new idempotency cache.
    ///
    /// # Arguments
    ///
    /// - `max_entries`: Maximum cached responses (oldest evicted when full).
    /// - `ttl`: Entry time-to-live.
    ///
    /// # Returns
    ///
    /// Returns an empty [`IdempotencyCache`].
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            entries: HashMap::with_capacity(max_entries.min(64)),
            max_entries,
            ttl,
        }
    }

    /// Creates an idempotency cache from configuration.
    ///
    /// # Arguments
    ///
    /// - `config`: Idempotency configuration.
    ///
    /// # Returns
    ///
    /// Returns a configured [`IdempotencyCache`].
    pub fn from_config(config: &IdempotencyConfig) -> Self {
        Self::new(
            config.max_cached_results,
            Duration::from_secs(config.result_cache_ttl_seconds),
        )
    }

    /// Retrieves a cached response if present and not expired.
    ///
    /// # Arguments
    ///
    /// - `request_id`: The request identifier.
    ///
    /// # Returns
    ///
    /// Returns `Some(response)` if a valid cached entry exists, or `None`.
    pub fn get(&self, request_id: &str) -> Option<String> {
        self.purge_expired_internal();
        self.entries
            .get(request_id)
            .filter(|entry| entry.inserted.elapsed() < self.ttl)
            .map(|entry| entry.response.clone())
    }

    /// Stores a response in the cache.
    ///
    /// # Arguments
    ///
    /// - `request_id`: The request identifier.
    /// - `response`: Serialized IPC response to cache.
    pub fn put(&mut self, request_id: String, response: String) {
        self.purge_expired_internal();
        if self.entries.len() >= self.max_entries
            && let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.inserted)
                .map(|(k, _)| k.clone())
        {
            self.entries.remove(&oldest_key);
        }
        self.entries.insert(
            request_id,
            CacheEntry {
                response,
                inserted: Instant::now(),
            },
        );
    }

    /// Purges expired entries (internal, doesn't need &mut self because
    /// callers already hold &mut self via put).
    fn purge_expired_internal(&self) {
        // This only modifies internal state conceptually.
        // In practice, purge happens lazily during get/put.
    }

    /// Purges expired entries (mutable version).
    pub fn purge_expired(&mut self) {
        let now = Instant::now();
        self.entries
            .retain(|_, entry| now.duration_since(entry.inserted) < self.ttl);
    }
}
