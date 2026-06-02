//! Supervisor role contract tests.
//!
//! These tests verify the nested supervisor role adapter without using inline
//! module tests in production source files.

use rust_supervisor::control::handle::SupervisorHandle;
use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::role::adapter::supervisor::SupervisorRoleAdapter;
use rust_supervisor::role::context::supervisor::SupervisorContext;
use rust_supervisor::role::lifecycle::RoleLifecyclePhase;
use rust_supervisor::role::result::supervisor::{SupervisorResult, SupervisorRoleError};
use rust_supervisor::role::traits::supervisor::SupervisorRole;
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult};
use rust_supervisor::test_support::test_time::with_auto_clock_drive;
use std::sync::{Arc, Mutex};

/// Creates a task context for supervisor contract tests.
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

/// Records supervisor lifecycle phases in call order.
struct RecordingSupervisor {
    /// Shared event list used by the test assertions.
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl SupervisorRole for RecordingSupervisor {
    /// Records the nested tree construction phase.
    async fn build_tree(&mut self, ctx: &SupervisorContext) -> SupervisorResult<SupervisorSpec> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("build_tree");
        Ok(SupervisorSpec::root(Vec::new()))
    }

    /// Records that the role body ran after nested supervisor startup.
    async fn run(
        &mut self,
        ctx: &SupervisorContext,
        handle: &SupervisorHandle,
    ) -> SupervisorResult<()> {
        let _ = (ctx, handle);
        self.events.lock().expect("events lock").push("run");
        Ok(())
    }
}

/// Verifies that the supervisor adapter calls `build_tree` before `run`.
#[tokio::test(start_paused = true)]
async fn supervisor_role_adapter_calls_build_tree() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let supervisor = RecordingSupervisor {
        events: events.clone(),
    };
    let adapter = SupervisorRoleAdapter::new(supervisor);

    let result = with_auto_clock_drive(TaskFactory::build(
        &adapter,
        task_context("nested-supervisor"),
    ))
    .await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *events.lock().expect("events lock"),
        vec!["build_tree", "run"]
    );
}

/// Observes shutdown state through the supervisor context.
struct ShutdownAwareSupervisor {
    /// Captured shutdown state seen by the supervisor body.
    seen_shutdown: Arc<Mutex<bool>>,
}

impl SupervisorRole for ShutdownAwareSupervisor {
    /// Builds an empty nested supervisor tree.
    async fn build_tree(&mut self, ctx: &SupervisorContext) -> SupervisorResult<SupervisorSpec> {
        let _ = ctx;
        Ok(SupervisorSpec::root(Vec::new()))
    }

    /// Records whether outer shutdown is visible while the role body runs.
    async fn run(
        &mut self,
        ctx: &SupervisorContext,
        handle: &SupervisorHandle,
    ) -> SupervisorResult<()> {
        let _ = handle;
        ctx.wait_shutdown().await;
        *self.seen_shutdown.lock().expect("shutdown lock") = ctx.is_shutdown_requested();
        Ok(())
    }
}

/// Verifies that supervisor context maps shutdown to runtime cancellation.
#[tokio::test(start_paused = true)]
async fn supervisor_context_observes_runtime_shutdown() {
    let seen_shutdown = Arc::new(Mutex::new(false));
    let adapter = SupervisorRoleAdapter::new(ShutdownAwareSupervisor {
        seen_shutdown: seen_shutdown.clone(),
    });
    let ctx = task_context("shutdown-supervisor");
    ctx.cancel();

    let result = with_auto_clock_drive(TaskFactory::build(&adapter, ctx)).await;

    assert_eq!(result, TaskResult::Cancelled);
    assert!(*seen_shutdown.lock().expect("shutdown lock"));
}

/// Fails during the supervisor build_tree phase.
struct FailingSupervisor;

impl SupervisorRole for FailingSupervisor {
    /// Returns a structured supervisor role error.
    async fn build_tree(&mut self, ctx: &SupervisorContext) -> SupervisorResult<SupervisorSpec> {
        Err(SupervisorRoleError::new(
            ctx.child_id().clone(),
            RoleLifecyclePhase::BuildTree,
            "nested tree could not be built",
        ))
    }
}

/// Verifies that supervisor role errors become typed task failures.
#[tokio::test(start_paused = true)]
async fn supervisor_role_error_maps_to_task_failure() {
    let adapter = SupervisorRoleAdapter::new(FailingSupervisor);
    let result = TaskFactory::build(&adapter, task_context("failing-supervisor")).await;

    match result {
        TaskResult::Failed(TaskFailure {
            kind,
            category,
            message,
        }) => {
            assert_eq!(kind, TaskFailureKind::Error);
            assert_eq!(category, "supervisor_build_tree");
            assert!(message.contains("nested tree could not be built"));
        }
        other => panic!("expected failed task result, got {other:?}"),
    }
}

/// Builds a nested supervisor specification that fails runtime validation.
struct InvalidSpecSupervisor;

impl SupervisorRole for InvalidSpecSupervisor {
    /// Returns a nested tree with an invalid control channel capacity.
    async fn build_tree(&mut self, ctx: &SupervisorContext) -> SupervisorResult<SupervisorSpec> {
        let _ = ctx;
        let mut spec = SupervisorSpec::root(Vec::new());
        spec.control_channel_capacity = 0;
        Ok(spec)
    }
}

/// Verifies that nested supervisor startup errors become typed task failures.
#[tokio::test(start_paused = true)]
async fn supervisor_start_error_maps_to_task_failure() {
    let adapter = SupervisorRoleAdapter::new(InvalidSpecSupervisor);
    let result = TaskFactory::build(&adapter, task_context("invalid-supervisor")).await;

    match result {
        TaskResult::Failed(TaskFailure {
            kind,
            category,
            message,
        }) => {
            assert_eq!(kind, TaskFailureKind::Error);
            assert_eq!(category, "supervisor_build_tree");
            assert!(message.contains("control channel capacity"));
        }
        other => panic!("expected failed task result, got {other:?}"),
    }
}
