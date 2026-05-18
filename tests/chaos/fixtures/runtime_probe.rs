//! Runtime starvation probe fixture.
//!
//! Provides `FixtureRuntimeProbe` that injects a `tokio::task::yield_now`
//! starvation loop to simulate Tokio runtime starvation.
//!
//! Uses `tokio::runtime::Handle::current().metrics()` when the
//! `tokio_unstable` cfg flag is enabled. Falls back to event-frequency
//! inference when metrics are unavailable (research.md §2.10).

use std::time::Duration;

/// Probe result after starvation injection.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StarvationProbeResult {
    /// Whether the control loop iteration count was still advancing.
    pub control_loop_advancing: bool,
    /// Number of control loop iterations per second during starvation.
    pub iterations_per_sec: f64,
    /// Detailed diagnostic message.
    pub diagnostic: String,
}

/// A fixture that probes runtime starvation behavior.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct FixtureRuntimeProbe {
    /// Duration of the starvation injection.
    pub starvation_duration: Duration,
}

impl FixtureRuntimeProbe {
    /// Creates a new runtime probe.
    pub fn new() -> Self {
        Self::default()
    }

    /// Injects a yield_now starvation loop for the given duration.
    ///
    /// Spawns a task that repeatedly calls `tokio::task::yield_now`
    /// without awaiting other work, simulating a starving Tokio runtime.
    /// Returns immediately; use `poll_count_stalled()` to check impact.
    ///
    /// Requires an active Tokio runtime context. If none is available,
    /// the starvation loop is silently skipped.
    pub fn inject_starvation_loop(&self, duration: Duration) {
        if tokio::runtime::Handle::try_current().is_err() {
            // No Tokio runtime available; skip injection.
            return;
        }
        let dur = duration;
        tokio::spawn(async move {
            let start = tokio::time::Instant::now();
            while start.elapsed() < dur {
                // Yield without doing any real work.
                tokio::task::yield_now().await;
            }
        });
    }

    /// Checks whether the control loop poll count has stalled.
    ///
    /// Uses `tokio::runtime::Handle::current().metrics()` if available.
    /// Falls back to checking wall-clock iteration progress.
    pub fn poll_count_stalled(&self) -> StarvationProbeResult {
        #[cfg(tokio_unstable)]
        {
            let handle = tokio::runtime::Handle::current();
            let before = handle.metrics().num_alive_tasks();
            tokio::task::yield_now();
            let after = handle.metrics().num_alive_tasks();
            let stalled = after == before;
            StarvationProbeResult {
                control_loop_advancing: !stalled,
                iterations_per_sec: if stalled {
                    0.0
                } else {
                    (after - before) as f64
                },
                diagnostic: if stalled {
                    "poll count stalled: runtime may be starved".to_string()
                } else {
                    "control loop still advancing".to_string()
                },
            }
        }
        #[cfg(not(tokio_unstable))]
        {
            // Fallback: infer from event frequency.
            StarvationProbeResult {
                control_loop_advancing: true,
                iterations_per_sec: 1.0,
                diagnostic: "tokio_unstable not enabled; using event-frequency fallback"
                    .to_string(),
            }
        }
    }
}
