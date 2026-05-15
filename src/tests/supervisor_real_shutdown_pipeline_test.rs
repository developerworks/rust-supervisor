//! Real shutdown pipeline integration tests.
//!
//! These tests verify cancellation delivery, ordered graceful drain, abort
//! escalation, late reports, and idempotent report caching.

use rust_supervisor::control::command::CommandResult;
use rust_supervisor::control::handle::SupervisorHandle;
use rust_supervisor::control::outcome::{ChildControlOperation, ChildStopState};
use rust_supervisor::id::types::ChildId;
use rust_supervisor::shutdown::coordinator::ShutdownResult;
use rust_supervisor::shutdown::report::ChildShutdownStatus;
use rust_supervisor::shutdown::stage::{ShutdownPhase, ShutdownPolicy};
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::{runtime::supervisor::Supervisor, task::factory::TaskFactory};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, mpsc};
use tokio::time::timeout;

/// Verifies that shutdown cancels every running child child_start_count.
#[tokio::test]
async fn shutdown_tree_cancels_all_running_children() {
    let (cancel_sender, mut cancel_receiver) = mpsc::channel(4);
    let spec = SupervisorSpec::root(vec![
        cancellable_child("alpha", cancel_sender.clone()),
        cancellable_child("beta", cancel_sender),
    ]);
    let handle = start_with_short_policy(spec, true).await;

    let result = shutdown_with_timeout(&handle).await;

    assert_completed_report(&result);
    let mut observed = Vec::new();
    for _index in 0..2 {
        observed.push(
            timeout(Duration::from_secs(1), cancel_receiver.recv())
                .await
                .expect("cancellation observation should arrive")
                .expect("cancellation channel should stay open"),
        );
    }
    observed.sort();
    assert_eq!(observed, vec![String::from("alpha"), String::from("beta")]);
    let report = result
        .report
        .expect("completed shutdown should include report");
    assert!(
        report
            .outcomes
            .iter()
            .all(|outcome| outcome.cancel_delivered)
    );
}

/// Verifies that shutdown uses runtime state handles for cancellation.
#[tokio::test]
async fn shutdown_pipeline_uses_child_runtime_state_handles_test() {
    let (cancel_sender, mut cancel_receiver) = mpsc::channel(1);
    let child_id = ChildId::new("worker");
    let spec = SupervisorSpec::root(vec![cancellable_child("worker", cancel_sender)]);
    let handle = start_with_short_policy(spec, true).await;

    let current_state = handle.current_state().await.expect("current state");
    match current_state {
        CommandResult::CurrentState { state } => {
            let record = state
                .child_runtime_records
                .iter()
                .find(|record| record.child_id == child_id)
                .expect("worker runtime state should exist");
            assert!(record.attempt.is_some());
        }
        other => panic!("unexpected current state result: {other:?}"),
    }

    let result = shutdown_with_timeout(&handle).await;

    assert_eq!(
        timeout(Duration::from_secs(1), cancel_receiver.recv())
            .await
            .expect("cancellation observation should arrive")
            .expect("cancellation channel should stay open"),
        "worker"
    );
    let report = result
        .report
        .expect("completed shutdown should include report");
    assert_eq!(report.outcomes[0].child_id, ChildId::new("worker"));
    assert!(report.outcomes[0].cancel_delivered);
}

/// Verifies that completed children are not cancelled again.
#[tokio::test]
async fn shutdown_tree_marks_inactive_children_already_exited() {
    let spec = SupervisorSpec::root(vec![finished_child("short")]);
    let handle = start_with_short_policy(spec, true).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result = shutdown_with_timeout(&handle).await;

    let report = result
        .report
        .expect("completed shutdown should include report");
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].child_id, ChildId::new("short"));
    assert_eq!(
        report.outcomes[0].status,
        ChildShutdownStatus::AlreadyExited
    );
    assert!(!report.outcomes[0].cancel_delivered);
}

/// Verifies that graceful drain follows shutdown order.
#[tokio::test]
async fn shutdown_tree_waits_in_shutdown_order() {
    let (cancel_sender, _cancel_receiver) = mpsc::channel(4);
    let spec = SupervisorSpec::root(vec![
        cancellable_child("first", cancel_sender.clone()),
        cancellable_child("second", cancel_sender.clone()),
        cancellable_child("third", cancel_sender),
    ]);
    let handle = start_with_short_policy(spec, true).await;

    let result = shutdown_with_timeout(&handle).await;

    let report = result
        .report
        .expect("completed shutdown should include report");
    let order = report
        .outcomes
        .iter()
        .map(|outcome| outcome.child_id.value.as_str())
        .collect::<Vec<_>>();
    assert_eq!(order, vec!["third", "second", "first"]);
}

/// Verifies that cooperative children become graceful shutdown outcomes.
#[tokio::test]
async fn shutdown_tree_records_graceful_child_outcomes() {
    let (cancel_sender, _cancel_receiver) = mpsc::channel(2);
    let spec = SupervisorSpec::root(vec![cancellable_child("worker", cancel_sender)]);
    let handle = start_with_short_policy(spec, true).await;

    let result = shutdown_with_timeout(&handle).await;

    let report = result
        .report
        .expect("completed shutdown should include report");
    assert_eq!(report.outcomes[0].status, ChildShutdownStatus::Graceful);
    assert_eq!(report.outcomes[0].phase, ShutdownPhase::GracefulDrain);
}

/// Verifies that non-cooperative children are aborted after graceful timeout.
#[tokio::test]
async fn shutdown_tree_aborts_straggler_after_timeout() {
    let spec = SupervisorSpec::root(vec![stubborn_child("stubborn")]);
    let handle = start_with_short_policy(spec, true).await;

    let result = shutdown_with_timeout(&handle).await;

    let report = result
        .report
        .expect("completed shutdown should include report");
    assert_eq!(report.outcomes[0].status, ChildShutdownStatus::Aborted);
    assert_eq!(report.outcomes[0].phase, ShutdownPhase::AbortStragglers);
}

/// Verifies that repeated shutdown returns the cached report as idempotent.
#[tokio::test]
async fn repeated_shutdown_tree_returns_cached_idempotent_report() {
    let (cancel_sender, _cancel_receiver) = mpsc::channel(2);
    let spec = SupervisorSpec::root(vec![cancellable_child("worker", cancel_sender)]);
    let handle = start_with_short_policy(spec, true).await;

    let first = shutdown_with_timeout(&handle).await;
    let second = shutdown_with_timeout(&handle).await;

    assert!(!first.idempotent);
    assert!(second.idempotent);
    let first_report = first.report.expect("first shutdown report");
    let second_report = second.report.expect("second shutdown report");
    assert!(!first_report.idempotent);
    assert!(second_report.idempotent);
    assert_eq!(first_report.outcomes, second_report.outcomes);
}

/// Verifies that abort-disabled policy records a late report.
#[tokio::test]
async fn shutdown_tree_records_late_child_report_when_abort_is_disabled() {
    let spec = SupervisorSpec::root(vec![late_reporting_child("late")]);
    let handle = start_with_short_policy(spec, false).await;

    let result = shutdown_with_timeout(&handle).await;

    let report = result
        .report
        .expect("completed shutdown should include report");
    assert_eq!(report.outcomes[0].status, ChildShutdownStatus::LateReport);
    assert_eq!(report.outcomes[0].phase, ShutdownPhase::AbortStragglers);
}

/// Verifies that paused runtime state still waits for the active report.
#[tokio::test]
async fn shutdown_pipeline_waits_for_paused_runtime_state_report() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let (cancelled_sender, mut cancelled_receiver) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![releasable_cancelled_child(
        "worker",
        started_sender,
        cancelled_sender,
        release.clone(),
    )]);
    let handle = start_with_t046_policy(spec).await;
    started_receiver.recv().await.expect("child should start");

    let pause = handle
        .pause_child(ChildId::new("worker"), "operator", "pause before shutdown")
        .await
        .expect("pause child");

    assert_control_operation(
        pause,
        ChildControlOperation::Paused,
        ChildStopState::CancelDelivered,
    );
    assert_eq!(
        cancelled_receiver
            .recv()
            .await
            .expect("cancellation should be observed"),
        "worker"
    );

    let shutdown_handle = handle.clone();
    let mut shutdown_task =
        tokio::spawn(async move { shutdown_with_timeout(&shutdown_handle).await });
    assert!(
        timeout(Duration::from_millis(20), &mut shutdown_task)
            .await
            .is_err(),
        "shutdown should wait for the paused child report"
    );

    release.notify_waiters();
    let result = shutdown_task.await.expect("shutdown task should join");
    let report = result
        .report
        .expect("completed shutdown should include report");
    assert_eq!(report.outcomes[0].status, ChildShutdownStatus::Graceful);
    assert!(report.outcomes[0].cancel_delivered);
}

/// Verifies that quarantined runtime state still waits for the active report.
#[tokio::test]
async fn shutdown_pipeline_waits_for_quarantined_runtime_state_report() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let (cancelled_sender, mut cancelled_receiver) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![releasable_cancelled_child(
        "worker",
        started_sender,
        cancelled_sender,
        release.clone(),
    )]);
    let handle = start_with_t046_policy(spec).await;
    started_receiver.recv().await.expect("child should start");

    let quarantine = handle
        .quarantine_child(
            ChildId::new("worker"),
            "operator",
            "quarantine before shutdown",
        )
        .await
        .expect("quarantine child");

    assert_control_operation(
        quarantine,
        ChildControlOperation::Quarantined,
        ChildStopState::CancelDelivered,
    );
    assert_eq!(
        cancelled_receiver
            .recv()
            .await
            .expect("cancellation should be observed"),
        "worker"
    );

    let shutdown_handle = handle.clone();
    let mut shutdown_task =
        tokio::spawn(async move { shutdown_with_timeout(&shutdown_handle).await });
    assert!(
        timeout(Duration::from_millis(20), &mut shutdown_task)
            .await
            .is_err(),
        "shutdown should wait for the quarantined child report"
    );

    release.notify_waiters();
    let result = shutdown_task.await.expect("shutdown task should join");
    let report = result
        .report
        .expect("completed shutdown should include report");
    assert_eq!(report.outcomes[0].status, ChildShutdownStatus::Graceful);
    assert!(report.outcomes[0].cancel_delivered);
}

/// Verifies that removed runtime state skips the shutdown path.
#[tokio::test]
async fn shutdown_pipeline_skips_removed_runtime_state() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let (cancelled_sender, mut cancelled_receiver) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![releasable_cancelled_child(
        "worker",
        started_sender,
        cancelled_sender,
        release.clone(),
    )]);
    let handle = start_with_t046_policy(spec).await;
    started_receiver.recv().await.expect("child should start");

    let remove = handle
        .remove_child(ChildId::new("worker"), "operator", "remove before shutdown")
        .await
        .expect("remove child");

    assert_control_operation(
        remove,
        ChildControlOperation::Removed,
        ChildStopState::CancelDelivered,
    );
    assert_eq!(
        cancelled_receiver
            .recv()
            .await
            .expect("cancellation should be observed"),
        "worker"
    );

    let result = shutdown_with_timeout(&handle).await;
    release.notify_waiters();

    let report = result
        .report
        .expect("completed shutdown should include report");
    assert_eq!(
        report.outcomes[0].status,
        ChildShutdownStatus::AlreadyExited
    );
    assert!(!report.outcomes[0].cancel_delivered);
}

/// Starts a supervisor with short shutdown budgets.
async fn start_with_short_policy(
    spec: SupervisorSpec,
    abort_after_timeout: bool,
) -> SupervisorHandle {
    Supervisor::start_with_policy(
        spec,
        ShutdownPolicy::new(
            Duration::from_millis(10),
            Duration::from_millis(200),
            abort_after_timeout,
        ),
    )
    .await
    .expect("supervisor should start")
}

/// Starts a supervisor with a wider graceful budget for T046 regression checks.
async fn start_with_t046_policy(spec: SupervisorSpec) -> SupervisorHandle {
    Supervisor::start_with_policy(
        spec,
        ShutdownPolicy::new(Duration::from_millis(100), Duration::from_millis(250), true),
    )
    .await
    .expect("supervisor should start")
}

/// Requests shutdown and unwraps the structured shutdown result.
async fn shutdown_with_timeout(handle: &SupervisorHandle) -> ShutdownResult {
    let command_result = timeout(
        Duration::from_secs(2),
        handle.shutdown_tree("operator", "real shutdown pipeline test"),
    )
    .await
    .expect("shutdown command should complete")
    .expect("shutdown command should succeed");
    match command_result {
        CommandResult::Shutdown { result } => result,
        other => panic!("unexpected shutdown result: {other:?}"),
    }
}

/// Asserts that a shutdown result completed with a report.
fn assert_completed_report(result: &ShutdownResult) {
    assert_eq!(result.phase, ShutdownPhase::Completed);
    assert!(result.report.is_some());
}

/// Asserts the operation result returned by a child control command.
fn assert_control_operation(
    result: CommandResult,
    operation: ChildControlOperation,
    stop_state: ChildStopState,
) {
    let outcome = match result {
        CommandResult::ChildControl { outcome } => outcome,
        other => panic!("unexpected child control result: {other:?}"),
    };
    assert_eq!(outcome.child_id, ChildId::new("worker"));
    assert_eq!(outcome.operation_after, operation);
    assert_eq!(outcome.stop_state, stop_state);
    assert!(outcome.cancel_delivered);
}

/// Creates a child that stops when its cancellation token is cancelled.
fn cancellable_child(name: &'static str, sender: mpsc::Sender<String>) -> ChildSpec {
    worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let sender = sender.clone();
            async move {
                ctx.cancellation_token().cancelled().await;
                let _ignored = sender.send(ctx.child_id.value.clone()).await;
                TaskResult::Cancelled
            }
        }),
    )
}

/// Creates a child that reports cancellation and waits for test release.
fn releasable_cancelled_child(
    name: &'static str,
    started_sender: mpsc::Sender<String>,
    cancelled_sender: mpsc::Sender<String>,
    release: Arc<Notify>,
) -> ChildSpec {
    worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let started_sender = started_sender.clone();
            let cancelled_sender = cancelled_sender.clone();
            let release = release.clone();
            async move {
                let _ignored = started_sender.send(ctx.child_id.value.clone()).await;
                ctx.cancellation_token().cancelled().await;
                let _ignored = cancelled_sender.send(ctx.child_id.value.clone()).await;
                release.notified().await;
                TaskResult::Cancelled
            }
        }),
    )
}

/// Creates a child that exits before shutdown starts.
fn finished_child(name: &'static str) -> ChildSpec {
    worker_child(
        name,
        service_fn(|_ctx: TaskContext| async { TaskResult::Succeeded }),
    )
}

/// Creates a child that never cooperates with cancellation.
fn stubborn_child(name: &'static str) -> ChildSpec {
    worker_child(
        name,
        service_fn(|_ctx: TaskContext| async { std::future::pending::<TaskResult>().await }),
    )
}

/// Creates a child that reports after the graceful timeout.
fn late_reporting_child(name: &'static str) -> ChildSpec {
    worker_child(
        name,
        service_fn(|ctx: TaskContext| async move {
            ctx.cancellation_token().cancelled().await;
            tokio::time::sleep(Duration::from_millis(30)).await;
            TaskResult::Cancelled
        }),
    )
}

/// Creates a worker child from a task factory.
fn worker_child(name: &'static str, factory: impl TaskFactory) -> ChildSpec {
    ChildSpec::worker(
        ChildId::new(name),
        name,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
}
