//! Observability fan-out pipeline.
//!
//! The pipeline records one lifecycle fact across event storage, structured
//! logs, tracing metadata, metrics, audit data, and a test recorder.

use crate::event::payload::{SupervisorEvent, What};
use crate::journal::ring::EventJournal;
use crate::observe::metrics::{MetricSample, MetricsFacade};
use crate::observe::tracing::{AttemptSpan, TracingEvent};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Structured log entry derived from a supervisor event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredLogRecord {
    /// Event sequence.
    pub sequence: u64,
    /// Correlation identifier shared by all signals.
    pub correlation_id: String,
    /// Payload name.
    pub event_name: String,
    /// Configuration version attached to the event.
    pub config_version: u64,
}

/// Audit record derived from command events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Event sequence.
    pub sequence: u64,
    /// Command identifier.
    pub command_id: String,
    /// Requesting actor.
    pub requested_by: String,
    /// Command result.
    pub result: String,
    /// Audit reason.
    pub reason: String,
    /// Runtime or command phase.
    pub phase: String,
}

/// Test recorder for observability assertions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TestRecorder {
    /// Events seen by the recorder.
    pub events: Vec<SupervisorEvent>,
    /// Structured log records seen by the recorder.
    pub logs: Vec<StructuredLogRecord>,
    /// Tracing spans seen by the recorder.
    pub spans: Vec<AttemptSpan>,
    /// Tracing events seen by the recorder.
    pub tracing_events: Vec<TracingEvent>,
    /// Metric samples seen by the recorder.
    pub metrics: Vec<MetricSample>,
    /// Audit records seen by the recorder.
    pub audits: Vec<AuditRecord>,
    /// Total subscriber lag observed by the recorder.
    pub subscriber_lag: u64,
}

impl TestRecorder {
    /// Creates an empty recorder.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a new [`TestRecorder`].
    ///
    /// # Examples
    ///
    /// ```
    /// let recorder = rust_supervisor::observe::pipeline::TestRecorder::new();
    /// assert!(recorder.events.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Records subscriber lag.
    ///
    /// # Arguments
    ///
    /// - `missed`: Number of missed events.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn record_lag(&mut self, missed: u64) {
        self.subscriber_lag = self.subscriber_lag.saturating_add(missed);
    }
}

/// Observability fan-out pipeline.
#[derive(Debug, Clone)]
pub struct ObservabilityPipeline {
    /// Event journal that retains recent lifecycle facts.
    pub journal: EventJournal,
    /// Metrics facade used to derive metric samples.
    pub metrics: MetricsFacade,
    /// Recorder that tests can inspect.
    pub test_recorder: TestRecorder,
    /// Subscriber queues used by simple fan-out.
    subscribers: Vec<VecDeque<SupervisorEvent>>,
    /// Maximum queued events per subscriber.
    subscriber_capacity: usize,
}

impl ObservabilityPipeline {
    /// Creates an observability pipeline.
    ///
    /// # Arguments
    ///
    /// - `journal_capacity`: Maximum event journal capacity.
    /// - `subscriber_capacity`: Maximum queued events per subscriber.
    ///
    /// # Returns
    ///
    /// Returns an [`ObservabilityPipeline`].
    ///
    /// # Examples
    ///
    /// ```
    /// let pipeline = rust_supervisor::observe::pipeline::ObservabilityPipeline::new(8, 4);
    /// assert_eq!(pipeline.journal.capacity, 8);
    /// ```
    pub fn new(journal_capacity: usize, subscriber_capacity: usize) -> Self {
        Self {
            journal: EventJournal::new(journal_capacity),
            metrics: MetricsFacade::new(),
            test_recorder: TestRecorder::new(),
            subscribers: Vec::new(),
            subscriber_capacity,
        }
    }

    /// Adds one in-memory subscriber queue.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the subscriber index.
    pub fn add_subscriber(&mut self) -> usize {
        self.subscribers.push(VecDeque::new());
        self.subscribers.len().saturating_sub(1)
    }

    /// Emits one event through all observability sinks.
    ///
    /// # Arguments
    ///
    /// - `event`: Lifecycle event to emit.
    ///
    /// # Returns
    ///
    /// Returns the number of lagged subscriber events.
    pub fn emit(&mut self, event: SupervisorEvent) -> u64 {
        let metrics = self.metrics.samples_for_event(&event);
        let log = structured_log(&event);
        let span = AttemptSpan::from_event(&event);
        let tracing_event = TracingEvent::from_event(&event);
        let audit = audit_record(&event);
        let lagged = self.fan_out(event.clone());
        self.journal.push(event.clone());
        self.test_recorder.events.push(event);
        self.test_recorder.logs.push(log);
        self.test_recorder.spans.push(span);
        self.test_recorder.tracing_events.push(tracing_event);
        self.test_recorder.metrics.extend(metrics);
        self.test_recorder.audits.extend(audit);
        self.test_recorder.record_lag(lagged);
        lagged
    }

    /// Drains queued events for a subscriber.
    ///
    /// # Arguments
    ///
    /// - `subscriber_index`: Index returned by [`ObservabilityPipeline::add_subscriber`].
    ///
    /// # Returns
    ///
    /// Returns queued events in oldest-to-newest order.
    pub fn drain_subscriber(&mut self, subscriber_index: usize) -> Vec<SupervisorEvent> {
        self.subscribers
            .get_mut(subscriber_index)
            .map(|queue| queue.drain(..).collect())
            .unwrap_or_default()
    }

    /// Sends an event to every subscriber queue.
    ///
    /// # Arguments
    ///
    /// - `event`: Event that should be queued.
    ///
    /// # Returns
    ///
    /// Returns how many events were dropped because queues were full.
    fn fan_out(&mut self, event: SupervisorEvent) -> u64 {
        let mut lagged = 0_u64;
        for subscriber in &mut self.subscribers {
            if subscriber.len() == self.subscriber_capacity {
                subscriber.pop_front();
                lagged = lagged.saturating_add(1);
            }
            subscriber.push_back(event.clone());
        }
        lagged
    }
}

/// Builds a structured log record from an event.
///
/// # Arguments
///
/// - `event`: Lifecycle event to translate.
///
/// # Returns
///
/// Returns a structured log record.
fn structured_log(event: &SupervisorEvent) -> StructuredLogRecord {
    StructuredLogRecord {
        sequence: event.sequence.value,
        correlation_id: event.correlation_id.value.to_string(),
        event_name: event.what.name().to_owned(),
        config_version: event.config_version,
    }
}

/// Extracts audit data from command events.
///
/// # Arguments
///
/// - `event`: Lifecycle event to inspect.
///
/// # Returns
///
/// Returns an audit record for command events.
fn audit_record(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::CommandAccepted { audit } | What::CommandCompleted { audit } => Some(AuditRecord {
            sequence: event.sequence.value,
            command_id: audit.command_id.clone(),
            requested_by: audit.requested_by.clone(),
            result: audit.result.clone(),
            reason: audit.reason.clone(),
            phase: "control_command".to_owned(),
        }),
        What::RuntimeControlLoopShutdownRequested {
            command_id,
            requested_by,
            reason,
        } => Some(AuditRecord {
            sequence: event.sequence.value,
            command_id: command_id.clone(),
            requested_by: requested_by.clone(),
            result: "accepted".to_owned(),
            reason: reason.clone(),
            phase: "shutdown".to_owned(),
        }),
        What::RuntimeControlLoopJoinCompleted {
            command_id,
            requested_by,
            state,
            phase,
            reason,
        } => Some(AuditRecord {
            sequence: event.sequence.value,
            command_id: command_id.clone(),
            requested_by: requested_by.clone(),
            result: state.clone(),
            reason: reason.clone(),
            phase: phase.clone(),
        }),
        What::RuntimeControlLoopFailed { phase, reason, .. } => Some(AuditRecord {
            sequence: event.sequence.value,
            command_id: "runtime-control-loop".to_owned(),
            requested_by: "runtime".to_owned(),
            result: "failed".to_owned(),
            reason: reason.clone(),
            phase: phase.clone(),
        }),
        _ => None,
    }
}
