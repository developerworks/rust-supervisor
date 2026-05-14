//! Shared dashboard data model.
//!
//! These structs are the JSON contract shared by target IPC, relay, and the
//! dashboard UI. They intentionally use owned values so callers can serialize,
//! clone, and test messages without borrowing runtime internals.

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
