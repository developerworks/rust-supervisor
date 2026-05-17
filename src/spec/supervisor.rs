//! Supervisor declaration model.
//!
//! This module owns the root and nested supervisor specification shape used by
//! tree construction and runtime startup.

use crate::error::types::SupervisorError;
use crate::id::types::{ChildId, SupervisorPath};
use crate::policy::role_defaults::{WorkRole, semantic_conflicts_for_child};
use crate::spec::child::{BackoffPolicy, ChildSpec, HealthPolicy, RestartPolicy, ShutdownPolicy};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

/// Strategy used when a child exits and a restart scope is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SupervisionStrategy {
    /// Restart only the failed child.
    OneForOne,
    /// Restart every child under the same supervisor.
    OneForAll,
    /// Restart the failed child and all children declared after it.
    RestForOne,
}

/// Policy used when a restart scope cannot remain local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EscalationPolicy {
    /// Escalate the failure to the parent supervisor.
    EscalateToParent,
    /// Shut down the current supervisor tree.
    ShutdownTree,
    /// Quarantine the selected restart scope.
    QuarantineScope,
}

/// Restart limit attached to supervisor, group, or child override settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RestartLimit {
    /// Maximum allowed restart count inside the accounting window.
    pub max_restarts: u32,
    /// Accounting window used for restart counts.
    pub window: Duration,
}

impl RestartLimit {
    /// Creates a restart limit.
    ///
    /// # Arguments
    ///
    /// - `max_restarts`: Maximum allowed restart count inside the accounting window.
    /// - `window`: Accounting window used for restart counts.
    ///
    /// # Returns
    ///
    /// Returns a [`RestartLimit`] value.
    pub fn new(max_restarts: u32, window: Duration) -> Self {
        Self {
            max_restarts,
            window,
        }
    }
}

/// Strategy and governance overrides for a named child group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupStrategy {
    /// Low-cardinality group tag shared by children.
    pub group: String,
    /// Restart strategy applied inside the group.
    pub strategy: SupervisionStrategy,
    /// Optional restart limit for this group.
    pub restart_limit: Option<RestartLimit>,
    /// Optional escalation policy for this group.
    pub escalation_policy: Option<EscalationPolicy>,
}

impl GroupStrategy {
    /// Creates a group strategy.
    ///
    /// # Arguments
    ///
    /// - `group`: Child tag that identifies the restart group.
    /// - `strategy`: Restart strategy applied to the group.
    ///
    /// # Returns
    ///
    /// Returns a [`GroupStrategy`] without restart limit or escalation override.
    pub fn new(group: impl Into<String>, strategy: SupervisionStrategy) -> Self {
        Self {
            group: group.into(),
            strategy,
            restart_limit: None,
            escalation_policy: None,
        }
    }
}

/// Per-child strategy and governance override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildStrategyOverride {
    /// Child identifier that owns the override.
    pub child_id: ChildId,
    /// Restart strategy used when this child fails.
    pub strategy: SupervisionStrategy,
    /// Optional restart limit for this child.
    pub restart_limit: Option<RestartLimit>,
    /// Optional escalation policy for this child.
    pub escalation_policy: Option<EscalationPolicy>,
}

impl ChildStrategyOverride {
    /// Creates a child strategy override.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier that owns the override.
    /// - `strategy`: Restart strategy used for the child.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildStrategyOverride`] value.
    pub fn new(child_id: ChildId, strategy: SupervisionStrategy) -> Self {
        Self {
            child_id,
            strategy,
            restart_limit: None,
            escalation_policy: None,
        }
    }
}

/// Dynamic supervisor policy for runtime child additions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicSupervisorPolicy {
    /// Whether runtime child additions are allowed.
    pub enabled: bool,
    /// Optional maximum number of declared and dynamic children.
    pub child_limit: Option<usize>,
}

impl DynamicSupervisorPolicy {
    /// Creates an unbounded dynamic supervisor policy.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a policy that allows dynamic child additions without a limit.
    pub fn unbounded() -> Self {
        Self {
            enabled: true,
            child_limit: None,
        }
    }

    /// Creates a limited dynamic supervisor policy.
    ///
    /// # Arguments
    ///
    /// - `child_limit`: Maximum declared and dynamic child count.
    ///
    /// # Returns
    ///
    /// Returns a policy that allows dynamic additions up to the limit.
    pub fn limited(child_limit: usize) -> Self {
        Self {
            enabled: true,
            child_limit: Some(child_limit),
        }
    }

    /// Reports whether another dynamic child can be added.
    ///
    /// # Arguments
    ///
    /// - `current_child_count`: Current declared plus dynamic child count.
    ///
    /// # Returns
    ///
    /// Returns `true` when the next addition is allowed.
    pub fn allows_addition(&self, current_child_count: usize) -> bool {
        self.enabled
            && self
                .child_limit
                .is_none_or(|limit| current_child_count < limit)
    }
}

/// Restart plan selected after strategy, group, and child overrides are merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyExecutionPlan {
    /// Child whose exit triggered the plan.
    pub failed_child: ChildId,
    /// Strategy selected for this execution.
    pub strategy: SupervisionStrategy,
    /// Child identifiers selected for restart.
    pub scope: Vec<ChildId>,
    /// Optional group that constrained the scope.
    pub group: Option<String>,
    /// Optional restart limit selected by this execution plan.
    pub restart_limit: Option<RestartLimit>,
    /// Optional escalation policy selected for the plan.
    pub escalation_policy: Option<EscalationPolicy>,
    /// Whether dynamic supervisor additions are allowed.
    pub dynamic_supervisor_enabled: bool,
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
    /// Optional supervisor-level restart limit.
    pub restart_limit: Option<RestartLimit>,
    /// Optional supervisor-level escalation policy.
    pub escalation_policy: Option<EscalationPolicy>,
    /// Group-level strategy overrides.
    pub group_strategies: Vec<GroupStrategy>,
    /// Child-level strategy overrides.
    pub child_strategy_overrides: Vec<ChildStrategyOverride>,
    /// Runtime policy for dynamic child additions.
    pub dynamic_supervisor_policy: DynamicSupervisorPolicy,
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
            restart_limit: None,
            escalation_policy: None,
            group_strategies: Vec::new(),
            child_strategy_overrides: Vec::new(),
            dynamic_supervisor_policy: DynamicSupervisorPolicy::unbounded(),
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
        validate_restart_limit(self.restart_limit)?;
        validate_group_strategies(&self.group_strategies, &self.children)?;
        validate_child_strategy_overrides(self)?;
        validate_work_roles(&self.children)?;
        validate_dynamic_policy(self.dynamic_supervisor_policy)?;
        Ok(())
    }
}

/// Validates an optional restart limit.
///
/// # Arguments
///
/// - `limit`: Optional restart limit to validate.
///
/// # Returns
///
/// Returns `Ok(())` when the limit is absent or valid.
fn validate_restart_limit(limit: Option<RestartLimit>) -> Result<(), SupervisorError> {
    let Some(limit) = limit else {
        return Ok(());
    };
    if limit.max_restarts == 0 {
        return Err(SupervisorError::fatal_config(
            "restart limit max_restarts must be greater than zero",
        ));
    }
    if limit.window.is_zero() {
        return Err(SupervisorError::fatal_config(
            "restart limit window must be greater than zero",
        ));
    }
    Ok(())
}

/// Validates group strategy declarations.
///
/// # Arguments
///
/// - `strategies`: Group strategies declared on the supervisor.
///
/// # Returns
///
/// Returns `Ok(())` when group names are unique and valid.
fn validate_group_strategies(
    strategies: &[GroupStrategy],
    children: &[ChildSpec],
) -> Result<(), SupervisorError> {
    let mut groups = HashSet::new();
    for strategy in strategies {
        if strategy.group.trim().is_empty() {
            return Err(SupervisorError::fatal_config(
                "group strategy group must not be empty",
            ));
        }
        if !groups.insert(strategy.group.clone()) {
            return Err(SupervisorError::fatal_config(format!(
                "duplicate group strategy: {}",
                strategy.group
            )));
        }
        validate_restart_limit(strategy.restart_limit)?;
    }
    validate_group_membership(strategies, children)?;
    Ok(())
}

/// Validates child membership against configured restart groups.
///
/// # Arguments
///
/// - `strategies`: Group strategies declared on the supervisor.
/// - `children`: Children declared under the supervisor.
///
/// # Returns
///
/// Returns `Ok(())` when every configured group is used without ambiguity.
fn validate_group_membership(
    strategies: &[GroupStrategy],
    children: &[ChildSpec],
) -> Result<(), SupervisorError> {
    let groups = strategies
        .iter()
        .map(|strategy| strategy.group.clone())
        .collect::<HashSet<_>>();
    for strategy in strategies {
        if !children
            .iter()
            .any(|child| child.tags.contains(&strategy.group))
        {
            return Err(SupervisorError::fatal_config(format!(
                "group strategy references unused group: {}",
                strategy.group
            )));
        }
    }
    for child in children {
        let configured_group_count = child
            .tags
            .iter()
            .filter(|tag| groups.contains(*tag))
            .count();
        if configured_group_count > 1 {
            return Err(SupervisorError::fatal_config(format!(
                "child strategy groups are ambiguous for child: {}",
                child.id
            )));
        }
    }
    Ok(())
}

/// Validates child strategy overrides.
///
/// # Arguments
///
/// - `spec`: Supervisor specification that owns children and overrides.
///
/// # Returns
///
/// Returns `Ok(())` when every override targets a known child once.
fn validate_child_strategy_overrides(spec: &SupervisorSpec) -> Result<(), SupervisorError> {
    let child_ids = spec
        .children
        .iter()
        .map(|child| child.id.clone())
        .collect::<HashSet<_>>();
    let mut overrides = HashSet::new();
    for strategy in &spec.child_strategy_overrides {
        if !child_ids.contains(&strategy.child_id) {
            return Err(SupervisorError::fatal_config(format!(
                "child strategy override references unknown child: {}",
                strategy.child_id
            )));
        }
        if !overrides.insert(strategy.child_id.clone()) {
            return Err(SupervisorError::fatal_config(format!(
                "duplicate child strategy override: {}",
                strategy.child_id
            )));
        }
        validate_restart_limit(strategy.restart_limit)?;
    }
    Ok(())
}

/// Validates work role relationships that require sibling context.
///
/// # Arguments
///
/// - `children`: Children declared under one supervisor.
///
/// # Returns
///
/// Returns `Ok(())` when sidecar bindings and semantic diagnostics are valid.
fn validate_work_roles(children: &[ChildSpec]) -> Result<(), SupervisorError> {
    let child_ids = children
        .iter()
        .map(|child| child.id.clone())
        .collect::<HashSet<_>>();

    for child in children {
        emit_role_conflict_warnings(child);
        if child.work_role != Some(WorkRole::Sidecar) {
            continue;
        }

        let sidecar_config = child.sidecar_config.as_ref().ok_or_else(|| {
            SupervisorError::fatal_config(format!(
                "sidecar child {} requires sidecar_config",
                child.id
            ))
        })?;

        if !child_ids.contains(&sidecar_config.primary_child_id) {
            return Err(SupervisorError::fatal_config(format!(
                "sidecar child {} references unknown primary_child_id {}",
                child.id, sidecar_config.primary_child_id
            )));
        }

        let primary_child = children
            .iter()
            .find(|candidate| candidate.id == sidecar_config.primary_child_id)
            .ok_or_else(|| {
                SupervisorError::fatal_config(format!(
                    "sidecar child {} references unknown primary_child_id {}",
                    child.id, sidecar_config.primary_child_id
                ))
            })?;

        if primary_child.work_role == Some(WorkRole::Sidecar) {
            return Err(SupervisorError::fatal_config(format!(
                "sidecar child {} must not use another sidecar {} as primary_child_id",
                child.id, sidecar_config.primary_child_id
            )));
        }
    }

    Ok(())
}

/// Emits warning diagnostics for role semantic conflicts.
///
/// # Arguments
///
/// - `child`: Child specification being inspected.
///
/// # Returns
///
/// This function does not return a value.
fn emit_role_conflict_warnings(child: &ChildSpec) {
    for conflict in semantic_conflicts_for_child(child) {
        tracing::warn!(
            child_id = %conflict.child_id,
            work_role = %conflict.work_role,
            conflicting_field = %conflict.conflicting_field,
            user_value = %conflict.user_value,
            expected_semantic = %conflict.expected_semantic,
            reason = %conflict.reason,
            "work role semantic conflict"
        );
    }
}

/// Validates dynamic supervisor policy.
///
/// # Arguments
///
/// - `policy`: Dynamic supervisor policy to validate.
///
/// # Returns
///
/// Returns `Ok(())` when the policy limit is coherent.
fn validate_dynamic_policy(policy: DynamicSupervisorPolicy) -> Result<(), SupervisorError> {
    if policy.child_limit == Some(0) {
        return Err(SupervisorError::fatal_config(
            "dynamic supervisor child_limit must be greater than zero",
        ));
    }
    Ok(())
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
