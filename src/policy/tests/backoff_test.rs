//! Backoff policy tests.
//!
//! These tests verify exponential delay, caps, reset, and jitter behavior.

use rust_supervisor::policy::backoff::BackoffPolicy;
use std::time::Duration;

#[test]
fn exponential_backoff_caps_at_maximum() {
    let policy = BackoffPolicy::new(
        Duration::from_millis(10),
        Duration::from_millis(25),
        0,
        Duration::from_secs(1),
    );

    assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(10));
    assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(20));
    assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(25));
}

#[test]
fn stable_runtime_resets_backoff_window() {
    let policy = BackoffPolicy::new(
        Duration::from_millis(10),
        Duration::from_millis(100),
        0,
        Duration::from_secs(5),
    );

    assert!(!policy.should_reset(Duration::from_secs(4)));
    assert!(policy.should_reset(Duration::from_secs(5)));
}
