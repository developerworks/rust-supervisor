//! Worker role adapter.
//!
//! The adapter owns the bridge from [`WorkerRole`] lifecycle methods to the
//! existing [`TaskFactory`] runtime contract.

use crate::role::context::worker::WorkerContext;
use crate::role::result::worker::WorkerError;
use crate::role::traits::worker::WorkerRole;
use crate::task::context::TaskContext;
use crate::task::factory::{BoxTaskFuture, Service, TaskResult};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Adapter that turns a worker role into a task factory service.
pub struct WorkerRoleAdapter<T> {
    /// Role instance protected across runtime task starts.
    role: Arc<Mutex<T>>,
}

impl<T> WorkerRoleAdapter<T>
where
    T: WorkerRole,
{
    /// Creates a worker role adapter.
    ///
    /// # Arguments
    ///
    /// - `role`: Worker role instance that owns user lifecycle state.
    ///
    /// # Returns
    ///
    /// Returns an adapter that implements the task factory service contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::adapter::worker::WorkerRoleAdapter;
    /// use rust_supervisor::role::context::worker::WorkerContext;
    /// use rust_supervisor::role::result::worker::WorkerResult;
    /// use rust_supervisor::role::traits::worker::WorkerRole;
    ///
    /// struct ExampleWorker;
    ///
    /// impl WorkerRole for ExampleWorker {
    ///     async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let _adapter = WorkerRoleAdapter::new(ExampleWorker);
    /// ```
    pub fn new(role: T) -> Self {
        Self {
            role: Arc::new(Mutex::new(role)),
        }
    }
}

impl<T> Service for WorkerRoleAdapter<T>
where
    T: WorkerRole,
{
    /// Builds one task future for the adapted worker role.
    fn call(&self, ctx: TaskContext) -> BoxTaskFuture {
        let role = self.role.clone();
        let worker_context = WorkerContext::new(ctx);
        Box::pin(async move {
            let mut role = role.lock().await;
            run_worker_lifecycle(&mut *role, &worker_context).await
        })
    }
}

/// Runs the worker lifecycle and maps role errors into task results.
///
/// # Arguments
///
/// - `role`: Worker role implementation.
/// - `ctx`: Worker-specific context passed to lifecycle methods.
///
/// # Returns
///
/// Returns a [`TaskResult`] consumed by the runtime.
async fn run_worker_lifecycle<T>(role: &mut T, ctx: &WorkerContext) -> TaskResult
where
    T: WorkerRole,
{
    if let Err(error) = role.init(ctx).await {
        return worker_error_to_task_result(error);
    }
    if let Err(error) = role.work(ctx).await {
        return worker_error_to_task_result(error);
    }
    if let Err(error) = role.complete(ctx).await {
        return worker_error_to_task_result(error);
    }
    if ctx.is_cancelled() {
        TaskResult::Cancelled
    } else {
        TaskResult::Succeeded
    }
}

/// Converts a worker error into a failed task result.
///
/// # Arguments
///
/// - `error`: Worker role error returned by a lifecycle method.
///
/// # Returns
///
/// Returns [`TaskResult::Failed`] with a typed task failure payload.
fn worker_error_to_task_result(error: WorkerError) -> TaskResult {
    TaskResult::Failed(error.into_task_failure())
}
