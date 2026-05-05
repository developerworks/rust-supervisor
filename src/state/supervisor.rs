//! Supervisor current-state model.
//!
//! The module owns the current tree state returned to callers. It stores child
//! state by stable path and avoids retaining event history.

use crate::event::time::EventSequence;
use crate::id::types::SupervisorPath;
use crate::state::child::ChildState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Shutdown state visible in current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShutdownState {
    /// Shutdown has not started.
    Idle,
    /// Stop was requested.
    RequestStop,
    /// Runtime waits for graceful task completion.
    GracefulDrain,
    /// Runtime aborts straggling async workers.
    AbortStragglers,
    /// Runtime reconciles registry, state, metrics, and journal.
    Reconcile,
    /// Shutdown completed.
    Completed,
}

/// Meltdown status visible in current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeltdownState {
    /// No fuse is tripped.
    Clear,
    /// A child-level fuse is tripped.
    ChildFuseTripped {
        /// Path that tripped the fuse.
        path: SupervisorPath,
    },
    /// A supervisor-level fuse is tripped.
    SupervisorFuseTripped {
        /// Path that tripped the fuse.
        path: SupervisorPath,
    },
}

/// Current state for a supervisor tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisorState {
    /// Root path for this state response.
    pub root_path: SupervisorPath,
    /// Generated time in nanoseconds since the Unix epoch.
    pub generated_at_unix_nanos: u128,
    /// Sequence assigned to this state generation.
    pub sequence: EventSequence,
    /// Configuration version that produced this state.
    pub config_version: u64,
    /// Child states indexed by stable path text.
    pub children: BTreeMap<String, ChildState>,
    /// Current meltdown status.
    pub meltdown_state: MeltdownState,
    /// Current shutdown status.
    pub shutdown_state: ShutdownState,
    /// Last event journal sequence known to the state owner.
    pub journal_sequence: Option<EventSequence>,
}

impl SupervisorState {
    /// Creates an empty supervisor current state.
    ///
    /// # Arguments
    ///
    /// - `root_path`: Root path for the state response.
    /// - `sequence`: State generation sequence.
    /// - `config_version`: Configuration version that produced the state.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorState`] without children.
    ///
    /// # Examples
    ///
    /// ```
    /// let state = rust_supervisor::state::supervisor::SupervisorState::new(
    ///     rust_supervisor::id::types::SupervisorPath::root(),
    ///     rust_supervisor::event::time::EventSequence::new(1),
    ///     1,
    /// );
    /// assert!(state.children.is_empty());
    /// ```
    pub fn new(root_path: SupervisorPath, sequence: EventSequence, config_version: u64) -> Self {
        Self {
            root_path,
            generated_at_unix_nanos: crate::state::supervisor::unix_nanos_now(),
            sequence,
            config_version,
            children: BTreeMap::new(),
            meltdown_state: MeltdownState::Clear,
            shutdown_state: ShutdownState::Idle,
            journal_sequence: None,
        }
    }

    /// Inserts or replaces one child state.
    ///
    /// # Arguments
    ///
    /// - `child`: Current state for one child.
    ///
    /// # Returns
    ///
    /// Returns the updated [`SupervisorState`].
    pub fn with_child(mut self, child: ChildState) -> Self {
        self.children.insert(child.path.to_string(), child);
        self
    }

    /// Updates shutdown state.
    ///
    /// # Arguments
    ///
    /// - `shutdown_state`: New shutdown phase.
    ///
    /// # Returns
    ///
    /// Returns the updated [`SupervisorState`].
    pub fn with_shutdown_state(mut self, shutdown_state: ShutdownState) -> Self {
        self.shutdown_state = shutdown_state;
        self
    }

    /// Updates meltdown state.
    ///
    /// # Arguments
    ///
    /// - `meltdown_state`: New meltdown state.
    ///
    /// # Returns
    ///
    /// Returns the updated [`SupervisorState`].
    pub fn with_meltdown_state(mut self, meltdown_state: MeltdownState) -> Self {
        self.meltdown_state = meltdown_state;
        self
    }

    /// Records the latest journal sequence known to this state.
    ///
    /// # Arguments
    ///
    /// - `journal_sequence`: Latest event sequence from the journal.
    ///
    /// # Returns
    ///
    /// Returns the updated [`SupervisorState`].
    pub fn with_journal_sequence(mut self, journal_sequence: EventSequence) -> Self {
        self.journal_sequence = Some(journal_sequence);
        self
    }
}

/// Reads the current wall-clock time as nanoseconds since Unix epoch.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns zero when the system clock is before the Unix epoch.
fn unix_nanos_now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_nanos()
}
