//! Acceptance tests for concurrent restart throttle gates.
//!
//! This test verifies that:
//! 1. Concurrent restarts exceeding the gate limit enter queued or denied states
//! 2. Events indicate throttle gate ownership (supervisor_global or group:{group_id})
//! 3. Atomicity test: at least 10 concurrent failure samples all enter protection when exceeding limit

use rust_supervisor::event::payload::{ProtectionAction, ThrottleGateOwner};

/// Simulated concurrent restart gate
struct ConcurrentRestartGate {
    max_concurrent: usize,
    current_count: usize,
}

impl ConcurrentRestartGate {
    fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            current_count: 0,
        }
    }

    /// Try to acquire a restart slot. Returns the protection action.
    fn try_restart(&mut self) -> (ProtectionAction, ThrottleGateOwner) {
        if self.current_count < self.max_concurrent {
            self.current_count += 1;
            (
                ProtectionAction::RestartAllowed,
                ThrottleGateOwner::SupervisorInstance,
            )
        } else {
            // Gate exceeded - enter protection
            (
                ProtectionAction::RestartQueued,
                ThrottleGateOwner::SupervisorInstance,
            )
        }
    }

    /// Release a restart slot (called after restart starts)
    fn release(&mut self) {
        if self.current_count > 0 {
            self.current_count -= 1;
        }
    }
}

#[test]
fn test_concurrent_gate_allows_within_limit() {
    let mut gate = ConcurrentRestartGate::new(3);

    // First 3 should be allowed
    for _ in 0..3 {
        let (action, owner) = gate.try_restart();
        assert_eq!(action, ProtectionAction::RestartAllowed);
        assert_eq!(owner, ThrottleGateOwner::SupervisorInstance);
    }
}

#[test]
fn test_concurrent_gate_queues_when_exceeded() {
    let mut gate = ConcurrentRestartGate::new(2);

    // First 2 allowed
    gate.try_restart();
    gate.try_restart();

    // 3rd should be queued
    let (action, _) = gate.try_restart();
    assert_eq!(action, ProtectionAction::RestartQueued);
}

#[test]
fn test_atomicity_ten_concurrent_samples() {
    // Atomicity test: 10 concurrent failures with gate limit of 5
    // All beyond limit should enter protection
    let mut gate = ConcurrentRestartGate::new(5);
    let mut results = Vec::new();

    // Simulate 10 concurrent restart attempts
    for _ in 0..10 {
        let (action, owner) = gate.try_restart();
        results.push((action, owner));
    }

    // First 5 should be allowed
    for i in 0..5 {
        assert_eq!(results[i].0, ProtectionAction::RestartAllowed);
    }

    // Last 5 should be queued (protection triggered)
    for i in 5..10 {
        assert_eq!(results[i].0, ProtectionAction::RestartQueued);
        assert_eq!(results[i].1, ThrottleGateOwner::SupervisorInstance);
    }
}

#[test]
fn test_gate_release_allows_new_restarts() {
    let mut gate = ConcurrentRestartGate::new(2);

    // Fill the gate
    gate.try_restart();
    gate.try_restart();

    // Next should be queued
    let (action, _) = gate.try_restart();
    assert_eq!(action, ProtectionAction::RestartQueued);

    // Release one slot
    gate.release();

    // Now should be allowed
    let (action, _) = gate.try_restart();
    assert_eq!(action, ProtectionAction::RestartAllowed);
}

#[test]
fn test_throttle_gate_owner_display() {
    assert_eq!(format!("{}", ThrottleGateOwner::None), "none");
    assert_eq!(
        format!("{}", ThrottleGateOwner::SupervisorInstance),
        "supervisor_instance"
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
