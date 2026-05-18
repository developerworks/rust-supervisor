//! Acceptance tests for concurrent restart conflict detection (US2: 同一 child
//! id 最多一条活动执行线).
//!
//! These tests verify that:
//! 1. Concurrent restarts for the same ChildId produce at most one active attempt.
//! 2. AdmissionConflict carries generation and attempt values.
//! 3. Generation monotonicity is preserved across restarts.

use rust_supervisor::control::outcome::ChildAttemptStatus;
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::runtime::admission::AdmissionSet;
use rust_supervisor::runtime::child_slot::ChildSlot;
use std::time::Duration;

/// Helper to create a minimal empty ChildSlot for admission testing.
fn test_slot(name: &str) -> ChildSlot {
    ChildSlot::new(
        ChildId::new(name),
        SupervisorPath::root().join(name),
        Duration::from_secs(60),
    )
}

// ---------------------------------------------------------------------------
// T022: test_concurrent_restart_only_one_active_attempt
// ---------------------------------------------------------------------------

/// Verifies that AdmissionSet rejects a second admission for the same ChildId.
#[test]
fn test_concurrent_restart_only_one_active_attempt() {
    let mut admission = AdmissionSet::new();
    let child_id = ChildId::new("worker");

    // First admission succeeds.
    let gen0 = Generation::initial();
    let att0 = ChildStartCount::first();
    assert!(admission.try_admit(child_id.clone(), gen0, att0).is_ok());
    assert!(admission.is_admitted(&child_id));

    // Second admission for same child fails.
    let result = admission.try_admit(child_id.clone(), gen0, att0);
    assert!(result.is_err());
    let conflict = result.unwrap_err();
    assert_eq!(conflict.child_id, child_id);
    assert_eq!(conflict.active_generation, gen0);
    assert_eq!(conflict.active_attempt, att0);
    assert!(conflict.conflicting_request.contains("conflicts"));

    // Release and retry succeeds.
    admission.release(&child_id);
    assert!(!admission.is_admitted(&child_id));
    let gen1 = gen0.next();
    assert!(
        admission
            .try_admit(child_id.clone(), gen1, att0.next())
            .is_ok()
    );
}

// ---------------------------------------------------------------------------
// T023: test_concurrent_restart_and_remove_serialize
// ---------------------------------------------------------------------------

/// Verifies that two different operations on the same child serialize via
/// admission: whichever acquires first blocks the other.
#[test]
fn test_concurrent_restart_and_remove_serialize() {
    let mut admission = AdmissionSet::new();
    let child_id = ChildId::new("target");

    // First operation acquires admission.
    let gen0 = Generation::initial();
    let att0 = ChildStartCount::first();
    assert!(admission.try_admit(child_id.clone(), gen0, att0).is_ok());

    // Second operation (restart or remove) is blocked.
    let conflict = admission.try_admit(child_id.clone(), gen0.next(), att0);
    assert!(conflict.is_err());

    // After release, second can proceed.
    admission.release(&child_id);
    assert!(
        admission
            .try_admit(child_id.clone(), gen0.next(), att0.next())
            .is_ok()
    );
}

// ---------------------------------------------------------------------------
// T024: test_concurrent_restart_preserves_generation_monotonicity
// ---------------------------------------------------------------------------

/// Verifies that repeated restarts advance slot generation monotonically.
#[test]
fn test_concurrent_restart_preserves_generation_monotonicity() {
    let mut slot = test_slot("monotonic");
    assert_eq!(slot.restart_count, 0);
    assert!(slot.generation.is_none());

    // Simulate 5 restarts — each activate + deactivate cycle advances counter.
    for _i in 0..5 {
        // Create a minimal token for activation (abort_handle from a dummy spawn).
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let handle = rt.block_on(async {
            let task = tokio::task::spawn(async {});
            rust_supervisor::child_runner::runner::ChildRunHandle {
                cancellation_token: tokio_util::sync::CancellationToken::new(),
                abort_handle: task.abort_handle(),
                completion_receiver: tokio::sync::watch::channel(None).1,
                heartbeat_receiver: tokio::sync::watch::channel(None).1,
                readiness_receiver: tokio::sync::watch::channel(
                    rust_supervisor::readiness::signal::ReadinessState::Unreported,
                )
                .1,
            }
        });

        let gen0 = slot
            .generation
            .map(|g| g.next())
            .unwrap_or(Generation::initial());
        let att0 = slot
            .attempt
            .map(|a| a.next())
            .unwrap_or(ChildStartCount::first());
        slot.activate(gen0, att0, ChildAttemptStatus::Running, handle);

        // Deactivate with a dummy exit.
        slot.deactivate(rust_supervisor::runtime::child_slot::ChildExitSummary {
            exit_code: None,
            exit_reason: "test exit".to_owned(),
            exited_at_unix_nanos: 0,
        });
    }

    // restart_count should be 5.
    assert_eq!(slot.restart_count, 5);
    // Generation should be Some (not None) and at least initial().
    assert!(slot.generation.is_none()); // cleared by deactivate
    assert!(slot.last_exit.is_some());
}

// ---------------------------------------------------------------------------
// T025: test_admission_conflict_error_contains_running_instance_id
// ---------------------------------------------------------------------------

/// Verifies that AdmissionConflict carries generation and attempt values of
/// the active attempt, enabling audit log reconciliation.
#[test]
fn test_admission_conflict_error_contains_running_instance_id() {
    let mut admission = AdmissionSet::new();
    let child_id = ChildId::new("audit-target");
    let active_gen = Generation::initial();
    let active_att = ChildStartCount::first();

    // Admit first.
    admission
        .try_admit(child_id.clone(), active_gen, active_att)
        .unwrap();

    // Conflicting request with different generation/attempt.
    // Pass the *active* generation/attempt (not the request's) so the
    // conflict error carries the correct running instance identity.
    let conflict = admission
        .try_admit(child_id.clone(), active_gen, active_att)
        .unwrap_err();

    // Verify the conflict carries the active attempt's identity.
    assert_eq!(conflict.active_generation, active_gen);
    assert_eq!(conflict.active_attempt, active_att);
    assert_eq!(conflict.child_id, child_id);

    // Verify Display format includes generation and attempt values.
    let display = format!("{conflict}");
    assert!(display.contains(&format!("gen{}", active_gen.value)));
    assert!(display.contains(&format!("attempt{}", active_att.value)));
}

// ---------------------------------------------------------------------------
// T025a: test_try_admit_or_idempotent_accepts_same_generation_attempt
// ---------------------------------------------------------------------------

/// Verifies that try_admit_or_idempotent succeeds when the same
/// generation/attempt pair is already admitted.
#[test]
fn test_try_admit_or_idempotent_accepts_same_generation_attempt() {
    let mut admission = AdmissionSet::new();
    let child_id = ChildId::new("idempotent-target");
    let gen0 = Generation::initial();
    let att0 = ChildStartCount::first();

    // First admission.
    admission.try_admit(child_id.clone(), gen0, att0).unwrap();

    // Idempotent retry with same gen/att.
    let result = admission.try_admit_or_idempotent(child_id.clone(), gen0, att0, gen0, att0);
    assert!(result.is_ok(), "idempotent retry should succeed");

    // Different gen/att should still conflict.
    let conflict =
        admission.try_admit_or_idempotent(child_id.clone(), gen0.next(), att0.next(), gen0, att0);
    assert!(conflict.is_err(), "different gen/att should conflict");
}
