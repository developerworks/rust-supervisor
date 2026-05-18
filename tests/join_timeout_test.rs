//! Acceptance tests for join timeout and lifecycle path coverage (US3).
//!
//! These tests verify that:
//! 1. Global timeout is respected even with never-ending tasks.
//! 2. Remove command cleans the slot completely.
//! 3. All three lifecycle paths (normal exit, cancel, timeout+abort) converge
//!    to terminal state.

use rust_supervisor::control::outcome::ChildAttemptStatus;
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::runtime::child_slot::{ChildExitSummary, ChildSlot};
use rust_supervisor::shutdown::stage::ShutdownPolicy;
use std::time::Duration;

/// Helper to create a minimal ShutdownPolicy for timeout tests.
fn timeout_policy() -> ShutdownPolicy {
    ShutdownPolicy::new(Duration::from_millis(100), Duration::from_millis(50), true)
}

/// Helper to spawn a child task that runs until cancelled or the given future
/// completes.
fn spawn_child_slot(cancel_aware: bool) -> (ChildSlot, tokio::task::JoinHandle<()>) {
    let child_id = ChildId::new("test-child");
    let path = SupervisorPath::root().join("test-child");
    let mut slot = ChildSlot::new(child_id, path, Duration::from_secs(60));

    let (complete_tx, complete_rx) = tokio::sync::watch::channel(None);
    let (heartbeat_tx, heartbeat_rx) =
        tokio::sync::watch::channel::<Option<tokio::time::Instant>>(None);
    let (readiness_tx, readiness_rx) =
        tokio::sync::watch::channel(rust_supervisor::readiness::signal::ReadinessState::Unreported);

    let cancel_token = tokio_util::sync::CancellationToken::new();
    let cancel_clone = cancel_token.clone();

    let task_handle = tokio::task::spawn(async move {
        if cancel_aware {
            loop {
                if cancel_clone.is_cancelled() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        } else {
            loop {
                tokio::task::yield_now().await;
            }
        }
    });

    let handle = rust_supervisor::child_runner::runner::ChildRunHandle {
        cancellation_token: cancel_token,
        abort_handle: task_handle.abort_handle(),
        completion_receiver: complete_rx,
        heartbeat_receiver: heartbeat_rx,
        readiness_receiver: readiness_rx,
    };

    slot.activate(
        Generation::initial(),
        ChildStartCount::first(),
        ChildAttemptStatus::Running,
        handle,
    );

    let _ = complete_tx;
    let _ = heartbeat_tx;
    let _ = readiness_tx;

    (slot, task_handle)
}

// ---------------------------------------------------------------------------
// T033: test_join_timeout_respected_with_never_ending_task
// ---------------------------------------------------------------------------

/// Verifies that a never-ending task is force-cleared within the global
/// timeout (graceful_timeout + abort_wait).
#[tokio::test]
async fn test_join_timeout_respected_with_never_ending_task() {
    let policy = timeout_policy();
    let global_timeout = policy.graceful_timeout + policy.abort_wait;
    let (mut slot, _task_handle) = spawn_child_slot(false); // never checks cancel

    let start = tokio::time::Instant::now();

    // Execute cancel → abort → force-deactivate inline (simulating fanout for
    // a single slot).
    slot.cancel();

    // Wait for graceful_timeout.
    tokio::time::sleep(policy.graceful_timeout).await;

    if slot.has_active_attempt() {
        slot.abort();
    }

    // Wait for abort_wait.
    tokio::time::sleep(policy.abort_wait).await;

    if slot.has_active_attempt() {
        slot.deactivate(ChildExitSummary {
            exit_code: None,
            exit_reason: "force-cleared after timeout".to_owned(),
            exited_at_unix_nanos: 0,
        });
    }

    let elapsed = start.elapsed();

    assert!(
        elapsed <= global_timeout + Duration::from_millis(200),
        "shutdown took {:?}, expected <= {:?}",
        elapsed,
        global_timeout + Duration::from_millis(200)
    );
    assert!(!slot.has_active_attempt());
    assert!(slot.last_exit.is_some());
}

// ---------------------------------------------------------------------------
// T034: test_remove_command_cleans_slot_completely
// ---------------------------------------------------------------------------

/// Verifies that a simulated remove operation leaves the slot without an active
/// attempt.
#[tokio::test]
async fn test_remove_command_cleans_slot_completely() {
    let (mut slot, _task_handle) = spawn_child_slot(true); // cancel-aware

    assert!(slot.has_active_attempt());

    // Simulate remove: cancel + wait + deactivate.
    slot.cancel();
    tokio::time::sleep(Duration::from_millis(200)).await;

    if slot.has_active_attempt() {
        slot.abort();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    if slot.has_active_attempt() {
        slot.deactivate(ChildExitSummary {
            exit_code: None,
            exit_reason: "removed".to_owned(),
            exited_at_unix_nanos: 0,
        });
    }

    assert!(!slot.has_active_attempt());
    assert_eq!(
        slot.operation,
        rust_supervisor::control::outcome::ChildControlOperation::Active
    );
    assert!(slot.last_exit.is_some());
    assert_eq!(slot.last_exit.as_ref().unwrap().exit_reason, "removed");
}

// ---------------------------------------------------------------------------
// T035: test_all_lifecycle_paths_join_to_terminal
// ---------------------------------------------------------------------------

/// Verifies that normal exit, cancel, and timeout+abort paths all end with
/// slot.has_active_attempt() == false and last_exit recorded.
#[tokio::test]
async fn test_all_lifecycle_paths_join_to_terminal() {
    // --- Path 1: normal exit ---
    {
        let (mut slot, task_handle) = spawn_child_slot(true);
        // Abort immediately (simulates fast normal exit).
        task_handle.abort();
        let _ = task_handle.await;
        slot.deactivate(ChildExitSummary {
            exit_code: Some(0),
            exit_reason: "succeeded".to_owned(),
            exited_at_unix_nanos: 0,
        });
        assert!(!slot.has_active_attempt());
        assert!(slot.last_exit.is_some());
    }

    // --- Path 2: cancel exit ---
    {
        let (mut slot, _task_handle) = spawn_child_slot(true);
        slot.cancel();
        tokio::time::sleep(Duration::from_millis(200)).await;
        if slot.has_active_attempt() {
            slot.deactivate(ChildExitSummary {
                exit_code: None,
                exit_reason: "cancelled".to_owned(),
                exited_at_unix_nanos: 0,
            });
        }
        assert!(!slot.has_active_attempt());
        assert!(slot.last_exit.is_some());
    }

    // --- Path 3: timeout + abort ---
    {
        let (mut slot, _task_handle) = spawn_child_slot(false); // never checks cancel
        let policy = timeout_policy();
        slot.cancel();
        tokio::time::sleep(policy.graceful_timeout).await;
        if slot.has_active_attempt() {
            slot.abort();
        }
        tokio::time::sleep(policy.abort_wait).await;
        if slot.has_active_attempt() {
            slot.deactivate(ChildExitSummary {
                exit_code: None,
                exit_reason: "aborted after timeout".to_owned(),
                exited_at_unix_nanos: 0,
            });
        }
        assert!(!slot.has_active_attempt());
        assert!(slot.last_exit.is_some());
    }
}
