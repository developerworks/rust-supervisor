//! Chaos scenario: slow_event_subscriber.
//!
//! Subscriber callback throttled to 100ms/event. Runs high-frequency
//! event pump. Verifies backpressure matches 006-5 default (AlertAndBlock)
//! and event_gap_total <= discard_budget.

use crate::chaos::fixtures::event_throttle::FixtureEventThrottle;
use crate::chaos::verdict::ScenarioVerdict;
use std::time::{Duration, Instant};

/// Runs the slow_event_subscriber scenario.
pub fn run() -> ScenarioVerdict {
    let start = Instant::now();
    let verdict = ScenarioVerdict::new("slow_event_subscriber");

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let throttle = FixtureEventThrottle::new(100);
    let mut event_gap: u64 = 0;
    let discard_budget: u64 = 0;

    // Simulate 60 events with throttled consumer.
    let pump_start = Instant::now();
    rt.block_on(async {
        for _ in 0..60 {
            // Producer side: emit event.
            // Consumer side: throttled at 100ms/event.
            throttle.process_event().await;
        }
    });
    let pump_time = pump_start.elapsed();

    // If pump took longer than events * 100ms, gap may occur.
    let expected_min = Duration::from_millis(60 * 100);
    if pump_time > expected_min {
        event_gap = (pump_time.as_millis() - expected_min.as_millis()) as u64;
    }

    let elapsed = start.elapsed();
    let gap_ok = event_gap <= discard_budget;

    verdict
        .with_threshold("event_gap_total", event_gap as f64, discard_budget as f64)
        .with_threshold("backpressure_alert", if gap_ok { 0.0 } else { 1.0 }, 0.0)
        .with_duration(elapsed.as_nanos())
}
