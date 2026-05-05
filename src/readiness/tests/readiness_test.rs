//! Readiness signal tests.
//!
//! These tests verify immediate and explicit readiness behavior.

use rust_supervisor::readiness::signal::{ReadinessPolicy, ReadySignal};

/// Verifies that immediate readiness reports an immediate status.
#[test]
fn immediate_policy_reports_immediate_status() {
    assert!(ReadinessPolicy::Immediate.is_immediate());
    assert!(!ReadinessPolicy::Explicit.is_immediate());
}

/// Verifies that explicit ready signals publish readiness.
#[test]
fn ready_signal_publishes_explicit_readiness() {
    let (signal, receiver) = ReadySignal::new();

    signal.mark_ready();

    assert!(*receiver.borrow());
}
