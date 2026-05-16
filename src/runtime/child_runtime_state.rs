//! Child runtime state records.

use crate::child_runner::runner::{ChildRunHandle, ChildRunReport, wait_for_report};
use crate::control::outcome::{
    ChildAttemptStatus, ChildControlFailure, ChildControlOperation, ChildLivenessState,
    ChildRuntimeRecord, ChildStopState, GenerationFencePhase, GenerationFenceState,
    PendingRestartSummary, RestartLimitState,
};
use crate::error::types::SupervisorError;
use crate::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use crate::readiness::signal::ReadinessState;
use crate::registry::entry::ChildRuntimeStatus;
use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;
use tokio::task::AbortHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

/// Default heartbeat stale threshold in seconds.
pub const DEFAULT_HEARTBEAT_TIMEOUT_SECS: u64 = 5;

/// Restart accounting history for one child runtime state record.
#[derive(Debug, Clone, Default)]
pub struct RestartLimitTracker {
    /// Failure timestamps that are still relevant to the restart window.
    failure_timestamps: VecDeque<u128>,
}

impl RestartLimitTracker {
    /// Creates an empty restart limit tracker.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`RestartLimitTracker`] without recorded failures.
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

    /// Removes failure timestamps that are outside the accounting window.
    ///
    /// # Arguments
    ///
    /// - `now_unix_nanos`: Current Unix timestamp in nanoseconds.
    /// - `window`: Restart accounting window.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
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
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`RuntimeTimeBase`] value.
    pub fn new() -> Self {
        Self {
            base_instant: Instant::now(),
            base_unix_nanos: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos()),
        }
    }

    /// Returns the current Unix epoch timestamp in nanoseconds.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the current nanosecond timestamp.
    pub fn now_unix_nanos(&self) -> u128 {
        self.instant_to_unix_nanos(Instant::now())
    }

    /// Converts a monotonic instant into a Unix epoch timestamp in nanoseconds.
    ///
    /// # Arguments
    ///
    /// - `instant`: Monotonic instant that should be converted.
    ///
    /// # Returns
    ///
    /// Returns a Unix epoch timestamp in nanoseconds.
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

/// Runtime state record for one child.
#[derive(Debug)]
pub struct ChildRuntimeState {
    /// Stable child identifier.
    pub child_id: ChildId,
    /// Child path in the supervisor tree.
    pub path: SupervisorPath,
    /// Current active generation.
    pub generation: Option<Generation>,
    /// Current active attempt.
    pub attempt: Option<ChildStartCount>,
    /// Current active attempt status.
    pub status: Option<ChildAttemptStatus>,
    /// Current control operation.
    pub operation: ChildControlOperation,
    /// Cancellation token for the active attempt.
    pub cancellation_token: Option<CancellationToken>,
    /// Abort handle for the active attempt.
    pub abort_handle: Option<AbortHandle>,
    /// Completion receiver for the active attempt.
    pub completion_receiver:
        Option<watch::Receiver<Option<Result<ChildRunReport, SupervisorError>>>>,
    /// Heartbeat receiver for the active attempt.
    pub heartbeat_receiver: Option<watch::Receiver<Option<Instant>>>,
    /// Readiness receiver for the active attempt.
    pub readiness_receiver: Option<watch::Receiver<ReadinessState>>,
    /// Last observed heartbeat timestamp in Unix epoch nanoseconds.
    pub last_observed_heartbeat_at_unix_nanos: Option<u128>,
    /// Last observed readiness state.
    pub last_observed_readiness: ReadinessState,
    /// Current restart limit state.
    pub restart_limit: RestartLimitState,
    /// Runtime-side restart accounting history.
    pub restart_limit_tracker: RestartLimitTracker,
    /// Whether cancellation has been delivered to the active attempt.
    pub attempt_cancel_delivered: bool,
    /// Whether abort has been requested for the active attempt.
    pub abort_requested: bool,
    /// Current stop progress.
    pub stop_state: ChildStopState,
    /// Stop deadline in Unix epoch nanoseconds.
    pub stop_deadline_at_unix_nanos: Option<u128>,
    /// Most recent control failure.
    pub last_control_failure: Option<ChildControlFailure>,
    /// Attempt for the most recent stale heartbeat event.
    pub stale_event_attempt: Option<ChildStartCount>,
    /// Generation fencing state for restart coordination.
    pub generation_fence: GenerationFenceState,
    /// Captured [`ChildRuntime`] identifiers registered immediately before a fenced restart advances the registry so spawn failures restore the superseded bookkeeping.
    pub registry_identity_anchor_for_spawn_attempt: Option<(Generation, ChildStartCount, u64)>,
}

impl ChildRuntimeState {
    /// Creates a runtime state record without an active attempt.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    /// - `path`: Child path in the supervisor tree.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildRuntimeState`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let state = rust_supervisor::runtime::child_runtime_state::ChildRuntimeState::new_placeholder(
    ///     rust_supervisor::id::types::ChildId::new("worker"),
    ///     rust_supervisor::id::types::SupervisorPath::root().join("worker"),
    /// );
    /// assert!(state.attempt.is_none());
    /// ```
    pub fn new_placeholder(child_id: ChildId, path: SupervisorPath) -> Self {
        Self {
            child_id,
            path,
            generation: None,
            attempt: None,
            status: None,
            operation: ChildControlOperation::Active,
            cancellation_token: None,
            abort_handle: None,
            completion_receiver: None,
            heartbeat_receiver: None,
            readiness_receiver: None,
            last_observed_heartbeat_at_unix_nanos: None,
            last_observed_readiness: ReadinessState::Unreported,
            restart_limit: RestartLimitState::default(),
            restart_limit_tracker: RestartLimitTracker::new(),
            attempt_cancel_delivered: false,
            abort_requested: false,
            stop_state: ChildStopState::NoActiveAttempt,
            stop_deadline_at_unix_nanos: None,
            last_control_failure: None,
            stale_event_attempt: None,
            generation_fence: GenerationFenceState::placeholder(),
            registry_identity_anchor_for_spawn_attempt: None,
        }
    }

    /// Activates an attempt on this runtime state record.
    ///
    /// # Arguments
    ///
    /// - `generation`: Generation owned by the active attempt.
    /// - `attempt`: Active attempt number.
    /// - `status`: Initial active attempt status.
    /// - `handle`: Child run handle.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn activate_instance(
        &mut self,
        generation: Generation,
        attempt: ChildStartCount,
        status: ChildAttemptStatus,
        handle: ChildRunHandle,
    ) {
        self.generation = Some(generation);
        self.attempt = Some(attempt);
        self.status = Some(status);
        self.generation_fence.active_generation = Some(generation);
        self.generation_fence.active_attempt = Some(attempt);
        self.cancellation_token = Some(handle.cancellation_token);
        self.abort_handle = Some(handle.abort_handle);
        self.completion_receiver = Some(handle.completion_receiver);
        self.heartbeat_receiver = Some(handle.heartbeat_receiver);
        self.readiness_receiver = Some(handle.readiness_receiver);
        self.last_observed_heartbeat_at_unix_nanos = None;
        self.last_observed_readiness = ReadinessState::Unreported;
        self.attempt_cancel_delivered = false;
        self.abort_requested = false;
        self.stop_state = ChildStopState::Idle;
        self.stop_deadline_at_unix_nanos = None;
        self.last_control_failure = None;
        self.stale_event_attempt = None;
        self.registry_identity_anchor_for_spawn_attempt = None;
        self.generation_fence.phase = GenerationFencePhase::Open;
    }
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
        self.status = None;
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

    /// Returns whether the record has an active attempt.
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
    /// Returns `true` when this call delivered cancellation.
    pub fn cancel(&mut self) -> bool {
        let Some(token) = &self.cancellation_token else {
            self.stop_state = ChildStopState::NoActiveAttempt;
            return false;
        };
        if self.attempt_cancel_delivered {
            return false;
        }
        token.cancel();
        self.attempt_cancel_delivered = true;
        self.status = Some(ChildAttemptStatus::Cancelling);
        self.stop_state = ChildStopState::CancelDelivered;
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
    /// Returns `true` when this call requested abort.
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
                message: "child runtime state has no active completion receiver".to_owned(),
            });
        };
        wait_for_report(receiver).await
    }

    /// Observes current liveness for the active attempt.
    ///
    /// # Arguments
    ///
    /// - `time_base`: Runtime time base.
    ///
    /// # Returns
    ///
    /// Returns the latest [`ChildLivenessState`] value.
    pub fn observe_liveness(&mut self, time_base: &RuntimeTimeBase) -> ChildLivenessState {
        if let Some(receiver) = &self.heartbeat_receiver {
            let heartbeat = *receiver.borrow();
            self.last_observed_heartbeat_at_unix_nanos =
                heartbeat.map(|instant| time_base.instant_to_unix_nanos(instant));
        }
        if let Some(receiver) = &self.readiness_receiver {
            self.last_observed_readiness = *receiver.borrow();
        }
        let heartbeat_stale = self
            .last_observed_heartbeat_at_unix_nanos
            .is_some_and(|heartbeat| {
                let elapsed_nanos = time_base.now_unix_nanos().saturating_sub(heartbeat);
                elapsed_nanos >= Duration::from_secs(DEFAULT_HEARTBEAT_TIMEOUT_SECS).as_nanos()
            });
        ChildLivenessState::new(
            self.last_observed_heartbeat_at_unix_nanos,
            heartbeat_stale,
            self.last_observed_readiness,
        )
    }

    /// Updates restart limit state.
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
    /// Returns the updated [`RestartLimitState`] value.
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
        self.restart_limit = RestartLimitState::new(window, limit, used, updated_at);
        self.restart_limit.clone()
    }

    /// Refreshes restart limit state from runtime accounting history.
    ///
    /// # Arguments
    ///
    /// - `window`: Restart accounting window.
    /// - `limit`: Restart limit inside the window.
    /// - `count_failure`: Whether the current exit should consume the limit.
    /// - `time_base`: Runtime time base.
    ///
    /// # Returns
    ///
    /// Returns the updated [`RestartLimitState`] value.
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

    /// Builds a public runtime state record.
    ///
    /// # Arguments
    ///
    /// - `liveness`: Liveness state observed by the caller.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildRuntimeRecord`] value.
    pub fn to_record(&self, liveness: ChildLivenessState) -> ChildRuntimeRecord {
        ChildRuntimeRecord::new(
            self.child_id.clone(),
            self.path.clone(),
            self.generation,
            self.attempt,
            self.status,
            self.operation,
            liveness,
            self.restart_limit.clone(),
            self.stop_state,
            self.last_control_failure.clone(),
            self.generation_fence.phase,
            self.generation_fence
                .pending_restart
                .as_ref()
                .map(PendingRestartSummary::from),
        )
    }
}

/// Maps a registry status into a public attempt status.
///
/// # Arguments
///
/// - `status`: Status stored in the registry.
///
/// # Returns
///
/// Returns the public child attempt status.
pub fn child_attempt_status_from_runtime(status: ChildRuntimeStatus) -> ChildAttemptStatus {
    match status {
        ChildRuntimeStatus::Registered | ChildRuntimeStatus::Starting => {
            ChildAttemptStatus::Starting
        }
        ChildRuntimeStatus::Running => ChildAttemptStatus::Running,
        ChildRuntimeStatus::Ready => ChildAttemptStatus::Ready,
        ChildRuntimeStatus::Exited => ChildAttemptStatus::Stopped,
    }
}
