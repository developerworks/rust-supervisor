//! Service role adapter.
//!
//! The adapter owns the bridge from [`ServiceRole`] lifecycle methods to the
//! existing [`TaskFactory`](crate::task::factory::TaskFactory) runtime contract.

use crate::role::context::service::ServiceContext;
use crate::role::result::service::ServiceError;
use crate::role::traits::service::ServiceRole;
use crate::task::context::TaskContext;
use crate::task::factory::{BoxTaskFuture, Service, TaskResult};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Adapter that turns a service role into a task factory service.
pub struct ServiceRoleAdapter<T> {
    /// Role instance protected across runtime task starts.
    role: Arc<Mutex<T>>,
}

impl<T> ServiceRoleAdapter<T>
where
    T: ServiceRole,
{
    /// Creates a service role adapter.
    ///
    /// # Arguments
    ///
    /// - `role`: Service role instance that owns user lifecycle state.
    ///
    /// # Returns
    ///
    /// Returns an adapter that implements the task factory service contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::adapter::service::ServiceRoleAdapter;
    /// use rust_supervisor::role::context::service::ServiceContext;
    /// use rust_supervisor::role::result::service::ServiceResult;
    /// use rust_supervisor::role::traits::service::ServiceRole;
    ///
    /// struct ExampleService;
    ///
    /// impl ServiceRole for ExampleService {
    ///     async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let _adapter = ServiceRoleAdapter::new(ExampleService);
    /// ```
    pub fn new(role: T) -> Self {
        Self {
            role: Arc::new(Mutex::new(role)),
        }
    }
}

impl<T> Service for ServiceRoleAdapter<T>
where
    T: ServiceRole,
{
    /// Builds one task future for the adapted service role.
    fn call(&self, ctx: TaskContext) -> BoxTaskFuture {
        let role = self.role.clone();
        let service_context = ServiceContext::new(ctx);
        Box::pin(async move {
            let mut role = role.lock().await;
            run_service_lifecycle(&mut *role, &service_context).await
        })
    }
}

/// Runs the service lifecycle and maps role errors into task results.
///
/// # Arguments
///
/// - `role`: Service role implementation.
/// - `ctx`: Service-specific context passed to lifecycle methods.
///
/// # Returns
///
/// Returns a [`TaskResult`] consumed by the runtime.
async fn run_service_lifecycle<T>(role: &mut T, ctx: &ServiceContext) -> TaskResult
where
    T: ServiceRole,
{
    if let Err(error) = role.init(ctx).await {
        return service_error_to_task_result(error);
    }
    if let Err(error) = role.run(ctx).await {
        return service_error_to_task_result(error);
    }
    if let Err(error) = role.shutdown(ctx).await {
        return service_error_to_task_result(error);
    }
    if ctx.is_shutdown_requested() {
        TaskResult::Cancelled
    } else {
        TaskResult::Succeeded
    }
}

/// Converts a service error into a failed task result.
///
/// # Arguments
///
/// - `error`: Service role error returned by a lifecycle method.
///
/// # Returns
///
/// Returns [`TaskResult::Failed`] with a typed task failure payload.
fn service_error_to_task_result(error: ServiceError) -> TaskResult {
    TaskResult::Failed(error.into_task_failure())
}
