//! Meltdown policy tests.
//!
//! These tests verify child and supervisor failure fuse behavior.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::meltdown::{MeltdownOutcome, MeltdownPolicy, MeltdownTracker};
use std::time::{Duration, Instant};

/// Verifies that child-level fuse fires after the restart limit.
#[test]
fn child_fuse_fires_after_restart_limit() {
    let policy = MeltdownPolicy::new(
        2, // Allow up to 2 restarts before fusing
        Duration::from_secs(10),
        10,
        Duration::from_secs(10),
        20,
        Duration::from_secs(30),
        Duration::from_secs(60),
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

/// Verifies that a stable window clears meltdown counters.
#[test]
fn stable_window_clears_counters() {
    let policy = MeltdownPolicy::new(
        2, // Allow up to 2 restarts before fusing
        Duration::from_secs(10),
        10,
        Duration::from_secs(10),
        20,
        Duration::from_secs(30),
        Duration::from_secs(60),
    );
    let mut tracker = MeltdownTracker::new(policy);
    let now = Instant::now();
    let child_id = ChildId::new("test-child".to_string());

    // Record one restart (below threshold)
    tracker.record_child_restart_with_group(child_id.clone(), Some("test-group".to_string()), now);

    // After stable period, counters should be cleared
    assert!(tracker.reset_if_stable(now + Duration::from_secs(60)));
    assert_eq!(tracker.child_failure_count(&child_id), 0);
}
