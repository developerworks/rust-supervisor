//! Runtime control plane lifecycle state.
//!
//! This module stores health state and final exit reports that a
//! `SupervisorHandle` can read repeatedly. It does not execute runtime control
//! loop commands.

use crate::error::types::SupervisorError;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Notify;

/// Stable runtime control plane state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeControlPlaneState {
    /// Control loop task has been created but is not yet accepting commands.
    Starting,
    /// Control loop can still accept commands.
    Alive,
    /// Control plane has received an explicit shutdown request.
    ShuttingDown,
    /// Control loop completed normally.
    Completed,
    /// Control loop failed.
    Failed,
}

impl RuntimeControlPlaneState {
    /// Returns a low-cardinality state label.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a stable state label.
    ///
    /// # Examples
    ///
    /// ```
    /// let state = rust_supervisor::runtime::lifecycle::RuntimeControlPlaneState::Alive;
    /// assert_eq!(state.as_str(), "alive");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Alive => "alive",
            Self::ShuttingDown => "shutting_down",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    /// Returns whether this state is terminal.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when the state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

/// Runtime control loop failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeFailureReason {
    /// Failure phase.
    pub phase: String,
    /// Human-readable failure reason.
    pub reason: String,
    /// Whether the failure came from panic.
    pub panic: bool,
    /// Whether callers can recover by creating a new supervisor.
    pub recoverable: bool,
}

impl RuntimeFailureReason {
    /// Creates a failure reason.
    ///
    /// # Arguments
    ///
    /// - `phase`: Failure phase.
    /// - `reason`: Human-readable reason.
    /// - `panic`: Whether the failure came from panic.
    /// - `recoverable`: Whether a new supervisor can recover.
    ///
    /// # Returns
    ///
    /// Returns a [`RuntimeFailureReason`].
    ///
    /// # Examples
    ///
    /// ```
    /// let failure = rust_supervisor::runtime::lifecycle::RuntimeFailureReason::new(
    ///     "watchdog",
    ///     "runtime control loop panic",
    ///     true,
    ///     true,
    /// );
    /// assert!(failure.panic);
    /// ```
    pub fn new(
        phase: impl Into<String>,
        reason: impl Into<String>,
        panic: bool,
        recoverable: bool,
    ) -> Self {
        Self {
            phase: phase.into(),
            reason: reason.into(),
            panic,
            recoverable,
        }
    }
}

/// Final runtime control loop exit report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeExitReport {
    /// Final state, which must be completed or failed.
    pub state: RuntimeControlPlaneState,
    /// Exit phase.
    pub phase: String,
    /// Human-readable exit reason.
    pub reason: String,
    /// Whether callers can recover by creating a new supervisor.
    pub recoverable: bool,
    /// Final report timestamp in Unix epoch nanoseconds.
    pub completed_at_unix_nanos: u128,
    /// Whether the report came from panic.
    pub panic: bool,
}

impl RuntimeExitReport {
    /// Creates a completed exit report.
    ///
    /// # Arguments
    ///
    /// - `phase`: Completion phase.
    /// - `reason`: Human-readable reason.
    ///
    /// # Returns
    ///
    /// Returns a completed [`RuntimeExitReport`].
    ///
    /// # Examples
    ///
    /// ```
    /// let report = rust_supervisor::runtime::lifecycle::RuntimeExitReport::completed(
    ///     "shutdown",
    ///     "operator requested shutdown",
    /// );
    /// assert_eq!(report.state.as_str(), "completed");
    /// ```
    pub fn completed(phase: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            state: RuntimeControlPlaneState::Completed,
            phase: phase.into(),
            reason: reason.into(),
            recoverable: false,
            completed_at_unix_nanos: unix_nanos_now(),
            panic: false,
        }
    }

    /// Creates a failed exit report.
    ///
    /// # Arguments
    ///
    /// - `phase`: Failure phase.
    /// - `reason`: Human-readable reason.
    /// - `panic`: Whether the failure came from panic.
    /// - `recoverable`: Whether a new supervisor can recover.
    ///
    /// # Returns
    ///
    /// Returns a failed [`RuntimeExitReport`].
    pub fn failed(
        phase: impl Into<String>,
        reason: impl Into<String>,
        panic: bool,
        recoverable: bool,
    ) -> Self {
        Self {
            state: RuntimeControlPlaneState::Failed,
            phase: phase.into(),
            reason: reason.into(),
            recoverable,
            completed_at_unix_nanos: unix_nanos_now(),
            panic,
        }
    }

    /// Converts this report into a health failure reason.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a failure reason when this report represents failure.
    pub fn failure_reason(&self) -> Option<RuntimeFailureReason> {
        (self.state == RuntimeControlPlaneState::Failed).then(|| {
            RuntimeFailureReason::new(
                self.phase.clone(),
                self.reason.clone(),
                self.panic,
                self.recoverable,
            )
        })
    }
}

/// Health report read by runtime callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHealthReport {
    /// Whether the control loop can still accept commands.
    pub alive: bool,
    /// Current control plane state.
    pub state: RuntimeControlPlaneState,
    /// Control plane startup timestamp in Unix epoch nanoseconds.
    pub started_at_unix_nanos: u128,
    /// Last observation timestamp in Unix epoch nanoseconds.
    pub last_observed_at_unix_nanos: u128,
    /// Structured reason for failed state.
    pub failure: Option<RuntimeFailureReason>,
    /// Final report for completed or failed state.
    pub exit_report: Option<RuntimeExitReport>,
}

/// Runtime control plane with repeatable reads.
#[derive(Debug, Clone)]
pub struct RuntimeControlPlane {
    /// Shared inner state.
    inner: Arc<Mutex<RuntimeControlPlaneInner>>,
    /// Terminal state notifier.
    notify: Arc<Notify>,
}

impl RuntimeControlPlane {
    /// Creates new control plane lifecycle state.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`RuntimeControlPlane`] in starting state.
    ///
    /// # Examples
    ///
    /// ```
    /// let control_plane = rust_supervisor::runtime::lifecycle::RuntimeControlPlane::new();
    /// assert!(!control_plane.is_alive());
    /// ```
    pub fn new() -> Self {
        let now = unix_nanos_now();
        Self {
            inner: Arc::new(Mutex::new(RuntimeControlPlaneInner {
                state: RuntimeControlPlaneState::Starting,
                started_at_unix_nanos: now,
                last_observed_at_unix_nanos: now,
                exit_report: None,
                failure: None,
                shutdown_requested_by: None,
                shutdown_reason: None,
            })),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Marks the control loop as accepting commands.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn mark_alive(&self) {
        let mut inner = self.lock_inner();
        if !inner.state.is_terminal() {
            inner.state = RuntimeControlPlaneState::Alive;
            inner.last_observed_at_unix_nanos = unix_nanos_now();
        }
    }

    /// Returns whether the control loop is alive.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when ordinary control commands may be sent.
    pub fn is_alive(&self) -> bool {
        let mut inner = self.lock_inner();
        inner.last_observed_at_unix_nanos = unix_nanos_now();
        inner.state == RuntimeControlPlaneState::Alive
    }

    /// Reads a health report.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`RuntimeHealthReport`] value for the current observation.
    pub fn health(&self) -> RuntimeHealthReport {
        let mut inner = self.lock_inner();
        inner.last_observed_at_unix_nanos = unix_nanos_now();
        RuntimeHealthReport {
            alive: inner.state == RuntimeControlPlaneState::Alive,
            state: inner.state,
            started_at_unix_nanos: inner.started_at_unix_nanos,
            last_observed_at_unix_nanos: inner.last_observed_at_unix_nanos,
            failure: inner.failure.clone(),
            exit_report: inner.exit_report.clone(),
        }
    }

    /// Marks that shutdown has been requested.
    ///
    /// # Arguments
    ///
    /// - `requested_by`: Actor that requested shutdown.
    /// - `reason`: Human-readable shutdown reason.
    ///
    /// # Returns
    ///
    /// Returns an existing final report when the control plane already ended.
    pub fn mark_shutdown_requested(
        &self,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<Option<RuntimeExitReport>, SupervisorError> {
        let requested_by = requested_by.into();
        let reason = reason.into();
        validate_required_text(&requested_by, "requested_by")?;
        validate_required_text(&reason, "reason")?;

        let mut inner = self.lock_inner();
        if let Some(report) = &inner.exit_report {
            return Ok(Some(report.clone()));
        }
        inner.state = RuntimeControlPlaneState::ShuttingDown;
        inner.shutdown_requested_by = Some(requested_by);
        inner.shutdown_reason = Some(reason);
        inner.last_observed_at_unix_nanos = unix_nanos_now();
        Ok(None)
    }

    /// Writes the final exit report.
    ///
    /// # Arguments
    ///
    /// - `report`: Final runtime exit report.
    ///
    /// # Returns
    ///
    /// Returns the cached final report.
    pub fn complete(&self, report: RuntimeExitReport) -> RuntimeExitReport {
        let mut inner = self.lock_inner();
        if let Some(existing) = &inner.exit_report {
            return existing.clone();
        }
        inner.state = report.state;
        inner.failure = report.failure_reason();
        inner.exit_report = Some(report.clone());
        inner.last_observed_at_unix_nanos = report.completed_at_unix_nanos;
        self.notify.notify_waiters();
        report
    }

    /// Returns the cached final exit report.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the final report when the control plane has ended.
    pub fn final_report(&self) -> Option<RuntimeExitReport> {
        self.lock_inner().exit_report.clone()
    }

    /// Waits for the control plane to reach a terminal state.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the cached final [`RuntimeExitReport`].
    pub async fn join(&self) -> RuntimeExitReport {
        loop {
            let notified = self.notify.notified();
            if let Some(report) = self.final_report() {
                return report;
            }
            notified.await;
        }
    }

    /// Acquires the inner state lock.
    fn lock_inner(&self) -> std::sync::MutexGuard<'_, RuntimeControlPlaneInner> {
        self.inner
            .lock()
            .expect("runtime control plane lock poisoned")
    }
}

impl Default for RuntimeControlPlane {
    /// Creates the default runtime control plane.
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime control plane inner state.
#[derive(Debug)]
struct RuntimeControlPlaneInner {
    /// Current state.
    state: RuntimeControlPlaneState,
    /// Startup timestamp.
    started_at_unix_nanos: u128,
    /// Last observation timestamp.
    last_observed_at_unix_nanos: u128,
    /// Final exit report.
    exit_report: Option<RuntimeExitReport>,
    /// Failure reason.
    failure: Option<RuntimeFailureReason>,
    /// Shutdown requester.
    shutdown_requested_by: Option<String>,
    /// Shutdown reason.
    shutdown_reason: Option<String>,
}

/// Validates required text.
fn validate_required_text(value: &str, field: &str) -> Result<(), SupervisorError> {
    if value.trim().is_empty() {
        return Err(SupervisorError::InvalidTransition {
            message: format!("runtime control plane {field} must not be empty"),
        });
    }
    Ok(())
}

/// Returns current Unix epoch nanoseconds.
fn unix_nanos_now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}
