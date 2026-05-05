//! Event timing primitives for lifecycle diagnostics.
//!
//! This module owns sequence numbers, correlation identifiers, and event time
//! capture. It does not depend on the runtime so tests can create deterministic
//! event timestamps.

use crate::id::types::{Attempt, Generation};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Monotonic event sequence allocated by an event source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EventSequence {
    /// One-based event sequence value.
    pub value: u64,
}

impl EventSequence {
    /// Creates an event sequence from a raw value.
    ///
    /// # Arguments
    ///
    /// - `value`: One-based sequence value assigned by the caller.
    ///
    /// # Returns
    ///
    /// Returns an [`EventSequence`] that preserves the provided value.
    ///
    /// # Examples
    ///
    /// ```
    /// let sequence = rust_supervisor::event::time::EventSequence::new(7);
    /// assert_eq!(sequence.value, 7);
    /// ```
    pub fn new(value: u64) -> Self {
        Self { value }
    }
}

/// Atomic allocator for monotonic event sequences.
#[derive(Debug)]
pub struct EventSequenceSource {
    /// Last sequence value handed to a caller.
    next_value: AtomicU64,
}

impl EventSequenceSource {
    /// Creates a sequence source that starts at one.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a new [`EventSequenceSource`].
    ///
    /// # Examples
    ///
    /// ```
    /// let source = rust_supervisor::event::time::EventSequenceSource::new();
    /// assert_eq!(source.next().value, 1);
    /// assert_eq!(source.next().value, 2);
    /// ```
    pub fn new() -> Self {
        Self {
            next_value: AtomicU64::new(1),
        }
    }

    /// Allocates the next sequence.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the next [`EventSequence`].
    pub fn next(&self) -> EventSequence {
        EventSequence::new(self.next_value.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for EventSequenceSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Identifier that connects related lifecycle facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId {
    /// UUID used by commands, attempts, and derived observability signals.
    pub value: Uuid,
}

impl CorrelationId {
    /// Creates a random correlation identifier.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a new [`CorrelationId`].
    ///
    /// # Examples
    ///
    /// ```
    /// let id = rust_supervisor::event::time::CorrelationId::new();
    /// assert!(!id.value.is_nil());
    /// ```
    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4(),
        }
    }

    /// Creates a correlation identifier from a UUID.
    ///
    /// # Arguments
    ///
    /// - `value`: UUID chosen by the caller.
    ///
    /// # Returns
    ///
    /// Returns a [`CorrelationId`] containing `value`.
    pub fn from_uuid(value: Uuid) -> Self {
        Self { value }
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

/// Time data attached to a lifecycle event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTime {
    /// Wall-clock time as nanoseconds since the Unix epoch.
    pub unix_nanos: u128,
    /// Monotonic time source as nanoseconds supplied by the runtime.
    pub monotonic_nanos: u128,
    /// Supervisor uptime in milliseconds.
    pub supervisor_uptime_ms: u64,
    /// Child generation related to the event.
    pub generation: Generation,
    /// Child attempt related to the event.
    pub attempt: Attempt,
}

impl EventTime {
    /// Captures wall-clock timing and caller-supplied monotonic timing.
    ///
    /// # Arguments
    ///
    /// - `monotonic_nanos`: Runtime monotonic clock value in nanoseconds.
    /// - `supervisor_uptime_ms`: Supervisor uptime in milliseconds.
    /// - `generation`: Child generation for this lifecycle fact.
    /// - `attempt`: Child attempt for this lifecycle fact.
    ///
    /// # Returns
    ///
    /// Returns an [`EventTime`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let time = rust_supervisor::event::time::EventTime::from_parts(
    ///     10,
    ///     2,
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::Attempt::first(),
    /// );
    /// assert_eq!(time.monotonic_nanos, 10);
    /// ```
    pub fn from_parts(
        monotonic_nanos: u128,
        supervisor_uptime_ms: u64,
        generation: Generation,
        attempt: Attempt,
    ) -> Self {
        Self {
            unix_nanos: system_time_nanos(SystemTime::now()),
            monotonic_nanos,
            supervisor_uptime_ms,
            generation,
            attempt,
        }
    }

    /// Creates deterministic event time for tests and replay.
    ///
    /// # Arguments
    ///
    /// - `unix_nanos`: Wall-clock timestamp in nanoseconds.
    /// - `monotonic_nanos`: Monotonic timestamp in nanoseconds.
    /// - `supervisor_uptime_ms`: Supervisor uptime in milliseconds.
    /// - `generation`: Child generation for this event.
    /// - `attempt`: Child attempt for this event.
    ///
    /// # Returns
    ///
    /// Returns an [`EventTime`] value with exact caller-provided fields.
    pub fn deterministic(
        unix_nanos: u128,
        monotonic_nanos: u128,
        supervisor_uptime_ms: u64,
        generation: Generation,
        attempt: Attempt,
    ) -> Self {
        Self {
            unix_nanos,
            monotonic_nanos,
            supervisor_uptime_ms,
            generation,
            attempt,
        }
    }
}

/// Wrapper that answers when a lifecycle fact happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct When {
    /// Detailed timing data for the event.
    pub time: EventTime,
}

impl When {
    /// Creates a `When` value from event time.
    ///
    /// # Arguments
    ///
    /// - `time`: Event time captured by the caller.
    ///
    /// # Returns
    ///
    /// Returns a [`When`] wrapper.
    ///
    /// # Examples
    ///
    /// ```
    /// let when = rust_supervisor::event::time::When::new(
    ///     rust_supervisor::event::time::EventTime::deterministic(
    ///         1,
    ///         1,
    ///         0,
    ///         rust_supervisor::id::types::Generation::initial(),
    ///         rust_supervisor::id::types::Attempt::first(),
    ///     ),
    /// );
    /// assert_eq!(when.time.unix_nanos, 1);
    /// ```
    pub fn new(time: EventTime) -> Self {
        Self { time }
    }
}

/// Converts system time into nanoseconds with a zero fallback before epoch.
///
/// # Arguments
///
/// - `time`: Wall-clock value that should be converted.
///
/// # Returns
///
/// Returns nanoseconds since Unix epoch.
fn system_time_nanos(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos()
}
