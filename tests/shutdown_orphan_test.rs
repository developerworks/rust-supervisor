//! Acceptance tests for orphan-free shutdown (US3: join 在所有生命周期路径上都可达).
//!
//! These tests verify that:
//! 1. After shutdown_tree completes, no ChildSlot holds residual handles.
//! 2. reconcile_shutdown_slots correctly reports orphaned slots.

use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::runtime::admission::AdmissionSet;
use rust_supervisor::runtime::child_slot::ChildSlot;
use rust_supervisor::runtime::shutdown::{reconcile_shutdown_slots, shutdown_tree_fanout};
use rust_supervisor::shutdown::stage::ShutdownPolicy;
use std::collections::HashMap;
use std::time::Duration;

/// Helper to create a ShutdownPolicy with short timeouts for fast tests.
fn test_shutdown_policy() -> ShutdownPolicy {
    ShutdownPolicy::new(Duration::from_millis(200), Duration::from_millis(100), true)
}

/// Helper to create a minimal ChildSlot that holds an active attempt
/// (via a spawned Tokio task).
fn active_slot(name: &str, cancel_aware: bool) -> (ChildSlot, tokio::task::JoinHandle<()>) {
    let child_id = ChildId::new(name);
    let path = SupervisorPath::root().join(name);
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
            // Cooperatively wait for cancellation.
            loop {
                if cancel_clone.is_cancelled() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        } else {
            // Never check cancellation — must be aborted.
            loop {
                tokio::task::yield_now().await;
            }
        }
    });

    let abort_handle = task_handle.abort_handle();

    let handle = rust_supervisor::child_runner::runner::ChildRunHandle {
        cancellation_token: cancel_token,
        abort_handle,
        completion_receiver: complete_rx,
        heartbeat_receiver: heartbeat_rx,
        readiness_receiver: readiness_rx,
    };

    let gen0 = Generation::initial();
    let att0 = ChildStartCount::first();
    slot.activate(
        gen0,
        att0,
        rust_supervisor::control::outcome::ChildAttemptStatus::Running,
        handle,
    );

    // Suppress unused warnings for channels.
    let _ = complete_tx;
    let _ = heartbeat_tx;
    let _ = readiness_tx;

    (slot, task_handle)
}

// ---------------------------------------------------------------------------
// T031: test_shutdown_completion_no_orphan_join_handles
// ---------------------------------------------------------------------------

/// Verifies that after shutdown_tree all slots are clean.
#[tokio::test]
async fn test_shutdown_completion_no_orphan_join_handles() {
    let mut slots: HashMap<ChildId, ChildSlot> = HashMap::new();
    let mut admission = AdmissionSet::new();
    let policy = test_shutdown_policy();

    // Create 5 slots, 1 slow that ignores cancellation.
    let mut task_handles = Vec::new();
    for i in 0..5 {
        let cancel_aware = i != 2; // slot #2 ignores cancellation
        let (slot, task_handle) = active_slot(&format!("child-{i}"), cancel_aware);
        slots.insert(slot.child_id.clone(), slot);
        task_handles.push(task_handle);
    }

    // Admit all active children.
    for (child_id, slot) in &slots {
        if slot.has_active_attempt() {
            admission
                .try_admit(
                    child_id.clone(),
                    slot.generation.unwrap(),
                    slot.attempt.unwrap(),
                )
                .ok();
        }
    }

    let _outcomes = shutdown_tree_fanout(&mut slots, &policy, &mut admission).await;

    // After shutdown, all slots must be clean.
    let reconcile = reconcile_shutdown_slots(&slots);
    assert!(
        reconcile.verified_clean,
        "reconcile should report clean: orphan_slots={:?}",
        reconcile.orphan_slots
    );
    assert_eq!(reconcile.total_slots_checked, 5);

    // Each slot individually: no active attempt, no handles.
    for slot in slots.values() {
        assert!(
            !slot.has_active_attempt(),
            "slot {} still has active attempt after shutdown",
            slot.child_id
        );
        assert!(
            slot.cancellation_token.is_none(),
            "slot {} still holds cancellation_token after shutdown",
            slot.child_id
        );
        assert!(
            slot.completion_receiver.is_none(),
            "slot {} still holds completion_receiver after shutdown",
            slot.child_id
        );
    }
}

// ---------------------------------------------------------------------------
// T032: test_shutdown_reconcile_report_lists_residual_slots
// ---------------------------------------------------------------------------

/// Verifies that reconcile_shutdown_slots detects residual handles.
#[tokio::test]
async fn test_shutdown_reconcile_report_lists_residual_slots() {
    let mut slots: HashMap<ChildId, ChildSlot> = HashMap::new();

    // Create a slot with handles still attached (simulate incomplete shutdown).
    let child_id = ChildId::new("orphan-1");
    let path = SupervisorPath::root().join("orphan-1");
    let mut slot = ChildSlot::new(child_id.clone(), path, Duration::from_secs(60));

    // Manually set handles to simulate an orphaned slot.
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let (_complete_tx, complete_rx) = tokio::sync::watch::channel::<
        Option<
            Result<
                rust_supervisor::child_runner::runner::ChildRunReport,
                rust_supervisor::error::types::SupervisorError,
            >,
        >,
    >(None);
    let (_hb_tx, hb_rx) = tokio::sync::watch::channel::<Option<tokio::time::Instant>>(None);
    let (rd_tx, rd_rx) =
        tokio::sync::watch::channel(rust_supervisor::readiness::signal::ReadinessState::Unreported);
    let _ = rd_tx;

    let handle = rust_supervisor::child_runner::runner::ChildRunHandle {
        cancellation_token: cancel_token,
        abort_handle: tokio::task::spawn(async {}).abort_handle(),
        completion_receiver: complete_rx,
        heartbeat_receiver: hb_rx,
        readiness_receiver: rd_rx,
    };

    let gen0 = Generation::initial();
    let att0 = ChildStartCount::first();
    slot.activate(
        gen0,
        att0,
        rust_supervisor::control::outcome::ChildAttemptStatus::Running,
        handle,
    );

    slots.insert(child_id, slot);

    // Now reconcile — should find the orphan.
    let reconcile = reconcile_shutdown_slots(&slots);
    assert!(
        !reconcile.verified_clean,
        "reconcile should detect orphaned slot"
    );
    assert_eq!(reconcile.orphan_slots.len(), 1);
    assert_eq!(reconcile.orphan_slots[0].value, "orphan-1");
    assert_eq!(reconcile.total_slots_checked, 1);
}
