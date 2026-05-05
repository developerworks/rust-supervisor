//! Meltdown policy tests.
//!
//! These tests verify child and supervisor failure fuse behavior.

use rust_supervisor::policy::meltdown::{MeltdownOutcome, MeltdownPolicy, MeltdownTracker};
use std::time::{Duration, Instant};

/// Verifies that child-level fuse fires after the restart limit.
#[test]
fn child_fuse_fires_after_restart_limit() {
    let policy = MeltdownPolicy::new(
        1,
        Duration::from_secs(10),
        10,
        Duration::from_secs(10),
        Duration::from_secs(60),
    );
    let mut tracker = MeltdownTracker::new(policy);
    let now = Instant::now();

    assert_eq!(tracker.record_child_restart(now), MeltdownOutcome::Continue);
    assert_eq!(
        tracker.record_child_restart(now + Duration::from_secs(1)),
        MeltdownOutcome::ChildFuse
    );
}

/// Verifies that a stable window clears meltdown counters.
#[test]
fn stable_window_clears_counters() {
    let policy = MeltdownPolicy::new(
        1,
        Duration::from_secs(10),
        10,
        Duration::from_secs(10),
        Duration::from_secs(60),
    );
    let mut tracker = MeltdownTracker::new(policy);
    let now = Instant::now();

    tracker.record_child_restart(now);

    assert!(tracker.reset_if_stable(now + Duration::from_secs(60)));
    assert_eq!(tracker.child_failure_count(), 0);
}
