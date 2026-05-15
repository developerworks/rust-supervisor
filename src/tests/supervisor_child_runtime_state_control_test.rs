//! Child runtime state control integration tests.
//!
//! These tests verify the runtime facts exposed through `CurrentState`.

use rust_supervisor::control::command::{CommandResult, CurrentState};
use rust_supervisor::control::handle::SupervisorHandle;
use rust_supervisor::control::outcome::{
    ChildAttemptStatus, ChildControlFailurePhase, ChildControlOperation, ChildRuntimeRecord,
    ChildStopState,
};
use rust_supervisor::error::types::{SupervisorError, TaskFailure, TaskFailureKind};
use rust_supervisor::event::payload::What;
use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::observe::metrics::SupervisorMetricName;
use rust_supervisor::readiness::signal::{ReadinessPolicy, ReadinessState};
use rust_supervisor::runtime::child_runtime_state::DEFAULT_HEARTBEAT_TIMEOUT_SECS;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::child::{
    BackoffPolicy, ChildSpec, RestartPolicy, ShutdownPolicy, TaskKind,
};
use rust_supervisor::spec::supervisor::{RestartLimit, SupervisorSpec};
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult, service_fn};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Notify, mpsc};

/// Verifies that current state exposes active child runtime records.
#[tokio::test]
async fn current_state_exposes_full_runtime_state_fields_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(2);
    let spec = SupervisorSpec::root(vec![
        ready_heartbeat_child("alpha", started_sender.clone()),
        ready_heartbeat_child("beta", started_sender),
    ]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 2).await;

    let state = current_state(&handle).await;

    assert_eq!(state.child_runtime_records.len(), 2);
    assert_eq!(
        state.child_runtime_records[0].child_id,
        ChildId::new("alpha")
    );
    assert_eq!(
        state.child_runtime_records[1].child_id,
        ChildId::new("beta")
    );
    for record in &state.child_runtime_records {
        assert_active_ready_record(record);
    }
    assert_current_state_fast_20_reads(&handle).await;
    shutdown(handle).await;
}

/// Verifies that readiness distinguishes unreported and not-ready values.
#[tokio::test]
async fn current_state_distinguishes_unreported_from_degraded_readiness_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let degrade = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![degradable_child(
        "worker",
        started_sender,
        degrade.clone(),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 1).await;

    let initial = current_state(&handle).await;
    assert_eq!(
        initial.child_runtime_records[0].liveness.readiness,
        ReadinessState::Unreported
    );

    degrade.notify_waiters();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let degraded = current_state(&handle).await;
    assert_eq!(
        degraded.child_runtime_records[0].liveness.readiness,
        ReadinessState::NotReady
    );
    assert_current_state_fast_20_reads(&handle).await;
    shutdown(handle).await;
}

/// Verifies that missing heartbeat and stale heartbeat are distinct.
#[tokio::test]
async fn current_state_distinguishes_no_heartbeat_from_stale_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let heartbeat = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![delayed_heartbeat_child(
        "worker",
        started_sender,
        heartbeat.clone(),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 1).await;

    let initial = current_state(&handle).await;
    let initial_record = &initial.child_runtime_records[0];
    assert_eq!(initial_record.child_id, ChildId::new("worker"));
    assert!(
        initial_record
            .liveness
            .last_heartbeat_at_unix_nanos
            .is_none()
    );
    assert!(!initial_record.liveness.heartbeat_stale);

    heartbeat.notify_waiters();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let observed = current_state(&handle).await;
    assert!(
        observed.child_runtime_records[0]
            .liveness
            .last_heartbeat_at_unix_nanos
            .is_some()
    );

    tokio::time::sleep(Duration::from_secs(
        DEFAULT_HEARTBEAT_TIMEOUT_SECS.saturating_add(1),
    ))
    .await;
    let stale = current_state(&handle).await;
    assert!(
        stale.child_runtime_records[0]
            .liveness
            .last_heartbeat_at_unix_nanos
            .is_some()
    );
    assert!(stale.child_runtime_records[0].liveness.heartbeat_stale);

    let recorder = handle.observability_recorder();
    assert_eq!(heartbeat_stale_events(&recorder.events, "worker"), 1);
    assert_eq!(
        heartbeat_stale_metrics_without_child_id(&recorder.metrics),
        1
    );

    let _again = current_state(&handle).await;
    let recorder = handle.observability_recorder();
    assert_eq!(heartbeat_stale_events(&recorder.events, "worker"), 1);
    assert_eq!(
        heartbeat_stale_metrics_without_child_id(&recorder.metrics),
        1
    );

    shutdown(handle).await;
}

/// Verifies that pausing a child delivers real cancellation to the task.
#[tokio::test]
async fn pause_child_delivers_real_cancellation_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let (cancelled_sender, mut cancelled_receiver) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![controlled_cancellable_child(
        "worker",
        started_sender,
        cancelled_sender,
        release.clone(),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 1).await;

    let result = handle
        .pause_child(ChildId::new("worker"), "operator", "pause worker")
        .await
        .expect("pause child");

    let outcome = match result {
        CommandResult::ChildControl { outcome } => outcome,
        other => panic!("unexpected pause result: {other:?}"),
    };
    assert_eq!(outcome.child_id, ChildId::new("worker"));
    assert_eq!(outcome.operation_after, ChildControlOperation::Paused);
    assert_eq!(outcome.status, Some(ChildAttemptStatus::Cancelling));
    assert!(outcome.cancel_delivered);
    assert!(!outcome.idempotent);
    assert_eq!(
        cancelled_receiver
            .recv()
            .await
            .expect("cancellation should be observed"),
        "worker"
    );

    let state = current_state(&handle).await;
    let record = state
        .child_runtime_records
        .iter()
        .find(|record| record.child_id == ChildId::new("worker"))
        .expect("worker record should exist");
    assert_eq!(record.operation, ChildControlOperation::Paused);
    assert_eq!(record.status, Some(ChildAttemptStatus::Cancelling));

    let recorder = handle.observability_recorder();
    assert!(recorder.events.iter().any(|event| {
        matches!(
            &event.what,
            What::ChildControlCancelDelivered {
                child_id,
                generation,
                attempt,
                command,
                command_id,
            } if *child_id == ChildId::new("worker")
                && generation.value == 0
                && attempt.value == 1
                && command == "pause_child"
                && !command_id.is_empty()
        )
    }));
    assert!(recorder.events.iter().any(|event| {
        matches!(
            &event.what,
            What::ChildControlOperationChanged {
                child_id,
                from,
                to,
                command,
                command_id,
            } if *child_id == ChildId::new("worker")
                && *from == ChildControlOperation::Active
                && *to == ChildControlOperation::Paused
                && command == "pause_child"
                && !command_id.is_empty()
        )
    }));

    release.notify_waiters();
    shutdown(handle).await;
}

/// Verifies that removing a running child cancels and removes its runtime record.
#[tokio::test]
async fn remove_child_cancels_and_eventually_removes_runtime_state_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let (cancelled_sender, mut cancelled_receiver) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![controlled_cancellable_child(
        "worker",
        started_sender,
        cancelled_sender,
        release.clone(),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 1).await;

    let outcome = child_control_result(
        handle
            .remove_child(ChildId::new("worker"), "operator", "remove worker")
            .await
            .expect("remove child"),
    );

    assert_eq!(outcome.operation_after, ChildControlOperation::Removed);
    assert!(outcome.cancel_delivered);
    assert_eq!(outcome.status, Some(ChildAttemptStatus::Cancelling));
    assert_eq!(
        cancelled_receiver
            .recv()
            .await
            .expect("cancellation should be observed"),
        "worker"
    );

    release.notify_waiters();
    wait_for_record_absent(&handle, "worker").await;

    let recorder = handle.observability_recorder();
    assert!(recorder.events.iter().any(|event| {
        matches!(
            &event.what,
            What::ChildRuntimeStateRemoved {
                child_id,
                path,
                final_status,
            } if *child_id == ChildId::new("worker")
                && *path == SupervisorPath::root().join("worker")
                && *final_status == Some(ChildAttemptStatus::Stopped)
        )
    }));

    shutdown(handle).await;
}

/// Verifies that quarantine prevents automatic restart after failure.
#[tokio::test]
async fn quarantine_child_blocks_auto_restart_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(2);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![release_then_fail_child(
        "worker",
        started_sender,
        release.clone(),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 1).await;

    let outcome = child_control_result(
        handle
            .quarantine_child(ChildId::new("worker"), "operator", "quarantine worker")
            .await
            .expect("quarantine child"),
    );
    assert_eq!(outcome.operation_after, ChildControlOperation::Quarantined);

    release.notify_waiters();
    assert_no_extra_start(&mut started_receiver).await;
    let state = current_state(&handle).await;
    let record = find_record(&state, "worker");
    assert_eq!(record.operation, ChildControlOperation::Quarantined);
    assert!(record.attempt.is_none());

    shutdown(handle).await;
}

/// Verifies that pause prevents automatic restart after the active attempt exits.
#[tokio::test]
async fn pause_child_blocks_auto_restart_after_exit_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(2);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![release_then_fail_child(
        "worker",
        started_sender,
        release.clone(),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 1).await;

    let before = current_state(&handle).await;
    let before_record = find_record(&before, "worker").clone();
    let outcome = child_control_result(
        handle
            .pause_child(ChildId::new("worker"), "operator", "pause worker")
            .await
            .expect("pause child"),
    );
    assert_eq!(outcome.operation_after, ChildControlOperation::Paused);

    release.notify_waiters();
    assert_no_extra_start(&mut started_receiver).await;
    let after = current_state(&handle).await;
    let after_record = find_record(&after, "worker");
    assert_eq!(after_record.operation, ChildControlOperation::Paused);
    if let Some(after_generation) = after_record.generation {
        assert!(after_generation <= before_record.generation.expect("generation"));
    }
    if let Some(after_attempt) = after_record.attempt {
        assert!(after_attempt <= before_record.attempt.expect("attempt"));
    }

    shutdown(handle).await;
}

/// Verifies that a control command targets the currently active attempt.
#[tokio::test]
async fn control_command_targets_current_instance_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(3);
    let (cancelled_sender, mut cancelled_receiver) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![restart_then_wait_child(
        "worker",
        started_sender,
        cancelled_sender,
        release.clone(),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 2).await;
    wait_for_record_attempt(&handle, "worker", 2).await;

    let outcome = child_control_result(
        handle
            .pause_child(ChildId::new("worker"), "operator", "pause current")
            .await
            .expect("pause child"),
    );

    assert_eq!(outcome.attempt.expect("active attempt").value, 2);
    assert_eq!(
        cancelled_receiver
            .recv()
            .await
            .expect("current attempt cancellation"),
        2
    );

    release.notify_waiters();
    shutdown(handle).await;
}

/// Verifies idempotent repeated stop commands after cancellation delivery.
#[tokio::test]
async fn repeated_stop_commands_are_idempotent_after_cancel_delivery_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(3);
    let (cancelled_sender, mut cancelled_receiver) = mpsc::channel(3);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![
        controlled_cancellable_child(
            "pause-worker",
            started_sender.clone(),
            cancelled_sender.clone(),
            release.clone(),
        ),
        controlled_cancellable_child(
            "remove-worker",
            started_sender.clone(),
            cancelled_sender.clone(),
            release.clone(),
        ),
        controlled_cancellable_child(
            "quarantine-worker",
            started_sender,
            cancelled_sender,
            release.clone(),
        ),
    ]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 3).await;

    assert_repeated_stop_is_idempotent(
        &handle,
        "pause-worker",
        ChildControlOperation::Paused,
        StopControlCommand::Pause,
    )
    .await;
    assert_repeated_stop_is_idempotent(
        &handle,
        "remove-worker",
        ChildControlOperation::Removed,
        StopControlCommand::Remove,
    )
    .await;
    assert_repeated_stop_is_idempotent(
        &handle,
        "quarantine-worker",
        ChildControlOperation::Quarantined,
        StopControlCommand::Quarantine,
    )
    .await;

    for _index in 0..3 {
        let _child = cancelled_receiver
            .recv()
            .await
            .expect("initial cancellation should be delivered");
    }
    assert_no_extra_cancel(&mut cancelled_receiver).await;

    assert_no_active_idempotent_stop_commands().await;

    release.notify_waiters();
    shutdown(handle).await;
}

/// Verifies removing a registered child without an active attempt.
#[tokio::test]
async fn remove_without_active_instance_returns_no_active_instance_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let spec = SupervisorSpec::root(vec![temporary_success_child("worker", started_sender)]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 1).await;
    wait_for_record_without_attempt(&handle, "worker").await;

    let outcome = child_control_result(
        handle
            .remove_child(ChildId::new("worker"), "operator", "remove inactive")
            .await
            .expect("remove child"),
    );

    assert_eq!(outcome.stop_state, ChildStopState::NoActiveAttempt);
    assert!(outcome.attempt.is_none());
    assert!(outcome.generation.is_none());
    assert!(outcome.status.is_none());
    assert!(!outcome.cancel_delivered);
    assert_eq!(outcome.operation_after, ChildControlOperation::Removed);
    assert!(!outcome.idempotent);
    wait_for_record_absent(&handle, "worker").await;

    let recorder = handle.observability_recorder();
    assert!(recorder.events.iter().any(|event| {
        matches!(
            &event.what,
            What::ChildRuntimeStateRemoved {
                child_id,
                path,
                final_status,
            } if *child_id == ChildId::new("worker")
                && *path == SupervisorPath::root().join("worker")
                && final_status.is_none()
        )
    }));

    shutdown(handle).await;
}

/// Verifies stop deadline failures surface in outcomes and events.
#[tokio::test]
async fn stop_failure_outcome_carries_phase_and_reason_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let mut spec = SupervisorSpec::root(vec![ignores_cancellation_child(
        "worker",
        started_sender,
        release.clone(),
    )]);
    spec.default_shutdown_policy =
        ShutdownPolicy::new(Duration::from_millis(20), Duration::from_millis(20));
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 1).await;

    let initial = child_control_result(
        handle
            .remove_child(ChildId::new("worker"), "operator", "remove stuck worker")
            .await
            .expect("remove child"),
    );
    assert!(initial.cancel_delivered);

    tokio::time::sleep(Duration::from_millis(40)).await;
    let state = current_state(&handle).await;
    let record = find_record(&state, "worker");
    let failure = record.failure.as_ref().expect("stop failure should exist");
    assert_eq!(failure.phase, ChildControlFailurePhase::WaitCompletion);
    assert!(!failure.reason.is_empty());
    assert_eq!(record.stop_state, ChildStopState::Failed);

    let repeated = child_control_result(
        handle
            .remove_child(ChildId::new("worker"), "operator", "repeat remove")
            .await
            .expect("repeat remove"),
    );
    let repeated_failure = repeated.failure.expect("outcome failure should exist");
    assert_eq!(
        repeated_failure.phase,
        ChildControlFailurePhase::WaitCompletion
    );
    assert!(!repeated_failure.reason.is_empty());

    let recorder = handle.observability_recorder();
    assert!(recorder.events.iter().any(|event| {
        matches!(
            &event.what,
            What::ChildControlStopFailed {
                child_id,
                generation,
                attempt,
                status,
                stop_state,
                phase,
                reason,
                recoverable,
            } if *child_id == ChildId::new("worker")
                && generation.value == 0
                && attempt.value == 1
                && *status == ChildAttemptStatus::Cancelling
                && *stop_state == ChildStopState::Failed
                && *phase == ChildControlFailurePhase::WaitCompletion
                && !reason.is_empty()
                && *recoverable
        )
    }));

    release.notify_waiters();
    shutdown(handle).await;
}

/// Verifies restart limit exhaustion is visible in control outcomes.
#[tokio::test]
async fn restart_limit_exhaustion_visible_in_outcome_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(4);
    let mut spec = SupervisorSpec::root(vec![always_fail_child("worker", started_sender)]);
    spec.restart_limit = Some(RestartLimit::new(2, Duration::from_secs(60)));
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 3).await;
    tokio::time::sleep(Duration::from_millis(30)).await;

    let state = current_state(&handle).await;
    let record = find_record(&state, "worker");
    assert_eq!(record.restart_limit.limit, 2);
    assert!(record.restart_limit.used >= 2);
    assert_eq!(record.restart_limit.remaining, 0);
    assert!(record.restart_limit.exhausted);
    assert_eq!(record.restart_limit.window, Duration::from_secs(60));

    let updated_at = record.restart_limit.updated_at_unix_nanos;
    let outcome = child_control_result(
        handle
            .pause_child(ChildId::new("worker"), "operator", "inspect restart limit")
            .await
            .expect("pause child"),
    );
    assert_eq!(outcome.restart_limit.limit, record.restart_limit.limit);
    assert!(outcome.restart_limit.used >= record.restart_limit.used);
    assert_eq!(outcome.restart_limit.remaining, 0);
    assert!(outcome.restart_limit.exhausted);
    assert!(outcome.restart_limit.updated_at_unix_nanos >= updated_at);

    shutdown(handle).await;
}

/// Verifies an operator operation wins over an automatic restart race.
#[tokio::test]
async fn operation_wins_over_auto_restart_race_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(2);
    let release = Arc::new(Notify::new());
    let spec = SupervisorSpec::root(vec![release_then_fail_child(
        "worker",
        started_sender,
        release.clone(),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 1).await;

    let outcome = child_control_result(
        handle
            .pause_child(ChildId::new("worker"), "operator", "pause before failure")
            .await
            .expect("pause child"),
    );
    assert_eq!(outcome.operation_after, ChildControlOperation::Paused);

    release.notify_waiters();
    assert_no_extra_start(&mut started_receiver).await;
    let state = current_state(&handle).await;
    let record = find_record(&state, "worker");
    assert_eq!(record.operation, ChildControlOperation::Paused);
    assert!(record.attempt.is_none());

    shutdown(handle).await;
}

/// Stop command variants used by repeated idempotency assertions.
#[derive(Debug, Clone, Copy)]
enum StopControlCommand {
    /// Pause the target child.
    Pause,
    /// Remove the target child.
    Remove,
    /// Quarantine the target child.
    Quarantine,
}

/// Extracts a child control outcome from a command result.
fn child_control_result(
    result: CommandResult,
) -> rust_supervisor::control::outcome::ChildControlResult {
    match result {
        CommandResult::ChildControl { outcome } => outcome,
        other => panic!("unexpected child control result: {other:?}"),
    }
}

/// Finds one child runtime record in a current state result.
fn find_record<'a>(state: &'a CurrentState, name: &str) -> &'a ChildRuntimeRecord {
    state
        .child_runtime_records
        .iter()
        .find(|record| record.child_id == ChildId::new(name))
        .unwrap_or_else(|| panic!("record {name} should exist"))
}

/// Sends one stop command through the public handle.
async fn send_stop_command(
    handle: &SupervisorHandle,
    name: &str,
    command: StopControlCommand,
) -> Result<CommandResult, SupervisorError> {
    match command {
        StopControlCommand::Pause => {
            handle
                .pause_child(ChildId::new(name), "operator", "repeat pause")
                .await
        }
        StopControlCommand::Remove => {
            handle
                .remove_child(ChildId::new(name), "operator", "repeat remove")
                .await
        }
        StopControlCommand::Quarantine => {
            handle
                .quarantine_child(ChildId::new(name), "operator", "repeat quarantine")
                .await
        }
    }
}

/// Asserts that repeated active stop commands become idempotent.
async fn assert_repeated_stop_is_idempotent(
    handle: &SupervisorHandle,
    name: &str,
    operation: ChildControlOperation,
    command: StopControlCommand,
) {
    let first = child_control_result(
        send_stop_command(handle, name, command)
            .await
            .expect("initial stop command"),
    );
    assert_eq!(first.operation_after, operation);
    assert!(first.cancel_delivered);
    assert!(!first.idempotent);

    let before = handle.observability_recorder();
    let cancel_events_before = child_cancel_delivered_events(&before.events, name);
    let operation_events_before = child_operation_changed_events(&before.events, name);

    for _index in 0..10 {
        let repeated = child_control_result(
            send_stop_command(handle, name, command)
                .await
                .expect("repeated stop command"),
        );
        assert!(repeated.idempotent);
        assert!(!repeated.cancel_delivered);
        assert_eq!(repeated.operation_before, repeated.operation_after);
        assert_eq!(repeated.operation_after, operation);
    }

    let after = handle.observability_recorder();
    assert_eq!(
        child_cancel_delivered_events(&after.events, name),
        cancel_events_before
    );
    assert_eq!(
        child_operation_changed_events(&after.events, name),
        operation_events_before
    );
}

/// Asserts idempotency for paused and quarantined records without active attempts.
async fn assert_no_active_idempotent_stop_commands() {
    let (started_sender, mut started_receiver) = mpsc::channel(2);
    let spec = SupervisorSpec::root(vec![
        temporary_success_child("paused-idle", started_sender.clone()),
        temporary_success_child("quarantined-idle", started_sender),
    ]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    wait_for_started(&mut started_receiver, 2).await;
    wait_for_record_without_attempt(&handle, "paused-idle").await;
    wait_for_record_without_attempt(&handle, "quarantined-idle").await;

    let first_pause = child_control_result(
        handle
            .pause_child(ChildId::new("paused-idle"), "operator", "pause inactive")
            .await
            .expect("pause inactive"),
    );
    assert!(!first_pause.idempotent);
    assert_eq!(first_pause.operation_after, ChildControlOperation::Paused);

    let first_quarantine = child_control_result(
        handle
            .quarantine_child(
                ChildId::new("quarantined-idle"),
                "operator",
                "quarantine inactive",
            )
            .await
            .expect("quarantine inactive"),
    );
    assert!(!first_quarantine.idempotent);
    assert_eq!(
        first_quarantine.operation_after,
        ChildControlOperation::Quarantined
    );

    for _index in 0..10 {
        let repeated_pause = child_control_result(
            handle
                .pause_child(ChildId::new("paused-idle"), "operator", "repeat pause")
                .await
                .expect("repeat pause inactive"),
        );
        assert!(repeated_pause.idempotent);
        assert!(!repeated_pause.cancel_delivered);
        assert_eq!(
            repeated_pause.operation_before,
            repeated_pause.operation_after
        );

        let repeated_quarantine = child_control_result(
            handle
                .quarantine_child(
                    ChildId::new("quarantined-idle"),
                    "operator",
                    "repeat quarantine",
                )
                .await
                .expect("repeat quarantine inactive"),
        );
        assert!(repeated_quarantine.idempotent);
        assert!(!repeated_quarantine.cancel_delivered);
        assert_eq!(
            repeated_quarantine.operation_before,
            repeated_quarantine.operation_after
        );
    }

    shutdown(handle).await;
}

/// Counts cancellation delivery events for one child.
fn child_cancel_delivered_events(
    events: &[rust_supervisor::event::payload::SupervisorEvent],
    name: &str,
) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                &event.what,
                What::ChildControlCancelDelivered { child_id, .. } if child_id.value == name
            )
        })
        .count()
}

/// Counts operation change events for one child.
fn child_operation_changed_events(
    events: &[rust_supervisor::event::payload::SupervisorEvent],
    name: &str,
) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                &event.what,
                What::ChildControlOperationChanged { child_id, .. } if child_id.value == name
            )
        })
        .count()
}

/// Waits until a record disappears from current state.
async fn wait_for_record_absent(handle: &SupervisorHandle, name: &str) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let state = current_state(handle).await;
        if !state
            .child_runtime_records
            .iter()
            .any(|record| record.child_id == ChildId::new(name))
        {
            return;
        }
        assert!(Instant::now() < deadline, "record {name} should disappear");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Waits until a record has no active attempt.
async fn wait_for_record_without_attempt(handle: &SupervisorHandle, name: &str) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let state = current_state(handle).await;
        let record = find_record(&state, name);
        if record.attempt.is_none() && record.generation.is_none() && record.status.is_none() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "record {name} should have no active attempt"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Waits until a record reaches the expected active attempt.
async fn wait_for_record_attempt(handle: &SupervisorHandle, name: &str, expected: u64) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let state = current_state(handle).await;
        let record = find_record(&state, name);
        if record
            .attempt
            .is_some_and(|attempt| attempt.value == expected)
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "record {name} should reach attempt {expected}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Asserts that no extra child attempt starts in a short observation window.
async fn assert_no_extra_start(receiver: &mut mpsc::Receiver<String>) {
    let result = tokio::time::timeout(Duration::from_millis(80), receiver.recv()).await;
    assert!(result.is_err(), "child should not start again");
}

/// Asserts that repeated commands do not deliver extra cancellations.
async fn assert_no_extra_cancel(receiver: &mut mpsc::Receiver<String>) {
    let result = tokio::time::timeout(Duration::from_millis(50), receiver.recv()).await;
    assert!(
        result.is_err(),
        "extra cancellation should not be delivered"
    );
}

/// Counts stale heartbeat events for one child.
fn heartbeat_stale_events(
    events: &[rust_supervisor::event::payload::SupervisorEvent],
    name: &str,
) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                &event.what,
                What::ChildHeartbeatStale {
                    child_id,
                    attempt,
                    since_unix_nanos,
                } if child_id.value == name && attempt.value == 1 && *since_unix_nanos > 0
            )
        })
        .count()
}

/// Counts stale heartbeat metric samples without child labels.
fn heartbeat_stale_metrics_without_child_id(
    metrics: &[rust_supervisor::observe::metrics::MetricSample],
) -> usize {
    metrics
        .iter()
        .filter(|sample| {
            sample.name == SupervisorMetricName::ChildRuntimeHeartbeatStaleTotal.as_str()
                && !sample.labels.contains_key("child_id")
        })
        .count()
}

/// Asserts that repeated current state reads stay fast.
async fn assert_current_state_fast_20_reads(handle: &SupervisorHandle) {
    let mut slowest = Duration::ZERO;
    for _index in 0..20 {
        let started_at = Instant::now();
        let state = current_state(handle).await;
        let elapsed = started_at.elapsed();
        slowest = slowest.max(elapsed);
        let record_count = state.child_runtime_records.len();
        assert!(
            elapsed < Duration::from_millis(1),
            "current state read took {elapsed:?}, slowest {slowest:?}, records {record_count}"
        );
    }
}

/// Reads the current state through the public handle.
async fn current_state(handle: &SupervisorHandle) -> CurrentState {
    match handle.current_state().await.expect("current state") {
        CommandResult::CurrentState { state } => state,
        other => panic!("unexpected current state result: {other:?}"),
    }
}

/// Asserts a runtime record for an active ready child.
fn assert_active_ready_record(record: &ChildRuntimeRecord) {
    assert!(record.generation.is_some());
    assert!(record.attempt.is_some());
    assert!(record.status.is_some());
    assert_eq!(record.operation, ChildControlOperation::Active);
    assert!(record.liveness.last_heartbeat_at_unix_nanos.is_some());
    assert_eq!(record.liveness.readiness, ReadinessState::Ready);
    assert!(!record.liveness.heartbeat_stale);
    assert!(record.restart_limit.remaining > 0);
    assert!(record.failure.is_none());
}

/// Waits until the expected child tasks report startup.
async fn wait_for_started(receiver: &mut mpsc::Receiver<String>, expected: usize) {
    for _index in 0..expected {
        receiver.recv().await.expect("child should start");
    }
}

/// Shuts down the supervisor after a test.
async fn shutdown(handle: SupervisorHandle) {
    let _result = handle
        .shutdown_tree("test", "finish child runtime state test")
        .await
        .expect("shutdown supervisor");
}

/// Creates a child that reports heartbeat and readiness.
fn ready_heartbeat_child(name: &'static str, sender: mpsc::Sender<String>) -> ChildSpec {
    worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let sender = sender.clone();
            async move {
                ctx.heartbeat();
                ctx.mark_ready();
                let _ignored = sender.send(ctx.child_id.value.clone()).await;
                ctx.cancellation_token().cancelled().await;
                TaskResult::Cancelled
            }
        }),
    )
}

/// Creates a child that can switch from unreported readiness to not ready.
fn degradable_child(
    name: &'static str,
    sender: mpsc::Sender<String>,
    degrade: Arc<Notify>,
) -> ChildSpec {
    let mut child = worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let sender = sender.clone();
            let degrade = degrade.clone();
            async move {
                let _ignored = sender.send(ctx.child_id.value.clone()).await;
                degrade.notified().await;
                ctx.set_readiness(ReadinessState::NotReady);
                ctx.cancellation_token().cancelled().await;
                TaskResult::Cancelled
            }
        }),
    );
    child.readiness_policy = ReadinessPolicy::Explicit;
    child
}

/// Creates a child that waits after observing cancellation.
fn controlled_cancellable_child(
    name: &'static str,
    started: mpsc::Sender<String>,
    cancelled: mpsc::Sender<String>,
    release: Arc<Notify>,
) -> ChildSpec {
    worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let started = started.clone();
            let cancelled = cancelled.clone();
            let release = release.clone();
            async move {
                ctx.heartbeat();
                ctx.mark_ready();
                let _ignored = started.send(ctx.child_id.value.clone()).await;
                ctx.cancellation_token().cancelled().await;
                let _ignored = cancelled.send(ctx.child_id.value.clone()).await;
                release.notified().await;
                TaskResult::Cancelled
            }
        }),
    )
}

/// Creates a child that reports heartbeat only after a test signal.
fn delayed_heartbeat_child(
    name: &'static str,
    sender: mpsc::Sender<String>,
    heartbeat: Arc<Notify>,
) -> ChildSpec {
    worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let sender = sender.clone();
            let heartbeat = heartbeat.clone();
            async move {
                let _ignored = sender.send(ctx.child_id.value.clone()).await;
                heartbeat.notified().await;
                ctx.heartbeat();
                ctx.cancellation_token().cancelled().await;
                TaskResult::Cancelled
            }
        }),
    )
}

/// Creates a child that exits successfully after reporting startup.
fn temporary_success_child(name: &'static str, sender: mpsc::Sender<String>) -> ChildSpec {
    let mut child = worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let sender = sender.clone();
            async move {
                let _ignored = sender.send(ctx.child_id.value.clone()).await;
                TaskResult::Succeeded
            }
        }),
    );
    child.restart_policy = RestartPolicy::Temporary;
    child
}

/// Creates a child that waits for release and then fails.
fn release_then_fail_child(
    name: &'static str,
    sender: mpsc::Sender<String>,
    release: Arc<Notify>,
) -> ChildSpec {
    worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let sender = sender.clone();
            let release = release.clone();
            async move {
                ctx.heartbeat();
                ctx.mark_ready();
                let _ignored = sender.send(ctx.child_id.value.clone()).await;
                release.notified().await;
                failed_result("released failure")
            }
        }),
    )
}

/// Creates a child that fails first and waits on the restarted attempt.
fn restart_then_wait_child(
    name: &'static str,
    started: mpsc::Sender<String>,
    cancelled: mpsc::Sender<u64>,
    release: Arc<Notify>,
) -> ChildSpec {
    let starts = Arc::new(AtomicUsize::new(0));
    let mut child = worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let started = started.clone();
            let cancelled = cancelled.clone();
            let release = release.clone();
            let starts = starts.clone();
            async move {
                let count = starts.fetch_add(1, Ordering::SeqCst).saturating_add(1);
                let _ignored = started.send(ctx.child_id.value.clone()).await;
                if count == 1 {
                    return failed_result("first attempt failed");
                }
                ctx.cancellation_token().cancelled().await;
                let _ignored = cancelled.send(ctx.child_start_count.value).await;
                release.notified().await;
                TaskResult::Cancelled
            }
        }),
    );
    child.backoff_policy = BackoffPolicy::new(Duration::ZERO, Duration::ZERO, 0.0);
    child
}

/// Creates a child that ignores cancellation until released.
fn ignores_cancellation_child(
    name: &'static str,
    sender: mpsc::Sender<String>,
    release: Arc<Notify>,
) -> ChildSpec {
    worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let sender = sender.clone();
            let release = release.clone();
            async move {
                ctx.heartbeat();
                ctx.mark_ready();
                let _ignored = sender.send(ctx.child_id.value.clone()).await;
                release.notified().await;
                TaskResult::Cancelled
            }
        }),
    )
}

/// Creates a child that fails on every attempt.
fn always_fail_child(name: &'static str, sender: mpsc::Sender<String>) -> ChildSpec {
    worker_child(
        name,
        service_fn(move |ctx: TaskContext| {
            let sender = sender.clone();
            async move {
                let _ignored = sender.send(ctx.child_id.value.clone()).await;
                failed_result("always failed")
            }
        }),
    )
}

/// Creates a typed task failure result for restart tests.
fn failed_result(message: &'static str) -> TaskResult {
    TaskResult::Failed(TaskFailure::new(
        TaskFailureKind::Error,
        "test_failure",
        message,
    ))
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
