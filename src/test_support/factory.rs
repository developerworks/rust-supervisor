//! Deterministic helpers for supervisor tests.
//!
//! The module provides small reusable fixtures for event collection, paused
//! time, fake task outcomes, and deterministic jitter.

use crate::error::types::TaskFailure;
use crate::event::payload::{SupervisorEvent, What, Where};
use crate::event::time::{CorrelationId, EventSequence, EventSequenceSource, EventTime, When};
use crate::id::types::{Attempt, ChildId, Generation, SupervisorPath};
use serde::{Deserialize, Serialize};
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
    /// - `attempt`: Child attempt for the event.
    ///
    /// # Returns
    ///
    /// Returns an [`EventTime`] value.
    pub fn event_time(&self, generation: Generation, attempt: Attempt) -> EventTime {
        EventTime::deterministic(
            self.unix_nanos,
            self.monotonic_nanos,
            self.uptime_ms,
            generation,
            attempt,
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

/// Fake task outcome for tests that do not need a real runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FakeTaskOutcome {
    /// Task completes successfully.
    Complete,
    /// Task is cancelled.
    Cancel,
    /// Task returns a typed failure.
    Fail {
        /// Failure payload returned by the task.
        failure: TaskFailure,
    },
}

/// Fake task factory that returns deterministic outcomes in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FakeTaskFactory {
    /// Ordered outcomes returned by attempts.
    pub outcomes: Vec<FakeTaskOutcome>,
    /// Next outcome index.
    pub next_index: usize,
}

impl FakeTaskFactory {
    /// Creates a fake task factory.
    ///
    /// # Arguments
    ///
    /// - `outcomes`: Ordered outcomes returned by attempts.
    ///
    /// # Returns
    ///
    /// Returns a [`FakeTaskFactory`].
    ///
    /// # Examples
    ///
    /// ```
    /// let mut factory = rust_supervisor::test_support::factory::FakeTaskFactory::new(vec![
    ///     rust_supervisor::test_support::factory::FakeTaskOutcome::Complete,
    /// ]);
    /// assert!(matches!(
    ///     factory.next_outcome(),
    ///     rust_supervisor::test_support::factory::FakeTaskOutcome::Complete
    /// ));
    /// ```
    pub fn new(outcomes: Vec<FakeTaskOutcome>) -> Self {
        Self {
            outcomes,
            next_index: 0,
        }
    }

    /// Returns the next deterministic outcome.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the next [`FakeTaskOutcome`], or `Complete` after configured
    /// outcomes are exhausted.
    pub fn next_outcome(&mut self) -> FakeTaskOutcome {
        let outcome = self
            .outcomes
            .get(self.next_index)
            .cloned()
            .unwrap_or(FakeTaskOutcome::Complete);
        self.next_index = self.next_index.saturating_add(1);
        outcome
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
                    .event_time(Generation::initial(), Attempt::first()),
            ),
            location,
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
