//! Runtime supervisor entry point.
//!
//! This module validates supervisor declarations, derives runtime options, and
//! returns a [`crate::control::handle::SupervisorHandle`].

use crate::control::handle::SupervisorHandle;
use crate::error::types::SupervisorError;
use crate::runtime::control_loop::run_control_loop;
use crate::shutdown::stage::ShutdownPolicy;
use crate::spec::supervisor::SupervisorSpec;
use tokio::sync::{broadcast, mpsc};

/// Supervisor runtime entry point.
#[derive(Debug, Clone, Copy, Default)]
pub struct Supervisor;

impl Supervisor {
    /// Starts a supervisor runtime from an owned specification value.
    ///
    /// # Arguments
    ///
    /// - `spec`: Supervisor specification owned by the caller.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`] connected to the runtime control loop.
    pub async fn start(spec: SupervisorSpec) -> Result<SupervisorHandle, SupervisorError> {
        let shutdown_policy = shutdown_policy_from_spec(&spec);
        Self::start_with_policy(spec, shutdown_policy).await
    }

    /// Starts a supervisor runtime with an explicit shutdown policy.
    ///
    /// # Arguments
    ///
    /// - `spec`: Supervisor specification owned by the caller.
    /// - `shutdown_policy`: Policy used by the control loop.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`] connected to the runtime control loop.
    pub async fn start_with_policy(
        spec: SupervisorSpec,
        shutdown_policy: ShutdownPolicy,
    ) -> Result<SupervisorHandle, SupervisorError> {
        spec.validate()?;
        let (command_sender, command_receiver) = mpsc::channel(spec.control_channel_capacity);
        let (event_sender, _) = broadcast::channel(spec.event_channel_capacity);
        tokio::spawn(run_control_loop(
            command_receiver,
            event_sender.clone(),
            shutdown_policy,
        ));
        Ok(SupervisorHandle::new(command_sender, event_sender))
    }
}

/// Builds the shutdown policy from supervisor defaults.
///
/// # Arguments
///
/// - `spec`: Supervisor declaration that owns default shutdown values.
///
/// # Returns
///
/// Returns a [`ShutdownPolicy`] for runtime shutdown coordination.
fn shutdown_policy_from_spec(spec: &SupervisorSpec) -> ShutdownPolicy {
    ShutdownPolicy::new(
        spec.default_shutdown_policy.graceful_timeout,
        spec.default_shutdown_policy.abort_wait,
        true,
    )
}
