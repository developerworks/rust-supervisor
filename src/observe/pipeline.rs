//! Observability fan-out pipeline.
//!
//! The pipeline records one lifecycle fact across event storage, structured
//! logs, tracing metadata, metrics, audit data, and a test recorder.

use crate::event::payload::{SupervisorEvent, What};
use crate::journal::ring::EventJournal;
use crate::observe::metrics::{MetricSample, MetricsFacade};
use crate::observe::tracing::{ChildStartCountSpan, TracingEvent};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

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
    /// Child identifier when the fact belongs to one child.
    pub child_id: Option<String>,
    /// Additional bounded context for shutdown audit facts.
    pub context: BTreeMap<String, String>,
}

/// Test recorder for observability assertions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TestRecorder {
    /// Events seen by the recorder.
    pub events: Vec<SupervisorEvent>,
    /// Structured log records seen by the recorder.
    pub logs: Vec<StructuredLogRecord>,
    /// Tracing spans seen by the recorder.
    pub spans: Vec<ChildStartCountSpan>,
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
        let span = ChildStartCountSpan::from_event(&event);
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
    audit_record_control_commands_and_runtime_shutdown(event)
        .or_else(|| audit_record_child_shutdown_pipeline(event))
        .or_else(|| audit_record_child_control_early(event))
        .or_else(|| audit_record_child_control_late(event))
        .or_else(|| audit_record_child_heartbeat_stale(event))
        .or_else(|| audit_record_generation_fence_entered(event))
        .or_else(|| audit_record_generation_fence_abort_requested(event))
        .or_else(|| audit_record_generation_fence_released(event))
        .or_else(|| audit_record_generation_fence_conflict(event))
        .or_else(|| audit_record_generation_fence_stale_attempt(event))
}

/// Attempts to build an audit record for operator commands and runtime shutdown loop facts.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches command acceptance, control loop lifecycle, or pipeline shutdown completion.
fn audit_record_control_commands_and_runtime_shutdown(
    event: &SupervisorEvent,
) -> Option<AuditRecord> {
    match &event.what {
        What::CommandAccepted { audit } | What::CommandCompleted { audit } => Some(AuditRecord {
            sequence: event.sequence.value,
            command_id: audit.command_id.clone(),
            requested_by: audit.requested_by.clone(),
            result: audit.result.clone(),
            reason: audit.reason.clone(),
            phase: "control_command".to_owned(),
            child_id: None,
            context: BTreeMap::new(),
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
            child_id: None,
            context: BTreeMap::new(),
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
            child_id: None,
            context: BTreeMap::new(),
        }),
        What::RuntimeControlLoopFailed { phase, reason, .. } => Some(AuditRecord {
            sequence: event.sequence.value,
            command_id: "runtime-control-loop".to_owned(),
            requested_by: "runtime".to_owned(),
            result: "failed".to_owned(),
            reason: reason.clone(),
            phase: phase.clone(),
            child_id: None,
            context: BTreeMap::new(),
        }),
        What::ShutdownCompleted {
            phase,
            result,
            duration_ms,
        } => {
            let mut context = BTreeMap::new();
            context.insert("duration_ms".to_owned(), duration_ms.to_string());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: "shutdown-pipeline".to_owned(),
                requested_by: "runtime".to_owned(),
                result: result.clone(),
                reason: "shutdown pipeline completed".to_owned(),
                phase: phase.clone(),
                child_id: None,
                context,
            })
        }
        _ => None,
    }
}

/// Attempts to build an audit record for per-child shutdown pipeline facts.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches a child shutdown audit fact.
fn audit_record_child_shutdown_pipeline(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::ChildShutdownCancelDelivered {
            child_id,
            generation,
            child_start_count,
            phase,
        } => Some(AuditRecord {
            sequence: event.sequence.value,
            command_id: "shutdown-pipeline".to_owned(),
            requested_by: "runtime".to_owned(),
            result: "cancel_delivered".to_owned(),
            reason: "cancellation token delivered".to_owned(),
            phase: phase.clone(),
            child_id: Some(child_id.to_string()),
            context: child_child_start_count_context(generation.value, child_start_count.value),
        }),
        What::ChildShutdownGraceful {
            child_id,
            generation,
            child_start_count,
            phase,
            exit,
        } => {
            let mut context =
                child_child_start_count_context(generation.value, child_start_count.value);
            context.insert("exit".to_owned(), exit.clone());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: "shutdown-pipeline".to_owned(),
                requested_by: "runtime".to_owned(),
                result: "graceful".to_owned(),
                reason: "child completed during graceful drain".to_owned(),
                phase: phase.clone(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        What::ChildShutdownAborted {
            child_id,
            generation,
            child_start_count,
            phase,
            result,
            reason,
        } => Some(AuditRecord {
            sequence: event.sequence.value,
            command_id: "shutdown-pipeline".to_owned(),
            requested_by: "runtime".to_owned(),
            result: result.clone(),
            reason: reason.clone(),
            phase: phase.clone(),
            child_id: Some(child_id.to_string()),
            context: child_child_start_count_context(generation.value, child_start_count.value),
        }),
        What::ChildShutdownLateReport {
            child_id,
            generation,
            child_start_count,
            phase,
            exit,
        } => {
            let mut context =
                child_child_start_count_context(generation.value, child_start_count.value);
            context.insert("exit".to_owned(), exit.clone());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: "shutdown-pipeline".to_owned(),
                requested_by: "runtime".to_owned(),
                result: "late_report".to_owned(),
                reason: "child reported after shutdown accounting window".to_owned(),
                phase: phase.clone(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        _ => None,
    }
}

/// Attempts to build an audit record for early child control command outcomes.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches command completion, cancel delivery, or successful stop completion.
fn audit_record_child_control_early(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::ChildControlCommandCompleted {
            child_id,
            command,
            command_id,
            requested_by,
            reason,
            result,
            outcome,
        } => Some(audit_child_control(ChildControlAuditInput {
            sequence: event.sequence.value,
            command_id,
            requested_by,
            reason,
            result,
            child_id,
            command,
            outcome,
        })),
        What::ChildControlCancelDelivered {
            child_id,
            generation,
            attempt,
            command,
            command_id,
        } => {
            let mut context = child_child_start_count_context(generation.value, attempt.value);
            context.insert("command".to_owned(), command.clone());
            context.insert("cancel_delivered".to_owned(), true.to_string());
            context.insert("stop_state".to_owned(), "CancelDelivered".to_owned());
            context.insert("idempotent".to_owned(), false.to_string());
            context.insert("failure".to_owned(), "none".to_owned());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: command_id.clone(),
                requested_by: "runtime".to_owned(),
                result: "cancel_delivered".to_owned(),
                reason: "child control cancellation delivered".to_owned(),
                phase: "child_control".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        What::ChildControlStopCompleted {
            child_id,
            generation,
            attempt,
            exit_kind,
        } => {
            let mut context = child_child_start_count_context(generation.value, attempt.value);
            context.insert("exit_kind".to_owned(), format!("{exit_kind:?}"));
            context.insert("stop_state".to_owned(), "Completed".to_owned());
            context.insert("failure".to_owned(), "none".to_owned());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: "child-control".to_owned(),
                requested_by: "runtime".to_owned(),
                result: "completed".to_owned(),
                reason: "child control stop completed".to_owned(),
                phase: "child_control".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        _ => None,
    }
}

/// Attempts to build an audit record for late child control facts and removals.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches stop failure, operation change, or runtime state removal.
fn audit_record_child_control_late(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::ChildControlStopFailed {
            child_id,
            generation,
            attempt,
            status,
            stop_state,
            phase,
            reason,
            recoverable,
        } => {
            let mut context = child_child_start_count_context(generation.value, attempt.value);
            context.insert("status".to_owned(), format!("{status:?}"));
            context.insert("stop_state".to_owned(), format!("{stop_state:?}"));
            context.insert("recoverable".to_owned(), recoverable.to_string());
            context.insert("failure_phase".to_owned(), format!("{phase:?}"));
            context.insert("failure".to_owned(), reason.clone());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: "child-control".to_owned(),
                requested_by: "runtime".to_owned(),
                result: "failed".to_owned(),
                reason: reason.clone(),
                phase: format!("{phase:?}"),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        What::ChildControlOperationChanged {
            child_id,
            from,
            to,
            command,
            command_id,
        } => {
            let mut context = BTreeMap::new();
            context.insert("operation_before".to_owned(), format!("{from:?}"));
            context.insert("operation_after".to_owned(), format!("{to:?}"));
            context.insert("command".to_owned(), command.clone());
            context.insert("idempotent".to_owned(), false.to_string());
            context.insert("failure".to_owned(), "none".to_owned());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: command_id.clone(),
                requested_by: "runtime".to_owned(),
                result: "operation_changed".to_owned(),
                reason: "child control operation changed".to_owned(),
                phase: "child_control".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        What::ChildRuntimeStateRemoved {
            child_id,
            path,
            final_status,
        } => {
            let mut context = BTreeMap::new();
            context.insert("path".to_owned(), path.to_string());
            if let Some(status) = final_status {
                context.insert("final_status".to_owned(), format!("{status:?}"));
                context.insert("status".to_owned(), format!("{status:?}"));
            } else {
                context.insert("final_status".to_owned(), "none".to_owned());
            }
            context.insert("operation_after".to_owned(), "Removed".to_owned());
            context.insert("failure".to_owned(), "none".to_owned());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: "child-control".to_owned(),
                requested_by: "runtime".to_owned(),
                result: "removed".to_owned(),
                reason: "child runtime state removed".to_owned(),
                phase: "child_control".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        _ => None,
    }
}

/// Attempts to build an audit record for child heartbeat staleness.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches heartbeat stale reporting.
fn audit_record_child_heartbeat_stale(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::ChildHeartbeatStale {
            child_id,
            attempt,
            since_unix_nanos,
        } => {
            let mut context = BTreeMap::new();
            context.insert("attempt".to_owned(), attempt.value.to_string());
            context.insert("since_unix_nanos".to_owned(), since_unix_nanos.to_string());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: "child-liveness".to_owned(),
                requested_by: "runtime".to_owned(),
                result: "heartbeat_stale".to_owned(),
                reason: "child heartbeat became stale".to_owned(),
                phase: "liveness".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        _ => None,
    }
}

/// Attempts to build an audit record when a generation fence is entered.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches fence entry.
fn audit_record_generation_fence_entered(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::ChildRestartFenceEntered {
            child_id,
            old_generation,
            old_attempt,
            target_generation,
            command_id,
            requested_by,
            reason,
            stop_deadline_at_unix_nanos,
        } => {
            let mut context =
                child_child_start_count_context(old_generation.value, old_attempt.value);
            context.insert(
                "target_generation".to_owned(),
                target_generation.value.to_string(),
            );
            context.insert(
                "stop_deadline_at_unix_nanos".to_owned(),
                stop_deadline_at_unix_nanos.to_string(),
            );
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: command_id.clone(),
                requested_by: requested_by.clone(),
                result: "fence_entered".to_owned(),
                reason: reason.clone(),
                phase: "generation_fence".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        _ => None,
    }
}

/// Attempts to build an audit record when a generation fence abort is requested.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches fence abort escalation.
fn audit_record_generation_fence_abort_requested(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::ChildRestartFenceAbortRequested {
            child_id,
            old_generation,
            old_attempt,
            target_generation,
            command_id,
            deadline_unix_nanos,
        } => {
            let mut context =
                child_child_start_count_context(old_generation.value, old_attempt.value);
            context.insert(
                "target_generation".to_owned(),
                target_generation.value.to_string(),
            );
            context.insert(
                "deadline_unix_nanos".to_owned(),
                deadline_unix_nanos.to_string(),
            );
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: command_id.clone(),
                requested_by: "runtime".to_owned(),
                result: "abort_requested".to_owned(),
                reason: "generation fence escalation after cooperative deadline".to_owned(),
                phase: "generation_fence".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        _ => None,
    }
}

/// Attempts to build an audit record when a generation fence releases the old attempt.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches fence release after drain.
fn audit_record_generation_fence_released(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::ChildRestartFenceReleased {
            child_id,
            old_generation,
            old_attempt,
            target_generation,
            exit_kind,
        } => {
            let mut context =
                child_child_start_count_context(old_generation.value, old_attempt.value);
            context.insert(
                "target_generation".to_owned(),
                target_generation.value.to_string(),
            );
            context.insert("exit_kind".to_owned(), format!("{exit_kind:?}"));
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: "generation-fence".to_owned(),
                requested_by: "runtime".to_owned(),
                result: "released".to_owned(),
                reason: "old attempt drained; queued generation may spawn".to_owned(),
                phase: "generation_fence".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        _ => None,
    }
}

/// Attempts to build an audit record for generation fence conflicts.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches restart merge or rejection under fencing.
fn audit_record_generation_fence_conflict(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::ChildRestartConflict {
            child_id,
            current_generation,
            current_attempt,
            target_generation,
            command_id,
            decision,
            reason,
        } => {
            let mut context = BTreeMap::new();
            context.insert(
                "old_generation".to_owned(),
                optional_u64(current_generation.map(|generation| generation.value)),
            );
            context.insert(
                "old_attempt".to_owned(),
                optional_u64(current_attempt.map(|attempt| attempt.value)),
            );
            context.insert(
                "target_generation".to_owned(),
                optional_u64(target_generation.map(|generation| generation.value)),
            );
            context.insert("generation_fence_decision".to_owned(), decision.clone());
            context.insert("failure".to_owned(), reason.clone());
            context.insert("stale_report".to_owned(), "none".to_owned());
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: command_id.clone(),
                requested_by: "runtime".to_owned(),
                result: decision.clone(),
                reason: reason.clone(),
                phase: "generation_fence".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        _ => None,
    }
}

/// Attempts to build an audit record when a stale completion triple is handled.
///
/// # Arguments
///
/// - `event`: Supervisor event carrying payload.
///
/// # Returns
///
/// Returns [`Some`] when `event.what` matches stale attempt reporting under fencing.
fn audit_record_generation_fence_stale_attempt(event: &SupervisorEvent) -> Option<AuditRecord> {
    match &event.what {
        What::ChildAttemptStaleReport {
            child_id,
            reported_generation,
            reported_attempt,
            current_generation,
            current_attempt,
            exit_kind,
            handled_as,
        } => {
            let mut context =
                child_child_start_count_context(reported_generation.value, reported_attempt.value);
            context.insert(
                "current_generation".to_owned(),
                optional_u64(current_generation.map(|generation| generation.value)),
            );
            context.insert(
                "current_attempt".to_owned(),
                optional_u64(current_attempt.map(|attempt| attempt.value)),
            );
            context.insert("exit_kind".to_owned(), format!("{exit_kind:?}"));
            context.insert("handled_as".to_owned(), format!("{handled_as:?}"));
            context.insert(
                "stale_report".to_owned(),
                format!(
                    "reported_generation={} reported_attempt={}",
                    reported_generation.value, reported_attempt.value
                ),
            );
            Some(AuditRecord {
                sequence: event.sequence.value,
                command_id: "generation-fence".to_owned(),
                requested_by: "runtime".to_owned(),
                result: "stale_report".to_owned(),
                reason: "completion triple did not match active or pending restart identities"
                    .to_owned(),
                phase: "generation_fence".to_owned(),
                child_id: Some(child_id.to_string()),
                context,
            })
        }
        _ => None,
    }
}

/// Merges generation fence projection fields into a child control audit context map.
///
/// # Arguments
///
/// - `context`: Audit context map being assembled.
/// - `outcome`: Child control outcome carrying optional generation fence projection.
///
/// # Returns
///
/// This function does not return a value.
fn merge_generation_fence_child_control_audit_fields(
    context: &mut BTreeMap<String, String>,
    outcome: &crate::control::outcome::ChildControlResult,
) {
    if let Some(fence) = &outcome.generation_fence {
        context.insert(
            "generation_fence_decision".to_owned(),
            format!("{:?}", fence.decision),
        );
        context.insert(
            "generation_fence_abort_requested".to_owned(),
            fence.abort_requested.to_string(),
        );
        context.insert(
            "generation_fence_cancel_delivered".to_owned(),
            fence.cancel_delivered.to_string(),
        );
        context.insert(
            "generation_fence_old_generation".to_owned(),
            optional_u64(fence.old_generation.map(|generation| generation.value)),
        );
        context.insert(
            "generation_fence_old_attempt".to_owned(),
            optional_u64(fence.old_attempt.map(|attempt| attempt.value)),
        );
        context.insert(
            "generation_fence_target_generation".to_owned(),
            optional_u64(fence.target_generation.map(|generation| generation.value)),
        );
        context.insert(
            "generation_fence_conflict".to_owned(),
            failure_context(&fence.conflict),
        );
    } else {
        context.insert("generation_fence_decision".to_owned(), "none".to_owned());
        context.insert(
            "generation_fence_abort_requested".to_owned(),
            "false".to_owned(),
        );
        context.insert(
            "generation_fence_cancel_delivered".to_owned(),
            "false".to_owned(),
        );
        context.insert("generation_fence_conflict".to_owned(), "none".to_owned());
    }
}

/// Input used to build a child control audit record.
struct ChildControlAuditInput<'a> {
    /// Event sequence.
    sequence: u64,
    /// Stable command identifier.
    command_id: &'a str,
    /// Actor that requested the command.
    requested_by: &'a str,
    /// Operator-provided reason.
    reason: &'a str,
    /// Low-cardinality command result.
    result: &'a str,
    /// Target child identifier.
    child_id: &'a crate::id::types::ChildId,
    /// Stable command name.
    command: &'a str,
    /// Full child control outcome.
    outcome: &'a crate::control::outcome::ChildControlResult,
}

/// Builds a child control audit record with full command outcome context.
///
/// # Arguments
///
/// - `input`: Child control audit values.
///
/// # Returns
///
/// Returns an [`AuditRecord`] for the child control command.
fn audit_child_control(input: ChildControlAuditInput<'_>) -> AuditRecord {
    let mut context = BTreeMap::new();
    context.insert("command".to_owned(), input.command.to_owned());
    context.insert(
        "generation".to_owned(),
        optional_u64(input.outcome.generation.map(|generation| generation.value)),
    );
    context.insert(
        "attempt".to_owned(),
        optional_u64(input.outcome.attempt.map(|attempt| attempt.value)),
    );
    context.insert("status".to_owned(), optional_debug(input.outcome.status));
    context.insert(
        "operation_before".to_owned(),
        format!("{:?}", input.outcome.operation_before),
    );
    context.insert(
        "operation_after".to_owned(),
        format!("{:?}", input.outcome.operation_after),
    );
    context.insert(
        "cancel_delivered".to_owned(),
        input.outcome.cancel_delivered.to_string(),
    );
    context.insert(
        "stop_state".to_owned(),
        format!("{:?}", input.outcome.stop_state),
    );
    context.insert(
        "restart_limit_remaining".to_owned(),
        input.outcome.restart_limit.remaining.to_string(),
    );
    context.insert(
        "idempotent".to_owned(),
        input.outcome.idempotent.to_string(),
    );
    context.insert(
        "failure".to_owned(),
        failure_context(&input.outcome.failure),
    );
    merge_generation_fence_child_control_audit_fields(&mut context, input.outcome);
    context.insert("stale_report".to_owned(), "none".to_owned());
    AuditRecord {
        sequence: input.sequence,
        command_id: input.command_id.to_owned(),
        requested_by: input.requested_by.to_owned(),
        result: input.result.to_owned(),
        reason: input.reason.to_owned(),
        phase: "child_control".to_owned(),
        child_id: Some(input.child_id.to_string()),
        context,
    }
}

/// Formats an optional numeric identifier for audit context.
fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

/// Formats an optional debug value for audit context.
fn optional_debug<T: std::fmt::Debug>(value: Option<T>) -> String {
    value
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|| "none".to_owned())
}

/// Formats an optional child control failure for audit context.
fn failure_context(failure: &Option<crate::control::outcome::ChildControlFailure>) -> String {
    failure
        .as_ref()
        .map(|failure| {
            format!(
                "{:?}:{}:recoverable={}",
                failure.phase, failure.reason, failure.recoverable
            )
        })
        .unwrap_or_else(|| "none".to_owned())
}

/// Builds compact child child_start_count context for audit records.
fn child_child_start_count_context(
    generation: u64,
    child_start_count: u64,
) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    context.insert("generation".to_owned(), generation.to_string());
    context.insert("attempt".to_owned(), child_start_count.to_string());
    context.insert(
        "child_start_count".to_owned(),
        child_start_count.to_string(),
    );
    context
}

/// Six-stage supervision pipeline stage identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineStage {
    /// Stage 1: Classify the exit reason and category.
    ClassifyExit,
    /// Stage 2: Record failure window accumulation.
    RecordFailureWindow,
    /// Stage 3: Evaluate restart budget and limits.
    EvaluateBudget,
    /// Stage 4: Decide protective action based on merged verdicts.
    DecideAction,
    /// Stage 5: Emit typed supervision event with all diagnostic fields.
    EmitTypedEvent,
    /// Stage 6: Execute the decided action (restart, queue, deny, etc.).
    ExecuteAction,
}

impl std::fmt::Display for PipelineStage {
    /// Formats the pipeline stage as a string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClassifyExit => write!(f, "classify_exit"),
            Self::RecordFailureWindow => write!(f, "record_failure_window"),
            Self::EvaluateBudget => write!(f, "evaluate_budget"),
            Self::DecideAction => write!(f, "decide_action"),
            Self::EmitTypedEvent => write!(f, "emit_typed_event"),
            Self::ExecuteAction => write!(f, "execute_action"),
        }
    }
}

/// Diagnostic record emitted at each pipeline stage for observability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineStageDiagnostic {
    /// Monotonic event sequence shared across all stages.
    pub sequence: u64,
    /// Correlation identifier tying related signals together.
    pub correlation_id: String,
    /// Pipeline stage that produced this diagnostic.
    pub stage: PipelineStage,
    /// Child identifier being supervised.
    pub child_id: Option<String>,
    /// Group identifier when the child belongs to a group.
    pub group_id: Option<String>,
    /// Supervisor path owning the supervision scope.
    pub supervisor_path: String,
    /// Exit classification result (stage 1 output).
    pub exit_classification: Option<String>,
    /// Failure window state after recording (stage 2 output).
    pub failure_window_state: Option<String>,
    /// Budget evaluation result (stage 3 output).
    pub budget_evaluation: Option<String>,
    /// Decided protective action (stage 4 output).
    pub decided_action: Option<String>,
    /// Event emission confirmation (stage 5 output).
    pub event_emitted: bool,
    /// Execution result summary (stage 6 output).
    pub execution_result: Option<String>,
    /// Timestamp in Unix epoch nanoseconds when this stage completed.
    pub completed_at_unix_nanos: u128,
}

impl PipelineStageDiagnostic {
    /// Creates a diagnostic record for a pipeline stage.
    ///
    /// # Arguments
    ///
    /// - `sequence`: Event sequence number.
    /// - `correlation_id`: Correlation identifier.
    /// - `stage`: Pipeline stage producing this diagnostic.
    /// - `completed_at_unix_nanos`: Completion timestamp.
    ///
    /// # Returns
    ///
    /// Returns a [`PipelineStageDiagnostic`] with default empty fields.
    pub fn new(
        sequence: u64,
        correlation_id: impl Into<String>,
        stage: PipelineStage,
        completed_at_unix_nanos: u128,
    ) -> Self {
        Self {
            sequence,
            correlation_id: correlation_id.into(),
            stage,
            child_id: None,
            group_id: None,
            supervisor_path: String::new(),
            exit_classification: None,
            failure_window_state: None,
            budget_evaluation: None,
            decided_action: None,
            event_emitted: false,
            execution_result: None,
            completed_at_unix_nanos,
        }
    }

    /// Sets the child identifier.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier string.
    ///
    /// # Returns
    ///
    /// Returns the updated [`PipelineStageDiagnostic`].
    pub fn with_child_id(mut self, child_id: impl Into<String>) -> Self {
        self.child_id = Some(child_id.into());
        self
    }

    /// Sets the group identifier.
    ///
    /// # Arguments
    ///
    /// - `group_id`: Group identifier string.
    ///
    /// # Returns
    ///
    /// Returns the updated [`PipelineStageDiagnostic`].
    pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = Some(group_id.into());
        self
    }

    /// Sets the supervisor path.
    ///
    /// # Arguments
    ///
    /// - `supervisor_path`: Supervisor path string.
    ///
    /// # Returns
    ///
    /// Returns the updated [`PipelineStageDiagnostic`].
    pub fn with_supervisor_path(mut self, supervisor_path: impl Into<String>) -> Self {
        self.supervisor_path = supervisor_path.into();
        self
    }
}
