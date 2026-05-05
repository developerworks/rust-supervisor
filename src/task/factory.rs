//! Task factory and service adapter types.
//!
//! This module owns the public task construction contract. Every call to
//! [`TaskFactory::build`] must create a fresh future for one attempt.

use crate::error::types::TaskFailure;
use crate::task::context::TaskContext;
use std::future::Future;
use std::pin::Pin;

/// Boxed task future returned by task factories.
pub type BoxTaskFuture = Pin<Box<dyn Future<Output = TaskResult> + Send + 'static>>;

/// Result produced by a supervised task attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskResult {
    /// The task completed successfully.
    Succeeded,
    /// The task observed cancellation and stopped cooperatively.
    Cancelled,
    /// The task failed with a typed failure payload.
    Failed(TaskFailure),
}

impl TaskResult {
    /// Returns whether this task result is successful.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` only for [`TaskResult::Succeeded`].
    ///
    /// # Examples
    ///
    /// ```
    /// let result = rust_supervisor::task::factory::TaskResult::Succeeded;
    /// assert!(result.is_success());
    /// ```
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

/// Factory that creates a fresh task future for each attempt.
pub trait TaskFactory: Send + Sync + 'static {
    /// Builds a new task future.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Per-attempt context with cancellation, heartbeat, and readiness.
    ///
    /// # Returns
    ///
    /// Returns a boxed future that resolves to [`TaskResult`].
    fn build(&self, ctx: TaskContext) -> BoxTaskFuture;
}

/// Service adapter that can be converted into a [`TaskFactory`].
pub trait Service: Send + Sync + 'static {
    /// Calls the service for one task attempt.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Per-attempt context passed to the service.
    ///
    /// # Returns
    ///
    /// Returns a boxed future for the attempt.
    fn call(&self, ctx: TaskContext) -> BoxTaskFuture;
}

impl<T> TaskFactory for T
where
    T: Service,
{
    /// Builds a task future through the service implementation.
    fn build(&self, ctx: TaskContext) -> BoxTaskFuture {
        self.call(ctx)
    }
}

/// Concrete service adapter returned by [`service_fn`].
pub struct ServiceFn<F> {
    /// Closure used to create a fresh task future.
    function: F,
}

impl<F, Fut> Service for ServiceFn<F>
where
    F: Fn(TaskContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = TaskResult> + Send + 'static,
{
    /// Calls the stored function and boxes its returned future.
    fn call(&self, ctx: TaskContext) -> BoxTaskFuture {
        Box::pin((self.function)(ctx))
    }
}

/// Creates a service from a function or closure.
///
/// # Arguments
///
/// - `function`: Function that creates a fresh future for each task attempt.
///
/// # Returns
///
/// Returns a [`Service`] implementation that also implements [`TaskFactory`].
///
/// # Examples
///
/// ```
/// let service = rust_supervisor::task::factory::service_fn(|_ctx| async {
///     rust_supervisor::task::factory::TaskResult::Succeeded
/// });
/// let (ctx, _heartbeat) = rust_supervisor::task::context::TaskContext::new(
///     rust_supervisor::id::types::ChildId::new("worker"),
///     rust_supervisor::id::types::SupervisorPath::root().join("worker"),
///     rust_supervisor::id::types::Generation::initial(),
///     rust_supervisor::id::types::Attempt::first(),
/// );
/// let _future = rust_supervisor::task::factory::TaskFactory::build(&service, ctx);
/// ```
pub fn service_fn<F, Fut>(function: F) -> ServiceFn<F>
where
    F: Fn(TaskContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = TaskResult> + Send + 'static,
{
    ServiceFn { function }
}
