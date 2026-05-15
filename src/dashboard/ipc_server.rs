//! Target-side dashboard IPC service.
//!
//! This module provides the target process dispatcher behind a local Unix
//! domain socket. The service can be tested without a socket and can be bound to
//! a socket by runtime code that owns process lifecycle.

use crate::control::command::CommandResult;
use crate::control::handle::SupervisorHandle;
use crate::dashboard::config::ValidatedDashboardIpcConfig;
use crate::dashboard::error::DashboardError;
use crate::dashboard::model::{
    ControlCommandKind, ControlCommandRequest, ControlCommandResult, DashboardCurrentState,
    DashboardState, TargetProcessRegistration, dashboard_command_result_value,
    runtime_state_from_child_runtime_record,
};
use crate::dashboard::protocol::{
    DASHBOARD_IPC_PROTOCOL_VERSION, IpcMethod, IpcRequest, IpcResponse, IpcResult,
    decode_command_params,
};
use crate::dashboard::registration::build_registration_payload;
use crate::dashboard::state::{DashboardStateInput, build_dashboard_state};
use crate::id::types::{ChildId, SupervisorPath};
use crate::journal::ring::EventJournal;
use crate::spec::supervisor::SupervisorSpec;
use crate::state::supervisor::SupervisorState;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::UnixStream as StdUnixStream;
use tokio::net::UnixListener;

/// Target-side dashboard IPC service.
#[derive(Clone)]
pub struct DashboardIpcService {
    /// Validated IPC configuration.
    config: ValidatedDashboardIpcConfig,
    /// Supervisor declaration used for topology payloads.
    spec: SupervisorSpec,
    /// Current supervisor state payload.
    state: SupervisorState,
    /// Recent event journal.
    journal: EventJournal,
    /// Optional runtime control handle.
    handle: Option<SupervisorHandle>,
    /// Monotonic state generation.
    state_generation: u64,
}

impl DashboardIpcService {
    /// Creates a dashboard IPC service.
    ///
    /// # Arguments
    ///
    /// - `config`: Validated target-side IPC configuration.
    /// - `spec`: Supervisor declaration used for topology state.
    /// - `state`: Current supervisor state.
    /// - `journal`: Recent event journal.
    ///
    /// # Returns
    ///
    /// Returns a [`DashboardIpcService`] without a control handle.
    pub fn new(
        config: ValidatedDashboardIpcConfig,
        spec: SupervisorSpec,
        state: SupervisorState,
        journal: EventJournal,
    ) -> Self {
        Self {
            config,
            spec,
            state,
            journal,
            handle: None,
            state_generation: 1,
        }
    }

    /// Adds a runtime control handle to the service.
    ///
    /// # Arguments
    ///
    /// - `handle`: Runtime supervisor handle used for control commands.
    ///
    /// # Returns
    ///
    /// Returns the updated service.
    pub fn with_handle(mut self, handle: SupervisorHandle) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Returns the target registration payload.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the registration payload or a validation error.
    pub fn registration_payload(&self) -> Result<TargetProcessRegistration, DashboardError> {
        build_registration_payload(&self.config)
    }

    /// Handles one parsed IPC request.
    ///
    /// # Arguments
    ///
    /// - `request`: Parsed IPC request.
    ///
    /// # Returns
    ///
    /// Returns a response that preserves the request identifier.
    pub async fn handle_request(&self, request: IpcRequest) -> IpcResponse {
        match self.dispatch(&request).await {
            Ok(result) => IpcResponse::ok(request.request_id, result),
            Err(error) => IpcResponse::error(request.request_id, error),
        }
    }

    /// Dispatches one request by method.
    ///
    /// # Arguments
    ///
    /// - `request`: Parsed IPC request.
    ///
    /// # Returns
    ///
    /// Returns a typed IPC result.
    async fn dispatch(&self, request: &IpcRequest) -> Result<IpcResult, DashboardError> {
        let method = IpcMethod::parse(&request.method)?;
        match method {
            IpcMethod::Hello => Ok(IpcResult::Hello {
                protocol_version: DASHBOARD_IPC_PROTOCOL_VERSION.to_owned(),
                registration: self.registration_payload()?,
            }),
            IpcMethod::CurrentState => {
                let state = self.current_dashboard_state().await?;
                Ok(IpcResult::State {
                    target_id: state.target.target_id.clone(),
                    state: Box::new(state),
                })
            }
            IpcMethod::EventsSubscribe => {
                require_session_trigger(request, &self.config.target_id)?;
                Ok(IpcResult::Subscription {
                    target_id: self.config.target_id.clone(),
                    subscription: "events".to_owned(),
                })
            }
            IpcMethod::LogsTail => {
                require_session_trigger(request, &self.config.target_id)?;
                Ok(IpcResult::Subscription {
                    target_id: self.config.target_id.clone(),
                    subscription: "logs".to_owned(),
                })
            }
            IpcMethod::CommandRestartChild
            | IpcMethod::CommandPauseChild
            | IpcMethod::CommandResumeChild
            | IpcMethod::CommandQuarantineChild
            | IpcMethod::CommandRemoveChild
            | IpcMethod::CommandAddChild
            | IpcMethod::CommandShutdownTree => self.command_result(request).await,
        }
    }

    /// Builds the current dashboard state.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the current [`DashboardState`].
    pub async fn current_dashboard_state(&self) -> Result<DashboardState, DashboardError> {
        let registration = self.registration_payload().ok();
        let mut state = build_dashboard_state(
            DashboardStateInput {
                target_id: self.config.target_id.clone(),
                display_name: registration
                    .as_ref()
                    .map(|registration| registration.display_name.clone())
                    .unwrap_or_else(|| self.config.target_id.clone()),
                state_generation: self.state_generation,
                recent_limit: 128,
            },
            &self.spec,
            &self.state,
            &self.journal,
        );
        if let Some(handle) = self.handle.as_ref() {
            let result = handle.current_state().await.map_err(|error| {
                DashboardError::new(
                    "current_state_failed",
                    "state",
                    Some(self.config.target_id.clone()),
                    error.to_string(),
                    true,
                )
            })?;
            if let CommandResult::CurrentState {
                state: runtime_state,
            } = result
            {
                let dashboard_state = DashboardCurrentState::from_current_state(&runtime_state);
                state.runtime_state = runtime_state
                    .child_runtime_records
                    .iter()
                    .map(|record| {
                        runtime_state_from_child_runtime_record(
                            record,
                            runtime_state.shutdown_completed,
                        )
                    })
                    .collect();
                state.child_runtime_records = dashboard_state.child_runtime_records;
            }
        }
        Ok(state)
    }

    /// Executes a control command request.
    ///
    /// # Arguments
    ///
    /// - `request`: IPC request carrying command parameters.
    ///
    /// # Returns
    ///
    /// Returns a typed command result IPC payload.
    async fn command_result(&self, request: &IpcRequest) -> Result<IpcResult, DashboardError> {
        let command = decode_command_params(request)?;
        validate_command(&command)?;
        if command.target_id != self.config.target_id {
            return Err(DashboardError::validation(
                "command_validate",
                Some(self.config.target_id.clone()),
                "command target_id must match target process",
            ));
        }
        let result = if let Some(handle) = self.handle.as_ref() {
            execute_command(handle, &command).await
        } else {
            Err(DashboardError::target_unavailable(
                "command_dispatch",
                command.target_id.clone(),
                "runtime control handle is not attached",
            ))
        };
        let result = match result {
            Ok(result) => {
                let state_delta = dashboard_command_result_value(&result).map_err(|error| {
                    DashboardError::new(
                        "command_result_model_failed",
                        "command_dispatch",
                        Some(command.target_id.clone()),
                        format!("failed to map command result: {error}"),
                        false,
                    )
                })?;
                ControlCommandResult {
                    command_id: command.command_id.clone(),
                    target_id: command.target_id.clone(),
                    accepted: true,
                    status: "completed".to_owned(),
                    error: None,
                    state_delta: Some(state_delta),
                    completed_at_unix_nanos: Some(unix_nanos_now()),
                }
            }
            Err(error) => ControlCommandResult {
                command_id: command.command_id.clone(),
                target_id: command.target_id.clone(),
                accepted: false,
                status: "failed".to_owned(),
                error: Some(error),
                state_delta: None,
                completed_at_unix_nanos: Some(unix_nanos_now()),
            },
        };
        Ok(IpcResult::CommandResult {
            target_id: command.target_id,
            result,
        })
    }
}

/// Binds a target-side Unix domain socket listener.
///
/// # Arguments
///
/// - `config`: Validated IPC configuration.
///
/// # Returns
///
/// Returns a bound [`UnixListener`].
pub fn bind_dashboard_listener(
    config: &ValidatedDashboardIpcConfig,
) -> Result<UnixListener, DashboardError> {
    prepare_socket_path(config)?;
    UnixListener::bind(&config.path).map_err(|error| {
        DashboardError::new(
            "ipc_bind_failed",
            "ipc_bind",
            Some(config.target_id.clone()),
            format!("failed to bind target IPC socket: {error}"),
            true,
        )
    })
}

/// Prepares the configured socket path before binding.
///
/// # Arguments
///
/// - `config`: Validated IPC configuration.
///
/// # Returns
///
/// Returns `Ok(())` when binding may continue.
fn prepare_socket_path(config: &ValidatedDashboardIpcConfig) -> Result<(), DashboardError> {
    let metadata = match std::fs::symlink_metadata(&config.path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(DashboardError::new(
                "ipc_path_metadata_failed",
                "ipc_bind",
                Some(config.target_id.clone()),
                format!("failed to inspect IPC path: {error}"),
                false,
            ));
        }
    };
    match config.bind_mode {
        crate::config::configurable::DashboardIpcBindMode::CreateNew => {
            Err(DashboardError::validation(
                "ipc_bind",
                Some(config.target_id.clone()),
                "IPC path already exists and bind_mode is create_new",
            ))
        }
        crate::config::configurable::DashboardIpcBindMode::ReplaceStale => {
            if metadata.file_type().is_symlink() {
                return Err(DashboardError::validation(
                    "ipc_bind",
                    Some(config.target_id.clone()),
                    "IPC path must not be a symlink",
                ));
            }
            if !metadata.file_type().is_socket() {
                return Err(DashboardError::validation(
                    "ipc_bind",
                    Some(config.target_id.clone()),
                    "IPC path must be a Unix socket before stale replacement",
                ));
            }
            if StdUnixStream::connect(&config.path).is_ok() {
                return Err(DashboardError::validation(
                    "ipc_bind",
                    Some(config.target_id.clone()),
                    "IPC path is served by a live process",
                ));
            }
            std::fs::remove_file(&config.path).map_err(|error| {
                DashboardError::new(
                    "ipc_stale_remove_failed",
                    "ipc_bind",
                    Some(config.target_id.clone()),
                    format!("failed to remove stale IPC path: {error}"),
                    true,
                )
            })
        }
    }
}

/// Validates that subscription was triggered by an established session.
///
/// # Arguments
///
/// - `request`: Subscription request parameters.
/// - `target_id`: Target process identifier.
///
/// # Returns
///
/// Returns `Ok(())` when the relay provided the session trigger flag.
fn require_session_trigger(request: &IpcRequest, target_id: &str) -> Result<(), DashboardError> {
    let established = request
        .params
        .get("session_established")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if established {
        Ok(())
    } else {
        Err(DashboardError::new(
            "session_required",
            "subscription",
            Some(target_id.to_owned()),
            "event and log subscription must be triggered by an established dashboard session",
            false,
        ))
    }
}

/// Validates dashboard control command rules.
///
/// # Arguments
///
/// - `command`: Command request supplied by relay.
///
/// # Returns
///
/// Returns `Ok(())` when command input is acceptable.
pub fn validate_command(command: &ControlCommandRequest) -> Result<(), DashboardError> {
    if command.reason.trim().is_empty() {
        return Err(DashboardError::validation(
            "command_validate",
            Some(command.target_id.clone()),
            "command reason must not be empty",
        ));
    }
    if command.requested_by.trim().is_empty() {
        return Err(DashboardError::validation(
            "command_validate",
            Some(command.target_id.clone()),
            "requested_by must be derived by relay",
        ));
    }
    if matches!(
        command.command,
        ControlCommandKind::ShutdownTree
            | ControlCommandKind::RemoveChild
            | ControlCommandKind::AddChild
    ) && !command.confirmed
    {
        return Err(DashboardError::validation(
            "command_validate",
            Some(command.target_id.clone()),
            "dangerous command requires confirmation",
        ));
    }
    Ok(())
}

/// Executes a validated command through a runtime handle.
///
/// # Arguments
///
/// - `handle`: Runtime control handle.
/// - `command`: Validated command request.
///
/// # Returns
///
/// Returns a runtime command result or dashboard error.
async fn execute_command(
    handle: &SupervisorHandle,
    command: &ControlCommandRequest,
) -> Result<CommandResult, DashboardError> {
    let result = match command.command {
        ControlCommandKind::RestartChild => {
            handle
                .restart_child(child_id(command)?, &command.requested_by, &command.reason)
                .await
        }
        ControlCommandKind::PauseChild => {
            handle
                .pause_child(child_id(command)?, &command.requested_by, &command.reason)
                .await
        }
        ControlCommandKind::ResumeChild => {
            handle
                .resume_child(child_id(command)?, &command.requested_by, &command.reason)
                .await
        }
        ControlCommandKind::QuarantineChild => {
            handle
                .quarantine_child(child_id(command)?, &command.requested_by, &command.reason)
                .await
        }
        ControlCommandKind::RemoveChild => {
            handle
                .remove_child(child_id(command)?, &command.requested_by, &command.reason)
                .await
        }
        ControlCommandKind::AddChild => {
            handle
                .add_child(
                    SupervisorPath::root(),
                    command.target.child_manifest.clone().unwrap_or_default(),
                    &command.requested_by,
                    &command.reason,
                )
                .await
        }
        ControlCommandKind::ShutdownTree => {
            handle
                .shutdown_tree(&command.requested_by, &command.reason)
                .await
        }
    };
    result.map_err(|error| {
        DashboardError::new(
            "command_failed",
            "command_dispatch",
            Some(command.target_id.clone()),
            error.to_string(),
            true,
        )
    })
}

/// Extracts a child identifier from a command target.
///
/// # Arguments
///
/// - `command`: Command request with child path target.
///
/// # Returns
///
/// Returns the final child path segment as [`ChildId`].
fn child_id(command: &ControlCommandRequest) -> Result<ChildId, DashboardError> {
    let child_path = command.target.child_path.as_deref().ok_or_else(|| {
        DashboardError::validation(
            "command_validate",
            Some(command.target_id.clone()),
            "child_path is required for child command",
        )
    })?;
    let value = child_path
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or(child_path);
    Ok(ChildId::new(value))
}

/// Reads current wall-clock time as Unix nanoseconds.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns zero when the clock is before the Unix epoch.
fn unix_nanos_now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_nanos()
}
