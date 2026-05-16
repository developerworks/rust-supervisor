//! Acceptance tests for meltdown group isolation (SC-002).
//!
//! SC-002: In group isolation tests, after one group triggers protection, other groups
//! should maintain at least 90% of cases that can still complete a controlled restart
//! attempt within the same time window, unless the supervisor-level threshold is independently exhausted.
//!
//! This test verifies that:
//! 1. A single MeltdownTracker instance maintains independent state per group
//! 2. When only one group has continuous failures, other groups are not affected
//! 3. Group-level meltdown does not consume supervisor-level quota unnecessarily

use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::meltdown::{MeltdownOutcome, MeltdownPolicy, MeltdownTracker};
use std::time::Duration;

/// Creates a test meltdown tracker with low thresholds for testing
fn create_test_tracker() -> MeltdownTracker {
    let policy = MeltdownPolicy::new(
        5,                        // child_max_restarts
        Duration::from_secs(10),  // child_window
        3,                        // group_max_failures (low threshold for testing)
        Duration::from_secs(30),  // group_window
        10,                       // supervisor_max_failures
        Duration::from_secs(60),  // supervisor_window
        Duration::from_secs(120), // reset_after
    );
    MeltdownTracker::new(policy)
}

#[test]
fn test_group_isolation_single_group_failure() {
    // SC-002: Use a SINGLE MeltdownTracker instance to verify in-process group isolation

    let mut tracker = create_test_tracker();
    let now = std::time::Instant::now();

    // Create children belonging to different groups
    let child_a1 = ChildId::new("group-a-child-1".to_string());
    let child_a2 = ChildId::new("group-a-child-2".to_string());
    let child_b1 = ChildId::new("group-b-child-1".to_string());

    // Inject failures into Group A only
    // Record 3 failures (exceeds group_max_failures=3)
    for i in 0..3 {
        let child = if i % 2 == 0 { &child_a1 } else { &child_a2 };
        let outcome = tracker.record_child_restart_with_group(
            child.clone(),
            Some("group-a".to_string()),
            now + Duration::from_secs(i),
        );
        println!("Group A failure {}: {:?}", i + 1, outcome);
    }

    // Group A should trip group fuse after exceeding threshold
    let group_a_outcome = tracker.get_group_outcome("group-a");
    assert_eq!(
        group_a_outcome,
        MeltdownOutcome::GroupFuse,
        "Group A should be in GroupFuse state after 3 failures"
    );

    // Group B should still be in Continue state (no failures injected)
    let group_b_outcome = tracker.get_group_outcome("group-b");
    assert_eq!(
        group_b_outcome,
        MeltdownOutcome::Continue,
        "Group B should remain in Continue state (isolated from Group A)"
    );

    // Verify Group B can still operate normally - record a restart
    let group_b_result = tracker.record_child_restart_with_group(
        child_b1.clone(),
        Some("group-b".to_string()),
        now + Duration::from_secs(5),
    );
    assert_eq!(
        group_b_result,
        MeltdownOutcome::Continue,
        "Group B restart should still be allowed"
    );
}

#[test]
fn test_multiple_groups_independent_counting() {
    // SC-002: Verify that multiple groups maintain independent failure counts in ONE tracker

    let mut tracker = create_test_tracker();
    let now = std::time::Instant::now();

    // Create children for three groups
    let child_alpha_1 = ChildId::new("alpha-1".to_string());
    let child_alpha_2 = ChildId::new("alpha-2".to_string());
    let child_beta_1 = ChildId::new("beta-1".to_string());
    let _child_gamma_1 = ChildId::new("gamma-1".to_string());

    // Alpha gets 2 failures
    tracker.record_child_restart_with_group(child_alpha_1.clone(), Some("alpha".to_string()), now);
    tracker.record_child_restart_with_group(
        child_alpha_2.clone(),
        Some("alpha".to_string()),
        now + Duration::from_secs(1),
    );

    // Beta gets 1 failure
    tracker.record_child_restart_with_group(
        child_beta_1.clone(),
        Some("beta".to_string()),
        now + Duration::from_secs(2),
    );

    // Gamma gets no failures

    // Verify independent counts via group outcomes
    assert_eq!(
        tracker.get_group_outcome("alpha"),
        MeltdownOutcome::Continue,
        "Alpha below threshold"
    );
    assert_eq!(
        tracker.get_group_outcome("beta"),
        MeltdownOutcome::Continue,
        "Beta below threshold"
    );
    assert_eq!(
        tracker.get_group_outcome("gamma"),
        MeltdownOutcome::Continue,
        "Gamma has no failures"
    );

    // All should still be in Continue state (below threshold of 3)
    // The key assertion: groups don't affect each other
}

#[test]
fn test_group_fuse_does_not_affect_other_groups() {
    // SC-002: Verify that when one group trips its fuse, other groups can still restart

    let mut tracker = create_test_tracker();
    let now = std::time::Instant::now();

    let failing_child_1 = ChildId::new("failing-group-1".to_string());
    let failing_child_2 = ChildId::new("failing-group-2".to_string());
    let healthy_child = ChildId::new("healthy-group-1".to_string());

    // Failing group exceeds threshold (3 failures >= group_max_failures=3)
    tracker.record_child_restart_with_group(
        failing_child_1.clone(),
        Some("failing".to_string()),
        now,
    );
    tracker.record_child_restart_with_group(
        failing_child_2.clone(),
        Some("failing".to_string()),
        now + Duration::from_secs(1),
    );
    tracker.record_child_restart_with_group(
        failing_child_1.clone(),
        Some("failing".to_string()),
        now + Duration::from_secs(2),
    );

    // Failing group should be at GroupFuse
    assert_eq!(
        tracker.get_group_outcome("failing"),
        MeltdownOutcome::GroupFuse,
        "Failing group should trip GroupFuse"
    );

    // Healthy group should still be able to operate
    let healthy_outcome = tracker.record_child_restart_with_group(
        healthy_child.clone(),
        Some("healthy".to_string()),
        now + Duration::from_secs(3),
    );
    assert_eq!(
        healthy_outcome,
        MeltdownOutcome::Continue,
        "Healthy group should not be affected by failing group's fuse"
    );
    assert_eq!(
        tracker.get_group_outcome("healthy"),
        MeltdownOutcome::Continue,
        "Healthy group outcome should remain Continue"
    );
}

#[test]
fn test_supervisor_level_threshold_independent_of_groups() {
    // SC-002: Verify that supervisor-level threshold is independent and not prematurely exhausted by single group

    let mut tracker = create_test_tracker();
    let now = std::time::Instant::now();

    // Exhaust one group completely (3 failures)
    for i in 0..3 {
        let child = ChildId::new(format!("exhausted-group-{i}"));
        tracker.record_child_restart_with_group(
            child,
            Some("exhausted".to_string()),
            now + Duration::from_secs(i),
        );
    }

    // Group should be fused
    assert_eq!(
        tracker.get_group_outcome("exhausted"),
        MeltdownOutcome::GroupFuse
    );

    // But supervisor-level should NOT be exhausted yet (supervisor_max_failures=10, only 3 used)
    let supervisor_outcome = tracker.get_supervisor_outcome();
    assert_eq!(
        supervisor_outcome,
        MeltdownOutcome::Continue,
        "Supervisor should still allow restarts (only 3/10 failures used)"
    );

    // Other groups should still be able to restart until supervisor threshold is reached
    let other_child = ChildId::new("other-group-child".to_string());
    let result = tracker.record_child_restart_with_group(
        other_child,
        Some("other".to_string()),
        now + Duration::from_secs(10),
    );
    assert_eq!(
        result,
        MeltdownOutcome::Continue,
        "Other groups should still operate within supervisor budget"
    );
}

#[test]
fn test_90_percent_isolation_metric() {
    // SC-002: Quantitative test - after one group triggers protection,
    // at least 90% of other group cases should still complete controlled restart attempts
    // UNLESS the supervisor-level threshold is independently exhausted.

    // Use higher supervisor threshold to avoid exhausting it during this test
    let policy = MeltdownPolicy::new(
        5,                        // child_max_restarts
        Duration::from_secs(10),  // child_window
        3,                        // group_max_failures (low threshold for testing)
        Duration::from_secs(30),  // group_window
        50,                       // supervisor_max_failures (high to avoid exhaustion)
        Duration::from_secs(60),  // supervisor_window
        Duration::from_secs(120), // reset_after
    );
    let mut tracker = MeltdownTracker::new(policy);
    let now = std::time::Instant::now();

    // Exhaust Group A completely (3 failures >= group_max_failures=3)
    for i in 0..3 {
        let child = ChildId::new(format!("group-a-{i}"));
        tracker.record_child_restart_with_group(
            child,
            Some("group-a".to_string()),
            now + Duration::from_secs(i),
        );
    }

    assert_eq!(
        tracker.get_group_outcome("group-a"),
        MeltdownOutcome::GroupFuse
    );

    // Now test 10 restart attempts from OTHER groups (each group gets only 1 attempt to stay below threshold)
    let mut successful_attempts = 0;
    let total_attempts = 10;

    for i in 0..total_attempts {
        // Use different group names to ensure no single group exceeds threshold
        let group_name = format!("other-group-{i}"); // Each attempt goes to a different group
        let child = ChildId::new(format!("{group_name}-child"));

        let outcome = tracker.record_child_restart_with_group(
            child,
            Some(group_name.clone()),
            now + Duration::from_secs(20 + i),
        );

        println!("Attempt {}: group={}, outcome={:?}", i, group_name, outcome);

        if outcome == MeltdownOutcome::Continue {
            successful_attempts += 1;
        }
    }

    // At least 90% should succeed (9 out of 10)
    // Supervisor threshold is NOT exhausted (only 13/50 used)
    let success_rate = successful_attempts as f64 / total_attempts as f64;
    assert!(
        success_rate >= 0.9,
        "At least 90% of other group restart attempts should succeed when supervisor budget is not exhausted. Got {:.0}% ({}/{})",
        success_rate * 100.0,
        successful_attempts,
        total_attempts
    );
}
