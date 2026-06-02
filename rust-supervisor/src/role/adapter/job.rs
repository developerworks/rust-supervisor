//! Job role adapter.
//!
//! The adapter owns the bridge from [`JobRole`] lifecycle methods to the
//! existing [`TaskFactory`] runtime contract.

use crate::role::context::job::JobContext;
use crate::role::result::job::JobError;
use crate::role::traits::job::JobRole;
use crate::task::context::TaskContext;
use crate::task::factory::{BoxTaskFuture, Service, TaskResult};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Adapter that turns a job role into a task factory service.
pub struct JobRoleAdapter<T> {
    /// Role instance protected across runtime task starts.
    role: Arc<Mutex<T>>,
}

impl<T> JobRoleAdapter<T>
where
    T: JobRole,
{
    /// Creates a job role adapter.
    ///
    /// # Arguments
    ///
    /// - `role`: Job role instance that owns user lifecycle state.
    ///
    /// # Returns
    ///
    /// Returns an adapter that implements the task factory service contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::adapter::job::JobRoleAdapter;
    /// use rust_supervisor::role::context::job::JobContext;
    /// use rust_supervisor::role::result::job::JobResult;
    /// use rust_supervisor::role::traits::job::JobRole;
    ///
    /// struct ExampleJob;
    ///
    /// impl JobRole for ExampleJob {
    ///     async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let _adapter = JobRoleAdapter::new(ExampleJob);
    /// ```
    pub fn new(role: T) -> Self {
        Self {
            role: Arc::new(Mutex::new(role)),
        }
    }
}

impl<T> Service for JobRoleAdapter<T>
where
    T: JobRole,
{
    /// Builds one task future for the adapted job role.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Runtime task context passed to the job adapter.
    ///
    /// # Returns
    ///
    /// Returns a boxed future for the child_start_count.
    fn call(&self, ctx: TaskContext) -> BoxTaskFuture {
        let role = self.role.clone();
        let job_context = JobContext::new(ctx);
        Box::pin(async move {
            let mut role = role.lock().await;
            run_job_lifecycle(&mut *role, &job_context).await
        })
    }
}

/// Runs the job lifecycle and maps role errors into task results.
///
/// # Arguments
///
/// - `role`: Job role implementation.
/// - `ctx`: Job-specific context passed to lifecycle methods.
///
/// # Returns
///
/// Returns a [`TaskResult`] consumed by the runtime.
async fn run_job_lifecycle<T>(role: &mut T, ctx: &JobContext) -> TaskResult
where
    T: JobRole,
{
    if let Err(error) = role.init(ctx).await {
        return job_error_to_task_result(error);
    }
    if let Err(error) = role.run(ctx).await {
        return job_error_to_task_result(error);
    }
    if let Err(error) = role.complete(ctx).await {
        return job_error_to_task_result(error);
    }
    if ctx.is_cancelled() {
        TaskResult::Cancelled
    } else {
        TaskResult::Succeeded
    }
}

/// Converts a job error into a failed task result.
///
/// # Arguments
///
/// - `error`: Job role error returned by a lifecycle method.
///
/// # Returns
///
/// Returns [`TaskResult::Failed`] with a typed task failure payload.
fn job_error_to_task_result(error: JobError) -> TaskResult {
    TaskResult::Failed(error.into_task_failure())
}
