//! Chaos scenario implementations.
//!
//! Each submodule implements one fault-injection scenario from the
//! ChaosScenario threshold table defined in spec.md. The `ScenarioRouter`
//! dispatches execution by scenario_id.

pub mod child_block_forever;
pub mod child_ignore_cancel;
pub mod child_panic_storm;
pub mod clock_step_backward;
pub mod command_channel_full;
pub mod ipc_connection_storm;
pub mod rapid_failure_10k;
pub mod relay_crash_loop;
pub mod runtime_starvation_probe;
pub mod slow_event_subscriber;
pub mod socket_path_contention;

use crate::chaos::verdict::ScenarioVerdict;

/// Routes scenario execution by scenario_id.
#[derive(Debug, Default)]
pub struct ScenarioRouter;

impl ScenarioRouter {
    /// Creates a new router.
    pub fn new() -> Self {
        Self
    }

    /// Runs a single scenario by its string identifier.
    pub fn run(&self, scenario_id: &str) -> ScenarioVerdict {
        match scenario_id {
            "child_panic_storm" => child_panic_storm::run(),
            "child_block_forever" => child_block_forever::run(),
            "child_ignore_cancel" => child_ignore_cancel::run(),
            "rapid_failure_10k" => rapid_failure_10k::run(),
            "slow_event_subscriber" => slow_event_subscriber::run(),
            "command_channel_full" => command_channel_full::run(),
            "ipc_connection_storm" => ipc_connection_storm::run(),
            "socket_path_contention" => socket_path_contention::run(),
            "relay_crash_loop" => relay_crash_loop::run(),
            "clock_step_backward" => clock_step_backward::run(),
            "runtime_starvation_probe" => runtime_starvation_probe::run(),
            _ => ScenarioVerdict::new("unknown")
                .with_error(format!("unknown scenario: {scenario_id}")),
        }
    }

    /// Runs all 11 scenarios in FR-001 order and returns their verdicts.
    pub fn run_all(&self) -> Vec<ScenarioVerdict> {
        let ids = [
            "child_panic_storm",
            "child_block_forever",
            "child_ignore_cancel",
            "rapid_failure_10k",
            "slow_event_subscriber",
            "command_channel_full",
            "ipc_connection_storm",
            "socket_path_contention",
            "relay_crash_loop",
            "clock_step_backward",
            "runtime_starvation_probe",
        ];
        ids.iter().map(|id| self.run(id)).collect()
    }
}
