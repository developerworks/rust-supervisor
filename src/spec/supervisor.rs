//! Supervisor declaration model.
//!
//! This module owns the root and nested supervisor specification shape used by
//! tree construction and runtime startup.

use crate::error::types::SupervisorError;
use crate::id::types::SupervisorPath;
use crate::spec::child::{BackoffPolicy, ChildSpec, HealthPolicy, RestartPolicy, ShutdownPolicy};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Strategy used when a child exits and a restart scope is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisionStrategy {
    /// Restart only the failed child.
    OneForOne,
    /// Restart every child under the same supervisor.
    OneForAll,
    /// Restart the failed child and all children declared after it.
    RestForOne,
}

/// Declarative specification for one supervisor node.
#[derive(Debug, Clone)]
pub struct SupervisorSpec {
    /// Stable path for this supervisor.
    pub path: SupervisorPath,
    /// Restart scope strategy for child exits.
    pub strategy: SupervisionStrategy,
    /// Children in declaration order.
    pub children: Vec<ChildSpec>,
    /// Configuration version that produced this declaration.
    pub config_version: String,
    /// Restart policy inherited by children that do not override it.
    pub default_restart_policy: RestartPolicy,
    /// Backoff policy inherited by children that do not override it.
    pub default_backoff_policy: BackoffPolicy,
    /// Health policy inherited by children that do not override it.
    pub default_health_policy: HealthPolicy,
    /// Shutdown policy inherited by children that do not override it.
    pub default_shutdown_policy: ShutdownPolicy,
    /// Maximum supervisor failures before parent escalation.
    pub supervisor_failure_limit: u32,
    /// Control command channel capacity.
    pub control_channel_capacity: usize,
    /// Event broadcast channel capacity.
    pub event_channel_capacity: usize,
}

impl SupervisorSpec {
    /// Creates a root supervisor specification.
    ///
    /// # Arguments
    ///
    /// - `children`: Children declared under the root supervisor.
    ///
    /// # Returns
    ///
    /// Returns a root [`SupervisorSpec`] with declaration-order children.
    ///
    /// # Examples
    ///
    /// ```
    /// let spec = rust_supervisor::spec::supervisor::SupervisorSpec::root(Vec::new());
    /// assert_eq!(spec.path.to_string(), "/");
    /// ```
    pub fn root(children: Vec<ChildSpec>) -> Self {
        let channel_capacity = channel_capacity_for_children(children.len());
        Self {
            path: SupervisorPath::root(),
            strategy: SupervisionStrategy::OneForOne,
            children,
            config_version: String::from("unversioned"),
            default_restart_policy: RestartPolicy::Transient,
            default_backoff_policy: BackoffPolicy::new(
                Duration::from_millis(10),
                Duration::from_secs(1),
                0.0,
            ),
            default_health_policy: HealthPolicy::new(
                Duration::from_secs(1),
                Duration::from_secs(3),
            ),
            default_shutdown_policy: ShutdownPolicy::new(
                Duration::from_secs(5),
                Duration::from_secs(1),
            ),
            supervisor_failure_limit: 1,
            control_channel_capacity: channel_capacity,
            event_channel_capacity: channel_capacity.saturating_mul(2),
        }
    }

    /// Validates this supervisor and its direct children.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the supervisor declaration is usable.
    pub fn validate(&self) -> Result<(), SupervisorError> {
        if self.config_version.trim().is_empty() {
            return Err(SupervisorError::fatal_config(
                "config version must not be empty",
            ));
        }
        if self.supervisor_failure_limit == 0 {
            return Err(SupervisorError::fatal_config(
                "supervisor failure limit must be greater than zero",
            ));
        }
        if self.control_channel_capacity == 0 {
            return Err(SupervisorError::fatal_config(
                "control channel capacity must be greater than zero",
            ));
        }
        if self.event_channel_capacity == 0 {
            return Err(SupervisorError::fatal_config(
                "event channel capacity must be greater than zero",
            ));
        }
        for child in &self.children {
            child.validate()?;
        }
        Ok(())
    }
}

/// Derives a channel capacity from declared children.
///
/// # Arguments
///
/// - `child_count`: Number of children declared under the supervisor.
///
/// # Returns
///
/// Returns a non-zero channel capacity.
fn channel_capacity_for_children(child_count: usize) -> usize {
    child_count.saturating_add(1)
}
