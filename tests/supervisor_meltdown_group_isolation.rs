//! Acceptance tests for meltdown group isolation.
//!
//! This test verifies that:
//! 1. When only one group has continuous failures, other groups are not affected
//! 2. Each group maintains independent failure counting
//! 3. Meltdown in one group does not trigger protection in other groups

use rust_supervisor::policy::meltdown::{MeltdownOutcome, MeltdownPolicy, MeltdownTracker};
use std::time::Duration;

#[test]
fn test_group_isolation_single_group_failure() {
    // Create separate trackers for different groups
    let policy = MeltdownPolicy::new(
        3,                        // child_max_restarts
        Duration::from_secs(10),  // child_window
        2,                        // group_max_failures (low threshold for testing)
        Duration::from_secs(30),  // group_window
        10,                       // supervisor_max_failures
        Duration::from_secs(60),  // supervisor_window
        Duration::from_secs(120), // reset_after
    );

    // Group A tracker - will experience failures
    let mut group_a_tracker = MeltdownTracker::new(policy);

    // Group B tracker - should remain unaffected
    let mut group_b_tracker = MeltdownTracker::new(policy);

    let now = std::time::Instant::now();

    // Inject failures into Group A
    for i in 0..3 {
        let outcome = group_a_tracker.record_child_restart(now + Duration::from_secs(i));
        println!("Group A failure {}: {:?}", i + 1, outcome);
    }

    // Group A should trip group fuse after exceeding group_max_failures (2)
    assert_eq!(
        group_a_tracker.current_outcome_for_test(),
        MeltdownOutcome::GroupFuse
    );

    // Group B should still be in Continue state (no failures injected)
    assert_eq!(
        group_b_tracker.current_outcome_for_test(),
        MeltdownOutcome::Continue
    );

    // Verify group B can still operate normally
    let group_b_outcome = group_b_tracker.record_child_restart(now + Duration::from_secs(5));
    assert_eq!(group_b_outcome, MeltdownOutcome::Continue);
}

#[test]
fn test_multiple_groups_independent_counting() {
    // Verify that multiple groups maintain independent failure counts
    let policy = MeltdownPolicy::new(
        3,
        Duration::from_secs(10),
        3,
        Duration::from_secs(30),
        10,
        Duration::from_secs(60),
        Duration::from_secs(120),
    );

    let mut group_alpha = MeltdownTracker::new(policy);
    let mut group_beta = MeltdownTracker::new(policy);
    let mut group_gamma = MeltdownTracker::new(policy);

    let now = std::time::Instant::now();

    // Alpha gets 2 failures
    group_alpha.record_child_restart(now);
    group_alpha.record_child_restart(now + Duration::from_secs(1));

    // Beta gets 1 failure
    group_beta.record_child_restart(now + Duration::from_secs(2));

    // Gamma gets no failures

    // Verify independent counts
    assert_eq!(group_alpha.group_failure_count(), 2);
    assert_eq!(group_beta.group_failure_count(), 1);
    assert_eq!(group_gamma.group_failure_count(), 0);

    // All should still be in Continue state (below threshold of 3)
    assert_eq!(
        group_alpha.current_outcome_for_test(),
        MeltdownOutcome::Continue
    );
    assert_eq!(
        group_beta.current_outcome_for_test(),
        MeltdownOutcome::Continue
    );
    assert_eq!(
        group_gamma.current_outcome_for_test(),
        MeltdownOutcome::Continue
    );
}

#[test]
fn test_group_fuse_does_not_affect_other_groups() {
    // Verify that when one group trips its fuse, other groups can still restart
    let policy = MeltdownPolicy::new(
        3,
        Duration::from_secs(10),
        2, // Low threshold to trigger quickly
        Duration::from_secs(30),
        10,
        Duration::from_secs(60),
        Duration::from_secs(120),
    );

    let mut failing_group = MeltdownTracker::new(policy);
    let mut healthy_group = MeltdownTracker::new(policy);

    let now = std::time::Instant::now();

    // Failing group exceeds threshold
    failing_group.record_child_restart(now);
    failing_group.record_child_restart(now + Duration::from_secs(1));
    failing_group.record_child_restart(now + Duration::from_secs(2));

    // Failing group should be at GroupFuse
    assert_eq!(
        failing_group.current_outcome_for_test(),
        MeltdownOutcome::GroupFuse
    );

    // Healthy group should still be able to operate
    let healthy_outcome = healthy_group.record_child_restart(now + Duration::from_secs(3));
    assert_eq!(healthy_outcome, MeltdownOutcome::Continue);
    assert_eq!(healthy_group.group_failure_count(), 1);
}
