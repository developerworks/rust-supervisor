//! Minimal child runner.
//!
//! This module starts one child attempt, advances readiness state, and records
//! the resulting task exit.

use crate::child_runner::attempt::TaskExit;
use crate::error::types::SupervisorError;
use crate::readiness::signal::{ReadinessPolicy, ReadySignal};
use crate::registry::entry::{ChildRuntime, ChildRuntimeStatus};
use crate::task::context::TaskContext;
use tokio::sync::watch;

/// Result of running one child attempt.
#[derive(Debug, Clone)]
pub struct ChildRunReport {
    /// Runtime record after the attempt.
    pub runtime: ChildRuntime,
    /// Final task exit classification.
    pub exit: TaskExit,
    /// Whether the task became ready during the attempt.
    pub became_ready: bool,
}

/// Runner that executes one child attempt.
#[derive(Debug, Clone, Default)]
pub struct ChildRunner;

impl ChildRunner {
    /// Creates a child runner.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildRunner`].
    ///
    /// # Examples
    ///
    /// ```
    /// let _runner = rust_supervisor::child_runner::runner::ChildRunner::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Runs one child attempt.
    ///
    /// # Arguments
    ///
    /// - `runtime`: Runtime record for the child attempt.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildRunReport`] when the child owns a task factory.
    pub async fn run_once(
        &self,
        mut runtime: ChildRuntime,
    ) -> Result<ChildRunReport, SupervisorError> {
        let factory =
            runtime.spec.factory.clone().ok_or_else(|| {
                SupervisorError::fatal_config("worker child requires a task factory")
            })?;
        runtime.status = ChildRuntimeStatus::Starting;
        let (ready_signal, ready_receiver) = ReadySignal::new();
        let (ctx, _heartbeat_receiver) = TaskContext::with_ready_signal(
            runtime.id.clone(),
            runtime.path.clone(),
            runtime.generation,
            runtime.attempt,
            ready_signal,
        );
        mark_immediate_ready(runtime.spec.readiness_policy, &ctx, &mut runtime);
        runtime.status = ChildRuntimeStatus::Running;
        let exit = run_factory(factory, ctx).await;
        let became_ready = observe_ready(ready_receiver);
        if became_ready {
            runtime.status = ChildRuntimeStatus::Ready;
        }
        runtime.last_exit = Some(exit.clone());
        Ok(ChildRunReport {
            runtime,
            exit,
            became_ready,
        })
    }
}

/// Marks a runtime ready when policy requires immediate readiness.
///
/// # Arguments
///
/// - `policy`: Readiness policy attached to the child.
/// - `ctx`: Task context that owns the readiness sender.
/// - `runtime`: Runtime record whose status should advance.
///
/// # Returns
///
/// This function does not return a value.
fn mark_immediate_ready(policy: ReadinessPolicy, ctx: &TaskContext, runtime: &mut ChildRuntime) {
    if policy.is_immediate() {
        ctx.mark_ready();
        runtime.status = ChildRuntimeStatus::Ready;
    }
}

/// Runs a factory and classifies the result.
///
/// # Arguments
///
/// - `factory`: Task factory for this child.
/// - `ctx`: Per-attempt task context.
///
/// # Returns
///
/// Returns the classified task exit.
async fn run_factory(
    factory: std::sync::Arc<dyn crate::task::factory::TaskFactory>,
    ctx: TaskContext,
) -> TaskExit {
    let task = tokio::spawn(factory.build(ctx));
    match task.await {
        Ok(result) => TaskExit::from_task_result(result),
        Err(error) if error.is_panic() => TaskExit::Panicked(String::from("task panicked")),
        Err(_error) => TaskExit::Cancelled,
    }
}

/// Observes whether readiness was reported.
///
/// # Arguments
///
/// - `ready_receiver`: Receiver that stores the latest readiness value.
///
/// # Returns
///
/// Returns `true` when the receiver observed readiness.
fn observe_ready(ready_receiver: watch::Receiver<bool>) -> bool {
    *ready_receiver.borrow()
}
