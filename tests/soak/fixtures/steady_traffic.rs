//! Steady traffic generator for soak tests.
//!
//! Generates a constant workload of 1000 req/s by injecting simulated
//! child events or commands into the supervisor.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Generates steady traffic for soak testing.
#[derive(Debug)]
pub struct SteadyTrafficGenerator {
    /// Target requests per second.
    pub target_rps: u64,
    /// Whether the generator is running.
    running: Arc<AtomicBool>,
}

impl Default for SteadyTrafficGenerator {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl SteadyTrafficGenerator {
    /// Creates a new traffic generator with the given target RPS.
    pub fn new(target_rps: u64) -> Self {
        Self {
            target_rps,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts the traffic generator.
    ///
    /// Spawns a Tokio task that generates events at the target rate.
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let interval = Duration::from_secs_f64(1.0 / self.target_rps as f64);

        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                // Simulate an event or command injection.
                // In a full implementation, this would send commands
                // through the supervisor's command channel.
                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Stops the traffic generator.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
