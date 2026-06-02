//! Shutdown coordinator tests.
//!
//! These tests verify idempotent four-stage shutdown transitions.

use rust_supervisor::shutdown::coordinator::ShutdownCoordinator;
use rust_supervisor::shutdown::stage::{ShutdownCause, ShutdownPhase};
use rust_supervisor::spec::shutdown::{ShutdownBudget, TreeShutdownPolicy};
use std::time::Duration;

/// Verifies that repeated shutdown requests are idempotent.
#[test]
fn shutdown_request_is_idempotent() {
    let policy = TreeShutdownPolicy::new(
        ShutdownBudget::new(Duration::from_secs(5), Duration::from_secs(1)),
        true,
        Duration::from_secs(5),
        3,
    );
    let mut coordinator = ShutdownCoordinator::new(policy);

    let first = coordinator.request_stop(ShutdownCause::new("operator", "deploy"));
    let second = coordinator.request_stop(ShutdownCause::new("operator", "repeat"));

    assert_eq!(first.phase, ShutdownPhase::RequestStop);
    assert!(second.idempotent);
    assert_eq!(second.cause.reason, "deploy");
}

/// Verifies that shutdown phases advance to completion.
#[test]
fn shutdown_phase_advances_to_completed() {
    let policy = TreeShutdownPolicy::new(
        ShutdownBudget::new(Duration::from_secs(5), Duration::from_secs(1)),
        true,
        Duration::from_secs(5),
        3,
    );
    let mut coordinator = ShutdownCoordinator::new(policy);

    coordinator.request_stop(ShutdownCause::new("operator", "deploy"));
    coordinator.advance();
    coordinator.advance();
    coordinator.advance();
    coordinator.advance();

    assert_eq!(coordinator.phase(), ShutdownPhase::Completed);
}

// ---------------------------------------------------------------------------
// TreeShutdownPolicy unit tests (G1, G6)
// ---------------------------------------------------------------------------

/// Verifies that `effective_global_deadline()` correctly sums all three
/// timeout components.
#[test]
fn effective_global_deadline_sums_all_components() {
    let policy = TreeShutdownPolicy::new(
        ShutdownBudget::new(Duration::from_secs(5), Duration::from_secs(1)),
        true,
        Duration::from_secs(5),
        3,
    );
    assert_eq!(policy.effective_global_deadline(), Duration::from_secs(11),);
}

/// Verifies that `TreeShutdownPolicy::default()` uses the recommended constants.
#[test]
fn tree_shutdown_policy_default_uses_recommended_constants() {
    let policy = TreeShutdownPolicy::default();
    assert_eq!(policy.budget.graceful_timeout, Duration::from_secs(5));
    assert_eq!(policy.budget.abort_wait, Duration::from_secs(1));
    assert!(policy.abort_after_timeout);
    assert_eq!(policy.force_kill_margin, Duration::from_secs(5));
    // Default is 0 (automatic) — the effective threshold is computed from
    // the number of tokio worker threads at runtime.
    assert_eq!(policy.max_orphan_threshold, 0);
    assert!(policy.effective_max_orphan_threshold() >= 1);
}

/// Verifies that `effective_global_deadline()` works when margin is zero.
#[test]
fn effective_global_deadline_with_zero_margin() {
    let policy = TreeShutdownPolicy::new(
        ShutdownBudget::new(Duration::from_secs(5), Duration::from_secs(1)),
        true,
        Duration::from_secs(0),
        3,
    );
    assert_eq!(policy.effective_global_deadline(), Duration::from_secs(6),);
}

/// Verifies that `effective_global_deadline()` works with sub-second durations.
#[test]
fn effective_global_deadline_with_subsecond_durations() {
    let policy = TreeShutdownPolicy::new(
        ShutdownBudget::new(Duration::from_millis(100), Duration::from_millis(50)),
        true,
        Duration::from_millis(50),
        3,
    );
    assert_eq!(
        policy.effective_global_deadline(),
        Duration::from_millis(200),
    );
}

/// Verifies that `TreeShutdownPolicy` serializes and deserializes all fields
/// correctly (G7).
#[test]
fn tree_shutdown_policy_roundtrip_serialization() {
    let policy = TreeShutdownPolicy::new(
        ShutdownBudget::new(Duration::from_secs(3), Duration::from_secs(2)),
        false,
        Duration::from_secs(4),
        5,
    );
    let json = serde_json::to_string(&policy).expect("serialize");
    let deserialized: TreeShutdownPolicy = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(policy, deserialized);
    assert_eq!(deserialized.force_kill_margin, Duration::from_secs(4));
    assert_eq!(deserialized.max_orphan_threshold, 5);
}
