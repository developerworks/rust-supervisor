//! Job role context wrapper.
//!
//! The wrapper keeps one-shot job code focused on readiness, heartbeat, and
//! cancellation observation without exposing the full runtime task context.

use crate::id::types::{ChildId, SupervisorPath};
use crate::task::context::TaskContext;

/// Narrow context passed to a job role.
#[derive(Debug, Clone)]
pub struct JobContext {
    /// Runtime task context hidden behind the job-specific API.
    inner: TaskContext,
}

impl JobContext {
    /// Creates a job context from a runtime task context.
    ///
    /// # Arguments
    ///
    /// - `inner`: Runtime context for one job child_start_count.
    ///
    /// # Returns
    ///
    /// Returns a job-specific context wrapper.
    ///
    /// # Examples
    ///
    /// ```
    /// let (task_context, _heartbeat) = rust_supervisor::task::context::TaskContext::new(
    ///     rust_supervisor::id::types::ChildId::new("job"),
    ///     rust_supervisor::id::types::SupervisorPath::root().join("job"),
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::ChildStartCount::first(),
    /// );
    /// let job_context = rust_supervisor::role::context::job::JobContext::new(
    ///     task_context,
    /// );
    /// assert!(!job_context.is_cancelled());
    /// ```
    pub fn new(inner: TaskContext) -> Self {
        Self { inner }
    }

    /// Reports that the job is ready.
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
    ///     rust_supervisor::id::types::ChildId::new("job"),
    ///     rust_supervisor::id::types::SupervisorPath::root().join("job"),
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::ChildStartCount::first(),
    /// );
    /// let job_context = rust_supervisor::role::context::job::JobContext::new(
    ///     task_context,
    /// );
    /// job_context.ready();
    /// ```
    pub fn ready(&self) {
        self.inner.mark_ready();
    }

    /// Emits a job heartbeat.
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

    /// Returns whether cancellation has been requested.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when runtime cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    /// Waits until cancellation has been requested.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns after the runtime cancellation token has been cancelled.
    pub async fn wait_cancelled(&self) {
        self.inner.cancellation_token().cancelled().await;
    }

    /// Returns the stable job child identifier.
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

    /// Returns the job path in the supervisor tree.
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
