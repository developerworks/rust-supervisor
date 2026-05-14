//! Shutdown pipeline report types.
//!
//! This module owns serializable public report types for real shutdown
//! execution. Runtime code fills these values without making the shutdown
//! module depend on runtime internals.

use crate::child_runner::run_exit::TaskExit;
use crate::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use crate::shutdown::stage::{ShutdownCause, ShutdownPhase};
use serde::{Deserialize, Serialize};

/// Final status for one child during shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildShutdownStatus {
    /// The child had no running child_start_count when shutdown began.
    AlreadyExited,
    /// The child returned before the graceful timeout expired.
    Graceful,
    /// The child completed after runtime abort was requested.
    Aborted,
    /// The child did not complete after runtime abort was requested.
    AbortFailed,
    /// The child reported completion after the expected phase.
    LateReport,
}

/// Shutdown outcome for one child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildShutdownOutcome {
    /// Stable child identifier.
    pub child_id: ChildId,
    /// Child path in the supervisor tree.
    pub path: SupervisorPath,
    /// Runtime slot generation observed during shutdown.
    pub generation: Generation,
    /// ChildStartCount number observed during shutdown.
    pub child_start_count: ChildStartCount,
    /// Final shutdown status for the child.
    pub status: ChildShutdownStatus,
    /// Whether runtime delivered cancellation to the running child_start_count.
    pub cancel_delivered: bool,
    /// Exit classification when one was available.
    pub exit: Option<TaskExit>,
    /// Shutdown phase where this outcome became final.
    pub phase: ShutdownPhase,
    /// Human-readable diagnostic reason.
    pub reason: String,
}

/// Constructor input for one child shutdown outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildShutdownOutcomeInput {
    /// Stable child identifier.
    pub child_id: ChildId,
    /// Child path in the supervisor tree.
    pub path: SupervisorPath,
    /// Runtime slot generation observed during shutdown.
    pub generation: Generation,
    /// Runtime child_start_count number observed during shutdown.
    pub child_start_count: ChildStartCount,
    /// Final shutdown status for the child.
    pub status: ChildShutdownStatus,
    /// Whether runtime delivered cancellation to the running child_start_count.
    pub cancel_delivered: bool,
    /// Exit classification when one was available.
    pub exit: Option<TaskExit>,
    /// Shutdown phase where this outcome became final.
    pub phase: ShutdownPhase,
    /// Human-readable diagnostic reason.
    pub reason: String,
}

impl ChildShutdownOutcome {
    /// Builds a child shutdown outcome.
    ///
    /// # Arguments
    ///
    /// - `input`: Constructor input for one child shutdown outcome.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildShutdownOutcome`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::{ChildStartCount, ChildId, Generation, SupervisorPath};
    /// use rust_supervisor::shutdown::report::{
    ///     ChildShutdownOutcome, ChildShutdownOutcomeInput, ChildShutdownStatus,
    /// };
    /// use rust_supervisor::shutdown::stage::ShutdownPhase;
    ///
    /// let outcome = ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
    ///     child_id: ChildId::new("worker"),
    ///     path: SupervisorPath::root().join("worker"),
    ///     generation: Generation::initial(),
    ///     child_start_count: ChildStartCount::first(),
    ///     status: ChildShutdownStatus::Graceful,
    ///     cancel_delivered: true,
    ///     exit: None,
    ///     phase: ShutdownPhase::GracefulDrain,
    ///     reason: "child completed before timeout".to_owned(),
    /// });
    ///
    /// assert_eq!(outcome.child_id.value, "worker");
    /// ```
    pub fn new(input: ChildShutdownOutcomeInput) -> Self {
        Self {
            child_id: input.child_id,
            path: input.path,
            generation: input.generation,
            child_start_count: input.child_start_count,
            status: input.status,
            cancel_delivered: input.cancel_delivered,
            exit: input.exit,
            phase: input.phase,
            reason: input.reason,
        }
    }
}

/// Resource reconciliation status after shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceReconcileStatus {
    /// The runtime cleaned the resource.
    Cleaned,
    /// The runtime or observability layer recorded the resource fact.
    Recorded,
    /// The resource is outside core runtime ownership.
    NotOwned,
    /// Reconciliation failed and warnings explain why.
    Failed,
}

/// Resource reconciliation summary after shutdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownReconcileReport {
    /// Registry reconciliation status.
    pub registry_status: ResourceReconcileStatus,
    /// Runtime handle reconciliation status.
    pub runtime_handle_status: ResourceReconcileStatus,
    /// Journal reconciliation status.
    pub journal_status: ResourceReconcileStatus,
    /// Metrics reconciliation status.
    pub metrics_status: ResourceReconcileStatus,
    /// Socket reconciliation status.
    pub socket_status: ResourceReconcileStatus,
    /// Non-fatal reconciliation warnings.
    pub warnings: Vec<String>,
}

impl ShutdownReconcileReport {
    /// Builds the default core-runtime reconciliation report.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownReconcileReport`] for resources owned by core
    /// runtime and adjacent observability.
    pub fn core_runtime_completed() -> Self {
        Self {
            registry_status: ResourceReconcileStatus::Cleaned,
            runtime_handle_status: ResourceReconcileStatus::Cleaned,
            journal_status: ResourceReconcileStatus::Recorded,
            metrics_status: ResourceReconcileStatus::Recorded,
            socket_status: ResourceReconcileStatus::NotOwned,
            warnings: Vec::new(),
        }
    }
}

/// Complete shutdown pipeline report returned to callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownPipelineReport {
    /// Shutdown cause recorded from the first accepted request.
    pub cause: ShutdownCause,
    /// Start timestamp in Unix epoch nanoseconds.
    pub started_at_unix_nanos: u128,
    /// Completion timestamp in Unix epoch nanoseconds.
    pub completed_at_unix_nanos: u128,
    /// Final shutdown phase.
    pub phase: ShutdownPhase,
    /// Per-child shutdown outcomes.
    pub outcomes: Vec<ChildShutdownOutcome>,
    /// Resource reconciliation summary.
    pub reconcile: ShutdownReconcileReport,
    /// Whether this report was returned for a repeated shutdown request.
    pub idempotent: bool,
}

impl ShutdownPipelineReport {
    /// Returns a copy marked as idempotent.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a cloned report with `idempotent` set to `true`.
    pub fn as_idempotent(&self) -> Self {
        let mut report = self.clone();
        report.idempotent = true;
        report
    }
}
