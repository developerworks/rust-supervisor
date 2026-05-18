//! Serializable child control result types used for generation fencing.
//!
//! These placeholder names align with the naming contract and future event payloads:
//! ChildRestartFenceEntered, ChildRestartFenceAbortRequested, ChildRestartFenceReleased,
//! ChildRestartConflict, ChildAttemptStaleReport.

use crate::child_runner::run_exit::TaskExit;
use crate::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use crate::readiness::signal::ReadinessState;
use crate::runtime::admission::AdmissionConflict;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Runtime phase for a child attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildAttemptStatus {
    /// The child attempt is starting.
    Starting,
    /// The child attempt is running.
    Running,
    /// The child attempt reported readiness.
    Ready,
    /// The child attempt is cancelling.
    Cancelling,
    /// The child attempt has stopped.
    Stopped,
}

/// Control operation requested for a child runtime state record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildControlOperation {
    /// The child runtime state remains active.
    Active,
    /// The child runtime state is paused.
    Paused,
    /// The child runtime state is quarantined.
    Quarantined,
    /// The child runtime state is waiting for removal or already removed.
    Removed,
}

/// Stop progress for child control commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildStopState {
    /// No stop action is in progress.
    Idle,
    /// The child currently has no active attempt.
    NoActiveAttempt,
    /// Cancellation was delivered to the child.
    CancelDelivered,
    /// The child completed stopping.
    Completed,
    /// The child failed to stop.
    Failed,
}

/// Failure phase for a child control command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildControlFailurePhase {
    /// Waiting for child completion failed.
    WaitCompletion,
}

/// Structured child control failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildControlFailure {
    /// Phase where the failure occurred.
    pub phase: ChildControlFailurePhase,
    /// Human-readable failure reason.
    pub reason: String,
    /// Whether callers can retry to recover.
    pub recoverable: bool,
}

impl ChildControlFailure {
    /// Creates a child control failure.
    ///
    /// # Arguments
    ///
    /// - `phase`: Phase where the failure occurred.
    /// - `reason`: Human-readable failure reason.
    /// - `recoverable`: Whether callers can retry to recover.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildControlFailure`] value.
    pub fn new(
        phase: ChildControlFailurePhase,
        reason: impl Into<String>,
        recoverable: bool,
    ) -> Self {
        Self {
            phase,
            reason: reason.into(),
            recoverable,
        }
    }
}

/// Generation fencing phase: whether a new attempt may start at a restart boundary for one child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GenerationFencePhase {
    /// No fence wait, or the active attempt runs on the normal path.
    #[default]
    Open,
    /// Restart accepted: cancellation was delivered to the old attempt; waiting for it to exit.
    WaitingForOldStop,
    /// Old attempt exceeded the graceful stop window; runtime requested abort.
    AbortingOld,
    /// Old attempt confirmed finished; allowed to start the new instance for the target generation.
    ReadyToStart,
    /// Record removed or supervisor tree in a shutdown window; must not proceed with restart start.
    Closed,
}

/// Discrete outcome for one restart command at the generation fence from the control plane view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationFenceDecision {
    /// No active attempt; target generation started immediately.
    StartedImmediately,
    /// Active attempt still present; restart queued to wait for the old stop.
    QueuedAfterStop,
    /// A pending restart already exists; this duplicate command was merged.
    AlreadyPending,
    /// Supervisor tree is shutting down; restart is not allowed.
    BlockedByShutdown,
    /// Request rejected; set [`GenerationFenceOutcome::conflict`] with the structured reason.
    Rejected,
}

/// Label for how the runtime treats a late exit report from an old generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleReportHandling {
    /// Do not change authoritative state; ignore the stale fact.
    IgnoredForState,
    /// Recorded for audit so operators can review it later.
    RecordedForAudit,
    /// Counted in a low-cardinality metrics bucket.
    CountedForMetrics,
}

/// Minimal stale report payload carried with a control command for diagnostics and dashboard projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaleAttemptReport {
    /// Stable identifier of the child this report belongs to.
    pub child_id: ChildId,
    /// Generation attached to the report that is now stale.
    pub reported_generation: Generation,
    /// Attempt index attached to the report that is now stale.
    pub reported_attempt: ChildStartCount,
    /// Active generation in the record when the report was classified as stale.
    pub current_generation: Option<Generation>,
    /// Active attempt in the record when the report was classified as stale.
    pub current_attempt: Option<ChildStartCount>,
    /// Exit shape for the old attempt; matches contract `ExitKind`.
    pub exit_kind: TaskExit,
    /// Branch the runtime chose when handling this stale report.
    pub handled_as: StaleReportHandling,
    /// Unix epoch nanoseconds when the report was classified as stale.
    pub observed_at_unix_nanos: u128,
}

impl StaleAttemptReport {
    /// Builds a stale attempt report record.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier for this report.
    /// - `reported_generation`: Stale generation carried from the report.
    /// - `reported_attempt`: Stale attempt index carried from the report.
    /// - `current_generation`: Active generation when classified as stale, or `None`.
    /// - `current_attempt`: Active attempt when classified as stale, or `None`.
    /// - `exit_kind`: Exit shape for the old attempt; matches contract `ExitKind`.
    /// - `handled_as`: How the runtime handled this stale report.
    /// - `observed_at_unix_nanos`: Unix epoch nanoseconds when the report was classified as stale.
    ///
    /// # Returns
    ///
    /// Returns an owned [`StaleAttemptReport`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::control::outcome::{StaleAttemptReport, StaleReportHandling};
    /// use rust_supervisor::child_runner::run_exit::TaskExit;
    /// let report = StaleAttemptReport::new(
    ///     rust_supervisor::id::types::ChildId::new("worker"),
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::ChildStartCount::first(),
    ///     None,
    ///     None,
    ///     TaskExit::Succeeded,
    ///     StaleReportHandling::IgnoredForState,
    ///     0,
    /// );
    /// assert_eq!(report.handled_as, StaleReportHandling::IgnoredForState);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        child_id: ChildId,
        reported_generation: Generation,
        reported_attempt: ChildStartCount,
        current_generation: Option<Generation>,
        current_attempt: Option<ChildStartCount>,
        exit_kind: TaskExit,
        handled_as: StaleReportHandling,
        observed_at_unix_nanos: u128,
    ) -> Self {
        Self {
            child_id,
            reported_generation,
            reported_attempt,
            current_generation,
            current_attempt,
            exit_kind,
            handled_as,
            observed_at_unix_nanos,
        }
    }
}

/// Accepted but incomplete restart request; pins the old triple until the old attempt leaves.
///
/// `command_id` stores the same UUID bytes as [`crate::control::command::CommandMeta`] to avoid a
/// module cycle between `command` and `outcome`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRestart {
    /// Restart command UUID bound to this request; matches audit `command_id`.
    pub command_id: Uuid,
    /// Human-readable restart initiator string.
    pub requested_by: String,
    /// Human-readable restart reason string.
    pub reason: String,
    /// Old generation that must be pinned when the restart is accepted.
    pub old_generation: Generation,
    /// Old attempt index that must be pinned when the restart is accepted.
    pub old_attempt: ChildStartCount,
    /// Target generation to start after the old attempt exits.
    pub target_generation: Generation,
    /// When the runtime accepted the request, in Unix epoch nanoseconds.
    pub requested_at_unix_nanos: u128,
    /// Graceful stop deadline for the old attempt after cancellation, in Unix epoch nanoseconds.
    pub stop_deadline_at_unix_nanos: u128,
    /// Whether the runtime has requested abort for the old attempt.
    pub abort_requested: bool,
    /// Count of duplicate restart requests merged into this pending request; must not bump generation allocation on merge.
    pub duplicate_request_count: u32,
}

impl PendingRestart {
    /// Creates a pending restart record.
    ///
    /// # Arguments
    ///
    /// Same meaning as the struct fields documented above.
    ///
    /// # Returns
    ///
    /// Returns an owned [`PendingRestart`].
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_id: Uuid,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
        old_generation: Generation,
        old_attempt: ChildStartCount,
        target_generation: Generation,
        requested_at_unix_nanos: u128,
        stop_deadline_at_unix_nanos: u128,
        abort_requested: bool,
        duplicate_request_count: u32,
    ) -> Self {
        Self {
            command_id,
            requested_by: requested_by.into(),
            reason: reason.into(),
            old_generation,
            old_attempt,
            target_generation,
            requested_at_unix_nanos,
            stop_deadline_at_unix_nanos,
            abort_requested,
            duplicate_request_count,
        }
    }
}

/// Minimal generation fence outcome bundled with one restart command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationFenceOutcome {
    /// Discrete fencing decision for this command.
    pub decision: GenerationFenceDecision,
    /// Old generation pinned or observed by this command, or `None`.
    pub old_generation: Option<Generation>,
    /// Old attempt pinned or observed by this command, or `None`.
    pub old_attempt: Option<ChildStartCount>,
    /// Generation planned to start after the old attempt exits.
    pub target_generation: Option<Generation>,
    /// Whether this command newly delivered cancellation to an active attempt.
    pub cancel_delivered: bool,
    /// Whether this command triggered or escalated abort semantics for the old attempt.
    pub abort_requested: bool,
    /// Structured failure when rejected or in conflict; required when `decision` is [`GenerationFenceDecision::Rejected`].
    pub conflict: Option<ChildControlFailure>,
}

impl GenerationFenceOutcome {
    /// Builds a minimal generation fence outcome.
    ///
    /// # Arguments
    ///
    /// - `decision`: Fencing decision for this command.
    /// - `old_generation`: Recorded old generation at decision time, or `None`.
    /// - `old_attempt`: Recorded old attempt at decision time, or `None`.
    /// - `target_generation`: Planned next generation after the old attempt exits, or `None`.
    /// - `cancel_delivered`: Whether cancellation was newly delivered.
    /// - `abort_requested`: Whether abort was requested.
    /// - `conflict`: Optional structured rejection or conflict payload.
    ///
    /// # Returns
    ///
    /// Returns a populated [`GenerationFenceOutcome`].
    pub fn new(
        decision: GenerationFenceDecision,
        old_generation: Option<Generation>,
        old_attempt: Option<ChildStartCount>,
        target_generation: Option<Generation>,
        cancel_delivered: bool,
        abort_requested: bool,
        conflict: Option<ChildControlFailure>,
    ) -> Self {
        Self {
            decision,
            old_generation,
            old_attempt,
            target_generation,
            cancel_delivered,
            abort_requested,
            conflict,
        }
    }
}

/// Generation fence bookkeeping on the runtime side; no timeline, only whether start is allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationFenceState {
    /// Current discrete fencing phase.
    pub phase: GenerationFencePhase,
    /// Active attempt generation from this record perspective, if any.
    pub active_generation: Option<Generation>,
    /// Active attempt index from this record perspective, if any.
    pub active_attempt: Option<ChildStartCount>,
    /// When `Some`, a pending restart is waiting for the old attempt to exit.
    pub pending_restart: Option<PendingRestart>,
    /// Latest recorded stale exit report for diagnostics replay.
    pub last_stale_report: Option<StaleAttemptReport>,
}

impl Default for GenerationFenceState {
    /// Default placeholder: open phase with no pending restart.
    fn default() -> Self {
        Self {
            phase: GenerationFencePhase::Open,
            active_generation: None,
            active_attempt: None,
            pending_restart: None,
            last_stale_report: None,
        }
    }
}

impl GenerationFenceState {
    /// Creates a placeholder fence state record.
    ///
    /// # Arguments
    ///
    /// None.
    ///
    /// # Returns
    ///
    /// Returns the default record with phase [`GenerationFencePhase::Open`].
    pub fn placeholder() -> Self {
        Self::default()
    }
}

/// Pending-restart triple summary shared with [`ChildRuntimeRecord`] and dashboards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRestartSummary {
    /// Pinned or observed old generation.
    pub old_generation: Generation,
    /// Pinned or observed old attempt index.
    pub old_attempt: ChildStartCount,
    /// Target generation expected after the old attempt exits.
    pub target_generation: Generation,
}

impl From<&PendingRestart> for PendingRestartSummary {
    /// Compresses a full [`PendingRestart`] into a dashboard-friendly summary.
    fn from(source: &PendingRestart) -> Self {
        Self {
            old_generation: source.old_generation,
            old_attempt: source.old_attempt,
            target_generation: source.target_generation,
        }
    }
}

/// Runtime restart limit state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestartLimitState {
    /// Restart accounting window.
    pub window: Duration,
    /// Restart limit inside the window.
    pub limit: u32,
    /// Restart count used so far.
    pub used: u32,
    /// Remaining restart count.
    pub remaining: u32,
    /// Whether the restart limit is exhausted.
    pub exhausted: bool,
    /// Last update timestamp in Unix epoch nanoseconds.
    pub updated_at_unix_nanos: u128,
}

impl RestartLimitState {
    /// Creates a restart limit state.
    ///
    /// # Arguments
    ///
    /// - `window`: Restart accounting window.
    /// - `limit`: Restart limit inside the window.
    /// - `used`: Restart count used so far.
    /// - `updated_at_unix_nanos`: Last update timestamp.
    ///
    /// # Returns
    ///
    /// Returns a [`RestartLimitState`] value.
    pub fn new(window: Duration, limit: u32, used: u32, updated_at_unix_nanos: u128) -> Self {
        let remaining = limit.saturating_sub(used);
        Self {
            window,
            limit,
            used,
            remaining,
            exhausted: remaining == 0,
            updated_at_unix_nanos,
        }
    }
}

impl Default for RestartLimitState {
    /// Creates the default restart limit state.
    fn default() -> Self {
        Self::new(Duration::from_secs(60), u32::MAX, 0, 0)
    }
}

/// Liveness state for one child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildLivenessState {
    /// Last heartbeat timestamp in Unix epoch nanoseconds.
    pub last_heartbeat_at_unix_nanos: Option<u128>,
    /// Whether heartbeat is stale.
    pub heartbeat_stale: bool,
    /// Latest readiness state.
    pub readiness: ReadinessState,
}

impl ChildLivenessState {
    /// Creates a child liveness state.
    ///
    /// # Arguments
    ///
    /// - `last_heartbeat_at_unix_nanos`: Last heartbeat timestamp.
    /// - `heartbeat_stale`: Whether heartbeat is stale.
    /// - `readiness`: Latest readiness state.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildLivenessState`] value.
    pub fn new(
        last_heartbeat_at_unix_nanos: Option<u128>,
        heartbeat_stale: bool,
        readiness: ReadinessState,
    ) -> Self {
        Self {
            last_heartbeat_at_unix_nanos,
            heartbeat_stale,
            readiness,
        }
    }
}

/// Public projection of one child runtime state record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildRuntimeRecord {
    /// Stable child identifier.
    pub child_id: ChildId,
    /// Child path in the supervisor tree.
    pub path: SupervisorPath,
    /// Current active generation.
    pub generation: Option<Generation>,
    /// Current active attempt.
    pub attempt: Option<ChildStartCount>,
    /// Current attempt status.
    pub status: Option<ChildAttemptStatus>,
    /// Current control operation.
    pub operation: ChildControlOperation,
    /// Current liveness state.
    pub liveness: ChildLivenessState,
    /// Current restart limit state.
    pub restart_limit: RestartLimitState,
    /// Current stop progress.
    pub stop_state: ChildStopState,
    /// Most recent control failure.
    pub failure: Option<ChildControlFailure>,
    /// Generation fence phase returned with the `CurrentState` projection.
    #[serde(default)]
    pub generation_fence_phase: GenerationFencePhase,
    /// Pending restart triple; present only while the fence queue still waits for the old attempt to exit.
    #[serde(default)]
    pub pending_restart: Option<PendingRestartSummary>,
}

impl ChildRuntimeRecord {
    /// Creates a public child runtime record.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    /// - `path`: Child path in the supervisor tree.
    /// - `generation`: Current active generation.
    /// - `attempt`: Current active attempt.
    /// - `status`: Current attempt status.
    /// - `operation`: Current control operation.
    /// - `liveness`: Current liveness state.
    /// - `restart_limit`: Current restart limit state.
    /// - `stop_state`: Current stop progress.
    /// - `failure`: Most recent control failure.
    /// - `generation_fence_phase`: Projection of generation fencing phase enum.
    /// - `pending_restart`: Optional queued restart fingerprint for dashboards.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildRuntimeRecord`] value.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        child_id: ChildId,
        path: SupervisorPath,
        generation: Option<Generation>,
        attempt: Option<ChildStartCount>,
        status: Option<ChildAttemptStatus>,
        operation: ChildControlOperation,
        liveness: ChildLivenessState,
        restart_limit: RestartLimitState,
        stop_state: ChildStopState,
        failure: Option<ChildControlFailure>,
        generation_fence_phase: GenerationFencePhase,
        pending_restart: Option<PendingRestartSummary>,
    ) -> Self {
        Self {
            child_id,
            path,
            generation,
            attempt,
            status,
            operation,
            liveness,
            restart_limit,
            stop_state,
            failure,
            generation_fence_phase,
            pending_restart,
        }
    }
}

/// Result returned by a child control command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildControlResult {
    /// Stable child identifier.
    pub child_id: ChildId,
    /// Active attempt targeted by the command.
    pub attempt: Option<ChildStartCount>,
    /// Active generation targeted by the command.
    pub generation: Option<Generation>,
    /// Control operation before command handling.
    pub operation_before: ChildControlOperation,
    /// Control operation after command handling.
    pub operation_after: ChildControlOperation,
    /// Current attempt status.
    pub status: Option<ChildAttemptStatus>,
    /// Whether this command delivered cancellation.
    pub cancel_delivered: bool,
    /// Stop progress after command handling.
    pub stop_state: ChildStopState,
    /// Current restart limit state.
    pub restart_limit: RestartLimitState,
    /// Current liveness state.
    pub liveness: ChildLivenessState,
    /// Whether this command reused existing state idempotently.
    pub idempotent: bool,
    /// Current failure reason.
    pub failure: Option<ChildControlFailure>,
    /// Optional generation fencing outcome exclusively used by restart control commands.
    #[serde(default)]
    pub generation_fence: Option<GenerationFenceOutcome>,
    /// Admission conflict detail when a concurrent request was rejected.
    #[serde(default)]
    pub admission_conflict: Option<AdmissionConflict>,
}

impl ChildControlResult {
    /// Creates a child control result.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    /// - `attempt`: Active attempt targeted by the command.
    /// - `generation`: Active generation targeted by the command.
    /// - `operation_before`: Control operation before command handling.
    /// - `operation_after`: Control operation after command handling.
    /// - `status`: Current attempt status.
    /// - `cancel_delivered`: Whether this command delivered cancellation.
    /// - `stop_state`: Stop progress after command handling.
    /// - `restart_limit`: Current restart limit state.
    /// - `liveness`: Current liveness state.
    /// - `idempotent`: Whether this command reused existing state idempotently.
    /// - `failure`: Current failure reason.
    /// - `generation_fence`: Optional restart-only fencing outcome payload.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildControlResult`] value.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        child_id: ChildId,
        attempt: Option<ChildStartCount>,
        generation: Option<Generation>,
        operation_before: ChildControlOperation,
        operation_after: ChildControlOperation,
        status: Option<ChildAttemptStatus>,
        cancel_delivered: bool,
        stop_state: ChildStopState,
        restart_limit: RestartLimitState,
        liveness: ChildLivenessState,
        idempotent: bool,
        failure: Option<ChildControlFailure>,
        generation_fence: Option<GenerationFenceOutcome>,
    ) -> Self {
        Self {
            child_id,
            attempt,
            generation,
            operation_before,
            operation_after,
            status,
            cancel_delivered,
            stop_state,
            restart_limit,
            liveness,
            idempotent,
            failure,
            generation_fence,
            admission_conflict: None,
        }
    }

    /// Creates a conflict result when admission is denied.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child that already has an active attempt.
    /// - `conflict`: Admission conflict detail.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildControlResult`] carrying the conflict.
    pub fn conflict(child_id: ChildId, conflict: AdmissionConflict) -> Self {
        Self {
            child_id,
            attempt: Some(conflict.active_attempt),
            generation: Some(conflict.active_generation),
            operation_before: ChildControlOperation::Active,
            operation_after: ChildControlOperation::Active,
            status: Some(ChildAttemptStatus::Running),
            cancel_delivered: false,
            stop_state: ChildStopState::Idle,
            restart_limit: RestartLimitState::default(),
            liveness: ChildLivenessState::new(None, false, ReadinessState::Unreported),
            idempotent: false,
            failure: None,
            generation_fence: None,
            admission_conflict: Some(conflict),
        }
    }
}
