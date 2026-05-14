//! Child current-state model.
//!
//! The module owns the state visible through current-state queries. It keeps
//! lifecycle history out of state and stores only the latest operational facts.

use crate::error::types::TaskFailure;
use crate::event::payload::PolicyDecision;
use crate::event::time::EventSequence;
use crate::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use serde::{Deserialize, Serialize};

/// Lifecycle phase for a child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildLifecycleState {
    /// Child was declared but not started.
    Declared,
    /// Child is starting.
    Starting,
    /// Child is running.
    Running,
    /// Child reported readiness.
    Ready,
    /// Child is restarting after a policy decision.
    Restarting,
    /// Child is paused by control command.
    Paused,
    /// Child is isolated from automatic restart.
    Quarantined,
    /// Child is shutting down.
    ShuttingDown,
    /// Child stopped without an active failure.
    Stopped,
    /// Child failed and is terminal for automatic restart.
    Failed,
}

impl ChildLifecycleState {
    /// Returns a low-cardinality state label.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a stable state label.
    ///
    /// # Examples
    ///
    /// ```
    /// let state = rust_supervisor::state::child::ChildLifecycleState::Ready;
    /// assert_eq!(state.as_label(), "ready");
    /// ```
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Ready => "ready",
            Self::Restarting => "restarting",
            Self::Paused => "paused",
            Self::Quarantined => "quarantined",
            Self::ShuttingDown => "shutting_down",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }

    /// Reports whether automatic restart treats the state as terminal.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` for terminal states.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Quarantined | Self::Stopped | Self::Failed)
    }
}

/// Health status visible in current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildHealth {
    /// No health signal has been reported.
    Unknown,
    /// Latest health signal is healthy.
    Healthy,
    /// Latest health signal is stale.
    Stale,
    /// Latest health signal is unhealthy.
    Unhealthy,
}

/// Readiness status visible in current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildReadiness {
    /// Readiness is not configured or has not been requested.
    NotRequired,
    /// Explicit readiness is still pending.
    Pending,
    /// Child is ready.
    Ready,
}

/// Current state for one child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildState {
    /// Stable tree path for the child.
    pub path: SupervisorPath,
    /// Stable child identifier.
    pub id: ChildId,
    /// Human-readable child name.
    pub name: String,
    /// Current lifecycle state.
    pub state: ChildLifecycleState,
    /// Current health status.
    pub health: ChildHealth,
    /// Current generation.
    pub generation: Generation,
    /// Current child_start_count.
    pub child_start_count: ChildStartCount,
    /// Restart count inside the active restart window.
    pub restart_count: u64,
    /// Last typed task failure.
    pub last_failure: Option<TaskFailure>,
    /// Last event sequence that changed this state.
    pub last_event_sequence: Option<EventSequence>,
    /// Last policy decision produced for this child.
    pub last_policy_decision: Option<PolicyDecision>,
    /// Current readiness status.
    pub readiness: ChildReadiness,
}

impl ChildState {
    /// Creates a declared child state.
    ///
    /// # Arguments
    ///
    /// - `path`: Stable child path in the supervisor tree.
    /// - `id`: Stable child identifier.
    /// - `name`: Human-readable child name.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildState`] in the declared phase.
    ///
    /// # Examples
    ///
    /// ```
    /// let state = rust_supervisor::state::child::ChildState::declared(
    ///     rust_supervisor::id::types::SupervisorPath::root().join("worker"),
    ///     rust_supervisor::id::types::ChildId::new("worker"),
    ///     "Worker",
    /// );
    /// assert_eq!(state.state.as_label(), "declared");
    /// ```
    pub fn declared(path: SupervisorPath, id: ChildId, name: impl Into<String>) -> Self {
        Self {
            path,
            id,
            name: name.into(),
            state: ChildLifecycleState::Declared,
            health: ChildHealth::Unknown,
            generation: Generation::initial(),
            child_start_count: ChildStartCount::first(),
            restart_count: 0,
            last_failure: None,
            last_event_sequence: None,
            last_policy_decision: None,
            readiness: ChildReadiness::Pending,
        }
    }

    /// Returns a state with a new lifecycle phase and event sequence.
    ///
    /// # Arguments
    ///
    /// - `state`: New lifecycle state.
    /// - `sequence`: Event sequence that caused the change.
    ///
    /// # Returns
    ///
    /// Returns an updated [`ChildState`].
    pub fn with_lifecycle_state(
        mut self,
        state: ChildLifecycleState,
        sequence: EventSequence,
    ) -> Self {
        self.state = state;
        self.last_event_sequence = Some(sequence);
        self
    }

    /// Marks the child as ready.
    ///
    /// # Arguments
    ///
    /// - `sequence`: Event sequence that reported readiness.
    ///
    /// # Returns
    ///
    /// Returns an updated [`ChildState`].
    pub fn mark_ready(mut self, sequence: EventSequence) -> Self {
        self.state = ChildLifecycleState::Ready;
        self.readiness = ChildReadiness::Ready;
        self.last_event_sequence = Some(sequence);
        self
    }

    /// Records a typed failure.
    ///
    /// # Arguments
    ///
    /// - `failure`: Failure reported by the task.
    /// - `sequence`: Event sequence that reported the failure.
    ///
    /// # Returns
    ///
    /// Returns an updated [`ChildState`].
    pub fn record_failure(mut self, failure: TaskFailure, sequence: EventSequence) -> Self {
        self.state = ChildLifecycleState::Failed;
        self.health = ChildHealth::Unhealthy;
        self.last_failure = Some(failure);
        self.last_event_sequence = Some(sequence);
        self
    }

    /// Records a policy decision and restart count.
    ///
    /// # Arguments
    ///
    /// - `decision`: Policy decision attached to the child.
    /// - `restart_count`: Restart count after the decision.
    ///
    /// # Returns
    ///
    /// Returns an updated [`ChildState`].
    pub fn with_policy_decision(mut self, decision: PolicyDecision, restart_count: u64) -> Self {
        self.last_policy_decision = Some(decision);
        self.restart_count = restart_count;
        self
    }
}
