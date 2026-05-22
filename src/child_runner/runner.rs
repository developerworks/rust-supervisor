//! Minimal child runner.
//!
//! This module starts one child child_start_count, advances readiness state, and records
//! the resulting task exit.

use crate::child_runner::run_exit::TaskExit;
use crate::error::types::SupervisorError;
use crate::readiness::signal::{ReadinessPolicy, ReadinessState, ReadySignal};
use crate::registry::entry::{ChildRuntime, ChildRuntimeStatus};
use crate::task::context::TaskContext;
use tokio::sync::{watch, watch::Receiver};
use tokio::task::{AbortHandle, JoinHandle};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

/// Result of running one child child_start_count.
#[derive(Debug, Clone)]
pub struct ChildRunReport {
    /// Runtime record after the child_start_count.
    pub runtime: ChildRuntime,
    /// Final task exit classification.
    pub exit: TaskExit,
    /// Whether the task became ready during the child_start_count.
    pub became_ready: bool,
}

/// Handle for one running child child_start_count.
#[derive(Debug)]
pub struct ChildRunHandle {
    /// Runtime cancellation token shared with the task context.
    pub cancellation_token: CancellationToken,
    /// Abort handle attached to the real child future.
    pub abort_handle: AbortHandle,
    /// Receiver that observes the completed child run report.
    pub completion_receiver: Receiver<Option<Result<ChildRunReport, SupervisorError>>>,
    /// Receiver that observes the latest child heartbeat.
    pub heartbeat_receiver: watch::Receiver<Option<Instant>>,
    /// Receiver that observes the latest child readiness state.
    pub readiness_receiver: watch::Receiver<ReadinessState>,
}

/// Runner that executes one child child_start_count.
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

    /// Runs one child child_start_count.
    ///
    /// # Arguments
    ///
    /// - `runtime`: Runtime record for the child task.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildRunReport`] when the child owns a task factory.
    pub async fn run_once(&self, runtime: ChildRuntime) -> Result<ChildRunReport, SupervisorError> {
        let mut completion_receiver = self.spawn_once(runtime)?.completion_receiver;
        wait_for_report(&mut completion_receiver).await
    }

    /// Spawns one child task and returns cancellation and abort handles.
    ///
    /// # Arguments
    ///
    /// - `runtime`: Runtime record for the child task.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildRunHandle`] when the child owns a task factory.
    pub fn spawn_once(&self, mut runtime: ChildRuntime) -> Result<ChildRunHandle, SupervisorError> {
        #[cfg(any(debug_assertions, feature = "test-support"))]
        #[cfg(any(test, feature = "test-support"))]
        if crate::test_support::child_spawn::take_child_spawn_failure_attempt(&runtime.id) {
            return Err(SupervisorError::InvalidTransition {
                message: "test hook: child spawn_once failure".to_owned(),
            });
        }
        let factory =
            runtime.spec.factory.clone().ok_or_else(|| {
                SupervisorError::fatal_config("worker child requires a task factory")
            })?;
        runtime.status = ChildRuntimeStatus::Starting;
        let (ready_signal, ready_receiver) = ReadySignal::new();
        let cancellation_token = CancellationToken::new();
        let (ctx, heartbeat_receiver) = TaskContext::with_ready_signal_and_cancellation_token(
            runtime.id.clone(),
            runtime.path.clone(),
            runtime.generation,
            runtime.child_start_count,
            ready_signal,
            cancellation_token.clone(),
        );
        mark_immediate_ready(runtime.spec.readiness_policy, &ctx, &mut runtime);
        runtime.status = ChildRuntimeStatus::Running;
        let (completion_sender, completion_receiver) = watch::channel(None);

        // Choose the spawn strategy based on the child's isolation setting.
        // BlockingPool tasks are spawned on the async worker pool normally but
        // wrapped with `tokio::task::block_in_place` at the very start. This
        // signals to the tokio scheduler that the worker thread may block and
        // allows a replacement worker to be spawned, preventing worker thread
        // starvation without creating a nested runtime.
        //
        // Background: `spawn_blocking` + `spawn_blocking` inner runtime is an
        // anti-pattern because it allocates a full current-thread runtime per
        // blocking thread, wasting memory and potentially exhausting the
        // blocking pool. `block_in_place` is the tokio-approved approach for
        // CPU-heavy or blocking async tasks.
        let child_task = match runtime.spec.isolation {
            crate::spec::child::Isolation::BlockingPool => {
                let ctx_clone = ctx.clone_for_blocking();
                let shared_factory = factory.clone();
                tokio::spawn(async move {
                    // Signal to the tokio scheduler that this task may block.
                    // The scheduler will spawn a replacement worker if needed.
                    tokio::task::block_in_place(move || {
                        // Inside block_in_place we create a minimal one-shot
                        // runtime to drive the factory future to completion.
                        // This is unavoidable because the factory returns an
                        // async future — there is no synchronous variant. The
                        // key difference from the previous approach is that
                        // `block_in_place` tells the outer runtime to lend us
                        // this worker thread rather than permanently stealing it.
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .expect("BlockingPool: failed to build one-shot runtime");
                        rt.block_on(shared_factory.build(ctx_clone))
                    })
                })
            }
            crate::spec::child::Isolation::AsyncWorker => tokio::spawn(factory.build(ctx)),
        };
        let abort_handle = child_task.abort_handle();
        let run_ready_receiver = ready_receiver.clone();
        tokio::spawn(async move {
            let report = run_factory(runtime, run_ready_receiver, child_task).await;
            let _ignored = completion_sender.send(Some(report));
        });
        Ok(ChildRunHandle {
            cancellation_token,
            abort_handle,
            completion_receiver,
            heartbeat_receiver,
            readiness_receiver: ready_receiver.clone(),
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
/// - `ctx`: Per-child_start_count task context.
///
/// # Returns
///
/// Returns the classified task exit.
async fn run_factory(
    mut runtime: ChildRuntime,
    ready_receiver: watch::Receiver<ReadinessState>,
    task: JoinHandle<crate::task::factory::TaskResult>,
) -> Result<ChildRunReport, SupervisorError> {
    match task.await {
        Ok(result) => {
            let exit = TaskExit::from_task_result(result);
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
        Err(error) if error.is_panic() => {
            let exit = TaskExit::Panicked(String::from("task panicked"));
            runtime.last_exit = Some(exit.clone());
            Ok(ChildRunReport {
                runtime,
                exit,
                became_ready: observe_ready(ready_receiver),
            })
        }
        Err(_error) => {
            let exit = TaskExit::Cancelled;
            runtime.last_exit = Some(exit.clone());
            Ok(ChildRunReport {
                runtime,
                exit,
                became_ready: observe_ready(ready_receiver),
            })
        }
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
fn observe_ready(ready_receiver: watch::Receiver<ReadinessState>) -> bool {
    matches!(*ready_receiver.borrow(), ReadinessState::Ready)
}

/// Waits for the report sender to publish a child run report.
///
/// # Arguments
///
/// - `completion_receiver`: Receiver published by the run observer task.
///
/// # Returns
///
/// Returns the completed run report.
pub(crate) async fn wait_for_report(
    completion_receiver: &mut Receiver<Option<Result<ChildRunReport, SupervisorError>>>,
) -> Result<ChildRunReport, SupervisorError> {
    loop {
        if let Some(result) = completion_receiver.borrow().clone() {
            return result;
        }
        if completion_receiver.changed().await.is_err() {
            return Err(SupervisorError::InvalidTransition {
                message: "child run report channel closed before completion".to_owned(),
            });
        }
    }
}
