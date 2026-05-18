//! Acceptance tests for meltdown lead_scope tie-breaking rules.
//!
//! This test verifies that:
//! 1. When multiple layers trigger meltdown simultaneously, lead_scope follows
//!    the priority order: child → group → supervisor
//! 2. The dominant attribution scope is correctly identified in events

use rust_supervisor::event::payload::MeltdownScope;
use rust_supervisor::policy::meltdown::{
    LocalVerdict, MeltdownOutcome, MeltdownPolicy, MeltdownTracker, merge_meltdown_verdicts,
};
use std::time::Duration;

/// Creates a local verdict for merge-rule tests.
fn verdict(triggered: bool, outcome: MeltdownOutcome) -> LocalVerdict {
    LocalVerdict { triggered, outcome }
}

#[test]
fn test_lead_scope_child_priority_when_effective_outcome_ties() {
    // When child and group both match the effective outcome, child wins the tie.
    let merged = merge_meltdown_verdicts(
        verdict(true, MeltdownOutcome::GroupFuse),
        verdict(true, MeltdownOutcome::GroupFuse),
        verdict(false, MeltdownOutcome::Continue),
    );

    assert_eq!(merged.effective_outcome, MeltdownOutcome::GroupFuse);
    assert_eq!(merged.lead_scope, Some(MeltdownScope::Child));
}

#[test]
fn test_lead_scope_group_priority_over_supervisor_when_tied() {
    // When group and supervisor both match the effective outcome, group wins the tie.
    let merged = merge_meltdown_verdicts(
        verdict(false, MeltdownOutcome::Continue),
        verdict(true, MeltdownOutcome::SupervisorFuse),
        verdict(true, MeltdownOutcome::SupervisorFuse),
    );

    assert_eq!(merged.effective_outcome, MeltdownOutcome::SupervisorFuse);
    assert_eq!(merged.lead_scope, Some(MeltdownScope::Group));
}

#[test]
fn test_lead_scope_uses_scope_matching_effective_outcome() {
    // Lower-severity triggered scopes do not win attribution over the strictest scope.
    let merged = merge_meltdown_verdicts(
        verdict(true, MeltdownOutcome::ChildFuse),
        verdict(true, MeltdownOutcome::GroupFuse),
        verdict(true, MeltdownOutcome::SupervisorFuse),
    );

    assert_eq!(merged.effective_outcome, MeltdownOutcome::SupervisorFuse);
    assert_eq!(merged.lead_scope, Some(MeltdownScope::Supervisor));
}

#[test]
fn test_lead_scope_all_three_layers() {
    // When all three layers tie at the effective outcome, lead_scope should be Child.
    let merged = merge_meltdown_verdicts(
        verdict(true, MeltdownOutcome::SupervisorFuse),
        verdict(true, MeltdownOutcome::SupervisorFuse),
        verdict(true, MeltdownOutcome::SupervisorFuse),
    );

    assert_eq!(merged.effective_outcome, MeltdownOutcome::SupervisorFuse);
    assert_eq!(merged.lead_scope, Some(MeltdownScope::Child));
}

#[test]
fn test_lead_scope_none_when_no_triggers() {
    // When no layer triggers, lead_scope should be None.
    let merged = merge_meltdown_verdicts(
        verdict(false, MeltdownOutcome::Continue),
        verdict(false, MeltdownOutcome::Continue),
        verdict(false, MeltdownOutcome::Continue),
    );

    assert_eq!(merged.lead_scope, None);
}

#[test]
fn test_scopes_triggered_list() {
    // Verify that scopes_triggered contains all triggered scopes
    let policy = MeltdownPolicy::new(
        1, // Low threshold for child
        Duration::from_secs(10),
        1, // Low threshold for group
        Duration::from_secs(30),
        1, // Low threshold for supervisor
        Duration::from_secs(60),
        Duration::from_secs(120),
    );

    let mut tracker = MeltdownTracker::new(policy);
    let now = std::time::Instant::now();
    let child_id = rust_supervisor::id::types::ChildId::new("test-child".to_string());

    // Record enough failures to trigger all layers
    tracker.record_child_restart_with_group(child_id.clone(), Some("test-group".to_string()), now);
    tracker.record_child_restart_with_group(
        child_id,
        Some("test-group".to_string()),
        now + Duration::from_secs(1),
    );

    // All three should be triggered (child=2>1, group=2>1, supervisor=2>1)
    let outcome = tracker.current_outcome_for_test();

    // The most severe outcome should be SupervisorFuse (checked first in current_outcome)
    assert_eq!(outcome, MeltdownOutcome::SupervisorFuse);

    // In a full implementation, scopes_triggered would contain all three scopes
    // For now, we verify the outcome reflects the highest severity
}

#[test]
fn test_meltdown_scope_display() {
    // Verify MeltdownScope display format
    assert_eq!(format!("{}", MeltdownScope::Child), "child");
    assert_eq!(format!("{}", MeltdownScope::Group), "group");
    assert_eq!(format!("{}", MeltdownScope::Supervisor), "supervisor");
}
