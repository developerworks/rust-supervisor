//! Deterministic helpers for supervisor tests.
//!
//! The module provides small reusable fixtures for event collection, paused
//! time, and deterministic jitter.

use crate::event::payload::{SupervisorEvent, What, Where};
use crate::event::time::{CorrelationId, EventSequence, EventSequenceSource, EventTime, When};
use crate::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use crate::runtime::lifecycle::{RuntimeControlPlane, RuntimeExitReport};
use crate::runtime::watchdog::RuntimeWatchdog;
use crate::{control::handle::SupervisorHandle, runtime::message::RuntimeLoopMessage};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

/// Paused time source for deterministic tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PausedTime {
    /// Wall-clock time in nanoseconds since the Unix epoch.
    pub unix_nanos: u128,
    /// Monotonic time in nanoseconds.
    pub monotonic_nanos: u128,
    /// Supervisor uptime in milliseconds.
    pub uptime_ms: u64,
}

impl PausedTime {
    /// Creates a paused time source.
    ///
    /// # Arguments
    ///
    /// - `unix_nanos`: Wall-clock timestamp in nanoseconds.
    /// - `monotonic_nanos`: Monotonic timestamp in nanoseconds.
    /// - `uptime_ms`: Supervisor uptime in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns a [`PausedTime`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let time = rust_supervisor::test_support::factory::PausedTime::new(1, 2, 3);
    /// assert_eq!(time.uptime_ms, 3);
    /// ```
    pub fn new(unix_nanos: u128, monotonic_nanos: u128, uptime_ms: u64) -> Self {
        Self {
            unix_nanos,
            monotonic_nanos,
            uptime_ms,
        }
    }

    /// Creates deterministic event time.
    ///
    /// # Arguments
    ///
    /// - `generation`: Child generation for the event.
    /// - `child_start_count`: Child child_start_count for the event.
    ///
    /// # Returns
    ///
    /// Returns an [`EventTime`] value.
    pub fn event_time(
        &self,
        generation: Generation,
        child_start_count: ChildStartCount,
    ) -> EventTime {
        EventTime::deterministic(
            self.unix_nanos,
            self.monotonic_nanos,
            self.uptime_ms,
            generation,
            child_start_count,
        )
    }
}

/// Deterministic jitter helper for backoff tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicJitter {
    /// Percentage points applied to the base delay.
    pub percent: i64,
}

impl DeterministicJitter {
    /// Creates a deterministic jitter source.
    ///
    /// # Arguments
    ///
    /// - `percent`: Signed percentage applied to the base delay.
    ///
    /// # Returns
    ///
    /// Returns a [`DeterministicJitter`] value.
    pub fn new(percent: i64) -> Self {
        Self { percent }
    }

    /// Applies jitter to a millisecond delay.
    ///
    /// # Arguments
    ///
    /// - `base_ms`: Base delay in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns the adjusted delay in milliseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// let jitter = rust_supervisor::test_support::factory::DeterministicJitter::new(10);
    /// assert_eq!(jitter.apply_ms(100), 110);
    /// ```
    pub fn apply_ms(&self, base_ms: u64) -> u64 {
        let base = i128::from(base_ms);
        let delta = base.saturating_mul(i128::from(self.percent)) / 100;
        base.saturating_add(delta).max(0) as u64
    }
}

/// Collector that stores supervisor events in memory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCollector {
    /// Events collected in receive order.
    pub events: Vec<SupervisorEvent>,
}

impl EventCollector {
    /// Creates an empty collector.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a new [`EventCollector`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes one event into the collector.
    ///
    /// # Arguments
    ///
    /// - `event`: Event to store.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn push(&mut self, event: SupervisorEvent) {
        self.events.push(event);
    }

    /// Returns collected event names.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns event names in receive order.
    pub fn event_names(&self) -> Vec<&'static str> {
        self.events.iter().map(|event| event.what.name()).collect()
    }
}

/// Fixture that builds deterministic lifecycle events.
#[derive(Debug)]
pub struct EventFixture {
    /// Paused time used for every event.
    pub paused_time: PausedTime,
    /// Sequence source used by the fixture.
    pub sequences: EventSequenceSource,
    /// Correlation identifier used by the fixture.
    pub correlation_id: CorrelationId,
    /// Configuration version attached to events.
    pub config_version: u64,
}

impl EventFixture {
    /// Creates an event fixture.
    ///
    /// # Arguments
    ///
    /// - `paused_time`: Time source for deterministic events.
    /// - `config_version`: Configuration version attached to events.
    ///
    /// # Returns
    ///
    /// Returns an [`EventFixture`].
    pub fn new(paused_time: PausedTime, config_version: u64) -> Self {
        Self {
            paused_time,
            sequences: EventSequenceSource::new(),
            correlation_id: CorrelationId::from_uuid(Uuid::nil()),
            config_version,
        }
    }

    /// Builds a deterministic event for a child.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier attached to the event.
    /// - `child_name`: Child name attached to the event.
    /// - `what`: Event payload.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorEvent`].
    pub fn child_event(
        &self,
        child_id: ChildId,
        child_name: impl Into<String>,
        what: What,
    ) -> SupervisorEvent {
        let path = SupervisorPath::root().join(child_id.to_string());
        let location = Where::new(path.clone()).with_child(child_id, child_name);
        SupervisorEvent::new(
            When::new(
                self.paused_time
                    .event_time(Generation::initial(), ChildStartCount::first()),
            ),
            location,
            what,
            self.sequences.next(),
            self.correlation_id,
            self.config_version,
        )
    }

    /// Builds a deterministic event for the root supervisor.
    ///
    /// # Arguments
    ///
    /// - `what`: Event payload.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorEvent`].
    pub fn supervisor_event(&self, what: What) -> SupervisorEvent {
        SupervisorEvent::new(
            When::new(
                self.paused_time
                    .event_time(Generation::initial(), ChildStartCount::first()),
            ),
            Where::new(SupervisorPath::root()),
            what,
            self.sequences.next(),
            self.correlation_id,
            self.config_version,
        )
    }

    /// Builds an event sequence value.
    ///
    /// # Arguments
    ///
    /// - `value`: Sequence value.
    ///
    /// # Returns
    ///
    /// Returns an [`EventSequence`].
    pub fn sequence(value: u64) -> EventSequence {
        EventSequence::new(value)
    }
}

/// Creates a handle whose control loop has failed through a watchdog.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns a [`SupervisorHandle`] whose health report is failed.
pub async fn runtime_control_plane_failed_handle() -> SupervisorHandle {
    let (command_sender, command_receiver) = mpsc::channel::<RuntimeLoopMessage>(1);
    drop(command_receiver);
    let (event_sender, _) = broadcast::channel(16);
    let control_plane = RuntimeControlPlane::new();
    control_plane.mark_alive();
    let join_handle = tokio::spawn(async move {
        panic!("runtime control loop panic fixture");
        #[allow(unreachable_code)]
        RuntimeExitReport::completed("unreachable", "unreachable")
    });
    RuntimeWatchdog::spawn(control_plane.clone(), join_handle, event_sender.clone());
    let handle = SupervisorHandle::new(command_sender, event_sender, control_plane);
    let _report = handle.join().await.expect("failed runtime joins");
    handle
}

/// Creates a backoff policy with deterministic jitter for reproducible tests.
///
/// # Arguments
///
/// - `initial`: Initial backoff delay.
/// - `max`: Maximum backoff delay cap.
/// - `jitter_percent`: Jitter percentage (0-100).
/// - `reset_after`: Duration after which restart counters reset.
/// - `seed`: Fixed RNG seed for deterministic jitter output.
///
/// # Returns
///
/// Returns a [`BackoffPolicy`] configured with deterministic jitter mode.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use rust_supervisor::test_support::factory::deterministic_backoff_policy;
///
/// let policy = deterministic_backoff_policy(
///     Duration::from_millis(10),
///     Duration::from_millis(1000),
///     50,
///     Duration::from_secs(300),
///     42,
/// );
/// // Same seed produces identical delays across test runs
/// let delay1 = policy.delay_for_child_start_count(1);
/// let delay2 = policy.delay_for_child_start_count(1);
/// assert_eq!(delay1, delay2);
/// ```
pub fn deterministic_backoff_policy(
    initial: std::time::Duration,
    max: std::time::Duration,
    jitter_percent: u8,
    reset_after: std::time::Duration,
    seed: u64,
) -> crate::policy::backoff::BackoffPolicy {
    crate::policy::backoff::BackoffPolicy::new(initial, max, jitter_percent, reset_after)
        .with_deterministic_jitter(seed)
}

/// Creates a backoff policy with full jitter for thundering herd prevention tests.
///
/// # Arguments
///
/// - `initial`: Initial backoff delay.
/// - `max`: Maximum backoff delay cap.
/// - `seed`: Fixed RNG seed for deterministic full jitter output.
///
/// # Returns
///
/// Returns a [`BackoffPolicy`] configured with full jitter mode.
pub fn full_jitter_backoff_policy(
    initial: std::time::Duration,
    max: std::time::Duration,
    seed: u64,
) -> crate::policy::backoff::BackoffPolicy {
    let mut policy = crate::policy::backoff::BackoffPolicy::new(
        initial,
        max,
        100,
        std::time::Duration::from_secs(300),
    );
    policy.jitter_mode = crate::policy::backoff::JitterMode::FullJitter { seed };
    policy
}

/// Creates a backoff policy with decorrelated jitter for correlation-breaking tests.
///
/// # Arguments
///
/// - `initial`: Initial backoff delay.
/// - `max`: Maximum backoff delay cap.
/// - `seed`: Fixed RNG seed for deterministic decorrelated jitter output.
///
/// # Returns
///
/// Returns a [`BackoffPolicy`] configured with decorrelated jitter mode.
pub fn decorrelated_jitter_backoff_policy(
    initial: std::time::Duration,
    max: std::time::Duration,
    seed: u64,
) -> crate::policy::backoff::BackoffPolicy {
    let mut policy = crate::policy::backoff::BackoffPolicy::new(
        initial,
        max,
        100,
        std::time::Duration::from_secs(300),
    );
    policy.jitter_mode = crate::policy::backoff::JitterMode::DecorrelatedJitter { seed };
    policy
}
