//! Child control outcome types.

use crate::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use crate::readiness::signal::ReadinessState;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
        }
    }
}
