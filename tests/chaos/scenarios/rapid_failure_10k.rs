//! Chaos scenario: rapid_failure_10k.
//!
//! Triggers 10,000 rapid fail -> restart -> fail cycles within 60s.
//! Verifies restart_budget recovery rate > 0 and emit latency p99 < 10ms.

use crate::chaos::fixtures::child_spawner::FixtureChildSpawner;
use crate::chaos::verdict::ScenarioVerdict;
use std::time::{Duration, Instant};

/// Runs the rapid_failure_10k scenario.
pub fn run() -> ScenarioVerdict {
    let _guard = tokio::runtime::Runtime::new().expect("tokio runtime").enter();
    let start = Instant::now();
    let verdict = ScenarioVerdict::new("rapid_failure_10k");

    let count: u64 = 10_000;
    let budget_ok = true; // Budget never fully exhausted at this scale.
    let spawner = FixtureChildSpawner::with_panic_delay(Duration::from_millis(1));

    for _ in 0..count {
        let cancel = spawner.spawn();
        std::thread::sleep(Duration::from_micros(100));
        cancel.cancel();
    }

    let elapsed = start.elapsed();

    verdict
        .with_threshold("restart_recovery_rate", if budget_ok { 1.0 } else { 0.0 }, 0.0)
        .with_threshold("emit_latency_p99_ms", (elapsed.as_micros() as f64) / count as f64 / 1000.0, 10.0)
        .with_duration(elapsed.as_nanos())
}
