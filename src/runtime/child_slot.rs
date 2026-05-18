//! ChildSlot placed on each supervised child runtime identity.
//!
//! A [`ChildSlot`] owns the live handles (cancellation token, join handle) for at
//! most one active attempt at any moment. The control loop manipulates slots
//! through the methods defined here rather than rewriting in-memory labels.

use crate::child_runner::runner::{ChildRunHandle, ChildRunReport, wait_for_report};
use crate::control::outcome::{
    ChildAttemptStatus, ChildControlFailure, ChildControlOperation, ChildLivenessState,
    ChildRuntimeRecord, ChildStopState, GenerationFenceState, RestartLimitState,
};
use crate::error::types::SupervisorError;
use crate::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use crate::readiness::signal::ReadinessState;
use serde::Serialize;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;
use tokio::task::AbortHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Shared types (migrated from child_runtime_state)
// ---------------------------------------------------------------------------

/// Default heartbeat stale threshold in seconds.
pub const DEFAULT_HEARTBEAT_TIMEOUT_SECS: u64 = 5;

/// Restart accounting history for one child runtime slot.
#[derive(Debug, Clone, Default)]
pub struct RestartLimitTracker {
    /// Failure timestamps that are still relevant to the restart window.
    failure_timestamps: VecDeque<u128>,
}

impl RestartLimitTracker {
    /// Creates an empty restart limit tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Refreshes accounting and optionally records the current failed exit.
    ///
    /// # Arguments
    ///
    /// - `now_unix_nanos`: Current Unix timestamp in nanoseconds.
    /// - `window`: Restart accounting window.
    /// - `count_failure`: Whether the current exit should consume the limit.
    ///
    /// # Returns
    ///
    /// Returns the number of failures inside the active window.
    pub fn refresh(&mut self, now_unix_nanos: u128, window: Duration, count_failure: bool) -> u32 {
        self.prune(now_unix_nanos, window);
        if count_failure {
            self.failure_timestamps.push_back(now_unix_nanos);
        }
        self.failure_timestamps.len().min(u32::MAX as usize) as u32
    }

    /// Removes failure timestamps outside the accounting window.
    fn prune(&mut self, now_unix_nanos: u128, window: Duration) {
        let window_nanos = window.as_nanos();
        while self
            .failure_timestamps
            .front()
            .is_some_and(|timestamp| now_unix_nanos.saturating_sub(*timestamp) > window_nanos)
        {
            self.failure_timestamps.pop_front();
        }
    }
}

/// Runtime time base used to convert monotonic instants into Unix timestamps.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeTimeBase {
    /// Monotonic instant captured when the runtime starts.
    pub base_instant: Instant,
    /// Unix epoch timestamp in nanoseconds captured when the runtime starts.
    pub base_unix_nanos: u128,
}

impl RuntimeTimeBase {
    /// Creates a runtime time base.
    pub fn new() -> Self {
        Self {
            base_instant: Instant::now(),
            base_unix_nanos: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos()),
        }
    }

    /// Returns the current Unix epoch timestamp in nanoseconds.
    pub fn now_unix_nanos(&self) -> u128 {
        self.instant_to_unix_nanos(Instant::now())
    }

    /// Converts a monotonic instant into a Unix epoch timestamp in nanoseconds.
    ///
    /// # Arguments
    ///
    /// - `instant`: Monotonic instant to convert.
    pub fn instant_to_unix_nanos(&self, instant: Instant) -> u128 {
        if instant >= self.base_instant {
            self.base_unix_nanos
                .saturating_add(instant.duration_since(self.base_instant).as_nanos())
        } else {
            self.base_unix_nanos
                .saturating_sub(self.base_instant.duration_since(instant).as_nanos())
        }
    }
}

impl Default for RuntimeTimeBase {
    /// Creates the default runtime time base.
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ChildExitSummary
// ---------------------------------------------------------------------------

/// Summary recorded when a child attempt exits.
#[derive(Debug, Clone, Serialize)]
pub struct ChildExitSummary {
    /// Process exit code when available.
    pub exit_code: Option<i32>,
    /// Human-readable exit reason.
    pub exit_reason: String,
    /// Unix epoch timestamp in nanoseconds when the exit was recorded.
    pub exited_at_unix_nanos: u128,
}

impl ChildExitSummary {
    /// Creates an exit summary from a [`ChildRunReport`].
    ///
    /// # Arguments
    ///
    /// - `report`: Completed child run report.
    /// - `exited_at_unix_nanos`: Timestamp when the exit was observed.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildExitSummary`].
    pub fn from_report(report: &ChildRunReport, exited_at_unix_nanos: u128) -> Self {
        let exit_reason = match &report.exit {
            crate::child_runner::run_exit::TaskExit::Succeeded => "succeeded".to_owned(),
            crate::child_runner::run_exit::TaskExit::Cancelled => "cancelled".to_owned(),
            crate::child_runner::run_exit::TaskExit::Failed(f) => f.message.clone(),
            crate::child_runner::run_exit::TaskExit::Panicked(msg) => format!("panicked: {msg}"),
            crate::child_runner::run_exit::TaskExit::TimedOut => "timed out".to_owned(),
        };
        Self {
            exit_code: None,
            exit_reason,
            exited_at_unix_nanos,
        }
    }
}

impl Display for ChildExitSummary {
    /// Formats the exit summary as `code=<code> reason=<reason>`.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self.exit_code {
            Some(code) => write!(formatter, "code={} reason={}", code, self.exit_reason),
            None => write!(formatter, "reason={}", self.exit_reason),
        }
    }
}

// ---------------------------------------------------------------------------
// ChildSlot
// ---------------------------------------------------------------------------

/// Runtime slot for one supervised child.
///
/// At most one active attempt may occupy the slot at any moment. The slot owns
/// the cancellation token, abort handle, and completion/health receivers for
/// the active attempt.
#[derive(Debug, Serialize)]
pub struct ChildSlot {
    /// Stable child identifier.
    pub child_id: ChildId,
    /// Child path in the supervisor tree.
    pub path: SupervisorPath,
    /// Current active attempt status.
    pub status: ChildAttemptStatus,
    /// Current control operation requested by the operator.
    pub operation: ChildControlOperation,
    /// Generation of the active attempt.
    pub generation: Option<Generation>,
    /// Monotonic attempt number for the active attempt.
    pub attempt: Option<ChildStartCount>,
    /// Cumulative restart count across all generations.
    pub restart_count: u64,
    /// Cancellation token for the active attempt (runtime-only, not serialized).
    #[serde(skip)]
    pub cancellation_token: Option<CancellationToken>,
    /// Abort handle for the active attempt (runtime-only, not serialized).
    #[serde(skip)]
    pub abort_handle: Option<AbortHandle>,
    /// Completion receiver for the active attempt (runtime-only, not serialized).
    #[serde(skip)]
    pub completion_receiver:
        Option<watch::Receiver<Option<Result<ChildRunReport, SupervisorError>>>>,
    /// Heartbeat receiver for the active attempt (runtime-only, not serialized).
    #[serde(skip)]
    pub heartbeat_receiver: Option<watch::Receiver<Option<Instant>>>,
    /// Readiness receiver for the active attempt (runtime-only, not serialized).
    #[serde(skip)]
    pub readiness_receiver: Option<watch::Receiver<ReadinessState>>,
    /// Summary of the most recent exit, if any.
    pub last_exit: Option<ChildExitSummary>,
    /// Unix epoch timestamp in nanoseconds when the child last reported ready.
    pub last_ready_at: Option<u128>,
    /// Unix epoch timestamp in nanoseconds of the last observed heartbeat.
    pub last_heartbeat_at: Option<u128>,
    /// Restart accounting window duration.
    pub restart_window: Duration,
    /// Whether a restart is pending but not yet activated.
    pub pending_restart: bool,
    /// Whether cancellation has been delivered to the active attempt.
    pub attempt_cancel_delivered: bool,
    /// Whether abort has been requested for the active attempt.
    pub abort_requested: bool,
    // --- Fields migrated from ChildRuntimeState for compatibility ---
    /// Current restart limit state.
    #[serde(skip)]
    pub restart_limit: RestartLimitState,
    /// Runtime-side restart accounting history.
    #[serde(skip)]
    pub restart_limit_tracker: RestartLimitTracker,
    /// Current stop progress.
    pub stop_state: ChildStopState,
    /// Stop deadline in Unix epoch nanoseconds.
    pub stop_deadline_at_unix_nanos: Option<u128>,
    /// Most recent control failure.
    pub last_control_failure: Option<ChildControlFailure>,
    /// Attempt for the most recent stale heartbeat event.
    pub stale_event_attempt: Option<ChildStartCount>,
    /// Generation fencing state for restart coordination.
    #[serde(skip)]
    pub generation_fence: GenerationFenceState,
    /// Registry identity anchor captured before a fenced restart.
    #[serde(skip)]
    pub registry_identity_anchor_for_spawn_attempt: Option<(Generation, ChildStartCount, u64)>,
    /// Last observed readiness state.
    #[serde(skip)]
    pub last_observed_readiness: ReadinessState,
}

impl ChildSlot {
    /// Creates an empty slot with no active attempt.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    /// - `path`: Child path in the supervisor tree.
    /// - `restart_window`: Restart accounting window duration.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildSlot`] in idle state.
    pub fn new(child_id: ChildId, path: SupervisorPath, restart_window: Duration) -> Self {
        Self {
            child_id,
            path,
            status: ChildAttemptStatus::Stopped,
            operation: ChildControlOperation::Active,
            generation: None,
            attempt: None,
            restart_count: 0,
            cancellation_token: None,
            abort_handle: None,
            completion_receiver: None,
            heartbeat_receiver: None,
            readiness_receiver: None,
            last_exit: None,
            last_ready_at: None,
            last_heartbeat_at: None,
            restart_window,
            pending_restart: false,
            attempt_cancel_delivered: false,
            abort_requested: false,
            restart_limit: RestartLimitState::default(),
            restart_limit_tracker: RestartLimitTracker::new(),
            stop_state: ChildStopState::NoActiveAttempt,
            stop_deadline_at_unix_nanos: None,
            last_control_failure: None,
            stale_event_attempt: None,
            generation_fence: GenerationFenceState::placeholder(),
            registry_identity_anchor_for_spawn_attempt: None,
            last_observed_readiness: ReadinessState::Unreported,
        }
    }

    /// Creates an empty slot with a default 60-second restart window.
    ///
    /// Convenience constructor for [`ChildSlot::new`] when the restart window
    /// is not yet known.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    /// - `path`: Child path in the supervisor tree.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildSlot`] in idle state.
    pub fn new_placeholder(child_id: ChildId, path: SupervisorPath) -> Self {
        Self::new(child_id, path, Duration::from_secs(60))
    }

    /// Activates an attempt on this slot.
    ///
    /// # Arguments
    ///
    /// - `generation`: Generation owned by the active attempt.
    /// - `attempt`: Monotonic attempt number.
    /// - `status`: Initial active attempt status.
    /// - `handle`: Child run handle carrying cancellation token and receivers.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn activate(
        &mut self,
        generation: Generation,
        attempt: ChildStartCount,
        status: ChildAttemptStatus,
        handle: ChildRunHandle,
    ) {
        self.generation = Some(generation);
        self.attempt = Some(attempt);
        self.status = status;
        self.generation_fence.active_generation = Some(generation);
        self.generation_fence.active_attempt = Some(attempt);
        self.cancellation_token = Some(handle.cancellation_token);
        self.abort_handle = Some(handle.abort_handle);
        self.completion_receiver = Some(handle.completion_receiver);
        self.heartbeat_receiver = Some(handle.heartbeat_receiver);
        self.readiness_receiver = Some(handle.readiness_receiver);
        self.last_exit = None;
        self.last_ready_at = None;
        self.last_heartbeat_at = None;
        self.last_observed_readiness = ReadinessState::Unreported;
        self.attempt_cancel_delivered = false;
        self.abort_requested = false;
        self.pending_restart = false;
        self.stop_state = ChildStopState::Idle;
        self.stop_deadline_at_unix_nanos = None;
        self.last_control_failure = None;
        self.stale_event_attempt = None;
        self.registry_identity_anchor_for_spawn_attempt = None;
        self.generation_fence.phase = GenerationFenceState::placeholder().phase;
    }

    /// Deactivates the current attempt and records its exit summary.
    ///
    /// The caller must have already awaited or consumed the completion
    /// receiver. This method clears handles and advances the restart counter.
    ///
    /// # Arguments
    ///
    /// - `exit_summary`: Summary captured from the completed child run.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn deactivate(&mut self, exit_summary: ChildExitSummary) {
        self.last_exit = Some(exit_summary);
        self.restart_count = self.restart_count.saturating_add(1);
        self.generation = None;
        self.attempt = None;
        self.status = ChildAttemptStatus::Stopped;
        self.cancellation_token = None;
        self.abort_handle = None;
        self.completion_receiver = None;
        self.heartbeat_receiver = None;
        self.readiness_receiver = None;
        self.last_ready_at = None;
        self.last_heartbeat_at = None;
        self.attempt_cancel_delivered = false;
        self.abort_requested = false;
        self.pending_restart = false;
        self.generation_fence.active_generation = None;
        self.generation_fence.active_attempt = None;
        self.stop_state = ChildStopState::NoActiveAttempt;
        self.stop_deadline_at_unix_nanos = None;
        self.stale_event_attempt = None;
        self.registry_identity_anchor_for_spawn_attempt = None;
    }

    /// Clears the active instance without recording an exit (migration
    /// compatibility with [`ChildRuntimeState::clear_instance`]).
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn clear_instance(&mut self) {
        self.generation = None;
        self.attempt = None;
        self.status = ChildAttemptStatus::Stopped;
        self.generation_fence.active_generation = None;
        self.generation_fence.active_attempt = None;
        self.cancellation_token = None;
        self.abort_handle = None;
        self.completion_receiver = None;
        self.heartbeat_receiver = None;
        self.readiness_receiver = None;
        self.attempt_cancel_delivered = false;
        self.abort_requested = false;
        self.stop_deadline_at_unix_nanos = None;
        self.stale_event_attempt = None;
        self.registry_identity_anchor_for_spawn_attempt = None;
        self.stop_state = ChildStopState::NoActiveAttempt;
    }

    /// Returns whether the slot currently holds an active attempt.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when an active attempt exists.
    pub fn has_active_attempt(&self) -> bool {
        self.attempt.is_some() && self.cancellation_token.is_some()
    }

    /// Delivers cancellation to the active attempt.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when this call delivered cancellation (first delivery).
    pub fn cancel(&mut self) -> bool {
        let Some(token) = &self.cancellation_token else {
            return false;
        };
        if self.attempt_cancel_delivered {
            return false;
        }
        token.cancel();
        self.attempt_cancel_delivered = true;
        self.status = ChildAttemptStatus::Cancelling;
        true
    }

    /// Requests abort for the active attempt.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when this call requested abort (first request).
    pub fn abort(&mut self) -> bool {
        let Some(handle) = &self.abort_handle else {
            return false;
        };
        if self.abort_requested {
            return false;
        }
        handle.abort();
        self.abort_requested = true;
        true
    }

    /// Waits for the active attempt report.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the completed child run report.
    pub async fn wait_for_report(&mut self) -> Result<ChildRunReport, SupervisorError> {
        let Some(receiver) = &mut self.completion_receiver else {
            return Err(SupervisorError::InvalidTransition {
                message: "child slot has no active completion receiver".to_owned(),
            });
        };
        wait_for_report(receiver).await
    }

    /// Observes current readiness and heartbeat from the active attempt.
    ///
    /// # Arguments
    ///
    /// - `now_unix_nanos`: Current Unix epoch timestamp in nanoseconds.
    ///
    /// # Returns
    ///
    /// Returns the latest [`ChildLivenessState`].
    pub fn observe_liveness(&mut self, now_unix_nanos: u128) -> ChildLivenessState {
        if let Some(receiver) = &self.heartbeat_receiver {
            let heartbeat = *receiver.borrow();
            if heartbeat.is_some() {
                self.last_heartbeat_at = Some(now_unix_nanos);
            }
        }
        let readiness = if let Some(receiver) = &self.readiness_receiver {
            let r = *receiver.borrow();
            if r == ReadinessState::Ready {
                self.last_ready_at = Some(now_unix_nanos);
            }
            r
        } else {
            ReadinessState::Unreported
        };
        let heartbeat_stale = self.last_heartbeat_at.is_some_and(|heartbeat| {
            let elapsed_nanos = now_unix_nanos.saturating_sub(heartbeat);
            elapsed_nanos >= Duration::from_secs(DEFAULT_HEARTBEAT_TIMEOUT_SECS).as_nanos()
        });
        ChildLivenessState::new(self.last_heartbeat_at, heartbeat_stale, readiness)
    }

    /// Updates restart limit state (migration compatibility with
    /// [`ChildRuntimeState::update_restart_limit`]).
    ///
    /// # Arguments
    ///
    /// - `window`: Restart accounting window.
    /// - `limit`: Restart limit inside the window.
    /// - `used`: Restart count used so far.
    /// - `time_base`: Runtime time base.
    ///
    /// # Returns
    ///
    /// Returns the updated [`RestartLimitState`].
    pub fn update_restart_limit(
        &mut self,
        window: Duration,
        limit: u32,
        used: u32,
        time_base: &RuntimeTimeBase,
    ) -> RestartLimitState {
        let mut updated_at = time_base.now_unix_nanos();
        if updated_at <= self.restart_limit.updated_at_unix_nanos {
            updated_at = self.restart_limit.updated_at_unix_nanos.saturating_add(1);
        }
        self.restart_limit = RestartLimitState {
            window,
            limit,
            used,
            remaining: limit.saturating_sub(used),
            exhausted: used >= limit,
            updated_at_unix_nanos: updated_at,
        };
        self.restart_limit.clone()
    }

    /// Refreshes the restart limit tracker and updates the state (migration
    /// compatibility).
    ///
    /// # Arguments
    ///
    /// - `window`: Restart accounting window.
    /// - `limit`: Restart limit inside the window.
    /// - `count_failure`: Whether the current exit counts as a failure.
    /// - `time_base`: Runtime time base.
    ///
    /// # Returns
    ///
    /// Returns the updated [`RestartLimitState`].
    pub fn refresh_restart_limit(
        &mut self,
        window: Duration,
        limit: u32,
        count_failure: bool,
        time_base: &RuntimeTimeBase,
    ) -> RestartLimitState {
        let now = time_base.now_unix_nanos();
        let used = self
            .restart_limit_tracker
            .refresh(now, window, count_failure);
        self.update_restart_limit(window, limit, used, time_base)
    }

    /// Builds a public runtime state record (migration compatibility with
    /// [`ChildRuntimeState::to_record`]).
    ///
    /// # Arguments
    ///
    /// - `liveness`: Liveness state observed by the caller.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildRuntimeRecord`].
    pub fn to_record(&self, liveness: ChildLivenessState) -> ChildRuntimeRecord {
        ChildRuntimeRecord::new(
            self.child_id.clone(),
            self.path.clone(),
            self.generation,
            self.attempt,
            Some(self.status),
            self.operation,
            liveness,
            self.restart_limit.clone(),
            self.stop_state,
            self.last_control_failure.clone(),
            self.generation_fence.phase,
            None, // pending_restart
        )
    }
}
