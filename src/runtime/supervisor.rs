//! Runtime supervisor entry point.
//!
//! This module validates supervisor declarations, derives runtime options, and
//! returns a [`crate::control::handle::SupervisorHandle`].

use crate::config::state::ConfigState;
use crate::control::handle::SupervisorHandle;
use crate::dashboard::config::validate_dashboard_ipc_config;
use crate::dashboard::error::DashboardError;
use crate::dashboard::runtime::start_dashboard_ipc_runtime;
use crate::error::types::SupervisorError;
use crate::runtime::control_loop::{RuntimeControlState, run_control_loop};
use crate::runtime::lifecycle::RuntimeControlPlane;
use crate::runtime::watchdog::RuntimeWatchdog;
use crate::shutdown::stage::ShutdownPolicy;
use crate::spec::supervisor::SupervisorSpec;
use std::path::Path;
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

    /// Starts a supervisor runtime from validated configuration state.
    ///
    /// # Arguments
    ///
    /// - `state`: Validated configuration state owned by the caller.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`] only after configuration has produced a
    /// valid supervisor specification.
    pub async fn start_from_config_state(
        state: ConfigState,
    ) -> Result<SupervisorHandle, SupervisorError> {
        let ipc_config = state.ipc.clone();
        let spec = state.to_supervisor_spec()?;
        let mut handle = Self::start(spec.clone()).await?;
        let dashboard_config =
            validate_dashboard_ipc_config(ipc_config.as_ref()).map_err(dashboard_startup_error)?;
        if let Some(dashboard_config) = dashboard_config {
            let dashboard_runtime =
                start_dashboard_ipc_runtime(dashboard_config, spec, handle.clone())
                    .map_err(dashboard_startup_error)?;
            handle = handle.with_dashboard_runtime(dashboard_runtime);
        }
        Ok(handle)
    }

    /// Starts a supervisor runtime from a YAML configuration file.
    ///
    /// # Arguments
    ///
    /// - `path`: Path to the YAML configuration file.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`] only after the configuration file has
    /// loaded and validated successfully.
    pub async fn start_from_config_file(
        path: impl AsRef<Path>,
    ) -> Result<SupervisorHandle, SupervisorError> {
        let state = crate::config::loader::load_config_state(path)?;
        Self::start_from_config_state(state).await
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
        let control_plane = RuntimeControlPlane::new();
        let state = RuntimeControlState::new(spec, shutdown_policy, command_sender.clone())?;
        let join_handle = tokio::spawn(run_control_loop(
            state,
            command_receiver,
            event_sender.clone(),
        ));
        RuntimeWatchdog::publish_started(control_plane.clone(), event_sender.clone());
        RuntimeWatchdog::spawn(control_plane.clone(), join_handle, event_sender.clone());
        Ok(SupervisorHandle::new(
            command_sender,
            event_sender,
            control_plane,
        ))
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

/// Converts dashboard startup failures into supervisor startup errors.
fn dashboard_startup_error(error: DashboardError) -> SupervisorError {
    SupervisorError::fatal_config(format!("dashboard IPC startup failed: {error}"))
}
