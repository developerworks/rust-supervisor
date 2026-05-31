//! Runtime supervisor entry point.
//!
//! This module validates supervisor declarations, derives runtime options, and
//! returns a [`crate::control::handle::SupervisorHandle`].

use crate::config::state::ConfigState;
use crate::control::handle::SupervisorHandle;
#[cfg(unix)]
use crate::dashboard::{
    config::validate_dashboard_ipc_config, error::DashboardError,
    runtime::start_dashboard_ipc_runtime,
};
use crate::error::types::SupervisorError;
use crate::observe::pipeline::ObservabilityPipeline;
use crate::runtime::control_loop::{RuntimeControlState, run_control_loop};
use crate::runtime::lifecycle::RuntimeControlPlane;
use crate::runtime::watchdog::RuntimeWatchdog;
use crate::shutdown::stage::ShutdownPolicy;
use crate::spec::supervisor::SupervisorSpec;
use crate::task::factory_registry::TaskFactoryRegistry;
use std::path::Path;
use std::sync::{Arc, Mutex};
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
        #[cfg(unix)]
        let audit_config = state.audit.clone();
        #[cfg(unix)]
        let dashboard_config = state.dashboard.clone();
        let spec = state.to_supervisor_spec()?;
        #[cfg(unix)]
        let handle = Self::start(spec.clone()).await?;
        #[cfg(not(unix))]
        let handle = Self::start(spec).await?;
        #[cfg(unix)]
        let handle = if let Some(dashboard_config) =
            validate_dashboard_ipc_config(dashboard_config.as_ref())
                .map_err(dashboard_startup_error)?
        {
            let dashboard_runtime =
                start_dashboard_ipc_runtime(dashboard_config, audit_config, spec, handle.clone())
                    .map_err(dashboard_startup_error)?;
            handle.with_dashboard_runtime(dashboard_runtime)
        } else {
            handle
        };
        Ok(handle)
    }

    /// Starts a supervisor runtime from validated configuration and a factory registry.
    ///
    /// # Arguments
    ///
    /// - `state`: Validated configuration state owned by the caller.
    /// - `task_factory_registry`: Registry used to bind worker `factory_key` values.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`] only after configuration has produced a
    /// valid supervisor specification with executable worker factories.
    pub async fn start_from_config_state_with_factories(
        state: ConfigState,
        task_factory_registry: TaskFactoryRegistry,
    ) -> Result<SupervisorHandle, SupervisorError> {
        #[cfg(unix)]
        let audit_config = state.audit.clone();
        #[cfg(unix)]
        let dashboard_config = state.dashboard.clone();
        let spec = state.to_supervisor_spec_with_factories(&task_factory_registry)?;
        #[cfg(unix)]
        let handle =
            Self::start_with_factory_registry(spec.clone(), task_factory_registry.clone()).await?;
        #[cfg(not(unix))]
        let handle = Self::start_with_factory_registry(spec, task_factory_registry).await?;
        #[cfg(unix)]
        let handle = if let Some(dashboard_config) =
            validate_dashboard_ipc_config(dashboard_config.as_ref())
                .map_err(dashboard_startup_error)?
        {
            let dashboard_runtime =
                start_dashboard_ipc_runtime(dashboard_config, audit_config, spec, handle.clone())
                    .map_err(dashboard_startup_error)?;
            handle.with_dashboard_runtime(dashboard_runtime)
        } else {
            handle
        };
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
        let state = crate::config::loader::load_config_from_yaml_file(path)?;
        Self::start_from_config_state(state).await
    }

    /// Starts a supervisor runtime from a YAML file and a factory registry.
    ///
    /// # Arguments
    ///
    /// - `path`: Path to the YAML configuration file.
    /// - `task_factory_registry`: Registry used to bind worker `factory_key` values.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`] after configuration and factory binding succeed.
    pub async fn start_from_config_file_with_factories(
        path: impl AsRef<Path>,
        task_factory_registry: TaskFactoryRegistry,
    ) -> Result<SupervisorHandle, SupervisorError> {
        let state = crate::config::loader::load_config_from_yaml_file(path)?;
        Self::start_from_config_state_with_factories(state, task_factory_registry).await
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
        Self::start_with_policy_and_factory_registry(
            spec,
            shutdown_policy,
            TaskFactoryRegistry::new(),
        )
        .await
    }

    /// Starts a supervisor runtime with an explicit task factory registry.
    ///
    /// # Arguments
    ///
    /// - `spec`: Supervisor specification owned by the caller.
    /// - `task_factory_registry`: Registry used by dynamic child declarations.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`] connected to the runtime control loop.
    pub async fn start_with_factory_registry(
        spec: SupervisorSpec,
        task_factory_registry: TaskFactoryRegistry,
    ) -> Result<SupervisorHandle, SupervisorError> {
        let shutdown_policy = shutdown_policy_from_spec(&spec);
        Self::start_with_policy_and_factory_registry(spec, shutdown_policy, task_factory_registry)
            .await
    }

    /// Starts a supervisor runtime with explicit shutdown policy and factories.
    ///
    /// # Arguments
    ///
    /// - `spec`: Supervisor specification owned by the caller.
    /// - `shutdown_policy`: Policy used by the control loop.
    /// - `task_factory_registry`: Registry used by dynamic child declarations.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`] connected to the runtime control loop.
    pub async fn start_with_policy_and_factory_registry(
        spec: SupervisorSpec,
        shutdown_policy: ShutdownPolicy,
        task_factory_registry: TaskFactoryRegistry,
    ) -> Result<SupervisorHandle, SupervisorError> {
        spec.validate()?;
        let backpressure_config = spec.backpressure_config.clone();
        let (command_sender, command_receiver) = mpsc::channel(spec.control_channel_capacity);
        let (event_sender, _) = broadcast::channel(spec.event_channel_capacity);
        let control_plane = RuntimeControlPlane::new();
        let observability = Arc::new(Mutex::new(ObservabilityPipeline::with_backpressure_config(
            spec.event_channel_capacity,
            spec.event_channel_capacity,
            spec.metrics_enabled,
            spec.audit_enabled,
            backpressure_config,
        )));
        let state = RuntimeControlState::new_with_factory_registry(
            spec,
            shutdown_policy,
            command_sender.clone(),
            observability.clone(),
            task_factory_registry,
        )?;
        let join_handle = tokio::spawn(run_control_loop(
            state,
            command_receiver,
            event_sender.clone(),
        ));
        RuntimeWatchdog::publish_started(control_plane.clone(), event_sender.clone());
        RuntimeWatchdog::spawn(control_plane.clone(), join_handle, event_sender.clone());
        Ok(SupervisorHandle::new_with_observability(
            command_sender,
            event_sender,
            control_plane,
            observability,
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
        spec.force_kill_margin,
        spec.max_orphan_threshold,
    )
}

/// Converts dashboard startup failures into supervisor startup errors.
#[cfg(unix)]
fn dashboard_startup_error(error: DashboardError) -> SupervisorError {
    SupervisorError::fatal_config(format!("dashboard IPC startup failed: {error}"))
}
