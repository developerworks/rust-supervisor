//! Supervisor policy integration tests.
//!
//! These tests cover restart policy decisions and meltdown fuses.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::backoff::BackoffPolicy;
use rust_supervisor::policy::decision::{
    PolicyEngine, PolicyFailureKind, RestartDecision, RestartPolicy, TaskExit,
};
use rust_supervisor::policy::meltdown::{MeltdownOutcome, MeltdownPolicy, MeltdownTracker};
use std::time::{Duration, Instant};

/// Verifies that transient recoverable failures restart after backoff.
#[test]
fn transient_failure_restarts_after_backoff() {
    let engine = PolicyEngine::new();
    let backoff = BackoffPolicy::new(
        Duration::from_millis(10),
        Duration::from_millis(100),
        0,
        Duration::from_secs(1),
    );
    let decision = engine.decide(
        RestartPolicy::Transient,
        TaskExit::Failed {
            kind: PolicyFailureKind::Recoverable,
        },
        1,
        &backoff,
    );

    assert!(matches!(decision, RestartDecision::RestartAfter { .. }));
}

/// Verifies that child meltdown limits trip a child fuse.
#[test]
fn child_meltdown_trips_child_fuse() {
    let policy = MeltdownPolicy::new(
        2, // Allow up to 2 restarts before fusing
        Duration::from_secs(10),
        5,
        Duration::from_secs(30),
        10,
        Duration::from_secs(60),
        Duration::from_secs(120),
    );
    let mut tracker = MeltdownTracker::new(policy);
    let now = Instant::now();
    let child_id = ChildId::new("test-child".to_string());

    // First restart: count=1, below threshold of 2 → Continue
    assert_eq!(
        tracker.record_child_restart_with_group(
            child_id.clone(),
            Some("test-group".to_string()),
            now
        ),
        MeltdownOutcome::Continue
    );
    // Second restart: count=2, reaches threshold of 2 → ChildFuse
    assert_eq!(
        tracker.record_child_restart_with_group(
            child_id,
            Some("test-group".to_string()),
            now + Duration::from_secs(1)
        ),
        MeltdownOutcome::ChildFuse
    );
}
