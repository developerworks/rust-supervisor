//! Sidecar role adapter.
//!
//! The adapter owns the bridge from [`SidecarRole`] lifecycle methods to the
//! existing [`TaskFactory`](crate::task::factory::TaskFactory) runtime contract.

use crate::id::types::ChildId;
use crate::role::context::sidecar::SidecarContext;
use crate::role::result::sidecar::SidecarError;
use crate::role::traits::sidecar::SidecarRole;
use crate::task::context::TaskContext;
use crate::task::factory::{BoxTaskFuture, Service, TaskResult};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Adapter that turns a sidecar role into a task factory service.
pub struct SidecarRoleAdapter<T> {
    /// Role instance protected across runtime task starts.
    role: Arc<Mutex<T>>,
    /// Stable child identifier for the primary child linked to the sidecar.
    primary_child_id: ChildId,
}

impl<T> SidecarRoleAdapter<T>
where
    T: SidecarRole,
{
    /// Creates a sidecar role adapter.
    ///
    /// # Arguments
    ///
    /// - `role`: Sidecar role instance that owns user lifecycle state.
    /// - `primary_child_id`: Stable child identifier for the primary child.
    ///
    /// # Returns
    ///
    /// Returns an adapter that implements the task factory service contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::ChildId;
    /// use rust_supervisor::role::adapter::sidecar::SidecarRoleAdapter;
    /// use rust_supervisor::role::context::sidecar::SidecarContext;
    /// use rust_supervisor::role::result::sidecar::SidecarResult;
    /// use rust_supervisor::role::traits::sidecar::SidecarRole;
    ///
    /// struct ExampleSidecar;
    ///
    /// impl SidecarRole for ExampleSidecar {
    ///     async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let _adapter = SidecarRoleAdapter::new(
    ///     ExampleSidecar,
    ///     ChildId::new("primary"),
    /// );
    /// ```
    pub fn new(role: T, primary_child_id: ChildId) -> Self {
        Self {
            role: Arc::new(Mutex::new(role)),
            primary_child_id,
        }
    }
}

impl<T> Service for SidecarRoleAdapter<T>
where
    T: SidecarRole,
{
    /// Builds one task future for the adapted sidecar role.
    fn call(&self, ctx: TaskContext) -> BoxTaskFuture {
        let role = self.role.clone();
        let primary_child_id = self.primary_child_id.clone();
        let sidecar_context = SidecarContext::new(ctx, primary_child_id);
        Box::pin(async move {
            let mut role = role.lock().await;
            run_sidecar_lifecycle(&mut *role, &sidecar_context).await
        })
    }
}

/// Runs the sidecar lifecycle and maps role errors into task results.
///
/// # Arguments
///
/// - `role`: Sidecar role implementation.
/// - `ctx`: Sidecar-specific context passed to lifecycle methods.
///
/// # Returns
///
/// Returns a [`TaskResult`] consumed by the runtime.
async fn run_sidecar_lifecycle<T>(role: &mut T, ctx: &SidecarContext) -> TaskResult
where
    T: SidecarRole,
{
    if let Err(error) = role.init(ctx).await {
        return sidecar_error_to_task_result(error);
    }
    if let Err(error) = role.run(ctx).await {
        return sidecar_error_to_task_result(error);
    }
    if let Err(error) = role.shutdown(ctx).await {
        return sidecar_error_to_task_result(error);
    }
    if ctx.is_shutdown_requested() {
        TaskResult::Cancelled
    } else {
        TaskResult::Succeeded
    }
}

/// Converts a sidecar error into a failed task result.
///
/// # Arguments
///
/// - `error`: Sidecar role error returned by a lifecycle method.
///
/// # Returns
///
/// Returns [`TaskResult::Failed`] with a typed task failure payload.
fn sidecar_error_to_task_result(error: SidecarError) -> TaskResult {
    TaskResult::Failed(error.into_task_failure())
}
