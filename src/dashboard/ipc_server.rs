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
use crate::dashboard::runtime::DEFAULT_MAX_FRAME_BYTES;
use crate::dashboard::state::{DashboardStateInput, build_dashboard_state};
use crate::id::types::{ChildId, SupervisorPath};
use crate::ipc::security::peer_identity::PeerIdentity;
use crate::ipc::security::{CheckOutcome, IpcSecurityPipeline};
use crate::journal::ring::EventJournal;
use crate::spec::supervisor::SupervisorSpec;
use crate::state::supervisor::SupervisorState;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::UnixListener;

/// Target-side dashboard IPC service.
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
    /// State serial number, incremented only when runtime state actually
    /// changes (child exit, restart, control command completion, etc.).
    /// Pure read operations (e.g. CurrentState polling) do NOT bump this
    /// counter so clients can correctly detect genuine state transitions.
    state_generation: AtomicU64,
    /// Optional IPC security pipeline (C1-C9).
    security_pipeline: Option<Arc<Mutex<IpcSecurityPipeline>>>,
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
            state_generation: AtomicU64::new(1),
            security_pipeline: None,
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

    /// Adds an IPC security pipeline to the service.
    ///
    /// # Arguments
    ///
    /// - `pipeline`: Configured IPC security pipeline (C1-C9).
    ///
    /// # Returns
    ///
    /// Returns the updated service.
    pub fn with_security_pipeline(mut self, pipeline: IpcSecurityPipeline) -> Self {
        self.security_pipeline = Some(Arc::new(Mutex::new(pipeline)));
        self
    }

    /// Returns the effective maximum frame size in bytes for the frame
    /// reader.
    ///
    /// When a security pipeline is configured with C5 request size limit
    /// enabled, returns the configured `max_bytes`. Otherwise falls back
    /// to [`DEFAULT_MAX_FRAME_BYTES`] (1 MiB).
    ///
    /// This ensures the frame reader rejects oversized frames **before**
    /// JSON deserialization, fulfilling C5's contract.
    pub fn max_frame_bytes(&self) -> usize {
        self.security_pipeline
            .as_ref()
            .and_then(|p| p.lock().ok())
            .and_then(|guard| guard.request_size_limit_bytes())
            .unwrap_or(DEFAULT_MAX_FRAME_BYTES)
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

    /// Handles one parsed IPC request with connection context.
    ///
    /// Runs IPC security checks before dispatching when a
    /// security pipeline is configured.
    ///
    /// **Control point ordering:**
    ///
    /// 1. C6 (rate limit) → C5 (size limit) → C2 (peer credentials) → C3
    ///    (authorization) — via [`check()`].
    /// 2. **C8 (idempotency) checked before C4 (replay)** — if a cached
    ///    response exists for the request_id, it is returned immediately
    ///    without recording in the replay window. This prevents C4 from
    ///    rejecting a legitimate retry that carries the same request_id.
    /// 3. C4 (replay protection) — records the request_id only after
    ///    confirming C8 had no cached result.
    /// 4. Dispatch → C8 cache result → C7 (audit, post-dispatch).
    ///
    /// High-risk commands fail closed on audit write failure: when the
    /// audit backend is unwritable and the failure strategy is
    /// `fail_closed`, a denial response is returned instead of the normal
    /// dispatch result.
    ///
    /// # Arguments
    ///
    /// - `request`: Parsed IPC request.
    /// - `peer`: Real peer identity extracted from the connected socket.
    /// - `connection_id`: Unique identifier for this connection.
    /// - `raw_body_len`: Byte length of the raw request body.
    ///
    /// # Returns
    ///
    /// Returns a response that preserves the request identifier.
    pub async fn handle_request(
        &self,
        request: IpcRequest,
        peer: &PeerIdentity,
        connection_id: &str,
        raw_body_len: usize,
    ) -> IpcResponse {
        let method = request.method.clone();
        let request_id = request.request_id.clone();
        let is_high_risk = is_high_risk_command(&method, self);

        if let Some(ref pipeline) = self.security_pipeline {
            // Recover from a poisoned mutex instead of letting a single
            // panic in a prior request deny service to all subsequent ones.
            let mut guard = match pipeline.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    tracing::error!(
                        target: "rust_supervisor::ipc::security",
                        %method,
                        "pipeline mutex poisoned, recovering"
                    );
                    poisoned.into_inner()
                }
            };

            // Step 1: C6 → C5 → C2 → C3 (pre-dispatch checks, no C4/C8)
            match guard.check(&method, raw_body_len, peer, connection_id) {
                CheckOutcome::Denied(err) => {
                    let err_code = err.code.clone();
                    // C7: audit denial
                    let _ = self.audit_or_fail(
                        &mut guard,
                        &method,
                        peer,
                        false,
                        Some(&err),
                        &err_code,
                        is_high_risk,
                        &request_id,
                    );
                    return IpcResponse::error(request.request_id.clone(), err);
                }
                CheckOutcome::Passed => {}
            }

            // Step 2: C8 idempotency check BEFORE C4 replay.
            // If a cached response exists, return it immediately without
            // recording the request_id in the replay window.
            if let Some(cached_json) = guard.check_idempotency(&request_id) {
                let method = method.clone();
                let peer_clone = peer.clone();
                drop(guard);
                // C7: audit cache hit
                if let Some(ref pipeline) = self.security_pipeline {
                    let mut guard = match pipeline.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => {
                            tracing::error!(
                                target: "rust_supervisor::ipc::security",
                                %method,
                                "pipeline mutex poisoned during C8 audit"
                            );
                            poisoned.into_inner()
                        }
                    };
                    let _ = self.audit_or_fail(
                        &mut guard,
                        &method,
                        &peer_clone,
                        true,
                        None,
                        "c8_idempotency_cache_hit",
                        is_high_risk,
                        &request_id,
                    );
                }
                // Deserialize cached JSON into IpcResponse
                return serde_json::from_str(&cached_json).unwrap_or_else(|_| {
                    IpcResponse::error(
                        request_id,
                        DashboardError::new(
                            "idempotency_cache_corrupted",
                            "c8_idempotency",
                            Some(self.config.target_id.clone()),
                            "cached response failed to deserialize".to_owned(),
                            false,
                        ),
                    )
                });
            }

            // Step 3: C4 replay protection — record request_id only after
            // confirming no cached response exists.
            if let Err(err) = guard.check_replay_and_record(&request_id) {
                let err_code = err.code.clone();
                // C7: audit replay denial
                let _ = self.audit_or_fail(
                    &mut guard,
                    &method,
                    peer,
                    false,
                    Some(&err),
                    &err_code,
                    is_high_risk,
                    &request_id,
                );
                return IpcResponse::error(request.request_id.clone(), err);
            }
            drop(guard);
        }

        // ---- dispatch ----
        let dispatch_result = self.dispatch(&request).await;
        let mut response = match &dispatch_result {
            Ok(result) => IpcResponse::ok(request.request_id.clone(), result.clone()),
            Err(error) => IpcResponse::error(request.request_id.clone(), error.clone()),
        };

        // ---- post-dispatch: cache + audit (fail-closed for high-risk) ----
        if let Some(ref pipeline) = self.security_pipeline {
            let mut guard = match pipeline.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    tracing::error!(
                        target: "rust_supervisor::ipc::security",
                        %method,
                        "pipeline mutex poisoned during post-dispatch"
                    );
                    poisoned.into_inner()
                }
            };

            // C8: cache dispatch result
            if let Ok(response_json) = serde_json::to_string(&response) {
                guard.cache_result(&request_id, &response_json);
            }

            // C7: audit dispatch outcome
            let (allowed, denial_error, denial_code): (bool, Option<&DashboardError>, &str) =
                match &dispatch_result {
                    Ok(_) => (true, None, "dispatch_ok"),
                    Err(err) => (false, Some(err), err.code.as_str()),
                };
            // When audit write fails for a high-risk command with fail_closed
            // strategy, override the normal response with a denial.
            if let Err(audit_err) = self.audit_or_fail(
                &mut guard,
                &method,
                peer,
                allowed,
                denial_error,
                denial_code,
                is_high_risk,
                &request_id,
            ) && is_high_risk
            {
                tracing::error!(
                    target: "rust_supervisor::ipc::security::audit",
                    %method,
                    %request_id,
                    ?audit_err,
                    "HIGH-RISK command denied because audit write failed (fail-closed)"
                );
                response = IpcResponse::error(request.request_id.clone(), audit_err);
            }
        }

        response
    }

    /// Writes an audit record and returns the result.
    ///
    /// For high-risk commands with `fail_closed` strategy, audit failure
    /// returns `Err(DashboardError)` so the caller can return a denial
    /// response. The `write_audit` method on the pipeline already implements
    /// the strategy dispatch; this method just logs and propagates.
    #[allow(clippy::too_many_arguments)]
    fn audit_or_fail(
        &self,
        guard: &mut std::sync::MutexGuard<'_, IpcSecurityPipeline>,
        method: &str,
        peer: &PeerIdentity,
        allowed: bool,
        denial_error: Option<&DashboardError>,
        denial_code: &str,
        is_high_risk: bool,
        request_id: &str,
    ) -> Result<(), DashboardError> {
        guard
            .write_audit(
                method,
                peer,
                allowed,
                denial_error,
                denial_code,
                is_high_risk,
            )
            .map_err(|err| {
                tracing::error!(
                    target: "rust_supervisor::ipc::security::audit",
                    %method,
                    %request_id,
                    high_risk = is_high_risk,
                    ?err,
                    "audit write failed"
                );
                err
            })
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
                require_session_trigger(request, &self.config.target_id, self)?;
                Ok(IpcResult::Subscription {
                    target_id: self.config.target_id.clone(),
                    subscription: "events".to_owned(),
                })
            }
            IpcMethod::LogsTail => {
                require_session_trigger(request, &self.config.target_id, self)?;
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
    /// The `state_generation` in the returned state reflects the current
    /// serial number of the runtime — it is incremented only when runtime
    /// state actually transitions (child exits, restarts, control commands).
    /// Pure read operations do not bump this counter, so dashboard clients
    /// can reliably detect genuine state changes for cache invalidation.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the current [`DashboardState`].
    pub async fn current_dashboard_state(&self) -> Result<DashboardState, DashboardError> {
        // Read the current generation without incrementing — this is a
        // pure read operation and should not create the illusion of change.
        let generation = self.state_generation.load(Ordering::Relaxed);
        let registration = self.registration_payload().ok();
        let mut state = build_dashboard_state(
            DashboardStateInput {
                target_id: self.config.target_id.clone(),
                display_name: registration
                    .as_ref()
                    .map(|registration| registration.display_name.clone())
                    .unwrap_or_else(|| self.config.target_id.clone()),
                state_generation: generation,
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
                // Dashboard model attaches generation fence phase and pending restart via `DashboardCurrentState`.
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
        // Bump state generation on every completed command — the runtime
        // state may have changed as a result of the command execution.
        // This ensures dashboard clients can reliably detect real changes
        // while pure read operations (CurrentState) do not bump the counter.
        let _ = self.state_generation.fetch_add(1, Ordering::Relaxed);
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
    // Ensure the parent directory exists before binding.
    if let Some(parent) = config.path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            DashboardError::new(
                "ipc_parent_dir_creation_failed",
                "ipc_bind",
                Some(config.target_id.clone()),
                format!("failed to create IPC parent directory: {error}"),
                false,
            )
        })?;
    }
    let listener = UnixListener::bind(&config.path).map_err(|error| {
        DashboardError::new(
            "ipc_bind_failed",
            "ipc_bind",
            Some(config.target_id.clone()),
            format!("failed to bind target IPC socket: {error}"),
            true,
        )
    })?;

    // ---- enforce socket permissions immediately after bind ----
    let mode = parse_permissions_string(&config.permissions, &config.target_id)?;
    set_socket_permissions(&config.path, mode, &config.target_id)?;

    Ok(listener)
}

/// Parses a Unix permission octal string (e.g. "0600", "0777") into a
/// `u32` mode bits, rejecting malformed or overly permissive values.
///
/// # Arguments
///
/// - `perm_str`: Permission string, typically "0600".
/// - `target_id`: Target identifier for error messages.
///
/// # Returns
///
/// Returns the mode bits on success.
fn parse_permissions_string(perm_str: &str, target_id: &str) -> Result<u32, DashboardError> {
    // Must be 4 octal digits, e.g. "0600".
    if perm_str.len() != 4 || !perm_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DashboardError::validation(
            "ipc_bind",
            Some(target_id.to_owned()),
            format!("dashboard.permissions must be a 4-digit octal string, got \"{perm_str}\""),
        ));
    }
    let mode = u32::from_str_radix(perm_str, 8).map_err(|_| {
        DashboardError::validation(
            "ipc_bind",
            Some(target_id.to_owned()),
            format!("dashboard.permissions \"{perm_str}\" is not valid octal"),
        )
    })?;
    // Reject world-writable sockets (others write bit set).
    if mode & 0o002 != 0 {
        return Err(DashboardError::validation(
            "ipc_bind",
            Some(target_id.to_owned()),
            format!(
                "dashboard.permissions \"{perm_str}\" grants world-write access, \
                 which is not allowed for Unix domain sockets"
            ),
        ));
    }
    Ok(mode)
}

/// Sets Unix socket file permissions using `std::fs::set_permissions`.
///
/// # Arguments
///
/// - `path`: Socket file path.
/// - `mode`: Permission mode bits (e.g. 0o600).
/// - `target_id`: Target identifier for error messages.
///
/// # Returns
///
/// Returns `Ok(())` when permissions were applied.
fn set_socket_permissions(
    path: &std::path::Path,
    mode: u32,
    target_id: &str,
) -> Result<(), DashboardError> {
    let permissions = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(path, permissions).map_err(|error| {
        DashboardError::new(
            "ipc_set_permissions_failed",
            "ipc_bind",
            Some(target_id.to_owned()),
            format!("failed to set socket permissions to {mode:#o}: {error}"),
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
            // C1: socket owner check before removal
            crate::ipc::security::peer_identity::prepare_socket_path_for_bind(&config.path)?;
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
/// The relay must have already established a real dashboard session (peer
/// identity verified through C2/C3 security checks) before forwarding
/// subscription requests. This function verifies that the request came
/// through a security pipeline that has performed peer authentication.
///
/// If no security pipeline is configured, the subscription is denied
/// because there is no way to verify the caller's identity — this
/// prevents unauthorized event/log access when the Unix socket is
/// exposed to untrusted processes.
///
/// # Arguments
///
/// - `request`: Subscription request parameters. The `session_established`
///   flag is a relay assertion that must be backed by a live security
///   pipeline; it is not trusted on its own.
/// - `target_id`: Target process identifier.
/// - `service`: Dashboard IPC service that owns the security pipeline.
///
/// # Returns
///
/// Returns `Ok(())` when the relay provided the session trigger flag and
/// a security pipeline is active.
fn require_session_trigger(
    request: &IpcRequest,
    target_id: &str,
    service: &DashboardIpcService,
) -> Result<(), DashboardError> {
    // A security pipeline must be present — otherwise there is no peer
    // identity verification and the `session_established` flag is just an
    // untrusted JSON parameter.
    if service.security_pipeline.is_none() {
        return Err(DashboardError::new(
            "session_required",
            "subscription",
            Some(target_id.to_owned()),
            "event and log subscription require a security pipeline; \
             without one the session_established flag cannot be verified",
            false,
        ));
    }

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
    // command_id must be a valid UUID — never silently replace it.
    // Silently generating a new UUID would break audit trails and
    // idempotency cache lookups.
    if uuid::Uuid::parse_str(&command.command_id).is_err() {
        return Err(DashboardError::validation(
            "command_validate",
            Some(command.target_id.clone()),
            format!("command_id is not a valid UUID: {}", command.command_id),
        ));
    }
    // AddChild requires a non-empty manifest.
    if matches!(command.command, ControlCommandKind::AddChild) {
        let has_manifest = command
            .target
            .child_manifest
            .as_ref()
            .is_some_and(|m| !m.trim().is_empty());
        if !has_manifest {
            return Err(DashboardError::validation(
                "command_validate",
                Some(command.target_id.clone()),
                "add_child command requires a non-empty child_manifest",
            ));
        }
    }
    // Child-targeting commands require a non-empty child_path.
    if matches!(
        command.command,
        ControlCommandKind::RestartChild
            | ControlCommandKind::PauseChild
            | ControlCommandKind::ResumeChild
            | ControlCommandKind::QuarantineChild
            | ControlCommandKind::RemoveChild
    ) && command
        .target
        .child_path
        .as_ref()
        .is_none_or(|p| p.trim().is_empty())
    {
        return Err(DashboardError::validation(
            "command_validate",
            Some(command.target_id.clone()),
            format!(
                "{:?} command requires a non-empty child_path",
                command.command
            ),
        ));
    }
    // requested_at_unix_nanos should not be 0 or obviously in the future.
    // A value of 0 means the field was not set (programming error).
    if command.requested_at_unix_nanos == 0 {
        return Err(DashboardError::validation(
            "command_validate",
            Some(command.target_id.clone()),
            "requested_at_unix_nanos must not be 0",
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

/// Default command timeout in seconds, matching the registration payload's
/// declared timeout_seconds = 30. This ensures the IPC layer enforces the
/// same deadline that relay and UI expect.
const COMMAND_TIMEOUT_SECS: u64 = 30;

/// Executes a validated command through a runtime handle,
/// preserving the relay-supplied command_id for end-to-end tracing.
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
    // Enforce the command timeout declared in the registration payload.
    // Without this, a hung control loop (e.g. shutdown blocked on an
    // unyielding child) would leave the relay and UI waiting indefinitely.
    tokio::time::timeout(
        std::time::Duration::from_secs(COMMAND_TIMEOUT_SECS),
        execute_command_inner(handle, command),
    )
    .await
    .map_err(|_elapsed| {
        DashboardError::new(
            "command_timeout",
            "command_dispatch",
            Some(command.target_id.clone()),
            format!(
                "command {:?} timed out after {}s",
                command.command, COMMAND_TIMEOUT_SECS,
            ),
            true,
        )
    })?
}

/// Inner body of [`execute_command`], without the timeout wrapper.
async fn execute_command_inner(
    handle: &SupervisorHandle,
    command: &ControlCommandRequest,
) -> Result<CommandResult, DashboardError> {
    // Build a CommandMeta with the relay-supplied command_id so that
    // audit events and runtime state carry the same identifier that
    // the relay and UI see. The command_id is validated as a legal
    // UUID by validate_command() before dispatch, so unwrap is safe.
    let command_id = command
        .command_id
        .parse::<uuid::Uuid>()
        .expect("command_id already validated as legal UUID");
    let meta = crate::control::command::CommandMeta::with_id(
        crate::control::command::CommandId::from_uuid(command_id),
        &command.requested_by,
        &command.reason,
    );

    let result = match command.command {
        ControlCommandKind::RestartChild => {
            handle
                .execute_with_command_id(crate::control::command::ControlCommand::RestartChild {
                    meta: meta.clone(),
                    child_id: child_id(command)?,
                })
                .await
        }
        ControlCommandKind::PauseChild => {
            handle
                .execute_with_command_id(crate::control::command::ControlCommand::PauseChild {
                    meta: meta.clone(),
                    child_id: child_id(command)?,
                })
                .await
        }
        ControlCommandKind::ResumeChild => {
            handle
                .execute_with_command_id(crate::control::command::ControlCommand::ResumeChild {
                    meta: meta.clone(),
                    child_id: child_id(command)?,
                })
                .await
        }
        ControlCommandKind::QuarantineChild => {
            handle
                .execute_with_command_id(crate::control::command::ControlCommand::QuarantineChild {
                    meta: meta.clone(),
                    child_id: child_id(command)?,
                })
                .await
        }
        ControlCommandKind::RemoveChild => {
            handle
                .execute_with_command_id(crate::control::command::ControlCommand::RemoveChild {
                    meta: meta.clone(),
                    child_id: child_id(command)?,
                })
                .await
        }
        ControlCommandKind::AddChild => {
            handle
                .execute_with_command_id(crate::control::command::ControlCommand::AddChild {
                    meta: meta.clone(),
                    target: SupervisorPath::root(),
                    child_manifest: command.target.child_manifest.clone().unwrap_or_default(),
                })
                .await
        }
        ControlCommandKind::ShutdownTree => {
            handle
                .execute_with_command_id(crate::control::command::ControlCommand::ShutdownTree {
                    meta: meta.clone(),
                })
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

/// Returns `true` when the method is a high-risk command that must not
/// execute without a successful audit write (fail-closed).
///
/// When a security pipeline is configured, reads the method list from
/// `AuthorizationConfig.high_risk_commands`. Otherwise falls back to
/// a hardcoded set that includes all write/destructive commands plus
/// pause and resume.
fn is_high_risk_command(method: &str, service: &DashboardIpcService) -> bool {
    // Prefer configured list when security pipeline is available.
    if let Some(ref pipeline) = service.security_pipeline
        && let Ok(guard) = pipeline.lock()
    {
        let configured = guard.high_risk_methods();
        if !configured.is_empty() {
            return configured.iter().any(|m| m == method);
        }
    }
    // Fallback: all write/destructive commands including pause/resume.
    matches!(
        method,
        "command.restart_child"
            | "command.pause_child"
            | "command.resume_child"
            | "command.quarantine_child"
            | "command.remove_child"
            | "command.shutdown_tree"
            | "command.add_child"
    )
}
