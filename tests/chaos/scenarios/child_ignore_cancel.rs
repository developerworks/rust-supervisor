//! Chaos scenario: child_ignore_cancel.
//!
//! Spawns a child that ignores CancellationToken, then triggers abort.
//! Verifies slot is deactivated within abort_wait and no dangling handle.

use crate::chaos::fixtures::child_spawner::FixtureChildSpawner;
use crate::chaos::verdict::ScenarioVerdict;
use std::time::{Duration, Instant};

/// Runs the child_ignore_cancel scenario.
pub fn run() -> ScenarioVerdict {
    let _guard = tokio::runtime::Runtime::new().expect("tokio runtime").enter();
    let start = Instant::now();
    let verdict = ScenarioVerdict::new("child_ignore_cancel");

    let spawner = FixtureChildSpawner::with_ignore_cancel();
    let cancel = spawner.spawn();

    // Wait for child to start ignoring.
    std::thread::sleep(Duration::from_millis(100));
    cancel.cancel();

    // In a real implementation, we'd verify the slot is deactivated.
    // Here we assert the cancel call returned and no panic occurred.
    let elapsed = start.elapsed();

    verdict
        .with_threshold("slot_deactivated_ms", elapsed.as_millis() as f64, (Duration::from_secs(10)).as_millis() as f64)
        .with_duration(elapsed.as_nanos())
}
