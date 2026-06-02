//! Supervisor role adapter.
//!
//! The adapter owns the bridge from [`SupervisorRole`] lifecycle methods to the
//! existing [`TaskFactory`] runtime contract.

use crate::role::context::supervisor::SupervisorContext;
use crate::role::lifecycle::RoleLifecyclePhase;
use crate::role::result::supervisor::SupervisorRoleError;
use crate::role::traits::supervisor::SupervisorRole;
use crate::runtime::supervisor::Supervisor;
use crate::task::context::TaskContext;
use crate::task::factory::{BoxTaskFuture, Service, TaskResult};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Adapter that turns a supervisor role into a task factory service.
pub struct SupervisorRoleAdapter<T> {
    /// Role instance protected across runtime task starts.
    role: Arc<Mutex<T>>,
}

impl<T> SupervisorRoleAdapter<T>
where
    T: SupervisorRole,
{
    /// Creates a supervisor role adapter.
    ///
    /// # Arguments
    ///
    /// - `role`: Supervisor role instance that owns user lifecycle state.
    ///
    /// # Returns
    ///
    /// Returns an adapter that implements the task factory service contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::adapter::supervisor::SupervisorRoleAdapter;
    /// use rust_supervisor::role::context::supervisor::SupervisorContext;
    /// use rust_supervisor::role::result::supervisor::SupervisorResult;
    /// use rust_supervisor::role::traits::supervisor::SupervisorRole;
    /// use rust_supervisor::spec::supervisor::SupervisorSpec;
    ///
    /// struct ExampleSupervisor;
    ///
    /// impl SupervisorRole for ExampleSupervisor {
    ///     async fn build_tree(
    ///         &mut self,
    ///         ctx: &SupervisorContext,
    ///     ) -> SupervisorResult<SupervisorSpec> {
    ///         let _ = ctx;
    ///         Ok(SupervisorSpec::root(Vec::new()))
    ///     }
    /// }
    ///
    /// let _adapter = SupervisorRoleAdapter::new(ExampleSupervisor);
    /// ```
    pub fn new(role: T) -> Self {
        Self {
            role: Arc::new(Mutex::new(role)),
        }
    }
}

impl<T> Service for SupervisorRoleAdapter<T>
where
    T: SupervisorRole,
{
    /// Builds one task future for the adapted supervisor role.
    fn call(&self, ctx: TaskContext) -> BoxTaskFuture {
        let role = self.role.clone();
        let supervisor_context = SupervisorContext::new(ctx);
        Box::pin(async move {
            let mut role = role.lock().await;
            run_supervisor_lifecycle(&mut *role, &supervisor_context).await
        })
    }
}

/// Runs the supervisor lifecycle and maps role errors into task results.
///
/// # Arguments
///
/// - `role`: Supervisor role implementation.
/// - `ctx`: Supervisor-specific context passed to lifecycle methods.
///
/// # Returns
///
/// Returns a [`TaskResult`] consumed by the runtime.
async fn run_supervisor_lifecycle<T>(role: &mut T, ctx: &SupervisorContext) -> TaskResult
where
    T: SupervisorRole,
{
    let spec = match role.build_tree(ctx).await {
        Ok(spec) => spec,
        Err(error) => return supervisor_error_to_task_result(error),
    };
    let handle = match Supervisor::start(spec).await {
        Ok(handle) => handle,
        Err(error) => {
            return supervisor_error_to_task_result(SupervisorRoleError::from_supervisor_error(
                ctx.child_id().clone(),
                RoleLifecyclePhase::BuildTree,
                error,
            ));
        }
    };

    let run_result = role.run(ctx, &handle).await;
    let shutdown_result = role.shutdown(ctx, &handle).await;
    if let Err(error) = run_result {
        return supervisor_error_to_task_result(error);
    }
    if let Err(error) = shutdown_result {
        return supervisor_error_to_task_result(error);
    }
    if ctx.is_shutdown_requested() {
        TaskResult::Cancelled
    } else {
        TaskResult::Succeeded
    }
}

/// Converts a supervisor role error into a failed task result.
///
/// # Arguments
///
/// - `error`: Supervisor role error returned by a lifecycle method.
///
/// # Returns
///
/// Returns [`TaskResult::Failed`] with a typed task failure payload.
fn supervisor_error_to_task_result(error: SupervisorRoleError) -> TaskResult {
    TaskResult::Failed(error.into_task_failure())
}
