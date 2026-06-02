//! Worker role contract tests.
//!
//! These tests verify the worker role contract adapter without using inline
//! module tests in production source files.

use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::role::adapter::worker::WorkerRoleAdapter;
use rust_supervisor::role::context::worker::WorkerContext;
use rust_supervisor::role::lifecycle::RoleLifecyclePhase;
use rust_supervisor::role::result::worker::{WorkerError, WorkerResult};
use rust_supervisor::role::traits::worker::WorkerRole;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult};
use std::sync::{Arc, Mutex};

/// Creates a task context for worker contract tests.
///
/// # Arguments
///
/// - `id`: Stable child identifier used by the test.
///
/// # Returns
///
/// Returns a runtime task context.
fn task_context(id: &str) -> TaskContext {
    let (ctx, _heartbeat) = TaskContext::new(
        ChildId::new(id),
        SupervisorPath::root().join(id),
        Generation::initial(),
        ChildStartCount::first(),
    );
    ctx
}

/// Records worker lifecycle phases in call order.
struct RecordingWorker {
    /// Shared event list used by the test assertions.
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl WorkerRole for RecordingWorker {
    /// Records the initialization phase.
    async fn init(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("init");
        Ok(())
    }

    /// Records the work phase.
    async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("work");
        Ok(())
    }

    /// Records the completion phase.
    async fn complete(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("complete");
        Ok(())
    }
}

/// Verifies that the worker adapter preserves lifecycle order.
#[tokio::test]
async fn worker_role_adapter_runs_lifecycle_in_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let worker = RecordingWorker {
        events: events.clone(),
    };
    let adapter = WorkerRoleAdapter::new(worker);

    let result = TaskFactory::build(&adapter, task_context("worker")).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *events.lock().expect("events lock"),
        vec!["init", "work", "complete"]
    );
}

/// Observes cancellation state through the worker context.
struct CancellationAwareWorker {
    /// Captured cancellation state seen by the worker body.
    seen_cancelled: Arc<Mutex<bool>>,
}

impl WorkerRole for CancellationAwareWorker {
    /// Records whether cancellation is already visible.
    async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        ctx.wait_cancelled().await;
        *self.seen_cancelled.lock().expect("cancel lock") = ctx.is_cancelled();
        Ok(())
    }
}

/// Verifies that worker context maps runtime cancellation into task cancellation.
#[tokio::test]
async fn worker_context_observes_runtime_cancellation() {
    let seen_cancelled = Arc::new(Mutex::new(false));
    let adapter = WorkerRoleAdapter::new(CancellationAwareWorker {
        seen_cancelled: seen_cancelled.clone(),
    });
    let ctx = task_context("cancelled-worker");
    ctx.cancel();

    let result = TaskFactory::build(&adapter, ctx).await;

    assert_eq!(result, TaskResult::Cancelled);
    assert!(*seen_cancelled.lock().expect("cancel lock"));
}

/// Fails during the worker work phase.
struct FailingWorker;

impl WorkerRole for FailingWorker {
    /// Returns a structured worker error.
    async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        Err(WorkerError::new(
            ctx.child_id().clone(),
            RoleLifecyclePhase::Work,
            "worker body failed",
        ))
    }
}

/// Verifies that worker errors become typed task failures.
#[tokio::test]
async fn worker_error_maps_to_task_failure() {
    let adapter = WorkerRoleAdapter::new(FailingWorker);
    let result = TaskFactory::build(&adapter, task_context("failing-worker")).await;

    match result {
        TaskResult::Failed(TaskFailure {
            kind,
            category,
            message,
        }) => {
            assert_eq!(kind, TaskFailureKind::Error);
            assert_eq!(category, "worker_work");
            assert!(message.contains("worker body failed"));
        }
        other => panic!("expected failed task result, got {other:?}"),
    }
}
