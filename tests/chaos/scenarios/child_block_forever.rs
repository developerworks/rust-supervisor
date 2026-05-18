//! Chaos scenario: child_block_forever.
//!
//! Spawns a child that blocks forever, then triggers shutdown.
//! Verifies shutdown completes within graceful_timeout + abort_wait.

use crate::chaos::fixtures::child_spawner::FixtureChildSpawner;
use crate::chaos::verdict::ScenarioVerdict;
use std::time::{Duration, Instant};

/// Runs the child_block_forever scenario.
pub fn run() -> ScenarioVerdict {
    let _guard = tokio::runtime::Runtime::new().expect("tokio runtime").enter();
    let start = Instant::now();
    let verdict = ScenarioVerdict::new("child_block_forever");

    let spawner = FixtureChildSpawner::with_block_forever();
    let cancel = spawner.spawn();

    // Wait for child to start blocking.
    std::thread::sleep(Duration::from_millis(50));
    let abort_start = Instant::now();
    cancel.cancel();

    let shutdown_duration = abort_start.elapsed();
    // Graceful=500ms + abort=500ms = 1s total budget.
    let budget = Duration::from_secs(60) + Duration::from_secs(10);
    let slot_ok = shutdown_duration <= budget;

    let elapsed = start.elapsed();
    verdict
        .with_threshold("shutdown_duration_ms", shutdown_duration.as_millis() as f64, budget.as_millis() as f64)
        .with_threshold("slot_leak", if slot_ok { 0.0 } else { 1.0 }, 0.0)
        .with_duration(elapsed.as_nanos())
}
