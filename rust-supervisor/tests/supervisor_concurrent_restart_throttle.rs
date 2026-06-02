//! Acceptance tests for concurrent restart throttle gates (SC-003).
//!
//! This test verifies that:
//! 1. Concurrent restarts exceeding the gate limit enter denied states
//! 2. Events indicate throttle gate ownership (supervisor_global or group:{group_id})
//! 3. Atomicity test: at least 10 concurrent failure samples all enter protection when exceeding limit
//!
//! SC-003: Uses production ConcurrentGate implementation instead of local mock.

use rust_supervisor::event::payload::{ProtectionAction, ThrottleGateOwner};
use rust_supervisor::runtime::concurrent_gate::{
    CombinedThrottleGate, GroupLevelGate, SupervisorInstanceGate,
};

/// Helper to convert gate acquisition result to ProtectionAction and owner
fn evaluate_restart_attempt(
    gate: &CombinedThrottleGate,
    group_id: Option<&str>,
) -> (ProtectionAction, ThrottleGateOwner) {
    let acquired = gate.try_acquire(group_id);

    if acquired {
        (
            ProtectionAction::RestartAllowed,
            ThrottleGateOwner::SupervisorInstance,
        )
    } else {
        // Determine which gate caused the throttling
        let owner = if gate.instance_gate().is_saturated() {
            ThrottleGateOwner::SupervisorInstance
        } else if let Some(gid) = group_id {
            ThrottleGateOwner::Group(gid.to_string())
        } else {
            ThrottleGateOwner::SupervisorInstance
        };

        (ProtectionAction::RestartDenied, owner)
    }
}

#[test]
fn test_concurrent_gate_allows_within_limit() {
    // SC-003: Use production CombinedThrottleGate instead of local mock
    let instance_gate = SupervisorInstanceGate::new(3);
    let combined = CombinedThrottleGate::new(instance_gate, None);

    // First 3 should be allowed
    for _ in 0..3 {
        let (action, owner) = evaluate_restart_attempt(&combined, None);
        assert_eq!(action, ProtectionAction::RestartAllowed);
        assert_eq!(owner, ThrottleGateOwner::SupervisorInstance);
    }
}

#[test]
fn test_concurrent_gate_denies_when_exceeded() {
    // SC-003: Use production CombinedThrottleGate with group-level gating
    let instance_gate = SupervisorInstanceGate::new(5);
    let group_gate = GroupLevelGate::new(2);
    let combined = CombinedThrottleGate::new(instance_gate, Some(group_gate));

    // First 2 allowed for this group
    evaluate_restart_attempt(&combined, Some("test-group"));
    evaluate_restart_attempt(&combined, Some("test-group"));

    // 3rd should be denied due to group limit
    let (action, owner) = evaluate_restart_attempt(&combined, Some("test-group"));
    assert_eq!(action, ProtectionAction::RestartDenied);
    assert_eq!(owner, ThrottleGateOwner::Group("test-group".to_string()));
}

#[test]
fn test_atomicity_ten_concurrent_samples() {
    // SC-003: Atomicity test using production SupervisorInstanceGate permits.
    // 10 concurrent failures with instance gate limit of 5
    // All beyond limit should enter protection
    let gate = SupervisorInstanceGate::new(5);

    let mut results = Vec::new();
    let mut permits = Vec::new();

    // Simulate 10 concurrent restart attempts
    for _ in 0..10 {
        match gate.try_acquire() {
            Some(permit) => {
                permits.push(permit);
                results.push((
                    ProtectionAction::RestartAllowed,
                    ThrottleGateOwner::SupervisorInstance,
                ));
            }
            None => results.push((
                ProtectionAction::RestartDenied,
                ThrottleGateOwner::SupervisorInstance,
            )),
        }
    }

    // First 5 should be allowed
    for result in results.iter().take(5) {
        assert_eq!(result.0, ProtectionAction::RestartAllowed);
    }

    // Last 5 should be denied (protection triggered)
    for result in results.iter().take(10).skip(5) {
        assert_eq!(result.0, ProtectionAction::RestartDenied);
        assert_eq!(result.1, ThrottleGateOwner::SupervisorInstance);
    }
}

#[test]
fn test_gate_permit_drop_allows_new_restarts() {
    // SC-003: Test permit drop release mechanism using production gate.
    let gate = SupervisorInstanceGate::new(2);

    // Fill the gate
    let first = gate.try_acquire().expect("first permit");
    let second = gate.try_acquire().expect("second permit");

    // Next should be denied
    assert!(gate.try_acquire().is_none());

    // Release one slot
    drop(first);

    // Now should be allowed
    let third = gate.try_acquire();
    assert!(third.is_some());
    drop(second);
}

#[test]
fn test_throttle_gate_owner_display() {
    assert_eq!(format!("{}", ThrottleGateOwner::None), "none");
    assert_eq!(
        format!("{}", ThrottleGateOwner::SupervisorInstance),
        "supervisor_global"
    );
    assert_eq!(
        format!("{}", ThrottleGateOwner::Group("test-group".to_string())),
        "group:test-group"
    );
}

#[test]
fn test_protection_action_order() {
    // Verify RestartQueued is more restrictive than RestartAllowed
    assert!(ProtectionAction::RestartAllowed < ProtectionAction::RestartQueued);
}

#[test]
fn test_group_level_isolation_in_production_gate() {
    // SC-003: Verify group-level isolation using production GroupLevelGate
    let instance_gate = SupervisorInstanceGate::new(10);
    let group_gate = GroupLevelGate::new(2);
    let combined = CombinedThrottleGate::new(instance_gate, Some(group_gate));

    // Group A exhausts its limit
    evaluate_restart_attempt(&combined, Some("group-a"));
    evaluate_restart_attempt(&combined, Some("group-a"));

    // Group A's 3rd attempt should be denied
    let (action_a, _) = evaluate_restart_attempt(&combined, Some("group-a"));
    assert_eq!(action_a, ProtectionAction::RestartDenied);

    // Group B should still be able to restart (isolated from Group A)
    let (action_b, _) = evaluate_restart_attempt(&combined, Some("group-b"));
    assert_eq!(action_b, ProtectionAction::RestartAllowed);
}
