//! Group isolation strategy module.
//!
//! Implements group-level fault boundary enforcement
//! with dependency edge propagation rules (US2: group fault stays within boundary).
//!
//! [`GroupDependencyEdge`] declares a directed failure propagation relationship
//! between two groups. [`PropagationPolicy`] controls the propagation semantics:
//! `None` (fully isolated), `EscalateOnly` (notify parent, don't block children),
//! or `Full` (mark all children in the dependent group as non-restartable).
//!
//! [`GroupIsolationPolicy`] evaluates whether a failure in one group affects
//! another, based on the declared DAG of dependency edges. Cyclic dependencies
//! are rejected at config load time.

use serde::{Deserialize, Serialize};

/// Failure propagation policy across group boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropagationPolicy {
    /// No propagation — groups are fully isolated.
    None,
    /// Escalate to parent supervisor only, do not affect current group.
    EscalateOnly,
    /// Full propagation — all children in the current group are marked
    /// non-restartable and the group enters meltdown.
    /// Propagation direction: fault flows from `to_group` to `from_group`
    /// (one-way), never reversed.
    Full,
}

/// Declares a failure propagation dependency between groups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupDependencyEdge {
    /// The group that depends on another group.
    pub from_group: String,
    /// The group that is depended on.
    pub to_group: String,
    /// How failures propagate from `to_group` to `from_group`.
    pub propagation: PropagationPolicy,
}

/// Evaluates whether a failure in one group affects another group.
///
/// Dependency edges form a directed acyclic graph (DAG). Cyclic dependencies
/// are detected at config load time and rejected with a structured error
/// listing the group names on the cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupIsolationPolicy {
    /// Declared cross-group dependency edges.
    dependencies: Vec<GroupDependencyEdge>,
}

impl GroupIsolationPolicy {
    /// Creates an isolation policy from declared dependency edges.
    pub fn new(dependencies: Vec<GroupDependencyEdge>) -> Self {
        Self { dependencies }
    }

    /// Checks whether `my_group` is affected by a failure in `failed_group`.
    ///
    /// Returns `true` when a dependency edge explicitly allows propagation,
    /// or when `my_group` is the same as `failed_group`.
    pub fn affected_by(&self, my_group: &str, failed_group: &str) -> bool {
        if my_group == failed_group {
            return true;
        }
        self.dependencies.iter().any(|edge| {
            edge.from_group == my_group
                && edge.to_group == failed_group
                && edge.propagation == PropagationPolicy::Full
        })
    }
}
