//! Shared dashboard data model.
//!
//! These structs are the JSON contract shared by target IPC, relay, and the
//! dashboard UI. They intentionally use owned values so callers can serialize,
//! clone, and test messages without borrowing runtime internals.

use crate::control::command::{CommandResult, CurrentState, ManagedChildState};
use crate::control::outcome::{
    ChildAttemptStatus, ChildControlFailure, ChildControlFailurePhase, ChildControlOperation,
    ChildControlResult as RuntimeChildControlResult, ChildLivenessState, ChildRuntimeRecord,
    ChildStopState, RestartLimitState,
};
use crate::readiness::signal::ReadinessState;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Supported command metadata sent to the relay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SupportedCommand {
    /// Wire command name.
    pub name: String,
    /// Whether the command can be retried with the same command identifier.
    pub idempotent: bool,
    /// Command timeout in seconds.
    pub timeout_seconds: u64,
}

/// Target process registration payload sent to the relay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TargetProcessRegistration {
    /// Stable target process identifier.
    pub target_id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Local Unix domain socket path exposed by the target.
    pub ipc_path: String,
    /// Lease duration in seconds.
    pub lease_seconds: u64,
    /// Commands supported by this target.
    pub supported_commands: Vec<SupportedCommand>,
}

/// Current registration state for a target process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationState {
    /// Registration was accepted and is visible.
    Active,
    /// Registration was rejected.
    Rejected,
    /// Registration lease expired.
    Expired,
}

/// Current relay connection state for a target process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TargetConnectionState {
    /// Target is registered but no session has bound it.
    Registered,
    /// Relay is connecting to target IPC.
    Connecting,
    /// Relay is connected to target IPC.
    Connected,
    /// Relay is reconnecting to target IPC.
    Reconnecting,
    /// Target IPC is unavailable.
    Unavailable,
    /// Registration lease expired.
    Expired,
}

/// Target identity shown in dashboard state payloads and target lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TargetProcessIdentity {
    /// Stable target process identifier.
    pub target_id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Current registration state.
    pub registration_state: RegistrationState,
    /// Current relay connection state.
    pub connection_state: TargetConnectionState,
}

/// Complete dashboard state returned when a target is opened or reconnected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DashboardState {
    /// Target process identity.
    pub target: TargetProcessIdentity,
    /// Supervisor topology.
    pub topology: SupervisorTopology,
    /// Runtime state rows indexed by child path.
    pub runtime_state: Vec<RuntimeState>,
    /// Runtime records returned by the control loop current state.
    pub child_runtime_records: Vec<DashboardChildRuntimeRecord>,
    /// Recent events retained by the target.
    pub recent_events: Vec<EventRecord>,
    /// Recent logs retained by the target.
    pub recent_logs: Vec<LogRecord>,
    /// Number of dropped events.
    pub dropped_event_count: u64,
    /// Number of dropped logs.
    pub dropped_log_count: u64,
    /// Configuration version string.
    pub config_version: String,
    /// Generated time as Unix nanoseconds.
    pub generated_at_unix_nanos: u128,
    /// Monotonic state generation for this target.
    pub state_generation: u64,
}

/// Supervisor graph for dashboard rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SupervisorTopology {
    /// Root supervisor node.
    pub root: SupervisorNode,
    /// All visible nodes including the root.
    pub nodes: Vec<SupervisorNode>,
    /// Parent-child and dependency edges.
    pub edges: Vec<SupervisorEdge>,
    /// Node paths in declaration order.
    pub declaration_order: Vec<String>,
}

/// Node kind visible in the topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorNodeKind {
    /// Root supervisor node.
    RootSupervisor,
    /// Child task node.
    ChildTask,
}

/// Criticality shown by dashboard nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardCriticality {
    /// Critical child.
    Critical,
    /// Standard child.
    Standard,
    /// Best-effort child.
    BestEffort,
}

/// Node displayed in the supervisor topology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SupervisorNode {
    /// Stable node identifier.
    pub node_id: String,
    /// Optional child identifier.
    pub child_id: Option<String>,
    /// Absolute child path.
    pub path: String,
    /// Human-readable node name.
    pub name: String,
    /// Node kind.
    pub kind: SupervisorNodeKind,
    /// Low-cardinality tags.
    pub tags: Vec<String>,
    /// Node criticality.
    pub criticality: DashboardCriticality,
    /// Current state summary.
    pub state_summary: String,
    /// Key diagnostic fields.
    pub diagnostics: BTreeMap<String, String>,
}

/// Edge kind visible in the topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorEdgeKind {
    /// Parent-child edge.
    ParentChild,
    /// Dependency edge.
    Dependency,
}

/// Edge displayed in the supervisor topology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SupervisorEdge {
    /// Stable edge identifier.
    pub edge_id: String,
    /// Source node path.
    pub source_path: String,
    /// Target node path.
    pub target_path: String,
    /// Edge kind.
    pub kind: SupervisorEdgeKind,
    /// Declaration or dependency order.
    pub order: usize,
}

/// Runtime state shown for one child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeState {
    /// Child path.
    pub child_path: String,
    /// Lifecycle state label.
    pub lifecycle_state: String,
    /// Health status label.
    pub health: String,
    /// Readiness status label.
    pub readiness: String,
    /// Child generation.
    pub generation: u64,
    /// Child child_start_count.
    pub child_start_count: u64,
    /// Restart count.
    pub restart_count: u64,
    /// Optional last failure summary.
    pub last_failure: Option<String>,
    /// Optional last policy decision summary.
    pub last_policy_decision: Option<String>,
    /// Supervisor shutdown state label.
    pub shutdown_state: String,
}

/// Managed child state derived for dashboard display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardManagedChildState {
    /// Child is active and should be displayed as running.
    Running,
    /// Child is paused.
    Paused,
    /// Child is quarantined.
    Quarantined,
    /// Child is removed.
    Removed,
}

impl From<ManagedChildState> for DashboardManagedChildState {
    /// Converts a runtime managed state into a dashboard managed state.
    fn from(value: ManagedChildState) -> Self {
        match value {
            ManagedChildState::Running => Self::Running,
            ManagedChildState::Paused => Self::Paused,
            ManagedChildState::Quarantined => Self::Quarantined,
            ManagedChildState::Removed => Self::Removed,
        }
    }
}

impl DashboardManagedChildState {
    /// Returns the stable dashboard label.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the lifecycle label used by runtime rows.
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Quarantined => "quarantined",
            Self::Removed => "removed",
        }
    }
}

/// Child control operation label for dashboard payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardChildControlOperation {
    /// Runtime state remains active.
    Active,
    /// Runtime state is paused.
    Paused,
    /// Runtime state is quarantined.
    Quarantined,
    /// Runtime state is removed or waiting for removal.
    Removed,
}

impl From<ChildControlOperation> for DashboardChildControlOperation {
    /// Converts a runtime control operation into a dashboard operation.
    fn from(value: ChildControlOperation) -> Self {
        match value {
            ChildControlOperation::Active => Self::Active,
            ChildControlOperation::Paused => Self::Paused,
            ChildControlOperation::Quarantined => Self::Quarantined,
            ChildControlOperation::Removed => Self::Removed,
        }
    }
}

/// Attempt status label for dashboard payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardChildAttemptStatus {
    /// Child attempt is starting.
    Starting,
    /// Child attempt is running.
    Running,
    /// Child attempt is ready.
    Ready,
    /// Child attempt is cancelling.
    Cancelling,
    /// Child attempt has stopped.
    Stopped,
}

impl From<ChildAttemptStatus> for DashboardChildAttemptStatus {
    /// Converts a runtime attempt status into a dashboard attempt status.
    fn from(value: ChildAttemptStatus) -> Self {
        match value {
            ChildAttemptStatus::Starting => Self::Starting,
            ChildAttemptStatus::Running => Self::Running,
            ChildAttemptStatus::Ready => Self::Ready,
            ChildAttemptStatus::Cancelling => Self::Cancelling,
            ChildAttemptStatus::Stopped => Self::Stopped,
        }
    }
}

/// Stop state label for dashboard payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardChildStopState {
    /// No stop action is in progress.
    Idle,
    /// No active attempt exists.
    NoActiveAttempt,
    /// Cancellation was delivered.
    CancelDelivered,
    /// Stop completed.
    Completed,
    /// Stop failed.
    Failed,
}

impl From<ChildStopState> for DashboardChildStopState {
    /// Converts a runtime stop state into a dashboard stop state.
    fn from(value: ChildStopState) -> Self {
        match value {
            ChildStopState::Idle => Self::Idle,
            ChildStopState::NoActiveAttempt => Self::NoActiveAttempt,
            ChildStopState::CancelDelivered => Self::CancelDelivered,
            ChildStopState::Completed => Self::Completed,
            ChildStopState::Failed => Self::Failed,
        }
    }
}

/// Readiness state label for dashboard payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardReadinessState {
    /// Readiness has not been reported.
    Unreported,
    /// Child reported readiness.
    Ready,
    /// Child reported that it is not ready.
    NotReady,
}

impl From<ReadinessState> for DashboardReadinessState {
    /// Converts a runtime readiness state into a dashboard readiness state.
    fn from(value: ReadinessState) -> Self {
        match value {
            ReadinessState::Unreported => Self::Unreported,
            ReadinessState::Ready => Self::Ready,
            ReadinessState::NotReady => Self::NotReady,
        }
    }
}

impl DashboardReadinessState {
    /// Returns the stable dashboard label.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the readiness label used by runtime rows.
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::Unreported => "unreported",
            Self::Ready => "ready",
            Self::NotReady => "not_ready",
        }
    }
}

/// Failure phase label for dashboard payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardChildControlFailurePhase {
    /// Waiting for completion failed.
    WaitCompletion,
}

impl From<ChildControlFailurePhase> for DashboardChildControlFailurePhase {
    /// Converts a runtime failure phase into a dashboard failure phase.
    fn from(value: ChildControlFailurePhase) -> Self {
        match value {
            ChildControlFailurePhase::WaitCompletion => Self::WaitCompletion,
        }
    }
}

/// Liveness facts shown by dashboard runtime records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DashboardChildLivenessState {
    /// Last heartbeat as Unix nanoseconds.
    pub last_heartbeat_at_unix_nanos: Option<u128>,
    /// Whether the heartbeat is stale.
    pub heartbeat_stale: bool,
    /// Latest readiness state.
    pub readiness: DashboardReadinessState,
}

impl DashboardChildLivenessState {
    /// Converts runtime liveness into a dashboard liveness model.
    ///
    /// # Arguments
    ///
    /// - `value`: Runtime liveness state.
    ///
    /// # Returns
    ///
    /// Returns a dashboard liveness state.
    pub fn from_liveness(value: &ChildLivenessState) -> Self {
        Self {
            last_heartbeat_at_unix_nanos: value.last_heartbeat_at_unix_nanos,
            heartbeat_stale: value.heartbeat_stale,
            readiness: DashboardReadinessState::from(value.readiness),
        }
    }
}

/// Restart limit facts shown by dashboard runtime records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DashboardRestartLimitState {
    /// Restart accounting window in milliseconds.
    pub window_millis: u128,
    /// Restart limit inside the window.
    pub limit: u32,
    /// Restart count used so far.
    pub used: u32,
    /// Remaining restart count.
    pub remaining: u32,
    /// Whether the restart limit is exhausted.
    pub exhausted: bool,
    /// Last update timestamp in Unix nanoseconds.
    pub updated_at_unix_nanos: u128,
}

impl DashboardRestartLimitState {
    /// Converts runtime restart limit into a dashboard restart limit model.
    ///
    /// # Arguments
    ///
    /// - `value`: Runtime restart limit state.
    ///
    /// # Returns
    ///
    /// Returns a dashboard restart limit state.
    pub fn from_restart_limit(value: &RestartLimitState) -> Self {
        Self {
            window_millis: value.window.as_millis(),
            limit: value.limit,
            used: value.used,
            remaining: value.remaining,
            exhausted: value.exhausted,
            updated_at_unix_nanos: value.updated_at_unix_nanos,
        }
    }
}

/// Structured control failure shown by dashboard payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DashboardChildControlFailure {
    /// Failure phase.
    pub phase: DashboardChildControlFailurePhase,
    /// Human-readable failure reason.
    pub reason: String,
    /// Whether callers can retry.
    pub recoverable: bool,
}

impl DashboardChildControlFailure {
    /// Converts a runtime control failure into a dashboard failure model.
    ///
    /// # Arguments
    ///
    /// - `value`: Runtime control failure.
    ///
    /// # Returns
    ///
    /// Returns a dashboard control failure.
    pub fn from_failure(value: &ChildControlFailure) -> Self {
        Self {
            phase: DashboardChildControlFailurePhase::from(value.phase),
            reason: value.reason.clone(),
            recoverable: value.recoverable,
        }
    }
}

/// Dashboard projection of one child runtime record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DashboardChildRuntimeRecord {
    /// Stable child identifier.
    pub child_id: String,
    /// Child path in the supervisor tree.
    pub child_path: String,
    /// Current active generation.
    pub generation: Option<u64>,
    /// Current active attempt.
    pub attempt: Option<u64>,
    /// Current attempt status.
    pub status: Option<DashboardChildAttemptStatus>,
    /// Current control operation.
    pub operation: DashboardChildControlOperation,
    /// Managed child state derived from operation.
    pub managed_child_state: DashboardManagedChildState,
    /// Current liveness state.
    pub liveness: DashboardChildLivenessState,
    /// Current restart limit state.
    pub restart_limit: DashboardRestartLimitState,
    /// Current stop progress.
    pub stop_state: DashboardChildStopState,
    /// Most recent control failure.
    pub failure: Option<DashboardChildControlFailure>,
}

impl DashboardChildRuntimeRecord {
    /// Converts a runtime record into a dashboard runtime record.
    ///
    /// # Arguments
    ///
    /// - `record`: Runtime child record returned by current state.
    ///
    /// # Returns
    ///
    /// Returns a dashboard runtime record.
    pub fn from_runtime_record(record: &ChildRuntimeRecord) -> Self {
        Self {
            child_id: record.child_id.to_string(),
            child_path: record.path.to_string(),
            generation: record.generation.map(|generation| generation.value),
            attempt: record.attempt.map(|attempt| attempt.value),
            status: record.status.map(DashboardChildAttemptStatus::from),
            operation: DashboardChildControlOperation::from(record.operation),
            managed_child_state: DashboardManagedChildState::from(
                managed_child_state_from_operation(record.operation),
            ),
            liveness: DashboardChildLivenessState::from_liveness(&record.liveness),
            restart_limit: DashboardRestartLimitState::from_restart_limit(&record.restart_limit),
            stop_state: DashboardChildStopState::from(record.stop_state),
            failure: record
                .failure
                .as_ref()
                .map(DashboardChildControlFailure::from_failure),
        }
    }
}

/// Dashboard projection of a runtime current state result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DashboardCurrentState {
    /// Number of children known to the control loop.
    pub child_count: usize,
    /// Whether tree shutdown has completed.
    pub shutdown_completed: bool,
    /// Runtime state records for declared children.
    pub child_runtime_records: Vec<DashboardChildRuntimeRecord>,
}

impl DashboardCurrentState {
    /// Converts a runtime current state into a dashboard current state.
    ///
    /// # Arguments
    ///
    /// - `state`: Runtime current state.
    ///
    /// # Returns
    ///
    /// Returns a dashboard current state.
    pub fn from_current_state(state: &CurrentState) -> Self {
        Self {
            child_count: state.child_count,
            shutdown_completed: state.shutdown_completed,
            child_runtime_records: state
                .child_runtime_records
                .iter()
                .map(DashboardChildRuntimeRecord::from_runtime_record)
                .collect(),
        }
    }
}

/// Dashboard projection of a child control command result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DashboardChildControlResult {
    /// Stable child identifier.
    pub child_id: String,
    /// Active attempt targeted by the command.
    pub attempt: Option<u64>,
    /// Active generation targeted by the command.
    pub generation: Option<u64>,
    /// Control operation before command handling.
    pub operation_before: DashboardChildControlOperation,
    /// Control operation after command handling.
    pub operation_after: DashboardChildControlOperation,
    /// Managed child state before command handling.
    pub managed_child_state_before: DashboardManagedChildState,
    /// Managed child state after command handling.
    pub managed_child_state_after: DashboardManagedChildState,
    /// Current attempt status.
    pub status: Option<DashboardChildAttemptStatus>,
    /// Whether this command delivered cancellation.
    pub cancel_delivered: bool,
    /// Stop progress after command handling.
    pub stop_state: DashboardChildStopState,
    /// Current restart limit state.
    pub restart_limit: DashboardRestartLimitState,
    /// Current liveness state.
    pub liveness: DashboardChildLivenessState,
    /// Whether this command reused existing state idempotently.
    pub idempotent: bool,
    /// Current failure reason.
    pub failure: Option<DashboardChildControlFailure>,
}

impl DashboardChildControlResult {
    /// Converts a runtime child control result into a dashboard control result.
    ///
    /// # Arguments
    ///
    /// - `outcome`: Runtime child control result.
    ///
    /// # Returns
    ///
    /// Returns a dashboard child control result.
    pub fn from_child_control_result(outcome: &RuntimeChildControlResult) -> Self {
        Self {
            child_id: outcome.child_id.to_string(),
            attempt: outcome.attempt.map(|attempt| attempt.value),
            generation: outcome.generation.map(|generation| generation.value),
            operation_before: DashboardChildControlOperation::from(outcome.operation_before),
            operation_after: DashboardChildControlOperation::from(outcome.operation_after),
            managed_child_state_before: DashboardManagedChildState::from(
                managed_child_state_from_operation(outcome.operation_before),
            ),
            managed_child_state_after: DashboardManagedChildState::from(
                managed_child_state_from_operation(outcome.operation_after),
            ),
            status: outcome.status.map(DashboardChildAttemptStatus::from),
            cancel_delivered: outcome.cancel_delivered,
            stop_state: DashboardChildStopState::from(outcome.stop_state),
            restart_limit: DashboardRestartLimitState::from_restart_limit(&outcome.restart_limit),
            liveness: DashboardChildLivenessState::from_liveness(&outcome.liveness),
            idempotent: outcome.idempotent,
            failure: outcome
                .failure
                .as_ref()
                .map(DashboardChildControlFailure::from_failure),
        }
    }
}

/// Command result shape returned through dashboard state deltas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DashboardCommandResult {
    /// Child was accepted by the control loop.
    ChildAdded {
        /// Child manifest stored by the runtime.
        child_manifest: String,
    },
    /// Child control result after a command.
    ChildControl {
        /// Dashboard child control result.
        outcome: DashboardChildControlResult,
    },
    /// Current state query result.
    CurrentState {
        /// Runtime current state.
        state: DashboardCurrentState,
    },
    /// Shutdown command result.
    Shutdown {
        /// Shutdown result serialized by the shutdown module.
        result: Value,
    },
}

impl DashboardCommandResult {
    /// Converts a runtime command result into a dashboard command result.
    ///
    /// # Arguments
    ///
    /// - `result`: Runtime command result.
    ///
    /// # Returns
    ///
    /// Returns a dashboard command result.
    pub fn from_command_result(result: &CommandResult) -> Result<Self, serde_json::Error> {
        match result {
            CommandResult::ChildAdded { child_manifest } => Ok(Self::ChildAdded {
                child_manifest: child_manifest.clone(),
            }),
            CommandResult::ChildControl { outcome } => Ok(Self::ChildControl {
                outcome: DashboardChildControlResult::from_child_control_result(outcome),
            }),
            CommandResult::CurrentState { state } => Ok(Self::CurrentState {
                state: DashboardCurrentState::from_current_state(state),
            }),
            CommandResult::Shutdown { result } => Ok(Self::Shutdown {
                result: serde_json::to_value(result)?,
            }),
        }
    }
}

/// Serializes a runtime command result using the dashboard return model.
///
/// # Arguments
///
/// - `result`: Runtime command result.
///
/// # Returns
///
/// Returns a JSON value with the dashboard command result shape.
pub fn dashboard_command_result_value(result: &CommandResult) -> Result<Value, serde_json::Error> {
    serde_json::to_value(DashboardCommandResult::from_command_result(result)?)
}

/// Derives the managed child state that corresponds to an operation.
///
/// # Arguments
///
/// - `operation`: Runtime control operation.
///
/// # Returns
///
/// Returns the managed child state required by the public mapping table.
pub fn managed_child_state_from_operation(operation: ChildControlOperation) -> ManagedChildState {
    match operation {
        ChildControlOperation::Active => ManagedChildState::Running,
        ChildControlOperation::Paused => ManagedChildState::Paused,
        ChildControlOperation::Quarantined => ManagedChildState::Quarantined,
        ChildControlOperation::Removed => ManagedChildState::Removed,
    }
}

/// Converts a child runtime record into the existing dashboard runtime row.
///
/// # Arguments
///
/// - `record`: Runtime child record returned by current state.
/// - `shutdown_completed`: Whether the supervisor shutdown has completed.
///
/// # Returns
///
/// Returns a dashboard runtime row that preserves the existing UI list shape.
pub fn runtime_state_from_child_runtime_record(
    record: &ChildRuntimeRecord,
    shutdown_completed: bool,
) -> RuntimeState {
    let managed_child_state =
        DashboardManagedChildState::from(managed_child_state_from_operation(record.operation));
    let readiness = DashboardReadinessState::from(record.liveness.readiness);
    RuntimeState {
        child_path: record.path.to_string(),
        lifecycle_state: managed_child_state.as_label().to_owned(),
        health: if record.liveness.heartbeat_stale {
            "stale".to_owned()
        } else {
            "healthy".to_owned()
        },
        readiness: readiness.as_label().to_owned(),
        generation: record
            .generation
            .map(|generation| generation.value)
            .unwrap_or(0),
        child_start_count: record.attempt.map(|attempt| attempt.value).unwrap_or(0),
        restart_count: u64::from(record.restart_limit.used),
        last_failure: record
            .failure
            .as_ref()
            .map(|failure| failure.reason.clone()),
        last_policy_decision: Some(format!(
            "restart_limit_remaining={}",
            record.restart_limit.remaining
        )),
        shutdown_state: if shutdown_completed {
            "completed".to_owned()
        } else {
            "running".to_owned()
        },
    }
}

/// Event record streamed from a target process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventRecord {
    /// Target process identifier.
    pub target_id: String,
    /// Target-local monotonic sequence.
    pub sequence: u64,
    /// Correlation identifier.
    pub correlation_id: String,
    /// Event type label.
    pub event_type: String,
    /// Severity label.
    pub severity: String,
    /// Target path.
    pub target_path: String,
    /// Optional child identifier.
    pub child_id: Option<String>,
    /// Occurred time as Unix nanoseconds.
    pub occurred_at_unix_nanos: u128,
    /// Configuration version.
    pub config_version: String,
    /// Event payload.
    pub payload: Value,
}

/// Log record streamed from a target process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LogRecord {
    /// Target process identifier.
    pub target_id: String,
    /// Optional target-local sequence.
    pub sequence: Option<u64>,
    /// Optional correlation identifier.
    pub correlation_id: Option<String>,
    /// Severity label.
    pub severity: String,
    /// Log message.
    pub message: String,
    /// Structured log fields.
    pub fields: BTreeMap<String, String>,
    /// Occurred time as Unix nanoseconds.
    pub occurred_at_unix_nanos: u128,
}

/// Supported control command names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ControlCommandKind {
    /// Restart a child.
    RestartChild,
    /// Pause a child.
    PauseChild,
    /// Resume a child.
    ResumeChild,
    /// Quarantine a child.
    QuarantineChild,
    /// Remove a child.
    RemoveChild,
    /// Add a child.
    AddChild,
    /// Shut down the whole tree.
    ShutdownTree,
}

/// Target selector for a control command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ControlCommandTarget {
    /// Optional child path for child-scoped commands.
    pub child_path: Option<String>,
    /// Optional child manifest for add-child commands.
    pub child_manifest: Option<String>,
}

/// Control command request forwarded by relay to target IPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ControlCommandRequest {
    /// Command identifier.
    pub command_id: String,
    /// Target process identifier.
    pub target_id: String,
    /// Command kind.
    pub command: ControlCommandKind,
    /// Command target.
    pub target: ControlCommandTarget,
    /// Non-empty reason.
    pub reason: String,
    /// Authenticated requester derived by relay.
    pub requested_by: String,
    /// Whether dangerous command confirmation is present.
    pub confirmed: bool,
    /// Request time as Unix nanoseconds.
    pub requested_at_unix_nanos: u128,
}

/// Control command result returned by target IPC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ControlCommandResult {
    /// Command identifier.
    pub command_id: String,
    /// Target process identifier.
    pub target_id: String,
    /// Whether target accepted the command.
    pub accepted: bool,
    /// Status label.
    pub status: String,
    /// Optional structured error.
    pub error: Option<crate::dashboard::error::DashboardError>,
    /// Optional state delta.
    pub state_delta: Option<Value>,
    /// Completion time as Unix nanoseconds.
    pub completed_at_unix_nanos: Option<u128>,
}

/// Audit event emitted for accepted, rejected, and completed commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AuditEvent {
    /// Audit event identifier.
    pub audit_id: String,
    /// Remote identity summary.
    pub identity: String,
    /// Target process identifier.
    pub target_id: String,
    /// Command identifier.
    pub command_id: String,
    /// Command kind.
    pub command: ControlCommandKind,
    /// Command target.
    pub target: ControlCommandTarget,
    /// Operator-provided reason.
    pub reason: String,
    /// Result summary.
    pub result: String,
    /// Occurred time as Unix nanoseconds.
    pub occurred_at_unix_nanos: u128,
}
