//! Task context and factory tests.
//!
//! These tests verify fresh task future construction and task context signals.

use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult, service_fn};

/// Verifies that service functions build fresh task futures.
#[tokio::test]
async fn service_fn_builds_fresh_task_future() {
    let service = service_fn(|ctx| async move {
        ctx.heartbeat();
        ctx.mark_ready();
        TaskResult::Succeeded
    });
    let (ctx, _heartbeat) = TaskContext::new(
        ChildId::new("worker"),
        SupervisorPath::root().join("worker"),
        Generation::initial(),
        ChildStartCount::first(),
    );
    let ready = ctx.readiness_receiver();

    let result = TaskFactory::build(&service, ctx).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert!(*ready.borrow());
}

/// Verifies that task context exposes cancellation state.
#[test]
fn task_context_exposes_cancellation_state() {
    let (ctx, _heartbeat) = TaskContext::new(
        ChildId::new("worker"),
        SupervisorPath::root().join("worker"),
        Generation::initial(),
        ChildStartCount::first(),
    );

    ctx.cancel();

    assert!(ctx.is_cancelled());
}
