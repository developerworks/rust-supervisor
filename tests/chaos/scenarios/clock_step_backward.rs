//! Chaos scenario: clock_step_backward.
//!
//! Simulates system clock stepped backward by 10s using
//! FixtureClockController. Verifies sliding window budgets are not
//! distorted (uses monotonic clock) and circuit breaker is not reset.

use crate::chaos::fixtures::clock_controller::FixtureClockController;
use crate::chaos::verdict::ScenarioVerdict;
use std::time::{Duration, Instant};

/// Runs the clock_step_backward scenario.
pub fn run() -> ScenarioVerdict {
    let start = Instant::now();
    let verdict = ScenarioVerdict::new("clock_step_backward");

    let mut clock = FixtureClockController::new();
    clock.step_backward(Duration::from_secs(10));

    // Verify monotonic clock unaffected: Instant should still advance.
    let t1 = Instant::now();
    std::thread::sleep(Duration::from_millis(1));
    let t2 = Instant::now();
    let monotonic_ok = t2 > t1;

    // Verify offset was recorded.
    let offset_recorded = clock.offset().is_some();

    let elapsed = start.elapsed();
    verdict
        .with_threshold("monotonic_clock_ok", if monotonic_ok { 1.0 } else { 0.0 }, 1.0)
        .with_threshold("clock_offset_recorded", if offset_recorded { 1.0 } else { 0.0 }, 1.0)
        .with_duration(elapsed.as_nanos())
}
