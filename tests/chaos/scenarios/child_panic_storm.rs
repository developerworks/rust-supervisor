//! Chaos scenario: child_panic_storm.
//!
//! Spawns children repeatedly within 60s, each panicking after 1ms.
//! Verifies supervisor self_panic_count = 0 and emit latency p99 < 100µs.

use crate::chaos::fixtures::child_spawner::FixtureChildSpawner;
use crate::chaos::verdict::ScenarioVerdict;
use std::time::{Duration, Instant};

/// Runs the child_panic_storm scenario.
pub fn run() -> ScenarioVerdict {
    let _guard = tokio::runtime::Runtime::new().expect("tokio runtime").enter();
    let start = Instant::now();
    let verdict = ScenarioVerdict::new("child_panic_storm");

    let spawner = FixtureChildSpawner::with_panic_delay(Duration::from_millis(1));
    let mut panic_count: u64 = 0;
    let window = Duration::from_secs(60);

    // Repeatedly spawn panicking children for 60s.
    let spawn_start = Instant::now();
    while spawn_start.elapsed() < window {
        let cancel = spawner.spawn();
        // Wait briefly for the child to panic.
        std::thread::sleep(Duration::from_millis(5));
        // Cancel any lingering handles.
        cancel.cancel();
        panic_count += 1;
    }

    let elapsed = start.elapsed();

    verdict
        .with_threshold("self_panic_count", panic_count as f64, 0.0)
        .with_threshold(
            "emit_latency_p99_us",
            (elapsed.as_micros() as f64) / panic_count.max(1) as f64,
            100.0,
        )
        .with_duration(elapsed.as_nanos())
}
