//! Job role contract tests.
//!
//! These tests verify the job role contract adapter without using inline
//! module tests in production source files.

use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::role::adapter::job::JobRoleAdapter;
use rust_supervisor::role::context::job::JobContext;
use rust_supervisor::role::lifecycle::RoleLifecyclePhase;
use rust_supervisor::role::result::job::{JobError, JobResult};
use rust_supervisor::role::traits::job::JobRole;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult};
use std::sync::{Arc, Mutex};

/// Creates a task context for job contract tests.
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

/// Records job lifecycle phases in call order.
struct RecordingJob {
    /// Shared event list used by the test assertions.
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl JobRole for RecordingJob {
    /// Records the initialization phase.
    async fn init(&mut self, ctx: &JobContext) -> JobResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("init");
        Ok(())
    }

    /// Records the run phase.
    async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("run");
        Ok(())
    }

    /// Records the completion phase.
    async fn complete(&mut self, ctx: &JobContext) -> JobResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("complete");
        Ok(())
    }
}

/// Verifies that the job adapter preserves lifecycle order.
#[tokio::test]
async fn job_role_adapter_runs_lifecycle_in_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let job = RecordingJob {
        events: events.clone(),
    };
    let adapter = JobRoleAdapter::new(job);

    let result = TaskFactory::build(&adapter, task_context("job")).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *events.lock().expect("events lock"),
        vec!["init", "run", "complete"]
    );
}

/// Observes cancellation state through the job context.
struct CancellationAwareJob {
    /// Captured cancellation state seen by the job body.
    seen_cancellation: Arc<Mutex<bool>>,
}

impl JobRole for CancellationAwareJob {
    /// Records whether cancellation is already visible.
    async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
        ctx.wait_cancelled().await;
        *self.seen_cancellation.lock().expect("cancellation lock") = ctx.is_cancelled();
        Ok(())
    }
}

/// Verifies that job context maps cancellation to runtime cancellation.
#[tokio::test]
async fn job_context_observes_runtime_cancellation() {
    let seen_cancellation = Arc::new(Mutex::new(false));
    let adapter = JobRoleAdapter::new(CancellationAwareJob {
        seen_cancellation: seen_cancellation.clone(),
    });
    let ctx = task_context("cancelled-job");
    ctx.cancel();

    let result = TaskFactory::build(&adapter, ctx).await;

    assert_eq!(result, TaskResult::Cancelled);
    assert!(*seen_cancellation.lock().expect("cancellation lock"));
}

/// Fails during the job run phase.
struct FailingJob;

impl JobRole for FailingJob {
    /// Returns a structured job error.
    async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
        Err(JobError::new(
            ctx.child_id().clone(),
            RoleLifecyclePhase::Run,
            "job body failed",
        ))
    }
}

/// Verifies that job errors become typed task failures.
#[tokio::test]
async fn job_error_maps_to_task_failure() {
    let adapter = JobRoleAdapter::new(FailingJob);
    let result = TaskFactory::build(&adapter, task_context("failing-job")).await;

    match result {
        TaskResult::Failed(TaskFailure {
            kind,
            category,
            message,
        }) => {
            assert_eq!(kind, TaskFailureKind::Error);
            assert_eq!(category, "job_run");
            assert!(message.contains("job body failed"));
        }
        other => panic!("expected failed task result, got {other:?}"),
    }
}
