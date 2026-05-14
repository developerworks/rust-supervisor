//! Owns the dashboard data scenario served by the demo example.

// Import dashboard error values used by validation.
use rust_supervisor::dashboard::error::DashboardError;
// Import dashboard model contracts served over IPC.
use rust_supervisor::dashboard::model::{
    // Continue the demo expression.
    ControlCommandKind,
    // Import command request shape.
    ControlCommandRequest,
    // Import command result shape.
    ControlCommandResult,
    // Import topology criticality values.
    DashboardCriticality,
    // Continue the demo expression.
    DashboardState,
    // Import event record shape.
    EventRecord,
    // Import log record shape.
    LogRecord,
    // Import registration state values.
    RegistrationState,
    // Import runtime row shape.
    RuntimeState,
    // Import topology edge shape.
    SupervisorEdge,
    // Continue the demo expression.
    SupervisorEdgeKind,
    // Import topology node shape.
    SupervisorNode,
    // Import topology node kind values.
    SupervisorNodeKind,
    // Import topology graph shape.
    SupervisorTopology,
    // Continue the demo expression.
    TargetConnectionState,
    // Import target identity shape.
    TargetProcessIdentity,
    // Continue the demo expression.
};
// Import JSON construction for command deltas and event payloads.
use serde_json::json;
// Import ordered maps for stable serialized payloads.
use std::collections::BTreeMap;
// Import mutex storage for mutable demo child states.
use std::sync::Mutex;
// Import atomic state generation counters.
use std::sync::atomic::{AtomicU64, Ordering};

// Define the root path shown in the demo topology.
const ROOT_PATH: &str = "/root";
// Define the demo configuration version.
const CONFIG_VERSION: &str = "demo-ui-scenario-v1";

/// Mutable dashboard scenario served by the demo IPC target.
pub(crate) struct DemoScenario {
    /// Stable target process identifier.
    target_id: String,
    /// Human-readable target name.
    display_name: String,
    /// State generation counter.
    state_generation: AtomicU64,
    /// Mutable child rows.
    children: Mutex<Vec<DemoChild>>,
    /// Command event sequence counter.
    activity_sequence: AtomicU64,
    // Continue the demo expression.
}

// Derive clone and debug helpers for static child declarations.
#[derive(Clone, Debug)]
/// One child row in the dashboard demo scenario.
struct DemoChild {
    /// Stable child identifier.
    id: String,
    /// Human-readable child name.
    name: String,
    /// Lifecycle state label.
    lifecycle: String,
    /// Health state label.
    health: String,
    /// Readiness state label.
    readiness: String,
    /// Restart count shown by the UI.
    restart_count: u64,
    /// Whether the child remains visible.
    present: bool,
    // Continue the demo expression.
}

// Derive clone and debug helpers for command transition payloads.
#[derive(Clone, Debug)]
/// Lifecycle transition caused by one demo command.
struct CommandTransition {
    /// Lifecycle state before command application.
    previous_lifecycle_state: String,
    /// Lifecycle state after command application.
    lifecycle_state: String,
    // Continue the demo expression.
}

// Continue the demo expression.
impl DemoScenario {
    /// Creates the default UI scenario.
    ///
    /// # Arguments
    ///
    /// - `target_id`: Target process identifier.
    /// - `display_name`: Human-readable target name.
    ///
    /// # Returns
    ///
    /// Returns a mutable scenario with the standard demo children.
    pub(crate) fn new(target_id: String, display_name: String) -> Self {
        // Store the target identity and initial rows.
        Self {
            // Keep the target identifier stable.
            target_id,
            // Keep the display name stable.
            display_name,
            // Start state generations at one.
            state_generation: AtomicU64::new(1),
            // Seed child rows for the UI topology.
            children: Mutex::new(seed_children()),
            // Start command activity sequences away from seed records.
            activity_sequence: AtomicU64::new(1),
            // End scenario initialization.
        }
        // End constructor.
    }

    /// Returns the target process identifier.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the target identifier as a string slice.
    pub(crate) fn target_id(&self) -> &str {
        // Return the stored target identifier.
        &self.target_id
        // End target identifier access.
    }

    /// Builds the current dashboard state.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a dashboard state payload for UI rendering.
    pub(crate) fn state(&self) -> DashboardState {
        // Lock child rows for a consistent state payload.
        let children = self.children.lock().expect("demo scenario mutex");
        // Collect visible child rows.
        let visible = visible_children(&children);
        // Build the full dashboard state.
        DashboardState {
            // Include target identity and connection state.
            target: self.target_identity(),
            // Include the topology graph.
            topology: topology(&visible),
            // Include runtime rows.
            runtime_state: runtime_rows(&visible),
            // Include recent event rows.
            recent_events: event_records(&self.target_id, &visible),
            // Include recent log rows.
            recent_logs: log_records(&self.target_id, &visible),
            // Show that events can be dropped by bounded buffers.
            dropped_event_count: 2,
            // Show that logs can be dropped by bounded buffers.
            dropped_log_count: 1,
            // Include a stable demo config version.
            config_version: CONFIG_VERSION.to_owned(),
            // Include the generation timestamp.
            generated_at_unix_nanos: unix_nanos_now(),
            // Increment state generation on every state build.
            state_generation: self.state_generation.fetch_add(1, Ordering::Relaxed),
            // End dashboard state construction.
        }
        // End state construction.
    }

    /// Applies one control command and returns its result.
    ///
    /// # Arguments
    ///
    /// - `command`: Decoded command request.
    ///
    /// # Returns
    ///
    /// Returns a structured command result or validation error.
    pub(crate) fn command_result(
        // Continue the demo expression.
        &self,
        // Continue the demo expression.
        command: ControlCommandRequest,
        // Continue the demo expression.
    ) -> Result<ControlCommandResult, DashboardError> {
        // Validate command input before state mutation.
        self.validate_command(&command)?;
        // Lock child rows while applying the command.
        let mut children = self.children.lock().expect("demo scenario mutex");
        // Apply the requested command.
        let transition = apply_command(&mut children, &command)?;
        // Build a UI-consumable state delta after mutation.
        let delta = command_state_delta(
            // Continue the demo expression.
            &self.target_id,
            // Continue the demo expression.
            &children,
            // Continue the demo expression.
            &command,
            // Continue the demo expression.
            &transition,
            // Continue the demo expression.
            self.activity_sequence.fetch_add(1, Ordering::Relaxed),
            // Continue the demo expression.
            self.state_generation.fetch_add(1, Ordering::Relaxed),
            // Continue the demo expression.
        );
        // Return a successful command result.
        Ok(ControlCommandResult {
            // Preserve the original command identifier.
            command_id: command.command_id,
            // Preserve the target identifier.
            target_id: command.target_id,
            // Mark the command as accepted.
            accepted: true,
            // Mark the command as completed.
            status: "completed".to_owned(),
            // No structured error is present.
            error: None,
            // Include the state delta summary.
            state_delta: Some(delta),
            // Record command completion time.
            completed_at_unix_nanos: Some(unix_nanos_now()),
            // End command result construction.
        })
        // End command result.
    }

    /// Builds the target identity portion of dashboard state.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns target identity metadata for the state payload.
    fn target_identity(&self) -> TargetProcessIdentity {
        // Build the connected target identity.
        TargetProcessIdentity {
            // Include target identifier.
            target_id: self.target_id.clone(),
            // Include display name.
            display_name: self.display_name.clone(),
            // Mark registration as active.
            registration_state: RegistrationState::Active,
            // Mark the target IPC as connected.
            connection_state: TargetConnectionState::Connected,
            // End target identity construction.
        }
        // End target identity construction.
    }

    /// Validates one command request.
    ///
    /// # Arguments
    ///
    /// - `command`: Command request supplied by relay.
    ///
    /// # Returns
    ///
    /// Returns success when the command can be applied.
    fn validate_command(&self, command: &ControlCommandRequest) -> Result<(), DashboardError> {
        // Reject commands for another target.
        if command.target_id != self.target_id {
            // Return target mismatch validation.
            return Err(validation(
                // Continue the demo expression.
                &self.target_id,
                // Continue the demo expression.
                "command target_id must match demo target",
                // Continue the demo expression.
            ));
            // End target mismatch branch.
        }
        // Reject missing command identifiers.
        if command.command_id.trim().is_empty() {
            // Return command identifier validation.
            return Err(validation(&self.target_id, "command_id must not be empty"));
            // End command identifier branch.
        }
        // Reject missing reasons.
        if command.reason.trim().is_empty() {
            // Return reason validation.
            return Err(validation(
                // Continue the demo expression.
                &self.target_id,
                // Continue the demo expression.
                "command reason must not be empty",
                // Continue the demo expression.
            ));
            // End reason branch.
        }
        // Reject missing requester identity.
        if command.requested_by.trim().is_empty() {
            // Return requester validation.
            return Err(validation(
                // Continue the demo expression.
                &self.target_id,
                // Continue the demo expression.
                "requested_by must not be empty",
                // Continue the demo expression.
            ));
            // End requester branch.
        }
        // Validate dangerous command confirmation.
        if is_dangerous(command.command) && !command.confirmed {
            // Return confirmation validation.
            return Err(validation(
                // Continue the demo expression.
                &self.target_id,
                // Continue the demo expression.
                "dangerous command requires confirmation",
                // Continue the demo expression.
            ));
            // End confirmation branch.
        }
        // Validate add-child manifests.
        if command.command == ControlCommandKind::AddChild && missing_child_manifest(command) {
            // Return child manifest validation.
            return Err(validation(
                // Continue the demo expression.
                &self.target_id,
                // Continue the demo expression.
                "add_child requires child_manifest",
                // Continue the demo expression.
            ));
            // End child manifest branch.
        }
        // Finish command validation successfully.
        Ok(())
        // End command validation.
    }
    // Continue the demo expression.
}

/// Builds the seed child rows for the demo.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns the static child rows used by the UI.
fn seed_children() -> Vec<DemoChild> {
    // Return every standard demo child.
    vec![
        // Include a failed child.
        child(
            // Continue the demo expression.
            "duplicate_guard",
            // Continue the demo expression.
            "duplicate guard",
            // Continue the demo expression.
            "failed",
            // Continue the demo expression.
            "unhealthy",
            // Continue the demo expression.
            "not_ready",
            // Continue the demo expression.
            2,
            // Continue the demo expression.
        ),
        // Include a restarting child.
        child(
            // Continue the demo expression.
            "retry_scheduler",
            // Continue the demo expression.
            "retry scheduler",
            // Continue the demo expression.
            "restarting",
            // Continue the demo expression.
            "stale",
            // Continue the demo expression.
            "not_ready",
            // Continue the demo expression.
            3,
            // Continue the demo expression.
        ),
        // Include a paused child.
        child(
            // Continue the demo expression.
            "invoice_writer",
            // Continue the demo expression.
            "invoice writer",
            // Continue the demo expression.
            "paused",
            // Continue the demo expression.
            "healthy",
            // Continue the demo expression.
            "ready",
            // Continue the demo expression.
            0,
            // Continue the demo expression.
        ),
        // Include a quarantined child.
        child(
            // Continue the demo expression.
            "index_stream",
            // Continue the demo expression.
            "index stream",
            // Continue the demo expression.
            "quarantined",
            // Continue the demo expression.
            "unhealthy",
            // Continue the demo expression.
            "not_ready",
            // Continue the demo expression.
            5,
            // Continue the demo expression.
        ),
        // Include a healthy running child.
        child(
            // Continue the demo expression.
            "healthy_worker",
            // Continue the demo expression.
            "healthy worker",
            // Continue the demo expression.
            "running",
            // Continue the demo expression.
            "healthy",
            // Continue the demo expression.
            "ready",
            // Continue the demo expression.
            0,
            // Continue the demo expression.
        ),
        // End child list.
    ]
    // End seed child construction.
}

/// Creates one demo child row.
///
/// # Arguments
///
/// - `id`: Stable child identifier.
/// - `name`: Human-readable child name.
/// - `lifecycle`: Lifecycle state label.
/// - `health`: Health state label.
/// - `readiness`: Readiness state label.
/// - `restart_count`: Restart count.
///
/// # Returns
///
/// Returns a child row.
fn child(
    // Continue the demo expression.
    id: &str,
    // Continue the demo expression.
    name: &str,
    // Continue the demo expression.
    lifecycle: &str,
    // Continue the demo expression.
    health: &str,
    // Continue the demo expression.
    readiness: &str,
    // Continue the demo expression.
    restart_count: u64,
    // Continue the demo expression.
) -> DemoChild {
    // Build one child row.
    DemoChild {
        // Store the child identifier.
        id: id.to_owned(),
        // Store the child display name.
        name: name.to_owned(),
        // Store the lifecycle label.
        lifecycle: lifecycle.to_owned(),
        // Store the health label.
        health: health.to_owned(),
        // Store the readiness label.
        readiness: readiness.to_owned(),
        // Store the restart count.
        restart_count,
        // Mark the row visible.
        present: true,
        // End child row construction.
    }
    // End child construction.
}

/// Filters visible child rows.
///
/// # Arguments
///
/// - `children`: All child rows.
///
/// # Returns
///
/// Returns visible child rows.
fn visible_children(children: &[DemoChild]) -> Vec<DemoChild> {
    // Keep only rows that remain present.
    children
        // Iterate through all rows.
        .iter()
        // Keep visible rows.
        .filter(|child| child.present)
        // Clone visible rows for state construction.
        .cloned()
        // Collect visible rows.
        .collect()
    // End visible child filtering.
}

/// Builds topology for visible demo children.
///
/// # Arguments
///
/// - `children`: Visible child rows.
///
/// # Returns
///
/// Returns a topology graph.
fn topology(children: &[DemoChild]) -> SupervisorTopology {
    // Build the root node.
    let root = root_node();
    // Start the node list with the root.
    let mut nodes = vec![root.clone()];
    // Start the edge list.
    let mut edges = Vec::new();
    // Start the declaration order with the root.
    let mut declaration_order = vec![ROOT_PATH.to_owned()];
    // Add one node and edge per child.
    for (index, child) in children.iter().enumerate() {
        // Build the child path.
        let path = child_path(&child.id);
        // Record declaration order.
        declaration_order.push(path.clone());
        // Add the child node.
        nodes.push(child_node(child, &path));
        // Add the parent-child edge.
        edges.push(parent_edge(index, &path));
        // End child topology row.
    }
    // Return the topology.
    SupervisorTopology {
        // Include the root node.
        root,
        // Include all visible nodes.
        nodes,
        // Include all visible edges.
        edges,
        // Include declaration order.
        declaration_order,
        // End topology construction.
    }
    // End topology construction.
}

/// Builds the root topology node.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns the root node.
fn root_node() -> SupervisorNode {
    // Build the root node.
    SupervisorNode {
        // Use the root path as node identifier.
        node_id: ROOT_PATH.to_owned(),
        // Root has no child identifier.
        child_id: None,
        // Use the root path.
        path: ROOT_PATH.to_owned(),
        // Use a readable root name.
        name: "root supervisor".to_owned(),
        // Mark the node as root.
        kind: SupervisorNodeKind::RootSupervisor,
        // Root has no tags.
        tags: Vec::new(),
        // Root is critical.
        criticality: DashboardCriticality::Critical,
        // Summarize the root state.
        state_summary: "root".to_owned(),
        // Root has no diagnostics.
        diagnostics: BTreeMap::new(),
        // End root node construction.
    }
    // End root node construction.
}

/// Builds one child topology node.
///
/// # Arguments
///
/// - `child`: Child row.
/// - `path`: Child path.
///
/// # Returns
///
/// Returns a child node.
fn child_node(child: &DemoChild, path: &str) -> SupervisorNode {
    // Build node diagnostics.
    let diagnostics = diagnostics_for(child);
    // Build the child node.
    SupervisorNode {
        // Use the path as the node identifier.
        node_id: path.to_owned(),
        // Include the child identifier.
        child_id: Some(child.id.clone()),
        // Include the child path.
        path: path.to_owned(),
        // Include the child name.
        name: child.name.clone(),
        // Mark the node as a child task.
        kind: SupervisorNodeKind::ChildTask,
        // Tag demo rows for filtering.
        tags: vec!["demo".to_owned(), "ui".to_owned()],
        // Use standard criticality.
        criticality: DashboardCriticality::Standard,
        // Use the lifecycle as state summary.
        state_summary: child.lifecycle.clone(),
        // Include diagnostics.
        diagnostics,
        // End child node construction.
    }
    // End child node construction.
}

/// Builds diagnostics for one child.
///
/// # Arguments
///
/// - `child`: Child row.
///
/// # Returns
///
/// Returns diagnostic fields.
fn diagnostics_for(child: &DemoChild) -> BTreeMap<String, String> {
    // Start with an empty map.
    let mut diagnostics = BTreeMap::new();
    // Add diagnostics for failed rows.
    if child.lifecycle == "failed" {
        // Add failure summary.
        diagnostics.insert(
            // Store the diagnostic key.
            "message".to_owned(),
            // Store the diagnostic message.
            "duplicate event window exceeded".to_owned(),
            // End diagnostic insertion.
        );
        // End failed diagnostic branch.
    }
    // Return diagnostics.
    diagnostics
    // End diagnostics construction.
}

/// Builds one parent-child edge.
///
/// # Arguments
///
/// - `index`: Child declaration index.
/// - `path`: Child path.
///
/// # Returns
///
/// Returns one topology edge.
fn parent_edge(index: usize, path: &str) -> SupervisorEdge {
    // Build the edge.
    SupervisorEdge {
        // Build a stable edge identifier.
        edge_id: format!("parent:{ROOT_PATH}->{path}"),
        // Root is the source.
        source_path: ROOT_PATH.to_owned(),
        // Child path is the target.
        target_path: path.to_owned(),
        // Mark the edge as parent-child.
        kind: SupervisorEdgeKind::ParentChild,
        // Preserve declaration order.
        order: index,
        // End edge construction.
    }
    // End edge construction.
}

/// Builds runtime rows for visible children.
///
/// # Arguments
///
/// - `children`: Visible child rows.
///
/// # Returns
///
/// Returns runtime state rows.
fn runtime_rows(children: &[DemoChild]) -> Vec<RuntimeState> {
    // Convert children to runtime rows.
    children
        // Iterate over visible children.
        .iter()
        // Build one runtime row per child.
        .map(runtime_row)
        // Collect runtime rows.
        .collect()
    // End runtime row construction.
}

/// Builds one runtime row.
///
/// # Arguments
///
/// - `child`: Child row.
///
/// # Returns
///
/// Returns one runtime row.
fn runtime_row(child: &DemoChild) -> RuntimeState {
    // Build the runtime row.
    RuntimeState {
        // Include the child path.
        child_path: child_path(&child.id),
        // Include the lifecycle state.
        lifecycle_state: child.lifecycle.clone(),
        // Include the health state.
        health: child.health.clone(),
        // Include the readiness state.
        readiness: child.readiness.clone(),
        // Include the generation number.
        generation: child.restart_count,
        // Include the attempt number.
        child_start_count: child.restart_count.saturating_add(1),
        // Include restart count.
        restart_count: child.restart_count,
        // Include last failure when present.
        last_failure: last_failure(child),
        // Include last policy decision when present.
        last_policy_decision: last_policy_decision(child),
        // Include shutdown state.
        shutdown_state: "running".to_owned(),
        // End runtime row construction.
    }
    // End runtime row construction.
}

/// Builds recent events for visible children.
///
/// # Arguments
///
/// - `target_id`: Target process identifier.
/// - `children`: Visible child rows.
///
/// # Returns
///
/// Returns recent event records.
fn event_records(target_id: &str, children: &[DemoChild]) -> Vec<EventRecord> {
    // Convert children to event records.
    children
        // Iterate over visible children.
        .iter()
        // Keep child indexes as stable sequences.
        .enumerate()
        // Build one event per child.
        .map(|(index, child)| event_record(target_id, index, child))
        // Collect event records.
        .collect()
    // End event record construction.
}

/// Builds one event record.
///
/// # Arguments
///
/// - `target_id`: Target process identifier.
/// - `index`: Child index.
/// - `child`: Child row.
///
/// # Returns
///
/// Returns one event record.
fn event_record(target_id: &str, index: usize, child: &DemoChild) -> EventRecord {
    // Compute a deterministic sequence.
    let sequence = 1001_u64.saturating_add(index as u64);
    // Build the event record.
    EventRecord {
        // Include target identifier.
        target_id: target_id.to_owned(),
        // Include target-local sequence.
        sequence,
        // Include correlation identifier.
        correlation_id: format!("demo-{sequence}"),
        // Include event type.
        event_type: event_type(child).to_owned(),
        // Include severity.
        severity: severity(child).to_owned(),
        // Include target path.
        target_path: child_path(&child.id),
        // Include child identifier.
        child_id: Some(child.id.clone()),
        // Include occurrence time.
        occurred_at_unix_nanos: unix_nanos_now().saturating_sub(sequence as u128),
        // Include configuration version.
        config_version: CONFIG_VERSION.to_owned(),
        // Include structured payload.
        payload: json!({
            // Include the child path field.
            "child_path": child_path(&child.id),
            // Include the previous lifecycle state field.
            "previous_lifecycle_state": "unknown",
            // Include the lifecycle state field.
            "lifecycle_state": child.lifecycle,
            // End event payload object.
        }),
        // End event record construction.
    }
    // End event record construction.
}

/// Builds recent log rows for visible children.
///
/// # Arguments
///
/// - `target_id`: Target process identifier.
/// - `children`: Visible child rows.
///
/// # Returns
///
/// Returns recent log records.
fn log_records(target_id: &str, children: &[DemoChild]) -> Vec<LogRecord> {
    // Convert children to log rows.
    children
        // Iterate over visible children.
        .iter()
        // Keep child indexes as stable sequences.
        .enumerate()
        // Build one log row per child.
        .map(|(index, child)| log_record(target_id, index, child))
        // Collect log rows.
        .collect()
    // End log record construction.
}

/// Builds one log record.
///
/// # Arguments
///
/// - `target_id`: Target process identifier.
/// - `index`: Child index.
/// - `child`: Child row.
///
/// # Returns
///
/// Returns one log record.
fn log_record(target_id: &str, index: usize, child: &DemoChild) -> LogRecord {
    // Compute a deterministic sequence.
    let sequence = 2001_u64.saturating_add(index as u64);
    // Build structured log fields.
    let mut fields = BTreeMap::new();
    // Insert child path.
    fields.insert("child_path".to_owned(), child_path(&child.id));
    // Insert previous lifecycle state.
    fields.insert("previous_lifecycle_state".to_owned(), "unknown".to_owned());
    // Insert lifecycle state.
    fields.insert("lifecycle_state".to_owned(), child.lifecycle.clone());
    // Build the log record.
    LogRecord {
        // Include target identifier.
        target_id: target_id.to_owned(),
        // Include log sequence.
        sequence: Some(sequence),
        // Include correlation identifier.
        correlation_id: Some(format!("demo-{sequence}")),
        // Include log severity.
        severity: severity(child).to_owned(),
        // Include log message.
        message: format!(
            // Continue the demo expression.
            "{} transitioned from unknown to {}",
            // Continue the demo expression.
            child.name,
            // Continue the demo expression.
            child.lifecycle,
            // End transition message expression.
        ),
        // Include structured fields.
        fields,
        // Include occurrence time.
        occurred_at_unix_nanos: unix_nanos_now().saturating_sub(sequence as u128),
        // End log record construction.
    }
    // End log record construction.
}

/// Applies a command to the demo rows.
///
/// # Arguments
///
/// - `children`: Mutable child rows.
/// - `command`: Command request.
///
/// # Returns
///
/// Returns a JSON state delta.
fn apply_command(
    // Continue the demo expression.
    children: &mut Vec<DemoChild>,
    // Continue the demo expression.
    command: &ControlCommandRequest,
    // Continue the demo expression.
) -> Result<CommandTransition, DashboardError> {
    // Apply tree shutdown without a child path.
    if command.command == ControlCommandKind::ShutdownTree {
        // Read first visible lifecycle before shutdown.
        let previous = children
            // Continue the demo expression.
            .iter()
            // Continue the demo expression.
            .find(|child| child.present)
            // Continue the demo expression.
            .map(|child| child.lifecycle.clone())
            // Continue the demo expression.
            .unwrap_or_else(|| "unknown".to_owned());
        // Mark visible children as stopped.
        children
            // Continue the demo expression.
            .iter_mut()
            // Continue the demo expression.
            .for_each(|child| child.lifecycle = "stopped".to_owned());
        // Return success.
        return Ok(CommandTransition {
            // Preserve the previous lifecycle state.
            previous_lifecycle_state: previous,
            // Store the lifecycle after shutdown.
            lifecycle_state: "stopped".to_owned(),
            // End transition construction.
        });
        // End shutdown branch.
    }
    // Resolve the target child identifier.
    let child_id = command_child_id(command)?;
    // Apply add-child separately.
    if command.command == ControlCommandKind::AddChild {
        // Read the lifecycle before adding or restoring.
        let previous = children
            // Continue the demo expression.
            .iter()
            // Continue the demo expression.
            .find(|row| row.id == child_id && row.present)
            // Continue the demo expression.
            .map(|row| row.lifecycle.clone())
            // Continue the demo expression.
            .unwrap_or_else(|| "absent".to_owned());
        // Add or restore the requested child.
        add_child(children, &child_id);
        // Return success.
        return Ok(CommandTransition {
            // Preserve the previous lifecycle state.
            previous_lifecycle_state: previous,
            // Store the lifecycle after adding or restoring.
            lifecycle_state: "running".to_owned(),
            // End transition construction.
        });
        // End add-child branch.
    }
    // Find the target child row.
    let child = children
        // Iterate through mutable rows.
        .iter_mut()
        // Match the target child identifier.
        .find(|row| row.id == child_id && row.present)
        // Convert absence into validation.
        .ok_or_else(|| {
            // Continue the demo expression.
            validation(
                // Continue the demo expression.
                &command.target_id,
                // Continue the demo expression.
                "child_path does not match a visible child",
                // Continue the demo expression.
            )
            // Continue the demo expression.
        })?;
    // Preserve the lifecycle before child command application.
    let previous = child.lifecycle.clone();
    // Apply the child command.
    match command.command {
        // Restart moves the child into restarting state.
        ControlCommandKind::RestartChild => {
            // Continue the demo expression.
            set_child_state(child, "restarting", "stale", "not_ready")
            // Continue the demo expression.
        }
        // Pause moves the child into paused state.
        ControlCommandKind::PauseChild => set_child_state(child, "paused", "healthy", "ready"),
        // Resume moves the child into running state.
        ControlCommandKind::ResumeChild => set_child_state(child, "running", "healthy", "ready"),
        // Quarantine moves the child into quarantined state.
        ControlCommandKind::QuarantineChild => {
            // Continue the demo expression.
            set_child_state(child, "quarantined", "unhealthy", "not_ready")
            // Continue the demo expression.
        }
        // Remove hides the child from later state responses.
        ControlCommandKind::RemoveChild => child.present = false,
        // Other variants are handled above.
        ControlCommandKind::AddChild | ControlCommandKind::ShutdownTree => {} // End command match.
                                                                              // Continue the demo expression.
    }
    // Return success.
    Ok(CommandTransition {
        // Preserve the previous lifecycle state.
        previous_lifecycle_state: previous,
        // Store the lifecycle after command application.
        lifecycle_state: lifecycle_after(command.command).to_owned(),
        // End transition construction.
    })
    // End command application.
}

/// Adds or restores a child row.
///
/// # Arguments
///
/// - `children`: Mutable child rows.
/// - `child_id`: Child identifier.
///
/// # Returns
///
/// This function has no return value.
fn add_child(children: &mut Vec<DemoChild>, child_id: &str) {
    // Restore an existing row when it already exists.
    if let Some(child) = children.iter_mut().find(|row| row.id == child_id) {
        // Mark the existing row present.
        child.present = true;
        // Mark the existing row running.
        set_child_state(child, "running", "healthy", "ready");
        // Finish existing row handling.
        return;
        // End existing row branch.
    }
    // Add a new row.
    children.push(child(child_id, child_id, "running", "healthy", "ready", 0));
    // End child addition.
}

/// Sets child state labels.
///
/// # Arguments
///
/// - `child`: Mutable child row.
/// - `lifecycle`: Lifecycle state label.
/// - `health`: Health state label.
/// - `readiness`: Readiness state label.
///
/// # Returns
///
/// This function has no return value.
fn set_child_state(child: &mut DemoChild, lifecycle: &str, health: &str, readiness: &str) {
    // Update lifecycle.
    child.lifecycle = lifecycle.to_owned();
    // Update health.
    child.health = health.to_owned();
    // Update readiness.
    child.readiness = readiness.to_owned();
    // Increment restart count for restarting state.
    if lifecycle == "restarting" {
        // Increase restart count.
        child.restart_count = child.restart_count.saturating_add(1);
        // End restarting branch.
    }
    // End state update.
}

/// Builds a command delta for the UI.
///
/// # Arguments
///
/// - `target_id`: Target identifier.
/// - `children`: Current child rows after mutation.
/// - `command`: Command request.
/// - `sequence`: Command activity sequence.
/// - `state_generation`: State generation value.
///
/// # Returns
///
/// Returns a JSON delta.
fn command_state_delta(
    // Accept target identifier.
    target_id: &str,
    // Accept current child rows.
    children: &[DemoChild],
    // Accept original command.
    command: &ControlCommandRequest,
    // Accept lifecycle transition.
    transition: &CommandTransition,
    // Accept activity sequence.
    sequence: u64,
    // Accept state generation.
    state_generation: u64,
    // End command delta signature.
) -> serde_json::Value {
    // Collect visible rows after command application.
    let visible = visible_children(children);
    // Build the JSON delta consumed by the UI.
    json!({
        // Include command kind.
        "command": command_name(command.command),
        // Include child path.
        "child_path": command.target.child_path,
        // Include previous lifecycle state.
        "previous_lifecycle_state": transition.previous_lifecycle_state.as_str(),
        // Include lifecycle state.
        "lifecycle_state": transition.lifecycle_state.as_str(),
        // Include state generation.
        "state_generation": state_generation,
        // Include current topology after mutation.
        "topology": topology(&visible),
        // Include current runtime rows after mutation.
        "runtime_state": runtime_rows(&visible),
        // Include command event rows.
        "recent_events": [command_event_record(target_id, sequence, command, transition)],
        // Include command log rows.
        "recent_logs": [command_log_record(target_id, sequence, command, transition)],
        // Include dropped event count.
        "dropped_event_count": 2,
        // Include dropped log count.
        "dropped_log_count": 1,
        // End delta object.
    })
    // End delta construction.
}

/// Builds an event row for a command.
fn command_event_record(
    // Accept target identifier.
    target_id: &str,
    // Accept activity sequence.
    sequence: u64,
    // Accept original command.
    command: &ControlCommandRequest,
    // Accept lifecycle transition.
    transition: &CommandTransition,
    // End command event signature.
) -> EventRecord {
    // Resolve target path.
    let target_path = command
        // Access command target.
        .target
        // Access child path.
        .child_path
        // Clone path value.
        .clone()
        // Use root when the command targets the tree.
        .unwrap_or_else(|| ROOT_PATH.to_owned());
    // Resolve child id.
    let child_id = child_id_from_path(&target_path);
    // Build event record.
    EventRecord {
        // Include target identifier.
        target_id: target_id.to_owned(),
        // Include command event sequence.
        sequence: 7000_u64.saturating_add(sequence),
        // Include command identifier.
        correlation_id: command.command_id.clone(),
        // Include command event type.
        event_type: command_event_type(command.command).to_owned(),
        // Include command severity.
        severity: command_severity(command.command).to_owned(),
        // Include affected target path.
        target_path,
        // Include affected child identifier.
        child_id,
        // Include occurrence time.
        occurred_at_unix_nanos: unix_nanos_now(),
        // Include config version.
        config_version: CONFIG_VERSION.to_owned(),
        // Include command payload.
        payload: json!({
            // Include child path.
            "child_path": command.target.child_path,
            // Include command name.
            "command": command_name(command.command),
            // Include previous lifecycle state.
            "previous_lifecycle_state": transition.previous_lifecycle_state.as_str(),
            // Include lifecycle state.
            "lifecycle_state": transition.lifecycle_state.as_str(),
            // Include command reason.
            "reason": command.reason,
            // End command payload.
        }),
        // End event record construction.
    }
    // End event record.
}

/// Builds a log row for a command.
fn command_log_record(
    // Accept target identifier.
    target_id: &str,
    // Accept activity sequence.
    sequence: u64,
    // Accept original command.
    command: &ControlCommandRequest,
    // Accept lifecycle transition.
    transition: &CommandTransition,
    // End command log signature.
) -> LogRecord {
    // Resolve target path.
    let target_path = command
        // Access command target.
        .target
        // Access child path.
        .child_path
        // Clone path value.
        .clone()
        // Use root when the command targets the tree.
        .unwrap_or_else(|| ROOT_PATH.to_owned());
    // Start structured fields.
    let mut fields = BTreeMap::new();
    // Include child path field.
    fields.insert("child_path".to_owned(), target_path.clone());
    // Include command field.
    fields.insert(
        // Include command field key.
        "command".to_owned(),
        // Include command field value.
        command_name(command.command).to_owned(),
        // End command field insert.
    );
    // Include previous lifecycle field.
    fields.insert(
        // Include previous lifecycle key.
        "previous_lifecycle_state".to_owned(),
        // Include previous lifecycle value.
        transition.previous_lifecycle_state.clone(),
        // End previous lifecycle field insert.
    );
    // Include lifecycle field.
    fields.insert(
        // Include lifecycle key.
        "lifecycle_state".to_owned(),
        // Include lifecycle value.
        transition.lifecycle_state.clone(),
        // End lifecycle field insert.
    );
    // Build log record.
    LogRecord {
        // Include target identifier.
        target_id: target_id.to_owned(),
        // Include log sequence.
        sequence: Some(8000_u64.saturating_add(sequence)),
        // Include command identifier.
        correlation_id: Some(command.command_id.clone()),
        // Include log severity.
        severity: command_severity(command.command).to_owned(),
        // Include log message.
        message: format!(
            // Continue the demo expression.
            "{} {} completed, transitioned from {} to {}",
            // Continue the demo expression.
            target_path,
            // Continue the demo expression.
            command_name(command.command),
            // Continue the demo expression.
            transition.previous_lifecycle_state,
            // Continue the demo expression.
            transition.lifecycle_state,
            // End transition message expression.
        ),
        // Include structured fields.
        fields,
        // Include occurrence time.
        occurred_at_unix_nanos: unix_nanos_now(),
        // End log record construction.
    }
    // End log record.
}

/// Returns command event type.
fn command_event_type(command: ControlCommandKind) -> &'static str {
    // Match command to event type.
    match command {
        // Return restart event type.
        ControlCommandKind::RestartChild => "child_restarted",
        // Return pause event type.
        ControlCommandKind::PauseChild => "child_paused",
        // Return resume event type.
        ControlCommandKind::ResumeChild => "child_resumed",
        // Return quarantine event type.
        ControlCommandKind::QuarantineChild => "child_quarantined",
        // Return remove event type.
        ControlCommandKind::RemoveChild => "child_removed",
        // Return add event type.
        ControlCommandKind::AddChild => "child_added",
        // Return shutdown event type.
        ControlCommandKind::ShutdownTree => "tree_stopped",
        // End event type match.
    }
    // End event type lookup.
}

/// Returns command severity.
fn command_severity(command: ControlCommandKind) -> &'static str {
    // Match command to severity.
    match command {
        // Mark disruptive commands as warning.
        ControlCommandKind::QuarantineChild | ControlCommandKind::ShutdownTree => "warning",
        // Mark other commands as info.
        _ => "info",
        // End severity match.
    }
    // End severity lookup.
}

/// Extracts child id from path.
fn child_id_from_path(path: &str) -> Option<String> {
    // Skip the root path.
    if path == ROOT_PATH {
        // Return no child identifier.
        return None;
        // End root branch.
    }
    // Extract final path segment.
    path.rsplit('/')
        // Keep non-empty segments.
        .find(|segment| !segment.is_empty())
        // Convert to owned string.
        .map(ToOwned::to_owned)
    // End child id extraction.
}

/// Extracts the child identifier from a command.
///
/// # Arguments
///
/// - `command`: Command request.
///
/// # Returns
///
/// Returns the final path segment.
fn command_child_id(command: &ControlCommandRequest) -> Result<String, DashboardError> {
    // Read the command child path.
    let path = command.target.child_path.as_deref().ok_or_else(|| {
        // Return missing child path validation.
        validation(
            // Continue the demo expression.
            &command.target_id,
            // Continue the demo expression.
            "child_path is required for child command",
            // Continue the demo expression.
        )
        // End missing child path validation.
    })?;
    // Extract the final non-empty path segment.
    let child_id = path
        // Continue the demo expression.
        .rsplit('/')
        // Continue the demo expression.
        .find(|segment| !segment.is_empty())
        // Continue the demo expression.
        .unwrap_or(path);
    // Return the child identifier.
    Ok(child_id.to_owned())
    // End child identifier extraction.
}

/// Returns whether a command is dangerous.
///
/// # Arguments
///
/// - `command`: Command kind.
///
/// # Returns
///
/// Returns whether confirmation is required.
fn is_dangerous(command: ControlCommandKind) -> bool {
    // Match commands that require confirmation.
    matches!(
        // Use the supplied command.
        command,
        // Include remove child.
        ControlCommandKind::RemoveChild
            // Include add child.
            | ControlCommandKind::AddChild
            // Include shutdown tree.
            | ControlCommandKind::ShutdownTree // End dangerous command match.
                                               // Continue the demo expression.
    )
    // End dangerous command predicate.
}

/// Returns whether add-child is missing a manifest.
///
/// # Arguments
///
/// - `command`: Command request.
///
/// # Returns
///
/// Returns true when the manifest is absent or blank.
fn missing_child_manifest(command: &ControlCommandRequest) -> bool {
    // Read the optional manifest.
    let manifest = command.target.child_manifest.as_deref().unwrap_or_default();
    // Return blank status.
    manifest.trim().is_empty()
    // End manifest predicate.
}

/// Returns lifecycle after a command.
///
/// # Arguments
///
/// - `command`: Command kind.
///
/// # Returns
///
/// Returns the lifecycle label after command application.
fn lifecycle_after(command: ControlCommandKind) -> &'static str {
    // Match command lifecycle outcomes.
    match command {
        // Restarted children are restarting.
        ControlCommandKind::RestartChild => "restarting",
        // Paused children are paused.
        ControlCommandKind::PauseChild => "paused",
        // Resumed children are running.
        ControlCommandKind::ResumeChild => "running",
        // Quarantined children are quarantined.
        ControlCommandKind::QuarantineChild => "quarantined",
        // Removed children are removed.
        ControlCommandKind::RemoveChild => "removed",
        // Added children are running.
        ControlCommandKind::AddChild => "running",
        // Shutdown is represented separately.
        ControlCommandKind::ShutdownTree => "stopped",
        // End lifecycle match.
    }
    // End lifecycle lookup.
}

/// Returns the command wire name.
///
/// # Arguments
///
/// - `command`: Command kind.
///
/// # Returns
///
/// Returns the command label.
fn command_name(command: ControlCommandKind) -> &'static str {
    // Match command names.
    match command {
        // Return restart name.
        ControlCommandKind::RestartChild => "restart_child",
        // Return pause name.
        ControlCommandKind::PauseChild => "pause_child",
        // Return resume name.
        ControlCommandKind::ResumeChild => "resume_child",
        // Return quarantine name.
        ControlCommandKind::QuarantineChild => "quarantine_child",
        // Return remove name.
        ControlCommandKind::RemoveChild => "remove_child",
        // Return add name.
        ControlCommandKind::AddChild => "add_child",
        // Return shutdown name.
        ControlCommandKind::ShutdownTree => "shutdown_tree",
        // End command name match.
    }
    // End command name lookup.
}

/// Builds child path from identifier.
///
/// # Arguments
///
/// - `child_id`: Child identifier.
///
/// # Returns
///
/// Returns an absolute demo child path.
fn child_path(child_id: &str) -> String {
    // Build the path string.
    format!("{ROOT_PATH}/{child_id}")
    // End child path construction.
}

/// Returns event type for one child state.
///
/// # Arguments
///
/// - `child`: Child row.
///
/// # Returns
///
/// Returns a dashboard event type.
fn event_type(child: &DemoChild) -> &'static str {
    // Match lifecycle state.
    match child.lifecycle.as_str() {
        // Failed rows emit child_failed.
        "failed" => "child_failed",
        // Restarting rows emit child_restarted.
        "restarting" => "child_restarted",
        // Paused rows emit child_paused.
        "paused" => "child_paused",
        // Quarantined rows emit child_quarantined.
        "quarantined" => "child_quarantined",
        // Other rows emit child_running.
        _ => "child_running",
        // End event type match.
    }
    // End event type lookup.
}

/// Returns severity for one child state.
///
/// # Arguments
///
/// - `child`: Child row.
///
/// # Returns
///
/// Returns a dashboard severity label.
fn severity(child: &DemoChild) -> &'static str {
    // Match lifecycle state.
    match child.lifecycle.as_str() {
        // Failed and quarantined rows are errors.
        "failed" | "quarantined" => "error",
        // Restarting rows are warnings.
        "restarting" => "warning",
        // Other rows are informational.
        _ => "info",
        // End severity match.
    }
    // End severity lookup.
}

/// Returns last failure for failed children.
///
/// # Arguments
///
/// - `child`: Child row.
///
/// # Returns
///
/// Returns an optional failure string.
fn last_failure(child: &DemoChild) -> Option<String> {
    // Return failure text for failed rows.
    if child.lifecycle == "failed" {
        // Return the failure string.
        Some("duplicate event window exceeded".to_owned())
        // End failed branch.
    } else {
        // Return no failure for other rows.
        None
        // End non-failed branch.
    }
    // End failure lookup.
}

/// Returns last policy decision for active policy rows.
///
/// # Arguments
///
/// - `child`: Child row.
///
/// # Returns
///
/// Returns an optional policy decision string.
fn last_policy_decision(child: &DemoChild) -> Option<String> {
    // Return policy summary for notable states.
    if child.lifecycle == "failed" || child.lifecycle == "quarantined" {
        // Return quarantine policy.
        Some("quarantine".to_owned())
        // End policy branch.
    } else if child.lifecycle == "restarting" {
        // Return restart policy.
        Some("restart".to_owned())
        // End restart branch.
    } else {
        // Return no policy summary.
        None
        // End default branch.
    }
    // End policy lookup.
}

/// Builds a validation error for the scenario.
///
/// # Arguments
///
/// - `target_id`: Target process identifier.
/// - `message`: Validation message.
///
/// # Returns
///
/// Returns a dashboard validation error.
fn validation(target_id: &str, message: &str) -> DashboardError {
    // Create a target-scoped validation error.
    DashboardError::validation("command_validate", Some(target_id.to_owned()), message)
    // End validation construction.
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
    // Read duration since Unix epoch.
    std::time::SystemTime::now()
        // Convert the system time into a duration.
        .duration_since(std::time::UNIX_EPOCH)
        // Fall back to zero on clock skew.
        .unwrap_or(std::time::Duration::ZERO)
        // Convert to nanoseconds.
        .as_nanos()
    // End time conversion.
}

// Compile tests only when the demo example is tested directly.
#[cfg(test)]
// Group scenario tests with the scenario module.
mod tests {
    // Import the scenario under test.
    use super::DemoScenario;
    // Import command model values for command tests.
    use rust_supervisor::dashboard::model::{
        // Import command kind values.
        ControlCommandKind,
        // Import command request shape.
        ControlCommandRequest,
        // Import command target shape.
        ControlCommandTarget,
        // Import dashboard state shape.
        DashboardState,
        // End imports.
    };

    // Define the target identifier used by tests.
    const TEST_TARGET_ID: &str = "payments-worker-a";
    // Define the target display name used by tests.
    const TEST_DISPLAY_NAME: &str = "payments worker a";

    /// Verifies that the state payload covers the visible UI surface.
    #[test]
    /// Runs the state surface test.
    fn state_contains_ui_surface() {
        // Build the demo scenario.
        let scenario = scenario();
        // Build the current state.
        let state = scenario.state();
        // Assert topology has root plus children.
        assert!(state.topology.nodes.len() >= 6);
        // Assert runtime rows include all demo children.
        assert!(state.runtime_state.len() >= 5);
        // Assert recent events are visible.
        assert!(!state.recent_events.is_empty());
        // Assert recent logs are visible.
        assert!(!state.recent_logs.is_empty());
        // Assert dropped event count is visible.
        assert_eq!(state.dropped_event_count, 2);
        // Assert dropped log count is visible.
        assert_eq!(state.dropped_log_count, 1);
        // Assert all lifecycle states are present.
        for lifecycle in ["failed", "restarting", "paused", "quarantined", "running"] {
            // Check the lifecycle state.
            let present = has_lifecycle(&state, lifecycle);
            // Assert lifecycle presence.
            assert!(present);
            // End lifecycle assertion.
        }
        // End state surface test.
    }

    /// Verifies that command results preserve command identifiers.
    #[test]
    /// Runs the command result test.
    fn command_result_preserves_command_id() {
        // Build the demo scenario.
        let scenario = scenario();
        // Build a pause command.
        let command = command(
            // Use pause command.
            ControlCommandKind::PauseChild,
            // Use command identifier.
            "cmd-1",
            // Use target child path.
            "/root/healthy_worker",
            // Mark confirmation present.
            true,
            // End pause command construction.
        );
        // Apply the command.
        let result = scenario.command_result(command).expect("command result");
        // Assert command identifier is preserved.
        assert_eq!(result.command_id, "cmd-1");
        // Assert command completed.
        assert_eq!(result.status, "completed");
        // Build updated state.
        let state = scenario.state();
        // Assert the child was paused.
        let paused = has_child_state(&state, "/root/healthy_worker", "paused");
        // Assert paused child state.
        assert!(paused);
        // End command result test.
    }

    /// Verifies that pause command deltas include UI-consumable runtime rows.
    #[test]
    /// Runs the pause command delta test.
    fn pause_child_delta_contains_runtime_state() {
        // Build the demo scenario.
        let scenario = scenario();
        // Build a pause command.
        let command = command(
            // Use pause command.
            ControlCommandKind::PauseChild,
            // Use command identifier.
            "cmd-pause",
            // Use target child path.
            "/root/healthy_worker",
            // Mark confirmation present.
            true,
            // End pause command construction.
        );
        // Apply the command.
        let result = scenario.command_result(command).expect("command result");
        // Read the delta.
        let delta = result.state_delta.expect("state delta");
        // Check the paused child appears in delta.
        let paused = delta_runtime_has_child_state(&delta, "/root/healthy_worker", "paused");
        // Assert the paused child appears in delta.
        assert!(paused);
        // End pause delta test.
    }

    /// Verifies that remove command deltas update topology and include logs.
    #[test]
    /// Runs the remove command delta test.
    fn remove_child_delta_removes_topology_node_and_logs_command() {
        // Build the demo scenario.
        let scenario = scenario();
        // Build a remove command.
        let command = command(
            // Use remove command.
            ControlCommandKind::RemoveChild,
            // Use command identifier.
            "cmd-remove",
            // Use target child path.
            "/root/healthy_worker",
            // Mark confirmation present.
            true,
            // End remove command construction.
        );
        // Apply the command.
        let result = scenario.command_result(command).expect("command result");
        // Read the delta.
        let delta = result.state_delta.expect("state delta");
        // Check removed child is absent.
        let removed = !delta_topology_has_node(&delta, "/root/healthy_worker");
        // Assert removed child is absent.
        assert!(removed);
        // Check remove command log exists.
        let logged = delta_logs_contain(&delta, "remove_child");
        // Assert remove command log exists.
        assert!(logged);
        // End remove delta test.
    }

    /// Verifies that command activity describes lifecycle transitions.
    #[test]
    /// Runs the lifecycle transition test.
    fn command_delta_describes_lifecycle_transition() {
        // Build the demo scenario.
        let scenario = scenario();
        // Build a pause command.
        let command = command(
            // Use pause command.
            ControlCommandKind::PauseChild,
            // Use command identifier.
            "cmd-transition",
            // Use target child path.
            "/root/healthy_worker",
            // Mark confirmation present.
            true,
            // End pause command construction.
        );
        // Apply the command.
        let result = scenario.command_result(command).expect("command result");
        // Read the delta.
        let delta = result.state_delta.expect("state delta");
        // Assert the previous lifecycle is present.
        assert_eq!(
            // Read previous lifecycle from event payload.
            delta["recent_events"][0]["payload"]["previous_lifecycle_state"].as_str(),
            // Compare expected previous lifecycle.
            Some("running"),
            // End previous lifecycle assertion.
        );
        // Assert the current lifecycle is present.
        assert_eq!(
            // Read current lifecycle from event payload.
            delta["recent_events"][0]["payload"]["lifecycle_state"].as_str(),
            // Compare expected current lifecycle.
            Some("paused"),
            // End current lifecycle assertion.
        );
        // Read the log message.
        let message = delta["recent_logs"][0]["message"]
            // Interpret the log message as a string.
            .as_str()
            // Fall back to an empty string.
            .unwrap_or_default();
        // Assert the log message describes the transition.
        assert!(message.contains("running to paused"));
        // Assert structured log fields describe the previous lifecycle.
        assert_eq!(
            // Read previous lifecycle from log fields.
            delta["recent_logs"][0]["fields"]["previous_lifecycle_state"].as_str(),
            // Compare expected previous lifecycle.
            Some("running"),
            // End previous log lifecycle assertion.
        );
        // Assert structured log fields describe the current lifecycle.
        assert_eq!(
            // Read current lifecycle from log fields.
            delta["recent_logs"][0]["fields"]["lifecycle_state"].as_str(),
            // Compare expected current lifecycle.
            Some("paused"),
            // End current log lifecycle assertion.
        );
        // End transition delta test.
    }

    /// Verifies that add-child requires a manifest.
    #[test]
    /// Runs the add-child validation test.
    fn add_child_requires_manifest() {
        // Build the demo scenario.
        let scenario = scenario();
        // Build an add-child command without a manifest.
        let command = command(
            // Use add-child command.
            ControlCommandKind::AddChild,
            // Use command identifier.
            "cmd-2",
            // Use target child path.
            "/root/new_worker",
            // Mark confirmation present.
            true,
            // End add-child command construction.
        );
        // Apply the command.
        let error = scenario
            // Apply command.
            .command_result(command)
            // Expect validation error.
            .expect_err("validation error");
        // Assert validation failure is returned.
        assert_eq!(error.code, "validation_failed");
        // End validation test.
    }

    /// Builds a demo command request.
    ///
    /// # Arguments
    ///
    /// - `kind`: Command kind.
    /// - `command_id`: Command identifier.
    /// - `child_path`: Child path.
    /// - `confirmed`: Confirmation flag.
    ///
    /// # Returns
    ///
    /// Returns a command request.
    fn command(
        // Accept command kind.
        kind: ControlCommandKind,
        // Accept command identifier.
        command_id: &str,
        // Accept child path.
        child_path: &str,
        // Accept confirmation flag.
        confirmed: bool,
        // End command helper signature.
    ) -> ControlCommandRequest {
        // Build command request.
        ControlCommandRequest {
            // Include command identifier.
            command_id: command_id.to_owned(),
            // Include target identifier.
            target_id: TEST_TARGET_ID.to_owned(),
            // Include command kind.
            command: kind,
            // Include child target.
            target: ControlCommandTarget {
                // Include child path.
                child_path: Some(child_path.to_owned()),
                // Omit child manifest.
                child_manifest: None,
                // End command target.
            },
            // Include reason.
            reason: "demo test".to_owned(),
            // Include requester identity.
            requested_by: "tester".to_owned(),
            // Include confirmation.
            confirmed,
            // Include request time.
            requested_at_unix_nanos: 1,
            // End command request.
        }
        // End command construction.
    }

    /// Builds a default test scenario.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a demo scenario.
    fn scenario() -> DemoScenario {
        // Build the test scenario.
        DemoScenario::new(TEST_TARGET_ID.to_owned(), TEST_DISPLAY_NAME.to_owned())
        // End test scenario construction.
    }

    /// Checks whether a delta contains one runtime row with a lifecycle.
    fn delta_runtime_has_child_state(
        // Accept delta value.
        delta: &serde_json::Value,
        // Accept child path.
        child_path: &str,
        // Accept lifecycle.
        lifecycle: &str,
        // End helper signature.
    ) -> bool {
        // Access runtime state.
        let Some(rows) = delta
            // Select runtime state.
            .get("runtime_state")
            // Interpret runtime state as rows.
            .and_then(|value| value.as_array())
        // Handle missing runtime state.
        else {
            // Return absence.
            return false;
            // End missing runtime state branch.
        };
        // Scan runtime rows.
        rows.iter().any(|row| {
            // Compare child path.
            row.get("child_path").and_then(|value| value.as_str()) == Some(child_path)
                // Compare lifecycle state.
                && row.get("lifecycle_state").and_then(|value| value.as_str()) == Some(lifecycle)
            // End runtime row predicate.
        })
        // End runtime delta lookup.
    }

    /// Checks whether a delta topology contains one node path.
    fn delta_topology_has_node(
        // Accept delta value.
        delta: &serde_json::Value,
        // Accept node path.
        path: &str,
        // End helper signature.
    ) -> bool {
        // Access topology nodes.
        let Some(nodes) = delta
            // Access topology.
            .get("topology")
            // Access node list.
            .and_then(|value| value.get("nodes"))
            // Convert to array.
            .and_then(|value| value.as_array())
        // Handle missing topology.
        else {
            // Return absence.
            return false;
            // End missing topology branch.
        };
        // Scan topology nodes.
        nodes
            // Iterate nodes.
            .iter()
            // Match path.
            .any(|node| node.get("path").and_then(|value| value.as_str()) == Some(path))
        // End topology lookup.
    }

    /// Checks whether a delta log message contains text.
    fn delta_logs_contain(
        // Accept delta value.
        delta: &serde_json::Value,
        // Accept expected text.
        expected: &str,
        // End helper signature.
    ) -> bool {
        // Access recent logs.
        let Some(logs) = delta
            // Access recent logs.
            .get("recent_logs")
            // Convert to array.
            .and_then(|value| value.as_array())
        // Handle missing logs.
        else {
            // Return absence.
            return false;
            // End missing log branch.
        };
        // Scan log rows.
        logs
            // Iterate logs.
            .iter()
            // Match message text.
            .any(|log| {
                // Read message.
                log.get("message")
                    // Convert to string.
                    .and_then(|value| value.as_str())
                    // Match expected text.
                    .is_some_and(|message| message.contains(expected))
                // End log predicate.
            })
        // End log lookup.
    }

    /// Checks whether a lifecycle appears in state.
    ///
    /// # Arguments
    ///
    /// - `state`: Dashboard state.
    /// - `lifecycle`: Lifecycle label.
    ///
    /// # Returns
    ///
    /// Returns true when a runtime row has the lifecycle.
    fn has_lifecycle(state: &DashboardState, lifecycle: &str) -> bool {
        // Scan runtime rows.
        state
            // Access runtime rows.
            .runtime_state
            // Iterate rows.
            .iter()
            // Match lifecycle.
            .any(|row| row.lifecycle_state == lifecycle)
        // End lifecycle lookup.
    }

    /// Checks whether a child has a lifecycle.
    ///
    /// # Arguments
    ///
    /// - `state`: Dashboard state.
    /// - `child_path`: Child path.
    /// - `lifecycle`: Lifecycle label.
    ///
    /// # Returns
    ///
    /// Returns true when the child row has the lifecycle.
    fn has_child_state(state: &DashboardState, child_path: &str, lifecycle: &str) -> bool {
        // Scan runtime rows.
        state
            // Access runtime rows.
            .runtime_state
            // Iterate rows.
            .iter()
            // Match child path and lifecycle.
            .any(|row| row.child_path == child_path && row.lifecycle_state == lifecycle)
        // End child state lookup.
    }
    // End scenario tests.
}
