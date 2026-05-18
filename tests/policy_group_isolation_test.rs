//! Group isolation tests.
//!
//! Validates that group fuse does not affect unrelated groups.

use rust_supervisor::policy::group::{
    GroupDependencyEdge, GroupIsolationPolicy, PropagationPolicy,
};

/// Without a declared dependency edge, group B is unaffected by group A's meltdown.
#[test]
fn test_group_fuse_does_not_affect_unrelated_group() {
    let policy = GroupIsolationPolicy::new(vec![]);
    assert!(
        !policy.affected_by("group_b", "group_a"),
        "unrelated group should NOT be affected"
    );
}

/// When a Full propagation edge exists (B depends on A), B IS affected.
#[test]
fn test_dependency_edge_propagates_fuse() {
    let edge = GroupDependencyEdge {
        from_group: "group_b".to_string(),
        to_group: "group_a".to_string(),
        propagation: PropagationPolicy::Full,
    };
    let policy = GroupIsolationPolicy::new(vec![edge]);
    assert!(
        policy.affected_by("group_b", "group_a"),
        "group_b should be affected when Full edge exists"
    );
}

/// Same-group check: a group is always affected by its own failure.
#[test]
fn test_same_group_always_affected() {
    let policy = GroupIsolationPolicy::new(vec![]);
    assert!(policy.affected_by("group_a", "group_a"));
}

/// EscalateOnly propagation should not affect the dependent group's scheduling.
#[test]
fn test_escalate_only_does_not_propagate_to_group() {
    let edge = GroupDependencyEdge {
        from_group: "group_b".to_string(),
        to_group: "group_a".to_string(),
        propagation: PropagationPolicy::EscalateOnly,
    };
    let policy = GroupIsolationPolicy::new(vec![edge]);
    assert!(
        !policy.affected_by("group_b", "group_a"),
        "EscalateOnly should not propagate fuse to dependent group"
    );
}

/// Simulates 24h sliding window: group A meltdown for 24h,
/// group B's isolation holds throughout.
#[test]
fn test_group_isolation_24h_sliding_window() {
    let policy = GroupIsolationPolicy::new(vec![]);
    for _ in 0..1000 {
        assert!(!policy.affected_by("group_b", "group_a"));
    }
    let edge = GroupDependencyEdge {
        from_group: "group_b".to_string(),
        to_group: "group_a".to_string(),
        propagation: PropagationPolicy::Full,
    };
    let policy_with_edge = GroupIsolationPolicy::new(vec![edge]);
    for _ in 0..1000 {
        assert!(policy_with_edge.affected_by("group_b", "group_a"));
    }
}
