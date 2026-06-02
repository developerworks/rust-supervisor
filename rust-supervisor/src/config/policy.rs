//! YAML-friendly policy configuration models.
//!
//! This module owns configuration input structs for lower-level supervision
//! policy objects whose runtime form uses `Duration`, `ChildId`, or other
//! strongly typed values.

use confique::Config;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::Duration;

use crate::id::types::ChildId;
use crate::policy::budget as runtime_budget;
use crate::policy::failure_window as runtime_failure_window;
use crate::policy::group::{GroupDependencyEdge, PropagationPolicy};
use crate::policy::meltdown::MeltdownPolicy;
use crate::policy::task_role_defaults::{SeverityClass, TaskRole};
use crate::spec::supervisor::{
    ChildStrategyOverride, DynamicSupervisorPolicy, EscalationPolicy,
    GroupConfig as RuntimeGroupConfig, GroupStrategy, RestartLimit, SupervisionStrategy,
};

/// Restart budget configuration loaded from YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct RestartBudgetConfig {
    /// Sliding window duration in seconds.
    #[config(default = 60)]
    #[serde(default = "default_restart_budget_window_secs")]
    pub window_secs: u64,
    /// Maximum burst failures allowed within the window.
    #[config(default = 10)]
    #[serde(default = "default_restart_budget_max_burst")]
    pub max_burst: u32,
    /// Token recovery rate per second.
    #[config(default = 0.5)]
    #[serde(default = "default_restart_budget_recovery_rate")]
    pub recovery_rate_per_sec: f64,
}

impl RestartBudgetConfig {
    /// Converts this YAML-friendly config into the runtime restart budget.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`runtime_budget::RestartBudgetConfig`] value.
    pub fn to_runtime(&self) -> runtime_budget::RestartBudgetConfig {
        runtime_budget::RestartBudgetConfig::new(
            Duration::from_secs(self.window_secs),
            self.max_burst,
            self.recovery_rate_per_sec,
        )
    }
}

impl Default for RestartBudgetConfig {
    /// Returns the default restart budget configuration.
    fn default() -> Self {
        Self {
            window_secs: default_restart_budget_window_secs(),
            max_burst: default_restart_budget_max_burst(),
            recovery_rate_per_sec: default_restart_budget_recovery_rate(),
        }
    }
}

/// Failure window mode loaded from YAML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureWindowMode {
    /// Count failures that occur inside a time window.
    TimeSliding,
    /// Keep the most recent failure samples by count.
    CountSliding,
}

impl Default for FailureWindowMode {
    /// Returns the default time-sliding mode.
    fn default() -> Self {
        Self::TimeSliding
    }
}

/// Failure window configuration loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct FailureWindowConfig {
    /// Window mode selection.
    #[config(default = "time_sliding")]
    #[serde(default)]
    pub mode: FailureWindowMode,
    /// Time window width in seconds for `time_sliding` mode.
    #[config(default = 60)]
    #[serde(default = "default_failure_window_secs")]
    pub window_secs: u64,
    /// Maximum retained failure count for `count_sliding` mode.
    #[config(default = 5)]
    #[serde(default = "default_failure_window_max_count")]
    pub max_count: usize,
    /// Failure threshold at which the window is considered exhausted.
    #[config(default = 5)]
    #[serde(default = "default_failure_window_threshold")]
    pub threshold: usize,
}

impl FailureWindowConfig {
    /// Converts this YAML-friendly config into the runtime failure window.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`runtime_failure_window::FailureWindowConfig`] value.
    pub fn to_runtime(&self) -> runtime_failure_window::FailureWindowConfig {
        match self.mode {
            FailureWindowMode::TimeSliding => {
                runtime_failure_window::FailureWindowConfig::time_sliding(
                    self.window_secs,
                    self.threshold,
                )
            }
            FailureWindowMode::CountSliding => {
                runtime_failure_window::FailureWindowConfig::count_sliding(
                    self.max_count,
                    self.threshold,
                )
            }
        }
    }
}

impl Default for FailureWindowConfig {
    /// Returns the default failure window configuration.
    fn default() -> Self {
        Self {
            mode: FailureWindowMode::default(),
            window_secs: default_failure_window_secs(),
            max_count: default_failure_window_max_count(),
            threshold: default_failure_window_threshold(),
        }
    }
}

/// Meltdown fuse configuration loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct MeltdownConfig {
    /// Maximum restarts allowed for one child inside the child window.
    #[config(default = 3)]
    #[serde(default = "default_meltdown_child_max_restarts")]
    pub child_max_restarts: u32,
    /// Window used to count child restarts, in seconds.
    #[config(default = 10)]
    #[serde(default = "default_meltdown_child_window_secs")]
    pub child_window_secs: u64,
    /// Maximum failures allowed for one group inside the group window.
    #[config(default = 5)]
    #[serde(default = "default_meltdown_group_max_failures")]
    pub group_max_failures: u32,
    /// Window used to count group failures, in seconds.
    #[config(default = 30)]
    #[serde(default = "default_meltdown_group_window_secs")]
    pub group_window_secs: u64,
    /// Maximum failures allowed for the supervisor inside the supervisor window.
    #[config(default = 10)]
    #[serde(default = "default_meltdown_supervisor_max_failures")]
    pub supervisor_max_failures: u32,
    /// Window used to count supervisor failures, in seconds.
    #[config(default = 60)]
    #[serde(default = "default_meltdown_supervisor_window_secs")]
    pub supervisor_window_secs: u64,
    /// Stable duration after which recorded counters may be cleared, in seconds.
    #[config(default = 120)]
    #[serde(default = "default_meltdown_reset_after_secs")]
    pub reset_after_secs: u64,
}

impl MeltdownConfig {
    /// Converts this YAML-friendly config into the runtime meltdown policy.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`MeltdownPolicy`] value.
    pub fn to_runtime(&self) -> MeltdownPolicy {
        MeltdownPolicy::new(
            self.child_max_restarts,
            Duration::from_secs(self.child_window_secs),
            self.group_max_failures,
            Duration::from_secs(self.group_window_secs),
            self.supervisor_max_failures,
            Duration::from_secs(self.supervisor_window_secs),
            Duration::from_secs(self.reset_after_secs),
        )
    }
}

impl Default for MeltdownConfig {
    /// Returns the default meltdown fuse configuration.
    fn default() -> Self {
        Self {
            child_max_restarts: default_meltdown_child_max_restarts(),
            child_window_secs: default_meltdown_child_window_secs(),
            group_max_failures: default_meltdown_group_max_failures(),
            group_window_secs: default_meltdown_group_window_secs(),
            supervisor_max_failures: default_meltdown_supervisor_max_failures(),
            supervisor_window_secs: default_meltdown_supervisor_window_secs(),
            reset_after_secs: default_meltdown_reset_after_secs(),
        }
    }
}

/// Supervision pipeline capacities loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct SupervisionPipelineConfig {
    /// Event journal capacity used by the supervision pipeline.
    #[config(default = 100)]
    #[serde(default = "default_pipeline_journal_capacity")]
    pub journal_capacity: usize,
    /// Subscriber queue capacity used by the supervision pipeline.
    #[config(default = 10)]
    #[serde(default = "default_pipeline_subscriber_capacity")]
    pub subscriber_capacity: usize,
    /// Maximum concurrent restarts allowed for one supervisor instance.
    #[config(default = 5)]
    #[serde(default = "default_concurrent_restart_limit")]
    pub concurrent_restart_limit: u32,
}

impl Default for SupervisionPipelineConfig {
    /// Returns the default supervision pipeline capacities.
    fn default() -> Self {
        Self {
            journal_capacity: default_pipeline_journal_capacity(),
            subscriber_capacity: default_pipeline_subscriber_capacity(),
            concurrent_restart_limit: default_concurrent_restart_limit(),
        }
    }
}

/// Dynamic child acceptance policy loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct DynamicSupervisorConfig {
    /// Whether runtime child additions are accepted.
    #[config(default = true)]
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional maximum number of declared and dynamic children.
    #[schemars(!default)]
    #[serde(default)]
    pub child_limit: Option<usize>,
}

impl DynamicSupervisorConfig {
    /// Converts this YAML-friendly config into the runtime dynamic policy.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`DynamicSupervisorPolicy`] value.
    pub fn to_runtime(&self) -> DynamicSupervisorPolicy {
        DynamicSupervisorPolicy {
            enabled: self.enabled,
            child_limit: self.child_limit,
        }
    }
}

impl Default for DynamicSupervisorConfig {
    /// Returns the default dynamic supervisor policy.
    fn default() -> Self {
        Self {
            enabled: true,
            child_limit: None,
        }
    }
}

/// Restart limit configuration loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct RestartLimitConfig {
    /// Maximum restart count inside the configured window.
    pub max_restarts: u32,
    /// Window used to count restarts, in milliseconds.
    pub window_ms: u64,
}

impl RestartLimitConfig {
    /// Converts this YAML-friendly config into a runtime restart limit.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`RestartLimit`] value.
    pub fn to_runtime(&self) -> RestartLimit {
        RestartLimit::new(self.max_restarts, Duration::from_millis(self.window_ms))
    }
}

/// Split-friendly nested section for group policy declarations.
///
/// Single-file configs use `groups: [...]`. Split `groups.yaml` files contain
/// only the group entry sequence for this section.
#[derive(Debug, Clone, PartialEq, Config)]
pub struct GroupsConfigSection {
    /// Group entries loaded from the `groups` configuration section.
    #[config(default = [])]
    pub items: Vec<GroupConfig>,
}

impl Default for GroupsConfigSection {
    /// Returns an empty group configuration section.
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl JsonSchema for GroupsConfigSection {
    /// Returns the schema name used for split group sections.
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("GroupsConfigSection")
    }

    /// Returns the transparent array schema for group declarations.
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        Vec::<GroupConfig>::json_schema(generator)
    }
}

impl Serialize for GroupsConfigSection {
    /// Serializes group section entries as a transparent array.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: confique::serde::Serializer,
    {
        self.items.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GroupsConfigSection {
    /// Deserializes group section entries from a transparent array.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: confique::serde::Deserializer<'de>,
    {
        Ok(Self {
            items: Vec::<GroupConfig>::deserialize(deserializer)?,
        })
    }
}

impl GroupsConfigSection {
    /// Returns group entries as a slice.
    pub fn as_slice(&self) -> &[GroupConfig] {
        &self.items
    }

    /// Returns the number of group entries.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns whether this section contains no group entries.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl From<GroupsConfigSection> for Vec<GroupConfig> {
    /// Converts a group section into its transparent entry vector.
    fn from(section: GroupsConfigSection) -> Self {
        section.items
    }
}

/// Group-level configuration loaded from YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct GroupConfig {
    /// Low-cardinality group name shared by member children.
    pub name: String,
    /// Optional group-specific restart budget override.
    #[schemars(!default)]
    #[serde(default)]
    pub budget: Option<RestartBudgetConfig>,
}

impl GroupConfig {
    /// Converts this YAML-friendly config into a runtime group config.
    ///
    /// # Arguments
    ///
    /// - `members`: Child identifiers derived from `children[].group` at load time.
    ///
    /// # Returns
    ///
    /// Returns a [`RuntimeGroupConfig`] value.
    pub fn to_runtime(&self, members: &[ChildId]) -> RuntimeGroupConfig {
        RuntimeGroupConfig::new(
            self.name.clone(),
            members.to_vec(),
            self.budget.as_ref().map(RestartBudgetConfig::to_runtime),
        )
    }
}

/// Group strategy override loaded from YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct GroupStrategyConfig {
    /// Group name that owns the strategy override.
    pub group: String,
    /// Restart strategy used when a member child fails.
    pub strategy: SupervisionStrategy,
    /// Optional group-level restart limit.
    #[schemars(!default)]
    #[serde(default)]
    pub restart_limit: Option<RestartLimitConfig>,
    /// Optional escalation policy for this group.
    #[schemars(!default)]
    #[serde(default)]
    pub escalation_policy: Option<EscalationPolicy>,
}

impl GroupStrategyConfig {
    /// Converts this YAML-friendly config into a runtime group strategy.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`GroupStrategy`] value.
    pub fn to_runtime(&self) -> GroupStrategy {
        let mut strategy = GroupStrategy::new(self.group.clone(), self.strategy);
        strategy.restart_limit = self
            .restart_limit
            .as_ref()
            .map(RestartLimitConfig::to_runtime);
        strategy.escalation_policy = self.escalation_policy;
        strategy
    }
}

/// Child strategy override loaded from YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct ChildStrategyOverrideConfig {
    /// Child name that owns the strategy override.
    pub child_id: String,
    /// Restart strategy used for this child.
    pub strategy: SupervisionStrategy,
    /// Optional child-level restart limit.
    #[schemars(!default)]
    #[serde(default)]
    pub restart_limit: Option<RestartLimitConfig>,
    /// Optional escalation policy for this child.
    #[schemars(!default)]
    #[serde(default)]
    pub escalation_policy: Option<EscalationPolicy>,
}

impl ChildStrategyOverrideConfig {
    /// Converts this YAML-friendly config into a runtime child strategy override.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildStrategyOverride`] value.
    pub fn to_runtime(&self) -> ChildStrategyOverride {
        let mut override_config =
            ChildStrategyOverride::new(ChildId::new(&self.child_id), self.strategy);
        override_config.restart_limit = self
            .restart_limit
            .as_ref()
            .map(RestartLimitConfig::to_runtime);
        override_config.escalation_policy = self.escalation_policy;
        override_config
    }
}

/// Group dependency edge loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct GroupDependencyConfig {
    /// Group that depends on another group.
    pub from_group: String,
    /// Group that is depended on.
    pub to_group: String,
    /// Failure propagation policy.
    pub propagation: PropagationPolicy,
}

impl GroupDependencyConfig {
    /// Converts this YAML-friendly config into a runtime dependency edge.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`GroupDependencyEdge`] value.
    pub fn to_runtime(&self) -> GroupDependencyEdge {
        GroupDependencyEdge {
            from_group: self.from_group.clone(),
            to_group: self.to_group.clone(),
            propagation: self.propagation,
        }
    }
}

/// Severity default loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct SeverityDefaultConfig {
    /// Task role that receives this default severity.
    pub task_role: TaskRole,
    /// Severity assigned to the task role.
    pub severity: SeverityClass,
}

/// Returns the default restart budget window in seconds.
fn default_restart_budget_window_secs() -> u64 {
    60
}

/// Returns the default restart budget burst.
fn default_restart_budget_max_burst() -> u32 {
    10
}

/// Returns the default restart budget recovery rate per second.
fn default_restart_budget_recovery_rate() -> f64 {
    0.5
}

/// Returns the default failure window width in seconds.
fn default_failure_window_secs() -> u64 {
    60
}

/// Returns the default retained failure count.
fn default_failure_window_max_count() -> usize {
    5
}

/// Returns the default failure threshold.
fn default_failure_window_threshold() -> usize {
    5
}

/// Returns the default child meltdown limit.
fn default_meltdown_child_max_restarts() -> u32 {
    3
}

/// Returns the default child meltdown window in seconds.
fn default_meltdown_child_window_secs() -> u64 {
    10
}

/// Returns the default group meltdown limit.
fn default_meltdown_group_max_failures() -> u32 {
    5
}

/// Returns the default group meltdown window in seconds.
fn default_meltdown_group_window_secs() -> u64 {
    30
}

/// Returns the default supervisor meltdown limit.
fn default_meltdown_supervisor_max_failures() -> u32 {
    10
}

/// Returns the default supervisor meltdown window in seconds.
fn default_meltdown_supervisor_window_secs() -> u64 {
    60
}

/// Returns the default stable reset window in seconds.
fn default_meltdown_reset_after_secs() -> u64 {
    120
}

/// Returns the default supervision pipeline journal capacity.
fn default_pipeline_journal_capacity() -> usize {
    100
}

/// Returns the default supervision pipeline subscriber capacity.
fn default_pipeline_subscriber_capacity() -> usize {
    10
}

/// Returns the default concurrent restart limit.
fn default_concurrent_restart_limit() -> u32 {
    5
}

/// Serde default helper: returns true.
fn default_true() -> bool {
    true
}
