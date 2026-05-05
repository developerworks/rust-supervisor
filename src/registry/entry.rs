//! Registry entry types.
//!
//! This module owns the runtime record for a registered child and keeps that
//! record independent from task execution.

use crate::child_runner::attempt::TaskExit;
use crate::id::types::{Attempt, ChildId, Generation, SupervisorPath};
use crate::spec::child::ChildSpec;

/// Runtime status for a registered child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildRuntimeStatus {
    /// The child exists in the registry but has not started.
    Registered,
    /// The child is starting.
    Starting,
    /// The child task is running.
    Running,
    /// The child task reported readiness.
    Ready,
    /// The child exited.
    Exited,
}

/// Runtime state owned by the registry for one child.
#[derive(Debug, Clone)]
pub struct ChildRuntime {
    /// Stable child identifier.
    pub id: ChildId,
    /// Full child path.
    pub path: SupervisorPath,
    /// Child declaration copied from the supervisor specification.
    pub spec: ChildSpec,
    /// Current runtime status.
    pub status: ChildRuntimeStatus,
    /// Current generation value.
    pub generation: Generation,
    /// Current attempt value.
    pub attempt: Attempt,
    /// Number of restarts that have occurred.
    pub restart_count: u64,
    /// Last known task exit.
    pub last_exit: Option<TaskExit>,
}

impl ChildRuntime {
    /// Creates a registry runtime record for a child.
    ///
    /// # Arguments
    ///
    /// - `spec`: Child declaration.
    /// - `path`: Full child path in the supervisor tree.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildRuntime`] in registered status.
    ///
    /// # Examples
    ///
    /// ```
    /// let factory = rust_supervisor::task::factory::service_fn(|_ctx| async {
    ///     rust_supervisor::task::factory::TaskResult::Succeeded
    /// });
    /// let spec = rust_supervisor::spec::child::ChildSpec::worker(
    ///     rust_supervisor::id::types::ChildId::new("worker"),
    ///     "worker",
    ///     rust_supervisor::spec::child::TaskKind::AsyncWorker,
    ///     std::sync::Arc::new(factory),
    /// );
    /// let runtime = rust_supervisor::registry::entry::ChildRuntime::new(
    ///     spec,
    ///     rust_supervisor::id::types::SupervisorPath::root().join("worker"),
    /// );
    /// assert!(matches!(runtime.status, rust_supervisor::registry::entry::ChildRuntimeStatus::Registered));
    /// ```
    pub fn new(spec: ChildSpec, path: SupervisorPath) -> Self {
        Self {
            id: spec.id.clone(),
            path,
            spec,
            status: ChildRuntimeStatus::Registered,
            generation: Generation::initial(),
            attempt: Attempt::first(),
            restart_count: 0,
            last_exit: None,
        }
    }
}
