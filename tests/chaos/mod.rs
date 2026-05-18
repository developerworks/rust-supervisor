//! Chaos test module.
//!
//! This module provides the chaos scenario framework including the
//! `ChaosScenario` enum, scenario routing, and shared test fixtures.
//! All chaos tests live under `tests/chaos/` and are only referenced via
//! `[dev-dependencies]`, never from `src/` production code.

pub mod fixtures;
pub mod scenarios;
pub mod verdict;

use crate::chaos::verdict::ScenarioVerdict;

/// All 11 chaos scenario identifiers from the FR-001 threshold table.
///
/// Each variant maps to one file in `scenarios/` with a corresponding
/// `run()` function that returns a `ScenarioVerdict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChaosScenario {
    /// Child tasks panic repeatedly within 60s (1ms delay).
    ChildPanicStorm,
    /// Child blocks forever without responding to cancellation.
    ChildBlockForever,
    /// Child ignores CancellationToken.
    ChildIgnoreCancel,
    /// 10,000 rapid fail -> restart -> fail cycles in 60s.
    RapidFailure10k,
    /// Event subscriber throttled to 100ms/event.
    SlowEventSubscriber,
    /// mpsc command channel filled to capacity=256.
    CommandChannelFull,
    /// 1000 concurrent junk TCP handshakes to IPC endpoint.
    IpcConnectionStorm,
    /// Dashboard IPC started on an already-occupied socket path.
    SocketPathContention,
    /// Relay process SIGKILL'd and restarted 5 times.
    RelayCrashLoop,
    /// System clock stepped backward by 10s.
    ClockStepBackward,
    /// Tokio runtime starvation via yield_now loop for 30s.
    RuntimeStarvationProbe,
}

impl ChaosScenario {
    /// Returns the snake_case scenario identifier string.
    pub const fn scenario_id(&self) -> &'static str {
        match self {
            Self::ChildPanicStorm => "child_panic_storm",
            Self::ChildBlockForever => "child_block_forever",
            Self::ChildIgnoreCancel => "child_ignore_cancel",
            Self::RapidFailure10k => "rapid_failure_10k",
            Self::SlowEventSubscriber => "slow_event_subscriber",
            Self::CommandChannelFull => "command_channel_full",
            Self::IpcConnectionStorm => "ipc_connection_storm",
            Self::SocketPathContention => "socket_path_contention",
            Self::RelayCrashLoop => "relay_crash_loop",
            Self::ClockStepBackward => "clock_step_backward",
            Self::RuntimeStarvationProbe => "runtime_starvation_probe",
        }
    }

    /// Returns the semantic version from `CARGO_PKG_VERSION`.
    pub const fn semver() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Returns all scenarios in FR-001 order.
    pub const fn all() -> [Self; 11] {
        [
            Self::ChildPanicStorm,
            Self::ChildBlockForever,
            Self::ChildIgnoreCancel,
            Self::RapidFailure10k,
            Self::SlowEventSubscriber,
            Self::CommandChannelFull,
            Self::IpcConnectionStorm,
            Self::SocketPathContention,
            Self::RelayCrashLoop,
            Self::ClockStepBackward,
            Self::RuntimeStarvationProbe,
        ]
    }

    /// Runs this scenario and returns the verdict.
    pub fn run(&self) -> ScenarioVerdict {
        let id = self.scenario_id();
        scenarios::ScenarioRouter::new().run(id)
    }
}
