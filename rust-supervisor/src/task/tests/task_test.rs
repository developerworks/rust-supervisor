//! Task context and factory tests.
//!
//! These tests verify fresh task future construction and task context signals.

use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::readiness::signal::ReadinessState;
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult, service_fn};
use rust_supervisor::task::factory_registry::{TaskFactoryDescriptor, TaskFactoryRegistry};
use std::sync::Arc;

/// Verifies that service functions build fresh task futures.
#[tokio::test(start_paused = true)]
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
    assert_eq!(*ready.borrow(), ReadinessState::Ready);
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

/// Builds a no-op task factory descriptor for registry tests.
fn descriptor(key: &str, kinds: impl IntoIterator<Item = TaskKind>) -> TaskFactoryDescriptor {
    TaskFactoryDescriptor::new(
        key,
        "Test Worker",
        "Runs a test worker.",
        kinds,
        Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
    )
}

/// Verifies that registry lookup returns a registered factory.
#[test]
fn task_factory_registry_resolves_registered_key() {
    let mut registry = TaskFactoryRegistry::new();
    registry
        .register(descriptor("worker", [TaskKind::AsyncWorker]))
        .expect("register factory");

    let factory = registry
        .resolve("worker", TaskKind::AsyncWorker)
        .expect("resolve factory");

    assert_eq!(registry.keys(), vec!["worker".to_string()]);
    assert_eq!(Arc::strong_count(&factory), 2);
}

/// Verifies that duplicate registry keys are rejected.
#[test]
fn task_factory_registry_rejects_duplicate_key() {
    let mut registry = TaskFactoryRegistry::new();
    registry
        .register(descriptor("worker", [TaskKind::AsyncWorker]))
        .expect("register first factory");

    let error = registry
        .register(descriptor("worker", [TaskKind::AsyncWorker]))
        .expect_err("duplicate key should fail");

    assert!(error.to_string().contains("duplicate task factory key"));
}

/// Verifies that registry lookup checks task kind compatibility.
#[test]
fn task_factory_registry_rejects_kind_mismatch() {
    let mut registry = TaskFactoryRegistry::new();
    registry
        .register(descriptor("worker", [TaskKind::AsyncWorker]))
        .expect("register factory");

    let error = match registry.resolve("worker", TaskKind::BlockingWorker) {
        Ok(_) => panic!("kind mismatch should fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("does not support task kind"));
}
