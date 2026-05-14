//! Dashboard state construction.
//!
//! The builder combines static supervisor declarations, current runtime state,
//! and recent journal records into one payload that relay and UI can consume
//! after session binding or reconnect.

use crate::dashboard::events::{journal_to_event_records, log_record_for_event};
use crate::dashboard::model::{
    DashboardCriticality, DashboardState, RegistrationState, RuntimeState, SupervisorEdge,
    SupervisorEdgeKind, SupervisorNode, SupervisorNodeKind, SupervisorTopology,
    TargetConnectionState, TargetProcessIdentity,
};
use crate::id::types::SupervisorPath;
use crate::journal::ring::EventJournal;
use crate::spec::child::Criticality;
use crate::spec::supervisor::SupervisorSpec;
use crate::state::supervisor::SupervisorState;
use std::collections::BTreeMap;

/// Input required to build one dashboard state payload.
#[derive(Debug, Clone)]
pub struct DashboardStateInput {
    /// Stable target process identifier.
    pub target_id: String,
    /// Human-readable target display name.
    pub display_name: String,
    /// State generation assigned by the target process.
    pub state_generation: u64,
    /// Number of recent records to include.
    pub recent_limit: usize,
}

/// Builds dashboard state from current supervisor facts.
///
/// # Arguments
///
/// - `input`: Target identity and generation data.
/// - `spec`: Supervisor declaration.
/// - `state`: Current runtime state.
/// - `journal`: Recent event journal.
///
/// # Returns
///
/// Returns a [`DashboardState`] ready for IPC serialization.
pub fn build_dashboard_state(
    input: DashboardStateInput,
    spec: &SupervisorSpec,
    state: &SupervisorState,
    journal: &EventJournal,
) -> DashboardState {
    let config_version = spec.config_version.clone();
    let recent_events = journal_to_event_records(
        &input.target_id,
        &config_version,
        journal,
        input.recent_limit,
    );
    let recent_logs = recent_events
        .iter()
        .map(|event| log_record_for_event(event, format!("event {}", event.event_type)))
        .collect::<Vec<_>>();
    DashboardState {
        target: TargetProcessIdentity {
            target_id: input.target_id,
            display_name: input.display_name,
            registration_state: RegistrationState::Active,
            connection_state: TargetConnectionState::Registered,
        },
        topology: topology_from_spec(spec),
        runtime_state: runtime_state_rows(state),
        recent_events,
        recent_logs,
        dropped_event_count: journal.dropped_count,
        dropped_log_count: 0,
        config_version,
        generated_at_unix_nanos: state.generated_at_unix_nanos,
        state_generation: input.state_generation,
    }
}

/// Builds the supervisor topology from a declaration.
///
/// # Arguments
///
/// - `spec`: Supervisor declaration.
///
/// # Returns
///
/// Returns a topology with one root and declaration-order children.
pub fn topology_from_spec(spec: &SupervisorSpec) -> SupervisorTopology {
    let root_path = spec.path.to_string();
    let root = SupervisorNode {
        node_id: root_path.clone(),
        child_id: None,
        path: root_path.clone(),
        name: "root supervisor".to_owned(),
        kind: SupervisorNodeKind::RootSupervisor,
        tags: Vec::new(),
        criticality: DashboardCriticality::Critical,
        state_summary: "root".to_owned(),
        diagnostics: BTreeMap::new(),
    };
    let mut nodes = vec![root.clone()];
    let mut edges = Vec::new();
    let mut declaration_order = vec![root_path.clone()];
    for (index, child) in spec.children.iter().enumerate() {
        let child_path = spec.path.join(child.id.value.clone()).to_string();
        declaration_order.push(child_path.clone());
        nodes.push(SupervisorNode {
            node_id: child_path.clone(),
            child_id: Some(child.id.to_string()),
            path: child_path.clone(),
            name: child.name.clone(),
            kind: SupervisorNodeKind::ChildTask,
            tags: child.tags.clone(),
            criticality: criticality(child.criticality),
            state_summary: "declared".to_owned(),
            diagnostics: BTreeMap::new(),
        });
        edges.push(SupervisorEdge {
            edge_id: format!("parent:{root_path}->{child_path}"),
            source_path: root_path.clone(),
            target_path: child_path.clone(),
            kind: SupervisorEdgeKind::ParentChild,
            order: index,
        });
        for (dependency_index, dependency) in child.dependencies.iter().enumerate() {
            let dependency_path = spec.path.join(dependency.value.clone()).to_string();
            edges.push(SupervisorEdge {
                edge_id: format!("dependency:{dependency_path}->{child_path}"),
                source_path: dependency_path,
                target_path: child_path.clone(),
                kind: SupervisorEdgeKind::Dependency,
                order: dependency_index,
            });
        }
    }
    SupervisorTopology {
        root,
        nodes,
        edges,
        declaration_order,
    }
}

/// Converts current supervisor state to dashboard rows.
///
/// # Arguments
///
/// - `state`: Current supervisor state.
///
/// # Returns
///
/// Returns runtime state rows sorted by child path.
pub fn runtime_state_rows(state: &SupervisorState) -> Vec<RuntimeState> {
    state
        .children
        .values()
        .map(|child| RuntimeState {
            child_path: child.path.to_string(),
            lifecycle_state: child.state.as_label().to_owned(),
            health: format!("{:?}", child.health).to_lowercase(),
            readiness: format!("{:?}", child.readiness).to_lowercase(),
            generation: child.generation.value,
            child_start_count: child.child_start_count.value,
            restart_count: child.restart_count,
            last_failure: child
                .last_failure
                .as_ref()
                .map(|failure| format!("{failure:?}")),
            last_policy_decision: child
                .last_policy_decision
                .as_ref()
                .map(|decision| decision.decision.clone()),
            shutdown_state: format!("{:?}", state.shutdown_state).to_lowercase(),
        })
        .collect()
}

/// Converts child criticality to dashboard criticality.
///
/// # Arguments
///
/// - `value`: Child criticality from the supervisor declaration.
///
/// # Returns
///
/// Returns dashboard criticality.
fn criticality(value: Criticality) -> DashboardCriticality {
    match value {
        Criticality::Critical => DashboardCriticality::Critical,
        Criticality::Optional => DashboardCriticality::Standard,
    }
}

/// Creates an empty current state for a supervisor spec.
///
/// # Arguments
///
/// - `spec`: Supervisor declaration.
///
/// # Returns
///
/// Returns a current state with declared children.
pub fn declared_state_from_spec(spec: &SupervisorSpec) -> SupervisorState {
    spec.children.iter().fold(
        SupervisorState::new(
            SupervisorPath::root(),
            crate::event::time::EventSequence::new(1),
            1,
        ),
        |state, child| {
            let path = spec.path.join(child.id.value.clone());
            state.with_child(crate::state::child::ChildState::declared(
                path,
                child.id.clone(),
                child.name.clone(),
            ))
        },
    )
}
