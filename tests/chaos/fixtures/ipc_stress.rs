//! IPC stress test fixtures.
//!
//! Provides `FixtureIpcStress` for generating concurrent IPC connections,
//! `RateLimiter` for connection rate control, and `ClientClassification`
//! for distinguishing legitimate vs. junk clients.
//!
//! NOTE: This file implements the base connection generator (T008).
//! RateLimiter and ClientClassification are extended in T029 (US3).

use std::time::{Duration, Instant};


/// Configuration for IPC stress generation.
#[derive(Debug, Clone)]
pub struct FixtureIpcStress {
    /// Number of concurrent client connections.
    #[allow(dead_code)]
    pub concurrent_clients: u32,
    /// Whether to send legitimate or junk payloads.
    pub payload_mode: PayloadMode,
}

/// Payload type for stress connections.
#[derive(Debug, Clone, Copy)]
pub enum PayloadMode {
    /// Send well-formed IPC handshake payloads.
    Legitimate,
    /// Send random/junk payloads.
    Junk,
}

impl Default for FixtureIpcStress {
    fn default() -> Self {
        Self {
            concurrent_clients: 1000,
            payload_mode: PayloadMode::Junk,
        }
    }
}

impl FixtureIpcStress {
    /// Creates a new IPC stress fixture.
    pub fn new(concurrent_clients: u32) -> Self {
        Self {
            concurrent_clients,
            payload_mode: PayloadMode::Junk,
        }
    }

    /// Sets the number of concurrent clients.
    #[allow(dead_code)]
    pub fn with_concurrent_clients(mut self, n: u32) -> Self {
        self.concurrent_clients = n;
        self
    }

    /// Uses legitimate payloads.
    pub fn with_legitimate_payload(mut self) -> Self {
        self.payload_mode = PayloadMode::Legitimate;
        self
    }

    /// Uses junk payloads.
    pub fn with_junk_payload(mut self) -> Self {
        self.payload_mode = PayloadMode::Junk;
        self
    }

    /// Generates a payload based on the current mode.
    pub fn generate_payload(&self) -> Vec<u8> {
        match self.payload_mode {
            PayloadMode::Legitimate => {
                r#"{"target_id":"dashboard","version":"1.0"}"#.into()
            }
            PayloadMode::Junk => {
                // Random-looking junk that is not valid JSON or missing target_id.
                let junk: String = (0..64).map(|_| (rand::random::<u8>() % 95 + 32) as char).collect();
                junk.into_bytes()
            }
        }
    }
}

/// Fixed-window + token bucket rate limiter.
///
/// Extended in T029 (US3) with full `try_acquire()` implementation.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RateLimiter {
    /// Window duration in seconds.
    pub window_duration: Duration,
    /// Token bucket capacity.
    pub token_capacity: u32,
    /// Token refill rate per second.
    pub refill_rate: f64,
    /// Current available tokens.
    pub tokens: f64,
    /// Last refill timestamp.
    pub last_refill: Instant,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            window_duration: Duration::from_secs(1),
            token_capacity: 100,
            refill_rate: 50.0,
            tokens: 100.0,
            last_refill: Instant::now(),
        }
    }
}

impl RateLimiter {
    /// Creates a new rate limiter.
    pub fn new(token_capacity: u32, refill_rate: f64) -> Self {
        Self {
            window_duration: Duration::from_secs(1),
            token_capacity,
            refill_rate,
            tokens: token_capacity as f64,
            last_refill: Instant::now(),
        }
    }

    /// Refills tokens based on elapsed time.
    pub fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_rate)
                .min(self.token_capacity as f64);
            self.last_refill = Instant::now();
        }
    }

    /// Tries to acquire a token. Returns true if acquired.
    ///
    /// Full implementation with `ResourceExhausted` error support
    /// is added in T029 (US3).
    pub fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Client classification: legitimate vs junk.
///
/// Extended in T029 (US3) with full payload validation.
#[derive(Debug, Clone)]
pub struct ClientClassification {
    /// Raw payload bytes.
    pub payload: Vec<u8>,
}

impl ClientClassification {
    /// Creates a new classifier from raw payload bytes.
    pub fn new(payload: Vec<u8>) -> Self {
        Self { payload }
    }

    /// Returns true if the payload is a legitimate IPC handshake.
    ///
    /// A legitimate payload must be valid JSON containing a `target_id`
    /// field with a string value.
    pub fn is_legitimate(&self) -> bool {
        // Attempt to parse as JSON and check for target_id field.
        let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&self.payload) else {
            return false;
        };
        match parsed.get("target_id") {
            Some(serde_json::Value::String(_)) => true,
            _ => false,
        }
    }
}
