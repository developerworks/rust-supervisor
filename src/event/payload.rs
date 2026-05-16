//! Lifecycle event payloads and event envelopes.
//!
//! This module owns the observable shape of supervisor lifecycle facts. It keeps
//! payloads typed so state, journal, metrics, and tests do not infer behavior
//! from strings.

use crate::child_runner::run_exit::TaskExit;
use crate::control::outcome::{
    ChildAttemptStatus, ChildControlFailurePhase, ChildControlOperation, ChildControlResult,
    ChildStopState, RestartLimitState, StaleReportHandling,
};
use crate::error::types::TaskFailure;
use crate::event::time::{CorrelationId, EventSequence, When};
use crate::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use serde::{Deserialize, Serialize};

/// Location data attached to a supervisor event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Where {
    /// Stable supervisor path that owns the fact.
    pub supervisor_path: SupervisorPath,
    /// Parent child identifier when the fact belongs to a nested node.
    pub parent_id: Option<ChildId>,
    /// Child identifier related to the fact.
    pub child_id: Option<ChildId>,
    /// Human-readable child name.
    pub child_name: Option<String>,
    /// Tokio task identifier when it is available.
    pub tokio_task_id: Option<String>,
    /// Host name reported by the runtime.
    pub host: Option<String>,
    /// Process identifier that emitted the event.
    pub pid: u32,
    /// Current thread name when available.
    pub thread_name: Option<String>,
    /// Rust module path that emitted the event.
    pub module_path: Option<String>,
    /// Source file that emitted the event.
    pub source_file: Option<String>,
    /// Source line that emitted the event.
    pub source_line: Option<u32>,
}

impl Where {
    /// Creates a location for a supervisor path.
    ///
    /// # Arguments
    ///
    /// - `supervisor_path`: Path that owns this lifecycle fact.
    ///
    /// # Returns
    ///
    /// Returns a [`Where`] value with process and thread defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// let location = rust_supervisor::event::payload::Where::new(
    ///     rust_supervisor::id::types::SupervisorPath::root(),
    /// );
    /// assert_eq!(location.supervisor_path.to_string(), "/");
    /// ```
    pub fn new(supervisor_path: SupervisorPath) -> Self {
        Self {
            supervisor_path,
            parent_id: None,
            child_id: None,
            child_name: None,
            tokio_task_id: None,
            host: None,
            pid: std::process::id(),
            thread_name: std::thread::current().name().map(ToOwned::to_owned),
            module_path: None,
            source_file: None,
            source_line: None,
        }
    }

    /// Adds child identity to the location.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    /// - `child_name`: Human-readable child name.
    ///
    /// # Returns
    ///
    /// Returns the updated [`Where`] value.
    pub fn with_child(mut self, child_id: ChildId, child_name: impl Into<String>) -> Self {
        self.child_id = Some(child_id);
        self.child_name = Some(child_name.into());
        self
    }
}

/// State transition recorded by an event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    /// State before the transition.
    pub from: String,
    /// State after the transition.
    pub to: String,
}

impl StateTransition {
    /// Creates a state transition description.
    ///
    /// # Arguments
    ///
    /// - `from`: Previous state name.
    /// - `to`: New state name.
    ///
    /// # Returns
    ///
    /// Returns a [`StateTransition`].
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

/// Policy decision data stored with an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Low-cardinality decision name.
    pub decision: String,
    /// Delay in milliseconds when restart is delayed.
    pub delay_ms: Option<u64>,
    /// Human-readable reason for diagnostics.
    pub reason: Option<String>,
}

impl PolicyDecision {
    /// Creates a policy decision value.
    ///
    /// # Arguments
    ///
    /// - `decision`: Low-cardinality decision name.
    /// - `delay_ms`: Optional delay in milliseconds.
    /// - `reason`: Optional diagnostic reason.
    ///
    /// # Returns
    ///
    /// Returns a [`PolicyDecision`].
    pub fn new(decision: impl Into<String>, delay_ms: Option<u64>, reason: Option<String>) -> Self {
        Self {
            decision: decision.into(),
            delay_ms,
            reason,
        }
    }
}

/// Command audit data attached to command lifecycle events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandAudit {
    /// Stable command identifier.
    pub command_id: String,
    /// Actor that requested the command.
    pub requested_by: String,
    /// Operator-provided reason.
    pub reason: String,
    /// Target path for the command.
    pub target_path: SupervisorPath,
    /// Accepted time in nanoseconds since the Unix epoch.
    pub accepted_at_unix_nanos: u128,
    /// Command result summary.
    pub result: String,
}

/// Typed payload for supervisor lifecycle events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum What {
    /// Child is being started.
    ChildStarting {
        /// Optional state transition carried by this event.
        transition: Option<StateTransition>,
    },
    /// Child is running.
    ChildRunning {
        /// Optional state transition carried by this event.
        transition: Option<StateTransition>,
    },
    /// Child is ready.
    ChildReady {
        /// Optional state transition carried by this event.
        transition: Option<StateTransition>,
    },
    /// Child emitted a heartbeat.
    ChildHeartbeat {
        /// Heartbeat age in milliseconds.
        age_ms: u64,
    },
    /// Child failed with a typed failure.
    ChildFailed {
        /// Failure payload reported by the task.
        failure: TaskFailure,
    },
    /// Child panicked.
    ChildPanicked {
        /// Panic category used for metrics.
        category: String,
    },
    /// Restart backoff was scheduled.
    BackoffScheduled {
        /// Backoff delay in milliseconds.
        delay_ms: u64,
    },
    /// Child is restarting.
    ChildRestarting {
        /// Restart generation after the transition.
        generation: u64,
    },
    /// Child restarted.
    ChildRestarted {
        /// Restart count for the child window.
        restart_count: u64,
    },
    /// Child was quarantined.
    ChildQuarantined {
        /// Quarantine reason.
        reason: String,
    },
    /// Child stopped.
    ChildStopped {
        /// Exit reason.
        reason: String,
    },
    /// Child became unhealthy.
    ChildUnhealthy {
        /// Unhealthy reason.
        reason: String,
    },
    /// Meltdown fuse was tripped.
    Meltdown {
        /// Scope that tripped the fuse.
        scope: String,
    },
    /// Shutdown was requested.
    ShutdownRequested {
        /// Shutdown cause.
        cause: String,
    },
    /// Shutdown phase changed.
    ShutdownPhaseChanged {
        /// Previous phase name.
        from: String,
        /// New phase name.
        to: String,
    },
    /// Shutdown completed.
    ShutdownCompleted {
        /// Final shutdown phase.
        phase: String,
        /// Shutdown result summary.
        result: String,
        /// Full pipeline duration in milliseconds.
        duration_ms: u64,
    },
    /// Child shutdown cancel delivered for one supervised child_start_count during shutdown draining.
    ChildShutdownCancelDelivered {
        /// Child that received cancellation.
        child_id: ChildId,
        /// Generation associated with the child child_start_count.
        generation: Generation,
        /// ChildStartCount associated with the child run.
        child_start_count: ChildStartCount,
        /// Shutdown phase that delivered cancellation.
        phase: String,
    },
    /// Child finished during graceful shutdown draining.
    ChildShutdownGraceful {
        /// Child that completed gracefully.
        child_id: ChildId,
        /// Generation associated with the child child_start_count.
        generation: Generation,
        /// ChildStartCount associated with the child run.
        child_start_count: ChildStartCount,
        /// Shutdown phase that recorded the outcome.
        phase: String,
        /// Exit classification reported by the child.
        exit: String,
    },
    /// Child was aborted during shutdown.
    ChildShutdownAborted {
        /// Child that was aborted.
        child_id: ChildId,
        /// Generation associated with the child child_start_count.
        generation: Generation,
        /// ChildStartCount associated with the child run.
        child_start_count: ChildStartCount,
        /// Shutdown phase that recorded the outcome.
        phase: String,
        /// Low-cardinality abort result.
        result: String,
        /// Human-readable abort reason.
        reason: String,
    },
    /// Child reported after its normal shutdown accounting window.
    ChildShutdownLateReport {
        /// Child that produced a late report.
        child_id: ChildId,
        /// Generation associated with the child child_start_count.
        generation: Generation,
        /// ChildStartCount associated with the child run.
        child_start_count: ChildStartCount,
        /// Shutdown phase that received the late report.
        phase: String,
        /// Exit classification reported by the child.
        exit: String,
    },
    /// Generation fence engaged for an accepted manual restart waiting for an old attempt to stop.
    ChildRestartFenceEntered {
        /// Child awaiting restart isolation.
        child_id: ChildId,
        /// Old generation pinned until the fence releases.
        old_generation: Generation,
        /// Old attempt pinned until the fence releases.
        old_attempt: ChildStartCount,
        /// Target generation queued after the old attempt completes.
        target_generation: Generation,
        /// Command identifier tying this fence to auditing metadata.
        command_id: String,
        /// Restart requester captured from command metadata.
        requested_by: String,
        /// Restart reason captured from command metadata.
        reason: String,
        /// Deadline for cooperative stop before escalation to abort paths.
        stop_deadline_at_unix_nanos: u128,
    },
    /// Runtime escalated restart isolation to abort the old attempt after the cooperative deadline elapsed.
    ChildRestartFenceAbortRequested {
        /// Child awaiting restart isolation.
        child_id: ChildId,
        /// Old generation that failed to exit before the graceful deadline expired.
        old_generation: Generation,
        /// Old attempt that failed to exit before the graceful deadline expired.
        old_attempt: ChildStartCount,
        /// Target generation queued for start after isolation completes.
        target_generation: Generation,
        /// Command identifier tied to the pending restart bookkeeping.
        command_id: String,
        /// Deadline that triggered the abort escalation.
        deadline_unix_nanos: u128,
    },
    /// Old attempt completed and a new generation may start under the pending restart request.
    ChildRestartFenceReleased {
        /// Child whose fence released.
        child_id: ChildId,
        /// Old generation that fully stopped.
        old_generation: Generation,
        /// Old attempt that fully stopped.
        old_attempt: ChildStartCount,
        /// Target generation allowed to start after this release.
        target_generation: Generation,
        /// Exit classification reported for the old attempt.
        exit_kind: TaskExit,
    },
    /// Conflicting restart intent that was merged, rejected, or superseded by policy.
    ChildRestartConflict {
        /// Child identifier for the fencing scope.
        child_id: ChildId,
        /// Generation that was active or pinned when the conflict was classified.
        current_generation: Option<Generation>,
        /// Attempt counter that was active or pinned when the conflict was classified.
        current_attempt: Option<ChildStartCount>,
        /// Generation the caller wanted to reach, if applicable.
        target_generation: Option<Generation>,
        /// Command identifier supplied by the caller when present.
        command_id: String,
        /// Low-cardinality conflict classifier (`already_pending`, `rejected`, ...).
        decision: String,
        /// Human-readable reason for observability dumps.
        reason: String,
    },
    /// Stale completion triple observed after authoritative state moved forward.
    ChildAttemptStaleReport {
        /// Child identifier tied to the completion triple.
        child_id: ChildId,
        /// Generation carried by the stale completion report.
        reported_generation: Generation,
        /// Attempt counter carried by the stale completion report.
        reported_attempt: ChildStartCount,
        /// Generation considered authoritative when the stale report arrived.
        current_generation: Option<Generation>,
        /// Attempt counter considered authoritative when the stale report arrived.
        current_attempt: Option<ChildStartCount>,
        /// Exit classification supplied by the stale report.
        exit_kind: TaskExit,
        /// Runtime-selected handling bucket for metrics and audits.
        handled_as: StaleReportHandling,
    },
    /// Pending restart bookkeeping drained because the pinned old attempt exited.
    ChildRestartFencePendingDrained {
        /// Child whose pending restart advanced past the cooperative stop barrier.
        child_id: ChildId,
    },
    /// Child control command delivered cancellation.
    ChildControlCancelDelivered {
        /// Child that received cancellation.
        child_id: ChildId,
        /// Generation that received cancellation.
        generation: Generation,
        /// Attempt that received cancellation.
        attempt: ChildStartCount,
        /// Control command name.
        command: String,
        /// Control command identifier.
        command_id: String,
    },
    /// Child control stop completed.
    ChildControlStopCompleted {
        /// Child that completed stopping.
        child_id: ChildId,
        /// Generation that completed stopping.
        generation: Generation,
        /// Attempt that completed stopping.
        attempt: ChildStartCount,
        /// Child exit classification.
        exit_kind: TaskExit,
    },
    /// Child control stop failed.
    ChildControlStopFailed {
        /// Child that failed to stop.
        child_id: ChildId,
        /// Generation that failed to stop.
        generation: Generation,
        /// Attempt that failed to stop.
        attempt: ChildStartCount,
        /// Current attempt status.
        status: ChildAttemptStatus,
        /// Current stop progress.
        stop_state: ChildStopState,
        /// Control failure phase.
        phase: ChildControlFailurePhase,
        /// Human-readable failure reason.
        reason: String,
        /// Whether callers can retry to recover.
        recoverable: bool,
    },
    /// Child control operation changed.
    ChildControlOperationChanged {
        /// Child whose operation changed.
        child_id: ChildId,
        /// Previous operation.
        from: ChildControlOperation,
        /// New operation.
        to: ChildControlOperation,
        /// Control command name.
        command: String,
        /// Control command identifier.
        command_id: String,
    },
    /// Child control command completed with a full outcome.
    ChildControlCommandCompleted {
        /// Child that the command targeted.
        child_id: ChildId,
        /// Stable control command name.
        command: String,
        /// Control command identifier.
        command_id: String,
        /// Actor that requested the command.
        requested_by: String,
        /// Operator-provided reason.
        reason: String,
        /// Low-cardinality command result.
        result: String,
        /// Full control outcome.
        outcome: Box<ChildControlResult>,
    },
    /// Child restart limit accounting was refreshed.
    ChildRuntimeRestartLimitUpdated {
        /// Child whose restart limit accounting changed.
        child_id: ChildId,
        /// Updated restart limit state.
        restart_limit: RestartLimitState,
    },
    /// Child runtime state record was removed.
    ChildRuntimeStateRemoved {
        /// Removed child.
        child_id: ChildId,
        /// Child path in the supervisor tree.
        path: SupervisorPath,
        /// Final attempt status.
        final_status: Option<ChildAttemptStatus>,
    },
    /// Child heartbeat became stale.
    ChildHeartbeatStale {
        /// Child with a stale heartbeat.
        child_id: ChildId,
        /// Attempt with a stale heartbeat.
        attempt: ChildStartCount,
        /// Last heartbeat timestamp in Unix epoch nanoseconds.
        since_unix_nanos: u128,
    },
    /// Control command was accepted.
    CommandAccepted {
        /// Command audit payload.
        audit: CommandAudit,
    },
    /// Control command completed.
    CommandCompleted {
        /// Command audit payload.
        audit: CommandAudit,
    },
    /// Runtime control loop started.
    RuntimeControlLoopStarted {
        /// Startup phase label.
        phase: String,
        /// Startup time in Unix epoch nanoseconds.
        started_at_unix_nanos: u128,
    },
    /// Runtime control loop shutdown was requested.
    RuntimeControlLoopShutdownRequested {
        /// Stable command identifier.
        command_id: String,
        /// Actor that requested shutdown.
        requested_by: String,
        /// Operator-provided reason.
        reason: String,
    },
    /// Runtime control loop completed normally.
    RuntimeControlLoopCompleted {
        /// Completion phase label.
        phase: String,
        /// Completion reason.
        reason: String,
        /// Completion time in Unix epoch nanoseconds.
        completed_at_unix_nanos: u128,
    },
    /// Runtime control loop failed.
    RuntimeControlLoopFailed {
        /// Failure phase label.
        phase: String,
        /// Failure reason.
        reason: String,
        /// Whether failure came from panic.
        panic: bool,
        /// Whether a new supervisor can recover.
        recoverable: bool,
    },
    /// Runtime control loop join completed.
    RuntimeControlLoopJoinCompleted {
        /// Stable command identifier.
        command_id: String,
        /// Actor that requested join.
        requested_by: String,
        /// Final state label.
        state: String,
        /// Final phase label.
        phase: String,
        /// Final reason.
        reason: String,
    },
    /// Event subscriber lagged.
    SubscriberLagged {
        /// Number of missed events.
        missed: u64,
    },
}

impl What {
    /// Returns a low-cardinality event name.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the stable event name.
    ///
    /// # Examples
    ///
    /// ```
    /// let event = rust_supervisor::event::payload::What::ChildRunning {
    ///     transition: None,
    /// };
    /// assert_eq!(event.name(), "ChildRunning");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            Self::ChildStarting { .. } => "ChildStarting",
            Self::ChildRunning { .. } => "ChildRunning",
            Self::ChildReady { .. } => "ChildReady",
            Self::ChildHeartbeat { .. } => "ChildHeartbeat",
            Self::ChildFailed { .. } => "ChildFailed",
            Self::ChildPanicked { .. } => "ChildPanicked",
            Self::BackoffScheduled { .. } => "BackoffScheduled",
            Self::ChildRestarting { .. } => "ChildRestarting",
            Self::ChildRestarted { .. } => "ChildRestarted",
            Self::ChildQuarantined { .. } => "ChildQuarantined",
            Self::ChildStopped { .. } => "ChildStopped",
            Self::ChildUnhealthy { .. } => "ChildUnhealthy",
            Self::Meltdown { .. } => "Meltdown",
            Self::ShutdownRequested { .. } => "ShutdownRequested",
            Self::ShutdownPhaseChanged { .. } => "ShutdownPhaseChanged",
            Self::ShutdownCompleted { .. } => "ShutdownCompleted",
            Self::ChildShutdownCancelDelivered { .. } => "ChildShutdownCancelDelivered",
            Self::ChildShutdownGraceful { .. } => "ChildShutdownGraceful",
            Self::ChildShutdownAborted { .. } => "ChildShutdownAborted",
            Self::ChildShutdownLateReport { .. } => "ChildShutdownLateReport",
            Self::ChildRestartFenceEntered { .. } => "ChildRestartFenceEntered",
            Self::ChildRestartFenceAbortRequested { .. } => "ChildRestartFenceAbortRequested",
            Self::ChildRestartFenceReleased { .. } => "ChildRestartFenceReleased",
            Self::ChildRestartConflict { .. } => "ChildRestartConflict",
            Self::ChildAttemptStaleReport { .. } => "ChildAttemptStaleReport",
            Self::ChildRestartFencePendingDrained { .. } => "ChildRestartFencePendingDrained",
            Self::ChildControlCancelDelivered { .. } => "ChildControlCancelDelivered",
            Self::ChildControlStopCompleted { .. } => "ChildControlStopCompleted",
            Self::ChildControlStopFailed { .. } => "ChildControlStopFailed",
            Self::ChildControlOperationChanged { .. } => "ChildControlOperationChanged",
            Self::ChildControlCommandCompleted { .. } => "ChildControlCommandCompleted",
            Self::ChildRuntimeRestartLimitUpdated { .. } => "ChildRuntimeRestartLimitUpdated",
            Self::ChildRuntimeStateRemoved { .. } => "ChildRuntimeStateRemoved",
            Self::ChildHeartbeatStale { .. } => "ChildHeartbeatStale",
            Self::CommandAccepted { .. } => "CommandAccepted",
            Self::CommandCompleted { .. } => "CommandCompleted",
            Self::RuntimeControlLoopStarted { .. } => "RuntimeControlLoopStarted",
            Self::RuntimeControlLoopShutdownRequested { .. } => {
                "RuntimeControlLoopShutdownRequested"
            }
            Self::RuntimeControlLoopCompleted { .. } => "RuntimeControlLoopCompleted",
            Self::RuntimeControlLoopFailed { .. } => "RuntimeControlLoopFailed",
            Self::RuntimeControlLoopJoinCompleted { .. } => "RuntimeControlLoopJoinCompleted",
            Self::SubscriberLagged { .. } => "SubscriberLagged",
        }
    }
}

/// Complete lifecycle event envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisorEvent {
    /// Time information for the lifecycle fact.
    pub when: When,
    /// Location information for the lifecycle fact.
    pub r#where: Where,
    /// Typed event payload.
    pub what: What,
    /// Optional policy decision related to the event.
    pub policy: Option<PolicyDecision>,
    /// Monotonic event sequence.
    pub sequence: EventSequence,
    /// Correlation identifier shared by related signals.
    pub correlation_id: CorrelationId,
    /// Configuration version that produced this fact.
    pub config_version: u64,
}

impl SupervisorEvent {
    /// Creates a supervisor lifecycle event.
    ///
    /// # Arguments
    ///
    /// - `when`: Event timing.
    /// - `r#where`: Event location.
    /// - `what`: Event payload.
    /// - `sequence`: Monotonic event sequence.
    /// - `correlation_id`: Correlation identifier for related signals.
    /// - `config_version`: Configuration version for this event.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorEvent`].
    ///
    /// # Examples
    ///
    /// ```
    /// let event = rust_supervisor::event::payload::SupervisorEvent::new(
    ///     rust_supervisor::event::time::When::new(
    ///         rust_supervisor::event::time::EventTime::deterministic(
    ///             1,
    ///             1,
    ///             0,
    ///             rust_supervisor::id::types::Generation::initial(),
    ///             rust_supervisor::id::types::ChildStartCount::first(),
    ///         ),
    ///     ),
    ///     rust_supervisor::event::payload::Where::new(
    ///         rust_supervisor::id::types::SupervisorPath::root(),
    ///     ),
    ///     rust_supervisor::event::payload::What::ChildRunning { transition: None },
    ///     rust_supervisor::event::time::EventSequence::new(1),
    ///     rust_supervisor::event::time::CorrelationId::from_uuid(uuid::Uuid::nil()),
    ///     1,
    /// );
    /// assert_eq!(event.what.name(), "ChildRunning");
    /// ```
    pub fn new(
        when: When,
        r#where: Where,
        what: What,
        sequence: EventSequence,
        correlation_id: CorrelationId,
        config_version: u64,
    ) -> Self {
        Self {
            when,
            r#where,
            what,
            policy: None,
            sequence,
            correlation_id,
            config_version,
        }
    }

    /// Attaches a policy decision to an event.
    ///
    /// # Arguments
    ///
    /// - `policy`: Policy decision produced for this lifecycle fact.
    ///
    /// # Returns
    ///
    /// Returns the updated [`SupervisorEvent`].
    pub fn with_policy(mut self, policy: PolicyDecision) -> Self {
        self.policy = Some(policy);
        self
    }
}
