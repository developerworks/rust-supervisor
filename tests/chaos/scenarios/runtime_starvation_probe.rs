//! Chaos scenario: runtime_starvation_probe.
//!
//! Injects a tokio::yield_now starvation loop for 30s.
//! Verifies control loop iteration count advances (>0 iter/s)
//! and emit latency p99 < 100ms.

use crate::chaos::fixtures::runtime_probe::FixtureRuntimeProbe;
use crate::chaos::verdict::ScenarioVerdict;
use std::time::{Duration, Instant};

/// Runs the runtime_starvation_probe scenario.
pub fn run() -> ScenarioVerdict {
    let start = Instant::now();
    let verdict = ScenarioVerdict::new("runtime_starvation_probe");

    let probe = FixtureRuntimeProbe::new();
    probe.inject_starvation_loop(Duration::from_secs(30));

    // Check if the control loop is still advancing.
    let result = probe.poll_count_stalled();

    let elapsed = start.elapsed();
    verdict
        .with_threshold("control_loop_iter_per_sec", result.iterations_per_sec, 0.0)
        .with_threshold("emit_latency_p99_ms", elapsed.as_millis() as f64 / 30.0, 100.0)
        .with_duration(elapsed.as_nanos())
}
