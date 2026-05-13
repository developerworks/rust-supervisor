//! Starts the demo-owned dashboard IPC and registration runtime.

// Import the demo scenario state holder.
use crate::scenario::DemoScenario;
// Import validated configuration state.
use rust_supervisor::config::state::ConfigState;
// Import dashboard configuration validation.
use rust_supervisor::dashboard::config::{
    // Continue the demo expression.
    ValidatedDashboardIpcConfig,
    // Import the IPC config validator.
    validate_dashboard_ipc_config,
    // Continue the demo expression.
};
// Import dashboard errors.
use rust_supervisor::dashboard::error::DashboardError;
// Import dashboard socket binding helper.
use rust_supervisor::dashboard::ipc_server::bind_dashboard_listener;
// Import dashboard protocol contracts.
use rust_supervisor::dashboard::protocol::{
    // Continue the demo expression.
    DASHBOARD_IPC_PROTOCOL_VERSION,
    // Import the method parser.
    IpcMethod,
    // Import the request shape.
    IpcRequest,
    // Import the response shape.
    IpcResponse,
    // Import successful result shapes.
    IpcResult,
    // Continue the demo expression.
    decode_command_params,
    // Import request line parsing.
    parse_request_line,
    // Import response line serialization.
    response_to_line,
    // Continue the demo expression.
};
// Import dashboard registration helpers.
use rust_supervisor::dashboard::registration::{
    // Continue the demo expression.
    build_registration_payload,
    // Import heartbeat execution.
    run_registration_heartbeat,
    // Continue the demo expression.
};
// Import formatting support for guard diagnostics.
use std::fmt;
// Import path storage for socket cleanup.
use std::path::PathBuf;
// Import shared ownership for per-connection services.
use std::sync::Arc;
// Import asynchronous line I/O traits.
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
// Import Unix socket types.
use tokio::net::{UnixListener, UnixStream};
// Import background task handles.
use tokio::task::{JoinHandle, JoinSet};

/// Demo dashboard runtime guard.
pub(crate) struct DemoDashboardRuntimeGuard {
    /// Socket path created by the demo runtime.
    ipc_path: PathBuf,
    /// Target-side IPC accept task.
    ipc_task: JoinHandle<()>,
    /// Optional registration heartbeat task.
    heartbeat_task: Option<JoinHandle<()>>,
    /// Target process identifier.
    target_id: String,
    /// Optional relay registration path.
    registration_path: Option<PathBuf>,
    // Continue the demo expression.
}

// Continue the demo expression.
impl DemoDashboardRuntimeGuard {
    /// Returns the target process identifier.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the target identifier.
    pub(crate) fn target_id(&self) -> &str {
        // Return target identifier.
        &self.target_id
        // End target identifier access.
    }

    /// Returns the IPC socket path.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the IPC path.
    pub(crate) fn ipc_path(&self) -> &std::path::Path {
        // Return IPC path.
        &self.ipc_path
        // End IPC path access.
    }

    /// Returns the registration socket path.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the optional registration path.
    pub(crate) fn registration_path(&self) -> Option<&std::path::Path> {
        // Return optional registration path.
        self.registration_path.as_deref()
        // End registration path access.
    }
    // Continue the demo expression.
}

// Continue the demo expression.
impl fmt::Debug for DemoDashboardRuntimeGuard {
    /// Formats guard diagnostics without exposing task internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Build a concise debug representation.
        formatter
            // Name the guard type.
            .debug_struct("DemoDashboardRuntimeGuard")
            // Include IPC path.
            .field("ipc_path", &self.ipc_path)
            // Include target identifier.
            .field("target_id", &self.target_id)
            // Include registration path.
            .field("registration_path", &self.registration_path)
            // Include heartbeat task presence.
            .field("has_heartbeat_task", &self.heartbeat_task.is_some())
            // Finish without exposing private task state.
            .finish_non_exhaustive()
        // End debug formatting.
    }
    // Continue the demo expression.
}

// Continue the demo expression.
impl Drop for DemoDashboardRuntimeGuard {
    /// Stops demo background tasks and removes the socket created by this runtime.
    fn drop(&mut self) {
        // Abort the IPC accept task.
        self.ipc_task.abort();
        // Abort the heartbeat task when present.
        if let Some(task) = self.heartbeat_task.as_ref() {
            // Abort registration heartbeat.
            task.abort();
            // End heartbeat branch.
        }
        // Remove the socket file owned by this process.
        if let Err(error) = std::fs::remove_file(&self.ipc_path) {
            // Ignore already-removed sockets.
            if error.kind() != std::io::ErrorKind::NotFound {
                // Print cleanup failure for the demo operator.
                eprintln!(
                    // Continue the demo expression.
                    "failed to remove demo IPC socket {}: {error}",
                    // Continue the demo expression.
                    self.ipc_path.display() // Continue the demo expression.
                                            // Finish cleanup warning output.
                );
                // End cleanup warning branch.
            }
            // End remove error branch.
        }
        // End runtime guard cleanup.
    }
    // Continue the demo expression.
}

/// Starts the demo dashboard runtime when IPC is enabled.
///
/// # Arguments
///
/// - `state`: Loaded supervisor configuration state.
///
/// # Returns
///
/// Returns a runtime guard when IPC is enabled.
pub(crate) fn start_demo_dashboard_runtime(
    // Continue the demo expression.
    state: &ConfigState,
    // Continue the demo expression.
) -> Result<Option<DemoDashboardRuntimeGuard>, Box<dyn std::error::Error + Send + Sync>> {
    // Validate the configured dashboard IPC section.
    let Some(config) = validate_dashboard_ipc_config(state.ipc.as_ref())? else {
        // Return no runtime when IPC is disabled.
        return Ok(None);
        // End disabled IPC branch.
    };
    // Bind the configured demo IPC socket.
    let listener = bind_dashboard_listener(&config)?;
    // Clone the IPC path for cleanup.
    let ipc_path = config.path.clone();
    // Clone the target identifier for summaries.
    let target_id = config.target_id.clone();
    // Clone the optional registration path for summaries.
    let registration_path = config
        // Borrow the optional registration config.
        .registration
        // Read the optional registration config.
        .as_ref()
        // Clone the configured relay path.
        .map(|registration| registration.relay_registration_path.clone());
    // Build the demo service.
    let service = Arc::new(DemoIpcService::new(config.clone()));
    // Start the IPC accept loop.
    let ipc_task = tokio::spawn(run_accept_loop(
        // Continue the demo expression.
        listener,
        // Continue the demo expression.
        Arc::clone(&service),
        // Continue the demo expression.
        target_id.clone(),
        // Continue the demo expression.
    ));
    // Start registration heartbeat when configured.
    let heartbeat_task = start_heartbeat_task(config);
    // Return the demo runtime guard.
    Ok(Some(DemoDashboardRuntimeGuard {
        // Store IPC path for cleanup.
        ipc_path,
        // Store IPC task.
        ipc_task,
        // Store optional heartbeat task.
        heartbeat_task,
        // Store target identifier.
        target_id,
        // Store optional registration path.
        registration_path,
        // End guard construction.
    }))
    // End runtime startup.
}

/// Starts the dynamic registration heartbeat when registration is enabled.
///
/// # Arguments
///
/// - `config`: Validated IPC configuration.
///
/// # Returns
///
/// Returns an optional task handle.
fn start_heartbeat_task(config: ValidatedDashboardIpcConfig) -> Option<JoinHandle<()>> {
    // Skip heartbeat when registration is absent.
    config.registration.as_ref()?;
    // Spawn the heartbeat loop.
    Some(tokio::spawn(async move {
        // Run registration heartbeat until it stops.
        if let Err(error) = run_registration_heartbeat(config).await {
            // Print non-retryable registration failure.
            eprintln!("demo registration heartbeat stopped: {error}");
            // End heartbeat error branch.
        }
        // End heartbeat task.
    }))
    // End heartbeat task startup.
}

/// Demo IPC request dispatcher.
struct DemoIpcService {
    /// Validated IPC configuration.
    config: ValidatedDashboardIpcConfig,
    /// Mutable dashboard scenario.
    scenario: DemoScenario,
    // Continue the demo expression.
}

// Continue the demo expression.
impl DemoIpcService {
    /// Creates the demo IPC service.
    ///
    /// # Arguments
    ///
    /// - `config`: Validated IPC configuration.
    ///
    /// # Returns
    ///
    /// Returns a demo service.
    fn new(config: ValidatedDashboardIpcConfig) -> Self {
        // Resolve the display name from registration config.
        let display_name = config
            // Borrow optional registration config.
            .registration
            // Read optional registration config.
            .as_ref()
            // Clone display name when present.
            .map(|registration| registration.display_name.clone())
            // Fall back to target identifier.
            .unwrap_or_else(|| config.target_id.clone());
        // Create the demo service.
        Self {
            // Store validated config.
            config: config.clone(),
            // Store mutable scenario.
            scenario: DemoScenario::new(config.target_id.clone(), display_name),
            // End service construction.
        }
        // End service construction.
    }

    /// Handles one parsed IPC request.
    ///
    /// # Arguments
    ///
    /// - `request`: Parsed IPC request.
    ///
    /// # Returns
    ///
    /// Returns an IPC response.
    async fn handle_request(&self, request: IpcRequest) -> IpcResponse {
        // Dispatch the request.
        match self.dispatch(&request).await {
            // Return success response.
            Ok(result) => IpcResponse::ok(request.request_id, result),
            // Return error response.
            Err(error) => IpcResponse::error(request.request_id, error),
            // End dispatch match.
        }
        // End request handling.
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
        // Parse the request method.
        let method = IpcMethod::parse(&request.method)?;
        // Dispatch by method.
        match method {
            // Return protocol hello response.
            IpcMethod::Hello => Ok(IpcResult::Hello {
                // Include protocol version.
                protocol_version: DASHBOARD_IPC_PROTOCOL_VERSION.to_owned(),
                // Include registration payload.
                registration: build_registration_payload(&self.config)?,
                // End hello payload.
            }),
            // Return the current demo state.
            IpcMethod::CurrentState => {
                // Build current scenario state.
                let state = self.scenario.state();
                // Return state payload.
                Ok(IpcResult::State {
                    // Include target identifier.
                    target_id: state.target.target_id.clone(),
                    // Include boxed dashboard state.
                    state: Box::new(state),
                    // End state payload.
                })
                // End current state branch.
            }
            // Accept event subscription.
            IpcMethod::EventsSubscribe => Ok(self.subscription("events")),
            // Accept log subscription.
            IpcMethod::LogsTail => Ok(self.subscription("logs")),
            // Dispatch control command methods.
            IpcMethod::CommandRestartChild
            // Continue the demo expression.
            | IpcMethod::CommandPauseChild
            // Continue the demo expression.
            | IpcMethod::CommandResumeChild
            // Continue the demo expression.
            | IpcMethod::CommandQuarantineChild
            // Continue the demo expression.
            | IpcMethod::CommandRemoveChild
            // Continue the demo expression.
            | IpcMethod::CommandAddChild
            // Continue the demo expression.
            | IpcMethod::CommandShutdownTree => self.command_result(request),
            // End method match.
        }
        // End dispatch.
    }

    /// Builds one subscription response.
    ///
    /// # Arguments
    ///
    /// - `subscription`: Subscription kind.
    ///
    /// # Returns
    ///
    /// Returns a subscription result.
    fn subscription(&self, subscription: &str) -> IpcResult {
        // Build the subscription payload.
        IpcResult::Subscription {
            // Include target identifier.
            target_id: self.scenario.target_id().to_owned(),
            // Include subscription kind.
            subscription: subscription.to_owned(),
            // End subscription payload.
        }
        // End subscription construction.
    }

    /// Handles one command request.
    ///
    /// # Arguments
    ///
    /// - `request`: IPC request.
    ///
    /// # Returns
    ///
    /// Returns a command result IPC payload.
    fn command_result(&self, request: &IpcRequest) -> Result<IpcResult, DashboardError> {
        // Decode command parameters.
        let command = decode_command_params(request)?;
        // Apply the command to the scenario.
        let result = self.scenario.command_result(command)?;
        // Return command result payload.
        Ok(IpcResult::CommandResult {
            // Include target identifier.
            target_id: self.scenario.target_id().to_owned(),
            // Include command result.
            result,
            // End command result payload.
        })
        // End command result handling.
    }
    // Continue the demo expression.
}

/// Accepts demo IPC connections until the task is aborted.
///
/// # Arguments
///
/// - `listener`: Bound Unix listener.
/// - `service`: Shared demo service.
/// - `target_id`: Target process identifier.
///
/// # Returns
///
/// This async task has no returned value.
async fn run_accept_loop(listener: UnixListener, service: Arc<DemoIpcService>, target_id: String) {
    // Track connection tasks.
    let mut connections = JoinSet::new();
    // Accept connections until listener failure.
    loop {
        // Wait for either a new connection or a completed task.
        tokio::select! {
            // Accept one socket connection.
            accepted = listener.accept() => {
                // Handle accept result.
                match accepted {
                    // Spawn a connection task.
                    Ok((stream, _)) => {
                        // Clone the shared service.
                        let service = Arc::clone(&service);
                        // Clone the target identifier.
                        let target_id = target_id.clone();
                        // Spawn the per-connection task.
                        connections.spawn(async move {
                            // Handle the socket connection.
                            handle_connection(stream, service, target_id).await
                            // End connection task.
                        });
                    // Continue the demo expression.
                    }
                    // Stop when accept fails.
                    Err(error) => {
                        // Print accept failure.
                        eprintln!("demo IPC accept loop stopped: {error}");
                        // Leave the accept loop.
                        break;
                    // Continue the demo expression.
                    }
                    // End accept match.
                }
            // Continue the demo expression.
            }
            // Collect completed connection tasks.
            Some(joined) = connections.join_next() => {
                // Report task failures.
                if let Err(error) = joined {
                    // Print task failure.
                    eprintln!("demo IPC connection task failed: {error}");
                    // End task error branch.
                }
            // Continue the demo expression.
            }
        // Continue the demo expression.
        }
        // Continue accept loop.
    }
    // End accept loop.
}

/// Handles one newline-delimited JSON IPC connection.
///
/// # Arguments
///
/// - `stream`: Accepted Unix socket.
/// - `service`: Shared demo service.
/// - `target_id`: Target process identifier.
///
/// # Returns
///
/// Returns success when the socket closes cleanly.
async fn handle_connection(
    // Continue the demo expression.
    stream: UnixStream,
    // Continue the demo expression.
    service: Arc<DemoIpcService>,
    // Continue the demo expression.
    target_id: String,
    // Continue the demo expression.
) -> Result<(), DashboardError> {
    // Wrap the stream in a line reader.
    let mut reader = BufReader::new(stream);
    // Read requests until EOF.
    loop {
        // Allocate the request line.
        let mut line = String::new();
        // Read one newline-delimited request.
        let bytes = reader.read_line(&mut line).await.map_err(|error| {
            // Build read error.
            io_error(
                // Continue the demo expression.
                "ipc_read_failed",
                // Continue the demo expression.
                "ipc_read",
                // Continue the demo expression.
                Some(target_id.clone()),
                // Continue the demo expression.
                error,
                // Continue the demo expression.
            )
            // End read error construction.
        })?;
        // Stop when the peer closes the socket.
        if bytes == 0 {
            // Return clean close.
            return Ok(());
            // End EOF branch.
        }
        // Convert the request line into a response.
        let response = response_for_line(&service, line.trim_end()).await;
        // Write the response to the socket.
        write_response(&mut reader, &response, &target_id).await?;
        // Continue reading requests.
    }
    // End connection handling.
}

/// Converts one request line into a response.
///
/// # Arguments
///
/// - `service`: Demo IPC service.
/// - `line`: One request line.
///
/// # Returns
///
/// Returns an IPC response.
async fn response_for_line(service: &DemoIpcService, line: &str) -> IpcResponse {
    // Parse the line.
    match parse_request_line(line) {
        // Dispatch parsed requests.
        Ok(request) => service.handle_request(request).await,
        // Return protocol errors.
        Err(error) => IpcResponse::error("invalid-request", error),
        // End parse match.
    }
    // End response conversion.
}

/// Writes one response line to the socket.
///
/// # Arguments
///
/// - `reader`: Socket reader wrapper.
/// - `response`: IPC response.
/// - `target_id`: Target process identifier.
///
/// # Returns
///
/// Returns success after the response is written.
async fn write_response(
    // Continue the demo expression.
    reader: &mut BufReader<UnixStream>,
    // Continue the demo expression.
    response: &IpcResponse,
    // Continue the demo expression.
    target_id: &str,
    // Continue the demo expression.
) -> Result<(), DashboardError> {
    // Serialize the response as one line.
    let line = response_to_line(response)?;
    // Write the response line.
    reader
        // Access the underlying stream.
        .get_mut()
        // Write bytes to the peer.
        .write_all(line.as_bytes())
        // Await completion.
        .await
        // Convert I/O failure into dashboard error.
        .map_err(|error| {
            // Continue the demo expression.
            io_error(
                // Continue the demo expression.
                "ipc_write_failed",
                // Continue the demo expression.
                "ipc_write",
                // Continue the demo expression.
                Some(target_id.to_owned()),
                // Continue the demo expression.
                error,
                // Continue the demo expression.
            )
            // Continue the demo expression.
        })
    // End response write.
}

/// Creates a structured IPC runtime I/O error.
///
/// # Arguments
///
/// - `code`: Error code.
/// - `stage`: Error stage.
/// - `target_id`: Optional target process identifier.
/// - `error`: Source I/O error.
///
/// # Returns
///
/// Returns a dashboard error.
fn io_error(
    // Continue the demo expression.
    code: &str,
    // Continue the demo expression.
    stage: &str,
    // Continue the demo expression.
    target_id: Option<String>,
    // Continue the demo expression.
    error: std::io::Error,
    // Continue the demo expression.
) -> DashboardError {
    // Create a retryable I/O error.
    DashboardError::new(code, stage, target_id, error.to_string(), true)
    // End I/O error construction.
}
