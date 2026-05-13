//! Dashboard IPC runtime lifecycle.
//!
//! The runtime owns the target-side Unix socket accept loop and the dynamic
//! registration heartbeat used by the relay integration.

use crate::control::handle::SupervisorHandle;
use crate::dashboard::config::ValidatedDashboardIpcConfig;
use crate::dashboard::error::DashboardError;
use crate::dashboard::ipc_server::{DashboardIpcService, bind_dashboard_listener};
use crate::dashboard::protocol::{IpcResponse, parse_request_line, response_to_line};
use crate::dashboard::registration::run_registration_heartbeat;
use crate::dashboard::state::declared_state_from_spec;
use crate::journal::ring::EventJournal;
use crate::spec::supervisor::SupervisorSpec;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::{JoinHandle, JoinSet};

/// Guard that owns dashboard IPC background tasks and socket cleanup.
pub struct DashboardIpcRuntimeGuard {
    /// Socket path created by this runtime.
    ipc_path: PathBuf,
    /// Target-side IPC accept task.
    ipc_task: JoinHandle<()>,
    /// Optional registration heartbeat task.
    heartbeat_task: Option<JoinHandle<()>>,
}

impl fmt::Debug for DashboardIpcRuntimeGuard {
    /// Formats guard diagnostics without exposing task internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DashboardIpcRuntimeGuard")
            .field("ipc_path", &self.ipc_path)
            .field("has_heartbeat_task", &self.heartbeat_task.is_some())
            .finish_non_exhaustive()
    }
}

impl Drop for DashboardIpcRuntimeGuard {
    /// Stops background tasks and removes the socket created by this runtime.
    fn drop(&mut self) {
        self.ipc_task.abort();
        if let Some(task) = self.heartbeat_task.as_ref() {
            task.abort();
        }
        if let Err(error) = std::fs::remove_file(&self.ipc_path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    ipc_path = %self.ipc_path.display(),
                    ?error,
                    "failed to remove dashboard IPC socket"
                );
            }
        }
    }
}

/// Starts the dashboard IPC runtime for an enabled target configuration.
///
/// # Arguments
///
/// - `config`: Validated dashboard IPC configuration.
/// - `spec`: Supervisor declaration used to build dashboard state.
/// - `handle`: Runtime control handle used by command requests.
///
/// # Returns
///
/// Returns a guard that stops runtime tasks and removes the socket on drop.
pub fn start_dashboard_ipc_runtime(
    config: ValidatedDashboardIpcConfig,
    spec: SupervisorSpec,
    handle: SupervisorHandle,
) -> Result<Arc<DashboardIpcRuntimeGuard>, DashboardError> {
    let listener = bind_dashboard_listener(&config)?;
    let ipc_path = config.path.clone();
    let target_id = config.target_id.clone();
    let service = dashboard_service(config.clone(), spec, handle);
    let ipc_task = tokio::spawn(run_accept_loop(listener, service, target_id));
    let heartbeat_task = start_heartbeat_task(config);

    Ok(Arc::new(DashboardIpcRuntimeGuard {
        ipc_path,
        ipc_task,
        heartbeat_task,
    }))
}

/// Builds the service used by all socket connections.
fn dashboard_service(
    config: ValidatedDashboardIpcConfig,
    spec: SupervisorSpec,
    handle: SupervisorHandle,
) -> Arc<DashboardIpcService> {
    let state = declared_state_from_spec(&spec);
    let journal = EventJournal::new(spec.event_channel_capacity);
    Arc::new(DashboardIpcService::new(config, spec, state, journal).with_handle(handle))
}

/// Starts the dynamic registration heartbeat when registration is enabled.
fn start_heartbeat_task(config: ValidatedDashboardIpcConfig) -> Option<JoinHandle<()>> {
    config.registration.as_ref()?;
    Some(tokio::spawn(async move {
        if let Err(error) = run_registration_heartbeat(config).await {
            tracing::warn!(?error, "dashboard registration heartbeat stopped");
        }
    }))
}

/// Accepts target-side IPC connections until the listener fails or is aborted.
async fn run_accept_loop(
    listener: UnixListener,
    service: Arc<DashboardIpcService>,
    target_id: String,
) {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _)) => {
                        let service = Arc::clone(&service);
                        let target_id = target_id.clone();
                        connections.spawn(async move {
                            handle_connection(stream, service, target_id).await
                        });
                    }
                    Err(error) => {
                        tracing::warn!(?error, "dashboard IPC accept loop stopped");
                        break;
                    }
                }
            }
            Some(joined) = connections.join_next() => {
                match joined {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        tracing::warn!(?error, "dashboard IPC connection ended with error");
                    }
                    Err(error) => {
                        tracing::warn!(?error, "dashboard IPC connection task failed");
                    }
                }
            }
        }
    }
}

/// Handles one newline-delimited JSON IPC connection.
async fn handle_connection(
    stream: UnixStream,
    service: Arc<DashboardIpcService>,
    target_id: String,
) -> Result<(), DashboardError> {
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).await.map_err(|error| {
            io_error(
                "ipc_read_failed",
                "ipc_read",
                Some(target_id.clone()),
                error,
            )
        })?;
        if bytes == 0 {
            return Ok(());
        }
        let response = response_for_line(&service, line.trim_end()).await;
        write_response(&mut reader, &response, &target_id).await?;
    }
}

/// Converts one request line into a response.
async fn response_for_line(service: &DashboardIpcService, line: &str) -> IpcResponse {
    match parse_request_line(line) {
        Ok(request) => service.handle_request(request).await,
        Err(error) => IpcResponse::error("invalid-request", error),
    }
}

/// Writes one response line to the socket.
async fn write_response(
    reader: &mut BufReader<UnixStream>,
    response: &IpcResponse,
    target_id: &str,
) -> Result<(), DashboardError> {
    let line = response_to_line(response)?;
    reader
        .get_mut()
        .write_all(line.as_bytes())
        .await
        .map_err(|error| {
            io_error(
                "ipc_write_failed",
                "ipc_write",
                Some(target_id.to_owned()),
                error,
            )
        })
}

/// Creates a structured IPC runtime I/O error.
fn io_error(
    code: &str,
    stage: &str,
    target_id: Option<String>,
    error: std::io::Error,
) -> DashboardError {
    DashboardError::new(code, stage, target_id, error.to_string(), true)
}
