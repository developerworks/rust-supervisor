//! Fairness probe tests.
//!
//! Validates starvation detection and normal scheduling.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::observe::fairness::FairnessProbe;

/// After 10s of only scheduling child A, child B should trigger starvation alert.
#[test]
fn test_fairness_probe_detects_starvation() {
    let now = 1_000_000_000_000u128;
    let mut probe = FairnessProbe::new(now);
    let child_a = ChildId::new("a".to_string());
    let child_b = ChildId::new("b".to_string());

    // Only record opportunities for child A for 10s
    for _ in 0..100 {
        probe.record_opportunity(&child_a);
    }

    // Check after probe interval
    let later = now + 10_000_000_001;
    let alert = probe.check(later, &[child_a.clone(), child_b.clone()]);
    assert!(alert.is_some(), "should detect starvation for child B");
    let alert = alert.unwrap();
    assert_eq!(alert.starved_child_id, child_b);
    assert!(alert.skip_count > 0);
}

/// When all children receive scheduling opportunities, no alert should fire.
#[test]
fn test_fairness_probe_ok_when_all_scheduled() {
    let now = 1_000_000_000_000u128;
    let mut probe = FairnessProbe::new(now);
    let child_a = ChildId::new("a".to_string());
    let child_b = ChildId::new("b".to_string());

    // Give both children opportunities
    for _ in 0..50 {
        probe.record_opportunity(&child_a);
        probe.record_opportunity(&child_b);
    }

    let later = now + 10_000_000_001;
    let alert = probe.check(later, &[child_a, child_b]);
    assert!(alert.is_none(), "no starvation when all children scheduled");
}
