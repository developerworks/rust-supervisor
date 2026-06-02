//! Supervisor role context wrapper.
//!
//! The wrapper keeps nested supervisor role code focused on readiness,
//! heartbeat, identity, path, and shutdown observation.

use crate::id::types::{ChildId, SupervisorPath};
use crate::task::context::TaskContext;

/// Narrow context passed to a supervisor role.
#[derive(Debug, Clone)]
pub struct SupervisorContext {
    /// Runtime task context hidden behind the supervisor-specific API.
    inner: TaskContext,
}

impl SupervisorContext {
    /// Creates a supervisor context from a runtime task context.
    ///
    /// # Arguments
    ///
    /// - `inner`: Runtime context for one supervisor child_start_count.
    ///
    /// # Returns
    ///
    /// Returns a supervisor-specific context wrapper.
    ///
    /// # Examples
    ///
    /// ```
    /// let (task_context, _heartbeat) = rust_supervisor::task::context::TaskContext::new(
    ///     rust_supervisor::id::types::ChildId::new("nested-supervisor"),
    ///     rust_supervisor::id::types::SupervisorPath::root().join("nested-supervisor"),
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::ChildStartCount::first(),
    /// );
    /// let supervisor_context = rust_supervisor::role::context::supervisor::SupervisorContext::new(
    ///     task_context,
    /// );
    /// assert!(!supervisor_context.is_shutdown_requested());
    /// ```
    pub fn new(inner: TaskContext) -> Self {
        Self { inner }
    }

    /// Reports that the supervisor role is ready.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    ///
    /// # Examples
    ///
    /// ```
    /// let (task_context, _heartbeat) = rust_supervisor::task::context::TaskContext::new(
    ///     rust_supervisor::id::types::ChildId::new("nested-supervisor"),
    ///     rust_supervisor::id::types::SupervisorPath::root().join("nested-supervisor"),
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::ChildStartCount::first(),
    /// );
    /// let supervisor_context = rust_supervisor::role::context::supervisor::SupervisorContext::new(
    ///     task_context,
    /// );
    /// supervisor_context.ready();
    /// ```
    pub fn ready(&self) {
        self.inner.mark_ready();
    }

    /// Emits a supervisor role heartbeat.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn heartbeat(&self) {
        self.inner.heartbeat();
    }

    /// Returns whether shutdown has been requested.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when runtime cancellation has been requested.
    pub fn is_shutdown_requested(&self) -> bool {
        self.inner.is_cancelled()
    }

    /// Waits until shutdown has been requested.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns after the runtime cancellation token has been cancelled.
    pub async fn wait_shutdown(&self) {
        self.inner.cancellation_token().cancelled().await;
    }

    /// Returns the stable supervisor child identifier.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the child identifier owned by the runtime context.
    pub fn child_id(&self) -> &ChildId {
        &self.inner.child_id
    }

    /// Returns the supervisor role path in the supervisor tree.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the supervisor tree path owned by the runtime context.
    pub fn path(&self) -> &SupervisorPath {
        &self.inner.path
    }
}
