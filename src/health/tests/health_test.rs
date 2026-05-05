//! Heartbeat health tests.
//!
//! These tests verify heartbeat freshness and stale detection.

use rust_supervisor::health::heartbeat::{HealthPolicy, Heartbeat};
use rust_supervisor::id::types::ChildId;
use std::time::{Duration, Instant};

#[test]
fn stale_detection_uses_monotonic_time() {
    let policy = HealthPolicy::new(Duration::from_secs(1), Duration::from_secs(3));
    let now = Instant::now();
    let heartbeat = Heartbeat::new(ChildId::new("worker"), now, None);

    assert!(!policy.is_stale(&heartbeat, now + Duration::from_secs(3)));
    assert!(policy.is_stale(&heartbeat, now + Duration::from_secs(4)));
}
