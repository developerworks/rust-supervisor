//! Supervisor declaration model.
//!
//! This module owns the root and nested supervisor specification shape used by
//! tree construction and runtime startup.

use crate::error::types::SupervisorError;
use crate::id::types::{ChildId, SupervisorPath};
use crate::policy::budget::RestartBudgetConfig;
use crate::policy::failure_window::FailureWindowConfig;
use crate::policy::group::GroupDependencyEdge;
use crate::policy::meltdown::MeltdownPolicy;
use crate::policy::task_role_defaults::{SeverityClass, TaskRole, semantic_conflicts_for_child};
use crate::spec::child::{BackoffPolicy, ChildSpec, HealthPolicy, RestartPolicy};
use crate::spec::shutdown::{ShutdownBudget, TreeShutdownPolicy};
use confique::Config;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Recommended default capacity for supervisor control and event channels.
pub const RECOMMENDED_CHANNEL_CAPACITY: usize = 256;

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

/// Group-level configuration for restart budget, dependency edges, and
/// severity defaults used by US1/US2/US3 policy evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupConfig {
    /// Low-cardinality group name shared by member children.
    pub name: String,
    /// Child identifiers that belong to this group.
    pub children: Vec<ChildId>,
    /// Restart budget configuration applied to this group.
    ///
    /// When `None`, the supervisor-level default budget is inherited.
    /// If the supervisor also has no default, [`RestartBudgetConfig::safe_default`]
    /// is used as a fallback.
    pub budget: Option<RestartBudgetConfig>,
}

impl GroupConfig {
    /// Creates a group configuration.
    ///
    /// # Arguments
    ///
    /// - `name`: Group name.
    /// - `children`: Child identifiers belonging to this group.
    /// - `budget`: Restart budget configuration for the group (None = inherit).
    ///
    /// # Returns
    ///
    /// Returns a [`GroupConfig`].
    pub fn new(
        name: impl Into<String>,
        children: Vec<ChildId>,
        budget: Option<RestartBudgetConfig>,
    ) -> Self {
        Self {
            name: name.into(),
            children,
            budget,
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

/// Backpressure strategy for slow event subscribers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackpressureStrategy {
    /// Alert and block the producer when buffers fill up; never drop events.
    AlertAndBlock,
    /// Sample and discard events when buffers fill up; record the ratio in the audit trail.
    SampleAndAudit,
}

impl Default for BackpressureStrategy {
    /// Returns the default non-dropping backpressure strategy.
    fn default() -> Self {
        Self::AlertAndBlock
    }
}

/// Configuration for event subscriber backpressure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct BackpressureConfig {
    /// Backpressure strategy selection.
    #[config(default = "alert_and_block")]
    #[serde(default)]
    pub strategy: BackpressureStrategy,
    /// Buffer occupancy soft threshold percentage (triggers warning alert).
    #[config(default = 80)]
    #[serde(default = "default_warn_threshold")]
    pub warn_threshold_pct: u8,
    /// Buffer occupancy hard threshold percentage (triggers degradation).
    #[config(default = 95)]
    #[serde(default = "default_critical_threshold")]
    pub critical_threshold_pct: u8,
    /// Sliding window duration in seconds for backpressure evaluation.
    #[config(default = 30)]
    #[serde(default = "default_window_secs")]
    pub window_secs: u64,
    /// Capacity of the dedicated audit channel.
    #[config(default = 1024)]
    #[serde(default = "default_audit_capacity")]
    pub audit_channel_capacity: usize,
}

/// Returns the default backpressure warning threshold (80%).
fn default_warn_threshold() -> u8 {
    80
}

/// Returns the default backpressure critical threshold (95%).
fn default_critical_threshold() -> u8 {
    95
}

/// Returns the default backpressure evaluation window in seconds (30).
fn default_window_secs() -> u64 {
    30
}

/// Returns the default audit channel capacity (1024).
fn default_audit_capacity() -> usize {
    1024
}

impl Default for BackpressureConfig {
    /// Returns the default backpressure configuration with `AlertAndBlock` strategy.
    fn default() -> Self {
        Self {
            strategy: BackpressureStrategy::AlertAndBlock,
            warn_threshold_pct: default_warn_threshold(),
            critical_threshold_pct: default_critical_threshold(),
            window_secs: default_window_secs(),
            audit_channel_capacity: default_audit_capacity(),
        }
    }
}

/// Declarative specification for one supervisor node.
#[derive(Debug, Clone)]
pub struct SupervisorSpec {
    /// Path for this supervisor.
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
    /// Tree shutdown policy for this supervisor and default child budgets.
    pub tree_shutdown: TreeShutdownPolicy,
    /// Maximum supervisor failures before the supervisor-level escalation path is selected.
    pub supervisor_failure_limit: u32,
    /// Optional supervisor-level restart limit.
    pub restart_limit: Option<RestartLimit>,
    /// Optional fallback escalation policy for execution plans that do not define
    /// a child-level or group-level policy.
    ///
    /// Root supervisors do not have a parent supervisor at runtime. When this
    /// field is set on a root supervisor, `EscalateToParent` should be treated
    /// as a configured policy label for planning and diagnostics, not as proof
    /// that a parent supervisor exists.
    pub escalation_policy: Option<EscalationPolicy>,
    /// Group-level strategy overrides.
    pub group_strategies: Vec<GroupStrategy>,
    /// Group-level configurations for restart budget, membership, and isolation.
    pub group_configs: Vec<GroupConfig>,
    /// Cross-group dependency edges for fault propagation.
    pub group_dependencies: Vec<GroupDependencyEdge>,
    /// Default severity class per task role for escalation bifurcation (US3).
    pub severity_defaults: HashMap<TaskRole, SeverityClass>,
    /// Child-level strategy overrides.
    pub child_strategy_overrides: Vec<ChildStrategyOverride>,
    /// Runtime policy for dynamic child additions.
    pub dynamic_supervisor_policy: DynamicSupervisorPolicy,
    /// Control command channel capacity.
    ///
    /// Runtime capacity for queued supervisor control commands.
    /// Recommended default: [`RECOMMENDED_CHANNEL_CAPACITY`].
    pub control_channel_capacity: usize,
    /// Event broadcast channel capacity.
    ///
    /// Runtime capacity for supervisor event broadcast delivery.
    /// Recommended default: [`RECOMMENDED_CHANNEL_CAPACITY`].
    pub event_channel_capacity: usize,
    /// Backpressure policy used by observability event subscribers.
    pub backpressure_config: BackpressureConfig,
    /// Failure fuse policy used by the supervision pipeline.
    pub meltdown_policy: MeltdownPolicy,
    /// Failure accumulation window used by the supervision pipeline.
    pub failure_window_config: FailureWindowConfig,
    /// Restart budget used by the supervision pipeline.
    pub restart_budget_config: RestartBudgetConfig,
    /// Event journal capacity used by the supervision pipeline.
    pub pipeline_journal_capacity: usize,
    /// Subscriber queue capacity used by the supervision pipeline.
    pub pipeline_subscriber_capacity: usize,
    /// Maximum concurrent restarts allowed for this supervisor instance.
    pub concurrent_restart_limit: u32,
    /// Whether metrics recording is enabled.
    pub metrics_enabled: bool,
    /// Whether audit event recording is enabled.
    pub audit_enabled: bool,
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
        let tree_shutdown = TreeShutdownPolicy::default();
        let mut spec = Self {
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
            tree_shutdown,
            supervisor_failure_limit: 1,
            restart_limit: None,
            escalation_policy: None,
            group_strategies: Vec::new(),
            group_configs: Vec::new(),
            group_dependencies: Vec::new(),
            severity_defaults: HashMap::new(),
            child_strategy_overrides: Vec::new(),
            dynamic_supervisor_policy: DynamicSupervisorPolicy::unbounded(),
            control_channel_capacity: RECOMMENDED_CHANNEL_CAPACITY,
            event_channel_capacity: RECOMMENDED_CHANNEL_CAPACITY,
            backpressure_config: BackpressureConfig::default(),
            meltdown_policy: MeltdownPolicy::new(
                3,
                Duration::from_secs(10),
                5,
                Duration::from_secs(30),
                10,
                Duration::from_secs(60),
                Duration::from_secs(120),
            ),
            failure_window_config: FailureWindowConfig::time_sliding(60, 5),
            restart_budget_config: RestartBudgetConfig::safe_default(),
            pipeline_journal_capacity: 100,
            pipeline_subscriber_capacity: 10,
            concurrent_restart_limit: 5,
            metrics_enabled: true,
            audit_enabled: true,
        };
        spec.propagate_tree_shutdown_budget();
        spec
    }

    /// Copies the tree shutdown budget into every declared child.
    ///
    /// Call this after changing [`Self::tree_shutdown`] when children should
    /// inherit the updated cooperative stop windows.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn propagate_tree_shutdown_budget(&mut self) {
        let budget = self.tree_shutdown.budget;
        for child in &mut self.children {
            child.shutdown_budget = budget;
        }
    }

    /// Returns the shutdown budget for one declared child.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    ///
    /// # Returns
    ///
    /// Returns the child override when present, otherwise the tree default budget.
    pub fn shutdown_budget_for(&self, child_id: &ChildId) -> ShutdownBudget {
        self.children
            .iter()
            .find(|child| &child.id == child_id)
            .map(|child| child.shutdown_budget)
            .unwrap_or(self.tree_shutdown.budget)
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
        validate_backpressure_config(&self.backpressure_config)?;
        for child in &self.children {
            child.validate()?;
        }
        validate_restart_limit(self.restart_limit)?;
        validate_group_strategies(&self.group_strategies, &self.children)?;
        validate_child_strategy_overrides(self)?;
        validate_task_roles(&self.children)?;
        validate_dynamic_policy(self.dynamic_supervisor_policy)?;
        validate_child_group_names(&self.children, &self.group_configs)?;
        validate_pipeline_policy(self)?;
        Ok(())
    }
}

/// Validates that every child referencing a group name actually points to an
/// existing [`GroupConfig`]. Unknown group names are rejected at load time
/// to prevent silent isolation failures due to typos.
fn validate_child_group_names(
    children: &[ChildSpec],
    group_configs: &[GroupConfig],
) -> Result<(), SupervisorError> {
    let group_names: std::collections::HashSet<&str> =
        group_configs.iter().map(|g| g.name.as_str()).collect();

    for child in children {
        if let Some(ref group_name) = child.group
            && !group_names.contains(group_name.as_str())
        {
            return Err(SupervisorError::fatal_config(format!(
                "child '{}' references unknown group '{}'; available groups: {:?}",
                child.id,
                group_name,
                group_names.iter().copied().collect::<Vec<_>>(),
            )));
        }
    }
    Ok(())
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
/// Returns `Ok(())` when every configured group strategy has at least one member child.
fn validate_group_membership(
    strategies: &[GroupStrategy],
    children: &[ChildSpec],
) -> Result<(), SupervisorError> {
    for strategy in strategies {
        if !children
            .iter()
            .any(|child| child.group.as_deref() == Some(strategy.group.as_str()))
        {
            return Err(SupervisorError::fatal_config(format!(
                "group strategy references unused group: {}",
                strategy.group
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

/// Validates task role relationships that require sibling context.
///
/// # Arguments
///
/// - `children`: Children declared under one supervisor.
///
/// # Returns
///
/// Returns `Ok(())` when sidecar bindings and semantic diagnostics are valid.
fn validate_task_roles(children: &[ChildSpec]) -> Result<(), SupervisorError> {
    let child_ids = children
        .iter()
        .map(|child| child.id.clone())
        .collect::<HashSet<_>>();

    for child in children {
        emit_role_conflict_warnings(child);
        if child.task_role != Some(TaskRole::Sidecar) {
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

        if primary_child.task_role == Some(TaskRole::Sidecar) {
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
            task_role = %conflict.task_role,
            conflicting_field = %conflict.conflicting_field,
            user_value = %conflict.user_value,
            expected_semantic = %conflict.expected_semantic,
            reason = %conflict.reason,
            "task role semantic conflict"
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

/// Validates supervision pipeline policy values.
///
/// # Arguments
///
/// - `spec`: Supervisor specification to validate.
///
/// # Returns
///
/// Returns `Ok(())` when pipeline policy values are usable.
fn validate_pipeline_policy(spec: &SupervisorSpec) -> Result<(), SupervisorError> {
    if spec.pipeline_journal_capacity == 0 {
        return Err(SupervisorError::fatal_config(
            "pipeline journal capacity must be greater than zero",
        ));
    }
    if spec.pipeline_subscriber_capacity == 0 {
        return Err(SupervisorError::fatal_config(
            "pipeline subscriber capacity must be greater than zero",
        ));
    }
    if spec.concurrent_restart_limit == 0 {
        return Err(SupervisorError::fatal_config(
            "concurrent restart limit must be greater than zero",
        ));
    }
    if spec.restart_budget_config.window.is_zero() {
        return Err(SupervisorError::fatal_config(
            "restart budget window must be greater than zero",
        ));
    }
    if spec.restart_budget_config.max_burst == 0 {
        return Err(SupervisorError::fatal_config(
            "restart budget max_burst must be greater than zero",
        ));
    }
    if spec.restart_budget_config.recovery_rate_per_sec <= 0.0 {
        return Err(SupervisorError::fatal_config(
            "restart budget recovery_rate_per_sec must be greater than zero",
        ));
    }
    Ok(())
}

/// Validates observability backpressure policy.
///
/// # Arguments
///
/// - `config`: Backpressure configuration to validate.
///
/// # Returns
///
/// Returns `Ok(())` when thresholds and capacities are coherent.
fn validate_backpressure_config(config: &BackpressureConfig) -> Result<(), SupervisorError> {
    if config.warn_threshold_pct == 0 || config.warn_threshold_pct > 100 {
        return Err(SupervisorError::fatal_config(
            "backpressure warn_threshold_pct must be between 1 and 100",
        ));
    }
    if config.critical_threshold_pct == 0 || config.critical_threshold_pct > 100 {
        return Err(SupervisorError::fatal_config(
            "backpressure critical_threshold_pct must be between 1 and 100",
        ));
    }
    if config.warn_threshold_pct >= config.critical_threshold_pct {
        return Err(SupervisorError::fatal_config(
            "backpressure warn_threshold_pct must be less than critical_threshold_pct",
        ));
    }
    if config.window_secs == 0 {
        return Err(SupervisorError::fatal_config(
            "backpressure window_secs must be greater than zero",
        ));
    }
    if config.audit_channel_capacity == 0 {
        return Err(SupervisorError::fatal_config(
            "backpressure audit_channel_capacity must be greater than zero",
        ));
    }
    Ok(())
}
