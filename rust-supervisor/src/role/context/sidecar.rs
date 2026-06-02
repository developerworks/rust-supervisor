//! Sidecar role context wrapper.
//!
//! The wrapper keeps sidecar code focused on readiness, heartbeat, primary
//! child visibility, and shutdown observation without exposing the full runtime
//! task context.

use crate::id::types::{ChildId, SupervisorPath};
use crate::task::context::TaskContext;

/// Narrow context passed to a sidecar role.
#[derive(Debug, Clone)]
pub struct SidecarContext {
    /// Runtime task context hidden behind the sidecar-specific API.
    inner: TaskContext,
    /// Stable child identifier for the primary child this sidecar follows.
    primary_child_id: ChildId,
}

impl SidecarContext {
    /// Creates a sidecar context from a runtime task context.
    ///
    /// # Arguments
    ///
    /// - `inner`: Runtime context for one sidecar child_start_count.
    /// - `primary_child_id`: Stable child identifier for the primary child.
    ///
    /// # Returns
    ///
    /// Returns a sidecar-specific context wrapper.
    ///
    /// # Examples
    ///
    /// ```
    /// let (task_context, _heartbeat) = rust_supervisor::task::context::TaskContext::new(
    ///     rust_supervisor::id::types::ChildId::new("sidecar"),
    ///     rust_supervisor::id::types::SupervisorPath::root().join("sidecar"),
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::ChildStartCount::first(),
    /// );
    /// let sidecar_context = rust_supervisor::role::context::sidecar::SidecarContext::new(
    ///     task_context,
    ///     rust_supervisor::id::types::ChildId::new("primary"),
    /// );
    /// assert_eq!(sidecar_context.primary_id().value, "primary");
    /// ```
    pub fn new(inner: TaskContext, primary_child_id: ChildId) -> Self {
        Self {
            inner,
            primary_child_id,
        }
    }

    /// Reports that the sidecar is ready.
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
    ///     rust_supervisor::id::types::ChildId::new("sidecar"),
    ///     rust_supervisor::id::types::SupervisorPath::root().join("sidecar"),
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::ChildStartCount::first(),
    /// );
    /// let sidecar_context = rust_supervisor::role::context::sidecar::SidecarContext::new(
    ///     task_context,
    ///     rust_supervisor::id::types::ChildId::new("primary"),
    /// );
    /// sidecar_context.ready();
    /// ```
    pub fn ready(&self) {
        self.inner.mark_ready();
    }

    /// Emits a sidecar heartbeat.
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

    /// Returns the primary child identifier linked to this sidecar.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the primary child identifier owned by the sidecar context.
    pub fn primary_id(&self) -> &ChildId {
        &self.primary_child_id
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

    /// Returns the stable sidecar child identifier.
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

    /// Returns the sidecar path in the supervisor tree.
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
