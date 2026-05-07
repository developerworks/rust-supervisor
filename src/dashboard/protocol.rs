//! Target-side dashboard IPC protocol.
//!
//! The relay and target process exchange newline-delimited JSON objects. This
//! module keeps the accepted methods explicit and rejects legacy aliases.

use crate::dashboard::error::DashboardError;
use crate::dashboard::model::{
    ControlCommandRequest, ControlCommandResult, DashboardState, EventRecord, LogRecord,
    TargetProcessRegistration,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Wire protocol version used by the dashboard IPC contract.
pub const DASHBOARD_IPC_PROTOCOL_VERSION: &str = "dashboard-ipc.v1";

/// IPC request accepted by the target process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcRequest {
    /// Caller-provided request identifier.
    pub request_id: String,
    /// Method name as it appeared on the wire.
    pub method: String,
    /// Method parameters.
    #[serde(default)]
    pub params: Value,
}

/// Typed IPC method accepted by the target process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcMethod {
    /// Protocol handshake.
    Hello,
    /// Full dashboard state request.
    CurrentState,
    /// Event subscription request.
    EventsSubscribe,
    /// Log tail subscription request.
    LogsTail,
    /// Restart child command.
    CommandRestartChild,
    /// Pause child command.
    CommandPauseChild,
    /// Resume child command.
    CommandResumeChild,
    /// Quarantine child command.
    CommandQuarantineChild,
    /// Remove child command.
    CommandRemoveChild,
    /// Add child command.
    CommandAddChild,
    /// Shutdown tree command.
    CommandShutdownTree,
}

impl IpcMethod {
    /// Parses a wire method and rejects unsupported aliases.
    ///
    /// # Arguments
    ///
    /// - `method`: Method name from the request.
    ///
    /// # Returns
    ///
    /// Returns a typed method or an unsupported-method error.
    pub fn parse(method: &str) -> Result<Self, DashboardError> {
        match method {
            "hello" => Ok(Self::Hello),
            "snapshot" => Ok(Self::CurrentState),
            "events.subscribe" => Ok(Self::EventsSubscribe),
            "logs.tail" => Ok(Self::LogsTail),
            "command.restart_child" => Ok(Self::CommandRestartChild),
            "command.pause_child" => Ok(Self::CommandPauseChild),
            "command.resume_child" => Ok(Self::CommandResumeChild),
            "command.quarantine_child" => Ok(Self::CommandQuarantineChild),
            "command.remove_child" => Ok(Self::CommandRemoveChild),
            "command.add_child" => Ok(Self::CommandAddChild),
            "command.shutdown_tree" => Ok(Self::CommandShutdownTree),
            _ => Err(DashboardError::unsupported_method(method)),
        }
    }

    /// Returns the canonical wire method name.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the canonical method name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hello => "hello",
            Self::CurrentState => "snapshot",
            Self::EventsSubscribe => "events.subscribe",
            Self::LogsTail => "logs.tail",
            Self::CommandRestartChild => "command.restart_child",
            Self::CommandPauseChild => "command.pause_child",
            Self::CommandResumeChild => "command.resume_child",
            Self::CommandQuarantineChild => "command.quarantine_child",
            Self::CommandRemoveChild => "command.remove_child",
            Self::CommandAddChild => "command.add_child",
            Self::CommandShutdownTree => "command.shutdown_tree",
        }
    }
}

/// Successful IPC result payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcResult {
    /// Handshake result.
    Hello {
        /// Protocol version.
        protocol_version: String,
        /// Registration payload advertised by the target.
        registration: TargetProcessRegistration,
    },
    /// Full target dashboard state.
    #[serde(rename = "snapshot")]
    State {
        /// Target process identifier.
        target_id: String,
        /// Dashboard state payload.
        #[serde(rename = "snapshot")]
        state: Box<DashboardState>,
    },
    /// Subscription acceptance.
    Subscription {
        /// Target process identifier.
        target_id: String,
        /// Subscription kind.
        subscription: String,
    },
    /// Control command result.
    CommandResult {
        /// Target process identifier.
        target_id: String,
        /// Command result.
        result: ControlCommandResult,
    },
}

/// IPC response sent by the target process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcResponse {
    /// Request identifier copied from the request.
    pub request_id: String,
    /// Whether the request succeeded.
    pub ok: bool,
    /// Optional successful result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<IpcResult>,
    /// Optional structured error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DashboardError>,
}

impl IpcResponse {
    /// Creates a successful IPC response.
    ///
    /// # Arguments
    ///
    /// - `request_id`: Request identifier copied from the request.
    /// - `result`: Successful result payload.
    ///
    /// # Returns
    ///
    /// Returns an [`IpcResponse`] with `ok=true`.
    pub fn ok(request_id: impl Into<String>, result: IpcResult) -> Self {
        Self {
            request_id: request_id.into(),
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    /// Creates an error IPC response.
    ///
    /// # Arguments
    ///
    /// - `request_id`: Request identifier copied from the request.
    /// - `error`: Structured error payload.
    ///
    /// # Returns
    ///
    /// Returns an [`IpcResponse`] with `ok=false`.
    pub fn error(request_id: impl Into<String>, error: DashboardError) -> Self {
        Self {
            request_id: request_id.into(),
            ok: false,
            result: None,
            error: Some(error),
        }
    }
}

/// Server push message sent after a subscription is established.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcServerPush {
    /// Event push.
    Event {
        /// Target process identifier.
        target_id: String,
        /// Event record.
        event: EventRecord,
    },
    /// Log push.
    Log {
        /// Target process identifier.
        target_id: String,
        /// Log record.
        log: LogRecord,
    },
    /// State delta push.
    StateDelta {
        /// Target process identifier.
        target_id: String,
        /// State delta payload.
        delta: Value,
    },
    /// Error push.
    Error {
        /// Structured error.
        error: DashboardError,
    },
}

/// Parses one newline-delimited JSON request line.
///
/// # Arguments
///
/// - `line`: One full JSON object line.
///
/// # Returns
///
/// Returns a typed request or a structured parser error.
pub fn parse_request_line(line: &str) -> Result<IpcRequest, DashboardError> {
    let request: IpcRequest = serde_json::from_str(line).map_err(|error| {
        DashboardError::new(
            "invalid_json",
            "protocol_parse",
            None,
            format!("failed to parse IPC JSON request: {error}"),
            false,
        )
    })?;
    IpcMethod::parse(&request.method)?;
    Ok(request)
}

/// Serializes a response as one newline-delimited JSON line.
///
/// # Arguments
///
/// - `response`: Response that should be serialized.
///
/// # Returns
///
/// Returns one JSON line ending with `\n`.
pub fn response_to_line(response: &IpcResponse) -> Result<String, DashboardError> {
    let mut line = serde_json::to_string(response).map_err(|error| {
        DashboardError::new(
            "serialization_failed",
            "protocol_write",
            response
                .error
                .as_ref()
                .and_then(|error| error.target_id.clone()),
            format!("failed to serialize IPC response: {error}"),
            false,
        )
    })?;
    line.push('\n');
    Ok(line)
}

/// Decodes command parameters from an IPC request.
///
/// # Arguments
///
/// - `request`: Request carrying command parameters.
///
/// # Returns
///
/// Returns a typed command request.
pub fn decode_command_params(
    request: &IpcRequest,
) -> Result<ControlCommandRequest, DashboardError> {
    serde_json::from_value(request.params.clone()).map_err(|error| {
        DashboardError::new(
            "invalid_command_params",
            "protocol_parse",
            None,
            format!("failed to parse command params: {error}"),
            false,
        )
    })
}
