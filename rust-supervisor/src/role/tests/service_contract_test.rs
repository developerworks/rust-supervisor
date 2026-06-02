//! Service role contract tests.
//!
//! These tests verify the first role contract adapter without using inline
//! module tests in production source files.

use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::role::adapter::service::ServiceRoleAdapter;
use rust_supervisor::role::context::service::ServiceContext;
use rust_supervisor::role::lifecycle::RoleLifecyclePhase;
use rust_supervisor::role::result::service::{ServiceError, ServiceResult};
use rust_supervisor::role::traits::service::ServiceRole;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult};
use std::sync::{Arc, Mutex};

/// Creates a task context for service contract tests.
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

/// Records service lifecycle phases in call order.
struct RecordingService {
    /// Shared event list used by the test assertions.
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl ServiceRole for RecordingService {
    /// Records the initialization phase.
    async fn init(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("init");
        Ok(())
    }

    /// Records the run phase.
    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("run");
        Ok(())
    }

    /// Records the shutdown phase.
    async fn shutdown(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("shutdown");
        Ok(())
    }
}

/// Verifies that the service adapter preserves lifecycle order.
#[tokio::test]
async fn service_role_adapter_runs_lifecycle_in_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let service = RecordingService {
        events: events.clone(),
    };
    let adapter = ServiceRoleAdapter::new(service);

    let result = TaskFactory::build(&adapter, task_context("service")).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *events.lock().expect("events lock"),
        vec!["init", "run", "shutdown"]
    );
}

/// Observes shutdown state through the service context.
struct ShutdownAwareService {
    /// Captured shutdown state seen by the service body.
    seen_shutdown: Arc<Mutex<bool>>,
}

impl ServiceRole for ShutdownAwareService {
    /// Records whether shutdown is already visible.
    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        ctx.wait_shutdown().await;
        *self.seen_shutdown.lock().expect("shutdown lock") = ctx.is_shutdown_requested();
        Ok(())
    }
}

/// Verifies that service context maps shutdown to runtime cancellation.
#[tokio::test]
async fn service_context_observes_runtime_shutdown() {
    let seen_shutdown = Arc::new(Mutex::new(false));
    let adapter = ServiceRoleAdapter::new(ShutdownAwareService {
        seen_shutdown: seen_shutdown.clone(),
    });
    let ctx = task_context("shutdown-service");
    ctx.cancel();

    let result = TaskFactory::build(&adapter, ctx).await;

    assert_eq!(result, TaskResult::Cancelled);
    assert!(*seen_shutdown.lock().expect("shutdown lock"));
}

/// Fails during the service run phase.
struct FailingService;

impl ServiceRole for FailingService {
    /// Returns a structured service error.
    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        Err(ServiceError::new(
            ctx.child_id().clone(),
            RoleLifecyclePhase::Run,
            "service body failed",
        ))
    }
}

/// Verifies that service errors become typed task failures.
#[tokio::test]
async fn service_error_maps_to_task_failure() {
    let adapter = ServiceRoleAdapter::new(FailingService);
    let result = TaskFactory::build(&adapter, task_context("failing-service")).await;

    match result {
        TaskResult::Failed(TaskFailure {
            kind,
            category,
            message,
        }) => {
            assert_eq!(kind, TaskFailureKind::Error);
            assert_eq!(category, "service_run");
            assert!(message.contains("service body failed"));
        }
        other => panic!("expected failed task result, got {other:?}"),
    }
}
