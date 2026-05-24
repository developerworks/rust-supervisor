//! Dashboard IPC runtime lifecycle.
//!
//! The runtime owns the target-side Unix socket accept loop and the dynamic
//! registration heartbeat used by the relay integration.

use crate::config::audit::AuditConfig;
use crate::control::handle::SupervisorHandle;
use crate::dashboard::config::ValidatedDashboardIpcConfig;
use crate::dashboard::error::DashboardError;
use crate::dashboard::ipc_server::{DashboardIpcService, bind_dashboard_listener};
use crate::dashboard::protocol::{IpcResponse, parse_request_line, response_to_line};
use crate::dashboard::registration::run_registration_heartbeat;
use crate::dashboard::state::declared_state_from_spec;
use crate::ipc::security::IpcSecurityPipeline;
use crate::ipc::security::peer_identity::{PeerIdentity, extract_peer_identity};
use crate::journal::ring::EventJournal;
use crate::spec::supervisor::SupervisorSpec;
use std::fmt;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::{JoinHandle, JoinSet};

/// Default maximum frame size for bounded frame reader: 1 MiB.
pub(crate) const DEFAULT_MAX_FRAME_BYTES: usize = 1_048_576;

/// Maximum concurrent IPC connections (C10 resource boundary).
const MAX_CONCURRENT_CONNECTIONS: usize = 64;

/// Per-connection idle timeout: drop connections that send no complete
/// frame within this duration (C10 resource boundary).
const CONNECTION_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Per-process connection counter for unique connection_id generation.
static CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(0);

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
        if let Err(error) = std::fs::remove_file(&self.ipc_path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(
                ipc_path = %self.ipc_path.display(),
                ?error,
                "failed to remove dashboard IPC socket"
            );
        }
    }
}

/// Starts the dashboard IPC runtime for an enabled target configuration.
///
/// # Arguments
///
/// - `config`: Validated dashboard IPC configuration.
/// - `audit_config`: Root audit persistence configuration.
/// - `spec`: Supervisor declaration used to build dashboard state.
/// - `handle`: Runtime control handle used by command requests.
///
/// # Returns
///
/// Returns a guard that stops runtime tasks and removes the socket on drop.
pub fn start_dashboard_ipc_runtime(
    config: ValidatedDashboardIpcConfig,
    audit_config: AuditConfig,
    spec: SupervisorSpec,
    handle: SupervisorHandle,
) -> Result<Arc<DashboardIpcRuntimeGuard>, DashboardError> {
    let listener = bind_dashboard_listener(&config)?;
    let ipc_path = config.path.clone();
    let target_id = config.target_id.clone();
    let service = dashboard_service(config.clone(), audit_config, spec, handle);
    let ipc_task = tokio::spawn(run_accept_loop(listener, service, target_id));
    let heartbeat_task = start_heartbeat_task(config);

    Ok(Arc::new(DashboardIpcRuntimeGuard {
        ipc_path,
        ipc_task,
        heartbeat_task,
    }))
}

/// Builds the service used by all socket connections.
///
/// When `config.security_config` is present, an IPC security pipeline is
/// constructed with the root audit config and wired into the service via
/// `with_security_pipeline`.
fn dashboard_service(
    config: ValidatedDashboardIpcConfig,
    audit_config: AuditConfig,
    spec: SupervisorSpec,
    handle: SupervisorHandle,
) -> Arc<DashboardIpcService> {
    let state = declared_state_from_spec(&spec);
    let journal = EventJournal::new(spec.event_channel_capacity);
    let mut service =
        DashboardIpcService::new(config.clone(), spec, state, journal).with_handle(handle);
    if let Some(security_config) = config.security_config {
        let pipeline = IpcSecurityPipeline::new(security_config, audit_config);
        service = service.with_security_pipeline(pipeline);
    }
    Arc::new(service)
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
///
/// Enforces a maximum concurrent connection limit (`MAX_CONCURRENT_CONNECTIONS`)
/// to prevent resource exhaustion. When the limit is reached, new connections
/// are accepted but immediately dropped with a log warning.
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
                        if connections.len() >= MAX_CONCURRENT_CONNECTIONS {
                            tracing::warn!(
                                target: "rust_supervisor::dashboard::ipc",
                                "dashboard IPC connection limit reached ({MAX_CONCURRENT_CONNECTIONS}), dropping new connection"
                            );
                            drop(stream);
                            continue;
                        }
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

/// Handles one IPC connection with bounded frame reading, real peer
/// credential extraction, per-connection unique identifier,
/// and idle timeout (C10 resource boundary).
async fn handle_connection(
    stream: UnixStream,
    service: Arc<DashboardIpcService>,
    target_id: String,
) -> Result<(), DashboardError> {
    // ---- extract real peer credential before wrapping into tokio ----
    let std_stream = stream.into_std().map_err(|error| {
        io_error(
            "ipc_into_std_failed",
            "ipc_connect",
            Some(target_id.clone()),
            error,
        )
    })?;
    let peer = extract_peer_identity(&std_stream)?;
    let raw_fd = std_stream.as_raw_fd();
    let connection_id = format!(
        "conn-{raw_fd}-{}",
        CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let stream = UnixStream::from_std(std_stream).map_err(|error| {
        io_error(
            "ipc_from_std_failed",
            "ipc_connect",
            Some(target_id.clone()),
            error,
        )
    })?;

    // Use C5 request size limit when security pipeline is configured,
    // otherwise fall back to 1 MiB default.
    let max_frame_bytes = service.max_frame_bytes();
    let mut reader = BoundedFrameReader::new(stream, max_frame_bytes);
    loop {
        // Enforce per-connection idle timeout: if no complete frame
        // arrives within CONNECTION_IDLE_TIMEOUT, drop the connection.
        let frame = tokio::time::timeout(CONNECTION_IDLE_TIMEOUT, reader.read_frame()).await;
        match frame {
            Ok(Ok(Some(raw_frame))) => {
                let raw_body_len = raw_frame.len();
                let response =
                    response_for_line(&service, &raw_frame, &peer, &connection_id, raw_body_len)
                        .await;
                write_response(&mut reader, &response, &target_id).await?;
            }
            Ok(Ok(None)) => {
                // EOF — peer closed connection gracefully
                return Ok(());
            }
            Ok(Err(error)) => {
                return Err(error);
            }
            Err(_elapsed) => {
                tracing::warn!(
                    target: "rust_supervisor::dashboard::ipc",
                    %connection_id,
                    "dashboard IPC connection idle timeout after {}s",
                    CONNECTION_IDLE_TIMEOUT.as_secs(),
                );
                return Err(DashboardError::new(
                    "ipc_idle_timeout",
                    "ipc_read",
                    Some(target_id),
                    "connection idle timeout".to_owned(),
                    false,
                ));
            }
        }
    }
}

/// Bounded frame reader that limits each frame to `max_bytes` before
/// allocating the target buffer.
struct BoundedFrameReader {
    /// Inner tokio stream.
    stream: UnixStream,
    /// Maximum frame size in bytes.
    max_bytes: usize,
    /// Read buffer reused across frames.
    buf: Vec<u8>,
}

impl BoundedFrameReader {
    /// Creates a new bounded frame reader.
    fn new(stream: UnixStream, max_bytes: usize) -> Self {
        Self {
            stream,
            max_bytes,
            buf: Vec::with_capacity(max_bytes.min(4096)),
        }
    }

    /// Reads one newline-delimited frame.
    ///
    /// Returns `Ok(Some(frame))` for a complete frame, `Ok(None)` for EOF
    /// before any data, or `Err` when the frame exceeds `max_bytes` or a
    /// read error occurs.
    async fn read_frame(&mut self) -> Result<Option<String>, DashboardError> {
        self.buf.clear();
        loop {
            let mut byte = [0u8; 1];
            match self.stream.read_exact(&mut byte).await {
                Ok(_bytes_read) => {
                    if byte[0] == b'\n' {
                        let frame = String::from_utf8(self.buf.clone()).map_err(|_| {
                            DashboardError::new(
                                "invalid_utf8",
                                "ipc_read",
                                None,
                                "frame is not valid UTF-8".to_owned(),
                                false,
                            )
                        })?;
                        return Ok(Some(frame));
                    }
                    self.buf.push(byte[0]);
                    if self.buf.len() > self.max_bytes {
                        return Err(DashboardError::new(
                            "frame_too_large",
                            "ipc_read",
                            None,
                            format!("frame exceeded maximum size of {max} bytes", max = self.max_bytes),
                            false,
                        ));
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    if self.buf.is_empty() {
                        return Ok(None);
                    }
                    return Err(DashboardError::new(
                        "incomplete_frame",
                        "ipc_read",
                        None,
                        "connection closed before newline delimiter".to_owned(),
                        false,
                    ));
                }
                Err(err) => {
                    return Err(io_error("ipc_read_failed", "ipc_read", None, err));
                }
            }
        }
    }

    /// Returns a mutable reference to the inner stream for writing.
    fn stream_mut(&mut self) -> &mut UnixStream {
        &mut self.stream
    }
}

impl std::os::unix::io::AsRawFd for BoundedFrameReader {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.stream.as_raw_fd()
    }
}

/// Converts one request line into a response, passing connection context.
async fn response_for_line(
    service: &DashboardIpcService,
    line: &str,
    peer: &PeerIdentity,
    connection_id: &str,
    raw_body_len: usize,
) -> IpcResponse {
    match parse_request_line(line) {
        Ok(request) => {
            service
                .handle_request(request, peer, connection_id, raw_body_len)
                .await
        }
        Err(error) => IpcResponse::error("invalid-request", error),
    }
}

/// Writes one response line to the socket.
async fn write_response(
    reader: &mut BoundedFrameReader,
    response: &IpcResponse,
    target_id: &str,
) -> Result<(), DashboardError> {
    let line = response_to_line(response)?;
    reader
        .stream_mut()
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
