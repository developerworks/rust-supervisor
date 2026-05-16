//! Integration tests for generation fencing.

use rust_supervisor::child_runner::run_exit::TaskExit;
use rust_supervisor::child_runner::runner::ChildRunReport;
use rust_supervisor::control::command::{CommandResult, CurrentState};
use rust_supervisor::control::outcome::{
    ChildAttemptStatus, ChildControlOperation, ChildControlResult, ChildLivenessState,
    ChildStopState, GenerationFenceDecision, RestartLimitState,
};
use rust_supervisor::dashboard::model::dashboard_command_result_value;
use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::event::payload::What;
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::readiness::signal::ReadinessState;
use rust_supervisor::registry::entry::{ChildRuntime, ChildRuntimeStatus};
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::{SupervisionStrategy, SupervisorSpec};
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult, service_fn};
use rust_supervisor::test_support::test_time::{advance_test_clock, with_auto_clock_drive};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;

/// Creates a worker child from a task factory.
fn worker_child(name: &'static str, factory: impl TaskFactory) -> ChildSpec {
    ChildSpec::worker(
        ChildId::new(name),
        name,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
}

/// Parses a [`CurrentState`] from a supervisor command envelope.
fn expect_current_state(result: CommandResult) -> CurrentState {
    match result {
        CommandResult::CurrentState { state } => state,
        other => panic!("expected CurrentState command result but got {other:?}"),
    }
}

/// Smoke test: only checks that this test target is registered and runs.
#[test]
fn supervisor_generation_fencing_smoke() {
    assert_eq!(Generation::initial().value, 0);
}

/// Dashboard `child_control` projection must include a nullable `generation_fence` key.
#[test]
fn generation_fence_optional_field_present_in_dashboard_child_control_projection_test() {
    let outcome = ChildControlResult::new(
        ChildId::new("demo_child"),
        Some(ChildStartCount::first()),
        Some(Generation::initial()),
        ChildControlOperation::Active,
        ChildControlOperation::Active,
        Some(ChildAttemptStatus::Running),
        false,
        ChildStopState::Idle,
        RestartLimitState::new(Duration::from_secs(60), 10, 0, 1),
        ChildLivenessState::new(Some(10), false, ReadinessState::Unreported),
        false,
        None,
        None,
    );
    let wrapped = CommandResult::ChildControl { outcome };
    let value = dashboard_command_result_value(&wrapped).expect("dashboard serialization");
    assert!(value["outcome"].get("generation_fence").is_some());
    assert_eq!(
        value["outcome"]["generation_fence"],
        serde_json::Value::Null
    );
}

/// Verifies restart delivers cancellation, fences the attempt, and keeps a single active handle.
#[tokio::test(start_paused = true)]
async fn restart_child_sends_cancel_before_second_spawn_test() {
    let child_id = ChildId::new("worker");
    let start_counter_inner = Arc::new(AtomicUsize::new(0));
    let start_counter_body = start_counter_inner.clone();

    let (boot_tx, mut boot_rx) = mpsc::channel(1);
    let (cancel_ack_tx, mut cancel_ack_rx) = mpsc::channel(1);
    let (second_boot_tx, mut second_boot_rx) = mpsc::channel(1);

    let boot_clone = boot_tx.clone();
    let cancel_clone = cancel_ack_tx.clone();
    let second_clone = second_boot_tx.clone();

    let spec = SupervisorSpec::root(vec![worker_child(
        "worker",
        service_fn(move |ctx: TaskContext| {
            let boot_clone = boot_clone.clone();
            let cancel_clone = cancel_clone.clone();
            let second_clone = second_clone.clone();
            let start_counter_body = start_counter_body.clone();
            async move {
                let invocation = start_counter_body.fetch_add(1, Ordering::SeqCst);
                if invocation == 0 {
                    let _ignored = boot_clone.send(()).await;
                    ctx.cancellation_token().cancelled().await;
                    let _ignored = cancel_clone.send(()).await;
                    tokio::time::sleep(Duration::from_millis(400)).await;
                    TaskResult::Cancelled
                } else {
                    let _ignored = second_clone.send(()).await;
                    ctx.cancellation_token().cancelled().await;
                    TaskResult::Cancelled
                }
            }
        }),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    boot_rx.recv().await.expect("child should boot");

    let restart = handle
        .restart_child(child_id.clone(), "operator", "fence-delivers-cancel")
        .await
        .expect("restart command");
    let outcome = match restart {
        CommandResult::ChildControl { outcome } => outcome,
        other => panic!("unexpected restart result: {other:?}"),
    };
    assert_eq!(
        outcome
            .generation_fence
            .as_ref()
            .expect("fence payload")
            .decision,
        GenerationFenceDecision::QueuedAfterStop
    );
    assert!(outcome.cancel_delivered);

    cancel_ack_rx
        .recv()
        .await
        .expect("cancellation must reach the supervised task");

    let recorder = handle.observability_recorder();
    assert!(recorder.events.iter().any(|event| matches!(
        &event.what,
        What::ChildRestartFenceEntered { child_id: id, .. } if *id == child_id
    )));
    assert!(recorder.events.iter().any(|event| matches!(
        &event.what,
        What::ChildControlCancelDelivered { child_id: id, .. } if *id == child_id
    )));

    let current = expect_current_state(handle.current_state().await.expect("state"));
    let worker = current
        .child_runtime_records
        .iter()
        .find(|record| record.child_id == child_id)
        .expect("worker projection");
    assert_eq!(
        worker.generation.map(|generation| generation.value),
        Some(0)
    );
    assert_eq!(worker.attempt.map(|attempt| attempt.value), Some(1));
    assert_eq!(worker.status, Some(ChildAttemptStatus::Cancelling));

    advance_test_clock(Duration::from_millis(400)).await;
    second_boot_rx
        .recv()
        .await
        .expect("second supervised attempt boots after draining the fenced stop");

    let refreshed =
        expect_current_state(handle.current_state().await.expect("state after respawn"));
    let worker_state = refreshed
        .child_runtime_records
        .iter()
        .find(|record| record.child_id == child_id)
        .expect("worker record after respawn");
    assert_eq!(
        worker_state.generation.map(|generation| generation.value),
        Some(1_u64),
        "{worker_state:?}"
    );

    let _ignored = with_auto_clock_drive(async {
        handle
            .shutdown_tree("operator", "finish fenced restart test")
            .await
    })
    .await;
}

/// Verifies manual restart waits for graceful stop semantics and cites the old identities.
#[tokio::test(start_paused = true)]
async fn restart_child_queues_after_stop_decision_test() {
    let child_id = ChildId::new("worker");
    let baseline = Generation::initial();
    let baseline_attempt = ChildStartCount::first();
    let (started_tx, mut started_rx) = mpsc::channel(1);
    let spec = SupervisorSpec::root(vec![worker_child(
        "worker",
        service_fn(move |ctx: TaskContext| {
            let started_tx = started_tx.clone();
            async move {
                let _ignored = started_tx.send(()).await;
                ctx.cancellation_token().cancelled().await;
                TaskResult::Cancelled
            }
        }),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    started_rx.recv().await.expect("child starts");

    let restart = handle
        .restart_child(child_id.clone(), "operator", "queued restart")
        .await
        .expect("restart command");
    let outcome = match restart {
        CommandResult::ChildControl { outcome } => outcome,
        other => panic!("unexpected restart outcome: {other:?}"),
    };
    assert_eq!(
        outcome
            .generation_fence
            .as_ref()
            .expect("fence payload present")
            .decision,
        GenerationFenceDecision::QueuedAfterStop
    );
    assert_eq!(outcome.generation, Some(baseline));
    assert_eq!(outcome.attempt, Some(baseline_attempt));
    assert_eq!(
        outcome
            .generation_fence
            .as_ref()
            .expect("fence payload holds target generation")
            .target_generation,
        Some(Generation { value: 1 })
    );

    let _ignored = with_auto_clock_drive(async {
        handle
            .shutdown_tree("operator", "finish queued restart test")
            .await
    })
    .await;
}

/// Verifies restart during shutdown returns a deterministic blocked outcome without spawning anew.
#[tokio::test(start_paused = true)]
async fn restart_child_blocked_during_tree_shutdown_test() {
    let child_id = ChildId::new("worker");
    let (started_tx, mut started_rx) = mpsc::channel(1);
    let spec = SupervisorSpec::root(vec![worker_child(
        "worker",
        service_fn(move |ctx: TaskContext| {
            let started_tx = started_tx.clone();
            async move {
                let _ignored = started_tx.send(()).await;
                ctx.cancellation_token().cancelled().await;
                TaskResult::Cancelled
            }
        }),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    started_rx.recv().await.expect("child starts");

    with_auto_clock_drive(async {
        handle
            .shutdown_tree("operator", "blocking restart test")
            .await
    })
    .await
    .expect("shutdown");
    let blocked = handle
        .restart_child(
            child_id.clone(),
            "operator",
            "restart while shutdown completes",
        )
        .await
        .expect("restart should still dispatch");
    let outcome = match blocked {
        CommandResult::ChildControl { outcome } => outcome,
        other => panic!("unexpected blocked restart result {other:?}"),
    };
    assert_eq!(
        outcome
            .generation_fence
            .as_ref()
            .expect("generation fence populated")
            .decision,
        GenerationFenceDecision::BlockedByShutdown
    );

    let recorder_after = handle.observability_recorder();
    assert!(!recorder_after.events.iter().any(|event| matches!(
        &event.what,
        What::ChildRestartFenceEntered { child_id: id, .. } if *id == child_id
    )));
}

/// Validates spawn failures after fencing retain the earlier exit verdict and expose the error.
#[tokio::test(start_paused = true)]
async fn pending_restart_target_spawn_failure_retains_prior_outcomes_test() {
    const SPAWN_HOOK_CASE_CHILD: &str = "worker_spawn_hook_fence_case";
    let child_id = ChildId::new(SPAWN_HOOK_CASE_CHILD);
    let (boot_tx, mut boot_rx) = mpsc::channel(1);
    let (cancel_ack_tx, mut cancel_ack_rx) = mpsc::channel(1);
    let boot_body = boot_tx.clone();
    let cancel_body = cancel_ack_tx.clone();

    let spec = SupervisorSpec::root(vec![worker_child(
        SPAWN_HOOK_CASE_CHILD,
        service_fn(move |ctx: TaskContext| {
            let boot_body = boot_body.clone();
            let cancel_body = cancel_body.clone();
            async move {
                let _ignored = boot_body.send(()).await;
                ctx.cancellation_token().cancelled().await;
                let _ignored = cancel_body.send(()).await;
                tokio::time::sleep(Duration::from_millis(300)).await;
                TaskResult::Cancelled
            }
        }),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    boot_rx.recv().await.expect("child starts");

    handle
        .restart_child(child_id.clone(), "operator", "fence for spawn hook")
        .await
        .expect("restart fences old attempt");

    cancel_ack_rx
        .recv()
        .await
        .expect("supervised task observes cancel before we arm the spawn hook");

    rust_supervisor::test_support::child_spawn::fail_next_child_spawns_for(child_id.clone(), 1);

    let mut stop_completed_seen = false;
    let mut retained_failure_observed = false;
    let mut recorder_capture = handle.observability_recorder();

    for _ in 0..400 {
        advance_test_clock(Duration::from_millis(10)).await;
        recorder_capture = handle.observability_recorder();
        stop_completed_seen |= recorder_capture.events.iter().any(|event| {
            matches!(
                &event.what,
                What::ChildControlStopCompleted { exit_kind, .. }
                    if matches!(exit_kind, TaskExit::Cancelled)
            )
        });

        let current = expect_current_state(handle.current_state().await.expect("state"));
        let Some(worker) = current
            .child_runtime_records
            .iter()
            .find(|record| record.child_id == child_id)
        else {
            continue;
        };
        if let Some(failure) = &worker.failure {
            retained_failure_observed = true;
            assert!(
                failure.reason.contains("test hook"),
                "unexpected diagnostic: {}",
                failure.reason
            );
            assert_eq!(worker.generation, Some(Generation::initial()), "{worker:?}");
            assert_eq!(worker.attempt, Some(ChildStartCount::first()), "{worker:?}");
            break;
        }
    }

    assert!(
        stop_completed_seen,
        "observer should record the graceful cancel exit before failing the next spawn"
    );
    assert!(
        retained_failure_observed,
        "runtime projection should expose the spawn failure alongside the preceding exit verdict"
    );
    assert!(
        !recorder_capture
            .events
            .iter()
            .any(|event| matches!(&event.what, What::ChildRestartFenceReleased { .. }))
    );

    let _ignored = with_auto_clock_drive(async {
        handle
            .shutdown_tree("operator", "finish spawn hook test")
            .await
    })
    .await;
}

/// Verifies duplicate restart commands collapse into [`GenerationFenceDecision::AlreadyPending`].
#[tokio::test(start_paused = true)]
async fn duplicate_restart_child_merges_to_already_pending_test() {
    let child_id = ChildId::new("worker_dup_fence");
    let (boot_tx, mut boot_rx) = mpsc::channel(1);
    let boot_clone = boot_tx.clone();

    let spec = SupervisorSpec::root(vec![worker_child(
        "worker_dup_fence",
        service_fn(move |ctx: TaskContext| {
            let boot_clone = boot_clone.clone();
            async move {
                let _ignored = boot_clone.send(()).await;
                ctx.cancellation_token().cancelled().await;
                tokio::time::sleep(Duration::from_millis(400)).await;
                TaskResult::Cancelled
            }
        }),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    boot_rx.recv().await.expect("child boots");

    let first = handle
        .restart_child(child_id.clone(), "operator", "dup fence first")
        .await
        .expect("first restart");
    let first_outcome = match first {
        CommandResult::ChildControl { outcome } => outcome,
        other => panic!("unexpected first restart result {other:?}"),
    };
    assert_eq!(
        first_outcome
            .generation_fence
            .as_ref()
            .expect("fence payload")
            .decision,
        GenerationFenceDecision::QueuedAfterStop
    );

    let second = handle
        .restart_child(child_id.clone(), "operator", "dup fence second")
        .await
        .expect("second restart");
    let second_outcome = match second {
        CommandResult::ChildControl { outcome } => outcome,
        other => panic!("unexpected second restart result {other:?}"),
    };
    assert_eq!(
        second_outcome
            .generation_fence
            .as_ref()
            .expect("fence payload")
            .decision,
        GenerationFenceDecision::AlreadyPending
    );
    assert!(!second_outcome.cancel_delivered);

    let recorder = handle.observability_recorder();
    assert!(recorder.events.iter().any(|event| matches!(
        &event.what,
        What::ChildRestartConflict {
            decision,
            child_id: id,
            ..
        } if *id == child_id && decision == "already_pending"
    )));

    let _ignored = with_auto_clock_drive(async {
        handle
            .shutdown_tree("operator", "finish dup fence test")
            .await
    })
    .await;
}

/// Verifies automatic restart scope respects manual pending restart gates without duplicate spawns.
#[tokio::test(start_paused = true)]
async fn auto_restart_and_manual_restart_share_fence_gate_test() {
    let beta_id = ChildId::new("scope_beta");
    let beta_invocations = Arc::new(AtomicUsize::new(0));
    let beta_invocations_body = beta_invocations.clone();

    let mut spec = SupervisorSpec::root(vec![
        worker_child(
            "scope_alpha",
            service_fn(move |_ctx: TaskContext| async move {
                tokio::time::sleep(Duration::from_millis(200)).await;
                TaskResult::Failed(TaskFailure::new(
                    TaskFailureKind::Error,
                    "test",
                    "scope alpha triggers restart plan",
                ))
            }),
        ),
        worker_child(
            "scope_beta",
            service_fn(move |ctx: TaskContext| {
                let beta_invocations_body = beta_invocations_body.clone();
                async move {
                    beta_invocations_body.fetch_add(1, Ordering::SeqCst);
                    ctx.cancellation_token().cancelled().await;
                    tokio::time::sleep(Duration::from_millis(400)).await;
                    TaskResult::Cancelled
                }
            }),
        ),
    ]);
    spec.strategy = SupervisionStrategy::OneForAll;

    let handle = Supervisor::start(spec).await.expect("start supervisor");
    advance_test_clock(Duration::from_millis(40)).await;
    assert_eq!(
        beta_invocations.load(Ordering::SeqCst),
        1,
        "beta should boot exactly once before fencing"
    );

    handle
        .restart_child(
            beta_id.clone(),
            "operator",
            "manual fence blocks automatic restart",
        )
        .await
        .expect("restart beta");

    for _step_millis in 0..250 {
        advance_test_clock(Duration::from_millis(1)).await;
        tokio::task::yield_now().await;
    }

    assert_eq!(
        beta_invocations.load(Ordering::SeqCst),
        1,
        "automatic restart path must not spawn a second beta attempt while fencing waits"
    );

    let recorder = handle.observability_recorder();
    let restart_conflicts_for_debug = recorder
        .events
        .iter()
        .filter_map(|event| match &event.what {
            What::ChildRestartConflict {
                child_id,
                command_id,
                decision,
                reason,
                ..
            } => Some((
                child_id.clone(),
                command_id.clone(),
                decision.clone(),
                reason.clone(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        recorder.events.iter().any(|event| matches!(
            &event.what,
            What::ChildRestartConflict {
                child_id: id,
                command_id,
                ..
            } if *id == beta_id && command_id == "runtime-policy"
        )),
        "expected runtime-policy automatic restart suppression on {:?}; conflicts observed {:?}",
        beta_id,
        restart_conflicts_for_debug
    );
    let _ignored = with_auto_clock_drive(async {
        handle
            .shutdown_tree("operator", "finish scope fence test")
            .await
    })
    .await;
}

/// Verifies stale completion triples publish observability facts without overwriting generation truth.
#[tokio::test(start_paused = true)]
async fn stale_exit_report_never_overwrites_current_attempt_test() {
    const STALE_CHILD: &str = "stale_fence_worker";
    let child_id = ChildId::new(STALE_CHILD);
    let start_counter_inner = Arc::new(AtomicUsize::new(0));
    let start_counter_body = start_counter_inner.clone();

    let (boot_tx, mut boot_rx) = mpsc::channel(1);
    let (cancel_ack_tx, mut cancel_ack_rx) = mpsc::channel(1);
    let (second_boot_tx, mut second_boot_rx) = mpsc::channel(1);

    let boot_clone = boot_tx.clone();
    let cancel_clone = cancel_ack_tx.clone();
    let second_clone = second_boot_tx.clone();

    let spec = SupervisorSpec::root(vec![worker_child(
        STALE_CHILD,
        service_fn(move |ctx: TaskContext| {
            let boot_clone = boot_clone.clone();
            let cancel_clone = cancel_clone.clone();
            let second_clone = second_clone.clone();
            let start_counter_body = start_counter_body.clone();
            async move {
                let invocation = start_counter_body.fetch_add(1, Ordering::SeqCst);
                if invocation == 0 {
                    let _ignored = boot_clone.send(()).await;
                    ctx.cancellation_token().cancelled().await;
                    let _ignored = cancel_clone.send(()).await;
                    tokio::time::sleep(Duration::from_millis(350)).await;
                    TaskResult::Cancelled
                } else {
                    let _ignored = second_clone.send(()).await;
                    ctx.cancellation_token().cancelled().await;
                    TaskResult::Cancelled
                }
            }
        }),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    boot_rx.recv().await.expect("child should boot");

    handle
        .restart_child(child_id.clone(), "operator", "stale replay fence")
        .await
        .expect("restart fences old attempt");

    cancel_ack_rx
        .recv()
        .await
        .expect("supervised task observes cancel");

    advance_test_clock(Duration::from_millis(350)).await;
    second_boot_rx
        .recv()
        .await
        .expect("target generation starts");

    let stale_leaf = worker_child(
        STALE_CHILD,
        service_fn(|_ctx: TaskContext| async { TaskResult::Succeeded }),
    );
    let mut stale_runtime = ChildRuntime::new(stale_leaf, SupervisorPath::root().join(STALE_CHILD));
    stale_runtime.generation = Generation::initial();
    stale_runtime.child_start_count = ChildStartCount::first();
    stale_runtime.status = ChildRuntimeStatus::Exited;

    let stale_report = ChildRunReport {
        runtime: stale_runtime,
        exit: TaskExit::Succeeded,
        became_ready: false,
    };

    handle
        .generation_fencing_replay_child_exit_for_test(stale_report)
        .await
        .expect("replay stale completion");

    advance_test_clock(Duration::from_millis(80)).await;

    let current = expect_current_state(handle.current_state().await.expect("state"));
    let worker = current
        .child_runtime_records
        .iter()
        .find(|record| record.child_id == child_id)
        .expect("worker projection");
    assert_eq!(
        worker.generation.map(|generation| generation.value),
        Some(1_u64),
        "{worker:?}"
    );

    let recorder = handle.observability_recorder();
    assert!(recorder.events.iter().any(|event| matches!(
        &event.what,
        What::ChildAttemptStaleReport {
            child_id: id,
            ..
        } if *id == child_id
    )));

    let _ignored = with_auto_clock_drive(async {
        handle
            .shutdown_tree("operator", "finish stale replay test")
            .await
    })
    .await;
}
