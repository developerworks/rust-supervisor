//! Sidecar role contract tests.
//!
//! These tests verify the sidecar role contract adapter without using inline
//! module tests in production source files.

use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::role::adapter::sidecar::SidecarRoleAdapter;
use rust_supervisor::role::context::sidecar::SidecarContext;
use rust_supervisor::role::lifecycle::RoleLifecyclePhase;
use rust_supervisor::role::result::sidecar::{SidecarError, SidecarResult};
use rust_supervisor::role::traits::sidecar::SidecarRole;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult};
use std::sync::{Arc, Mutex};

/// Creates a task context for sidecar contract tests.
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

/// Creates a primary child identifier for sidecar contract tests.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns the primary child identifier used by the test adapter.
fn primary_child_id() -> ChildId {
    ChildId::new("primary-service")
}

/// Records sidecar lifecycle phases in call order.
struct RecordingSidecar {
    /// Shared event list used by the test assertions.
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl SidecarRole for RecordingSidecar {
    /// Records the initialization phase.
    async fn init(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("init");
        Ok(())
    }

    /// Records the run phase.
    async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("run");
        Ok(())
    }

    /// Records the shutdown phase.
    async fn shutdown(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("shutdown");
        Ok(())
    }
}

/// Verifies that the sidecar adapter preserves lifecycle order.
#[tokio::test]
async fn sidecar_role_adapter_runs_lifecycle_in_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sidecar = RecordingSidecar {
        events: events.clone(),
    };
    let adapter = SidecarRoleAdapter::new(sidecar, primary_child_id());

    let result = TaskFactory::build(&adapter, task_context("metrics-sidecar")).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *events.lock().expect("events lock"),
        vec!["init", "run", "shutdown"]
    );
}

/// Captures the primary child identifier visible through sidecar context.
struct PrimaryAwareSidecar {
    /// Primary child identifier observed by the sidecar body.
    seen_primary_id: Arc<Mutex<Option<ChildId>>>,
}

impl SidecarRole for PrimaryAwareSidecar {
    /// Records the primary child identifier exposed by the context.
    async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        *self.seen_primary_id.lock().expect("primary id lock") = Some(ctx.primary_id().clone());
        Ok(())
    }
}

/// Verifies that sidecar context exposes the primary child identifier.
#[tokio::test]
async fn sidecar_context_exposes_primary_child_id() {
    let seen_primary_id = Arc::new(Mutex::new(None));
    let adapter = SidecarRoleAdapter::new(
        PrimaryAwareSidecar {
            seen_primary_id: seen_primary_id.clone(),
        },
        primary_child_id(),
    );

    let result = TaskFactory::build(&adapter, task_context("primary-aware-sidecar")).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *seen_primary_id.lock().expect("primary id lock"),
        Some(primary_child_id())
    );
}

/// Observes shutdown state through the sidecar context.
struct ShutdownAwareSidecar {
    /// Captured shutdown state seen by the sidecar body.
    seen_shutdown: Arc<Mutex<bool>>,
}

impl SidecarRole for ShutdownAwareSidecar {
    /// Records whether shutdown is already visible.
    async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        ctx.wait_shutdown().await;
        *self.seen_shutdown.lock().expect("shutdown lock") = ctx.is_shutdown_requested();
        Ok(())
    }
}

/// Verifies that sidecar context maps shutdown to runtime cancellation.
#[tokio::test]
async fn sidecar_context_observes_runtime_shutdown() {
    let seen_shutdown = Arc::new(Mutex::new(false));
    let adapter = SidecarRoleAdapter::new(
        ShutdownAwareSidecar {
            seen_shutdown: seen_shutdown.clone(),
        },
        primary_child_id(),
    );
    let ctx = task_context("shutdown-sidecar");
    ctx.cancel();

    let result = TaskFactory::build(&adapter, ctx).await;

    assert_eq!(result, TaskResult::Cancelled);
    assert!(*seen_shutdown.lock().expect("shutdown lock"));
}

/// Fails during the sidecar run phase.
struct FailingSidecar;

impl SidecarRole for FailingSidecar {
    /// Returns a structured sidecar error.
    async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        Err(SidecarError::new(
            ctx.child_id().clone(),
            ctx.primary_id().clone(),
            RoleLifecyclePhase::Run,
            "sidecar body failed",
        ))
    }
}

/// Verifies that sidecar errors become typed task failures.
#[tokio::test]
async fn sidecar_error_maps_to_task_failure() {
    let adapter = SidecarRoleAdapter::new(FailingSidecar, primary_child_id());
    let result = TaskFactory::build(&adapter, task_context("failing-sidecar")).await;

    match result {
        TaskResult::Failed(TaskFailure {
            kind,
            category,
            message,
        }) => {
            assert_eq!(kind, TaskFailureKind::Error);
            assert_eq!(category, "sidecar_run");
            assert!(message.contains("primary=primary-service"));
            assert!(message.contains("sidecar body failed"));
        }
        other => panic!("expected failed task result, got {other:?}"),
    }
}
