//! Runtime control loop watchdog.
//!
//! This module consumes a `JoinHandle` and writes its one-shot exit result into
//! `RuntimeControlPlane` so public handles can read the final report repeatedly.

use crate::runtime::lifecycle::{RuntimeControlPlane, RuntimeControlPlaneState, RuntimeExitReport};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// Watchdog that observes runtime control loop exit results.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeWatchdog;

impl RuntimeWatchdog {
    /// Publishes a control loop started event.
    ///
    /// # Arguments
    ///
    /// - `control_plane`: Control plane that should become alive.
    /// - `event_sender`: Event channel used for diagnostic text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn publish_started(
        control_plane: RuntimeControlPlane,
        event_sender: broadcast::Sender<String>,
    ) {
        control_plane.mark_alive();
        let _ignored = event_sender.send("runtime_control_loop_started:startup".to_owned());
    }

    /// Spawns the background watchdog.
    ///
    /// # Arguments
    ///
    /// - `control_plane`: Control plane that stores the final report.
    /// - `join_handle`: Runtime control loop task handle.
    /// - `event_sender`: Event channel used for diagnostic text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn spawn(
        control_plane: RuntimeControlPlane,
        join_handle: JoinHandle<RuntimeExitReport>,
        event_sender: broadcast::Sender<String>,
    ) {
        tokio::spawn(async move {
            let report = match join_handle.await {
                Ok(report) => report,
                Err(error) => RuntimeExitReport::failed(
                    "watchdog",
                    format!("runtime control loop panic or cancellation: {error}"),
                    error.is_panic(),
                    true,
                ),
            };
            let report = control_plane.complete(report);
            let event_name = match report.state {
                RuntimeControlPlaneState::Completed => "runtime_control_loop_completed",
                RuntimeControlPlaneState::Failed => "runtime_control_loop_failed",
                RuntimeControlPlaneState::Starting
                | RuntimeControlPlaneState::Alive
                | RuntimeControlPlaneState::ShuttingDown => "runtime_control_loop_unexpected",
            };
            let _ignored =
                event_sender.send(format!("{event_name}:{}:{}", report.phase, report.reason));
        });
    }
}
