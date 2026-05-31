//! Builder for [`SupervisorSpec`](crate::spec::supervisor::SupervisorSpec).
//!
//! Use this module when constructing supervisor specifications in code. The
//! builder mirrors [`SupervisorSpec::root`](crate::spec::supervisor::SupervisorSpec::root)
//! defaults, then lets callers override policy, topology, and runtime settings
//! through a fluent API.

use crate::error::types::SupervisorError;
use crate::id::types::SupervisorPath;
use crate::policy::budget::RestartBudgetConfig;
use crate::policy::failure_window::FailureWindowConfig;
use crate::policy::group::GroupDependencyEdge;
use crate::policy::meltdown::MeltdownPolicy;
use crate::policy::task_role_defaults::{SeverityClass, TaskRole};
use crate::spec::child::{BackoffPolicy, ChildSpec, HealthPolicy, RestartPolicy, ShutdownPolicy};
use crate::spec::supervisor::{
    BackpressureConfig, ChildStrategyOverride, DynamicSupervisorPolicy, EscalationPolicy,
    GroupConfig, GroupStrategy, RestartLimit, SupervisionStrategy, SupervisorSpec,
};
use std::collections::HashMap;
use std::time::Duration;

/// Builder for [`SupervisorSpec`](crate::spec::supervisor::SupervisorSpec).
///
/// Public constructors and setters keep returning [`SupervisorSpecBuilder`] for
/// chaining. Call [`build`](SupervisorSpecBuilder::build) to consume the
/// builder, validate local invariants, and receive the final [`SupervisorSpec`].
#[derive(Debug, Clone)]
pub struct SupervisorSpecBuilder {
    /// Supervisor specification under construction.
    spec: SupervisorSpec,
}

impl SupervisorSpecBuilder {
    /// Creates a root supervisor specification builder.
    ///
    /// # Arguments
    ///
    /// - `children`: Children declared under the root supervisor.
    ///
    /// # Returns
    ///
    /// Returns a builder seeded with the same defaults as [`SupervisorSpec::root`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::spec::supervisor_builder::SupervisorSpecBuilder;
    ///
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// let spec = SupervisorSpecBuilder::root(Vec::new()).build()?;
    /// assert_eq!(spec.path.to_string(), "/");
    /// # Ok(())
    /// # }
    /// ```
    pub fn root(children: Vec<ChildSpec>) -> Self {
        Self {
            spec: SupervisorSpec::root(children),
        }
    }

    /// Creates a root supervisor specification builder.
    ///
    /// # Arguments
    ///
    /// - `children`: Children declared under the root supervisor.
    ///
    /// # Returns
    ///
    /// Returns a builder seeded with the same defaults as [`SupervisorSpec::root`].
    pub fn new(children: Vec<ChildSpec>) -> Self {
        Self::root(children)
    }

    /// Sets the supervisor path.
    ///
    /// # Arguments
    ///
    /// - `path`: Stable path for this supervisor.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn path(mut self, path: SupervisorPath) -> Self {
        self.spec.path = path;
        self
    }

    /// Sets the restart scope strategy.
    ///
    /// # Arguments
    ///
    /// - `strategy`: Restart strategy for child exits.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn strategy(mut self, strategy: SupervisionStrategy) -> Self {
        self.spec.strategy = strategy;
        self
    }

    /// Replaces supervisor children.
    ///
    /// # Arguments
    ///
    /// - `children`: Children declared under the supervisor.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn children(mut self, children: Vec<ChildSpec>) -> Self {
        self.spec.children = children;
        self
    }

    /// Appends one supervisor child.
    ///
    /// # Arguments
    ///
    /// - `child`: Child specification appended in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn child(mut self, child: ChildSpec) -> Self {
        self.spec.children.push(child);
        self
    }

    /// Sets the configuration version.
    ///
    /// # Arguments
    ///
    /// - `config_version`: Version string that produced this declaration.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn config_version(mut self, config_version: impl Into<String>) -> Self {
        self.spec.config_version = config_version.into();
        self
    }

    /// Sets the default restart policy.
    ///
    /// # Arguments
    ///
    /// - `default_restart_policy`: Restart policy inherited by children.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn default_restart_policy(mut self, default_restart_policy: RestartPolicy) -> Self {
        self.spec.default_restart_policy = default_restart_policy;
        self
    }

    /// Sets the default backoff policy.
    ///
    /// # Arguments
    ///
    /// - `default_backoff_policy`: Backoff policy inherited by children.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn default_backoff_policy(mut self, default_backoff_policy: BackoffPolicy) -> Self {
        self.spec.default_backoff_policy = default_backoff_policy;
        self
    }

    /// Sets the default health policy.
    ///
    /// # Arguments
    ///
    /// - `default_health_policy`: Health policy inherited by children.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn default_health_policy(mut self, default_health_policy: HealthPolicy) -> Self {
        self.spec.default_health_policy = default_health_policy;
        self
    }

    /// Sets the default shutdown policy.
    ///
    /// # Arguments
    ///
    /// - `default_shutdown_policy`: Shutdown policy inherited by children.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn default_shutdown_policy(mut self, default_shutdown_policy: ShutdownPolicy) -> Self {
        self.spec.default_shutdown_policy = default_shutdown_policy;
        self
    }

    /// Sets the supervisor failure limit.
    ///
    /// # Arguments
    ///
    /// - `supervisor_failure_limit`: Maximum supervisor failures before escalation.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn supervisor_failure_limit(mut self, supervisor_failure_limit: u32) -> Self {
        self.spec.supervisor_failure_limit = supervisor_failure_limit;
        self
    }

    /// Sets the supervisor-level restart limit.
    ///
    /// # Arguments
    ///
    /// - `restart_limit`: Restart limit applied at supervisor level.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn restart_limit(mut self, restart_limit: RestartLimit) -> Self {
        self.spec.restart_limit = Some(restart_limit);
        self
    }

    /// Clears the supervisor-level restart limit.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn without_restart_limit(mut self) -> Self {
        self.spec.restart_limit = None;
        self
    }

    /// Sets the supervisor-level fallback escalation policy.
    ///
    /// The selected policy is used when a child-level or group-level execution
    /// plan does not define its own escalation policy. Root supervisors do not
    /// have a parent supervisor at runtime, so `EscalateToParent` is only a
    /// configured planning and diagnostic label for root supervisors.
    ///
    /// # Arguments
    ///
    /// - `escalation_policy`: Fallback escalation policy applied at supervisor level.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn escalation_policy(mut self, escalation_policy: EscalationPolicy) -> Self {
        self.spec.escalation_policy = Some(escalation_policy);
        self
    }

    /// Clears the supervisor-level fallback escalation policy.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn without_escalation_policy(mut self) -> Self {
        self.spec.escalation_policy = None;
        self
    }

    /// Replaces group strategy overrides.
    ///
    /// # Arguments
    ///
    /// - `group_strategies`: Group strategy overrides.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn group_strategies(mut self, group_strategies: Vec<GroupStrategy>) -> Self {
        self.spec.group_strategies = group_strategies;
        self
    }

    /// Appends one group strategy override.
    ///
    /// # Arguments
    ///
    /// - `group_strategy`: Group strategy override appended to the supervisor.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn group_strategy(mut self, group_strategy: GroupStrategy) -> Self {
        self.spec.group_strategies.push(group_strategy);
        self
    }

    /// Replaces group configurations.
    ///
    /// # Arguments
    ///
    /// - `group_configs`: Group-level membership and budget configurations.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn group_configs(mut self, group_configs: Vec<GroupConfig>) -> Self {
        self.spec.group_configs = group_configs;
        self
    }

    /// Appends one group configuration.
    ///
    /// # Arguments
    ///
    /// - `group_config`: Group configuration appended to the supervisor.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn group_config(mut self, group_config: GroupConfig) -> Self {
        self.spec.group_configs.push(group_config);
        self
    }

    /// Replaces cross-group dependency edges.
    ///
    /// # Arguments
    ///
    /// - `group_dependencies`: Cross-group dependency edges.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn group_dependencies(mut self, group_dependencies: Vec<GroupDependencyEdge>) -> Self {
        self.spec.group_dependencies = group_dependencies;
        self
    }

    /// Appends one cross-group dependency edge.
    ///
    /// # Arguments
    ///
    /// - `group_dependency`: Cross-group dependency edge appended to the supervisor.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn group_dependency(mut self, group_dependency: GroupDependencyEdge) -> Self {
        self.spec.group_dependencies.push(group_dependency);
        self
    }

    /// Replaces default severity classes by task role.
    ///
    /// # Arguments
    ///
    /// - `severity_defaults`: Default severity map by task role.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn severity_defaults(
        mut self,
        severity_defaults: HashMap<TaskRole, SeverityClass>,
    ) -> Self {
        self.spec.severity_defaults = severity_defaults;
        self
    }

    /// Sets one default severity class.
    ///
    /// # Arguments
    ///
    /// - `task_role`: Task role receiving the default severity class.
    /// - `severity`: Severity class used for the task role.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn severity_default(mut self, task_role: TaskRole, severity: SeverityClass) -> Self {
        self.spec.severity_defaults.insert(task_role, severity);
        self
    }

    /// Removes one default severity class.
    ///
    /// # Arguments
    ///
    /// - `task_role`: Task role whose default severity class is removed.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn without_severity_default(mut self, task_role: TaskRole) -> Self {
        self.spec.severity_defaults.remove(&task_role);
        self
    }

    /// Replaces child-level strategy overrides.
    ///
    /// # Arguments
    ///
    /// - `child_strategy_overrides`: Child-level strategy overrides.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn child_strategy_overrides(
        mut self,
        child_strategy_overrides: Vec<ChildStrategyOverride>,
    ) -> Self {
        self.spec.child_strategy_overrides = child_strategy_overrides;
        self
    }

    /// Appends one child-level strategy override.
    ///
    /// # Arguments
    ///
    /// - `child_strategy_override`: Child-level strategy override appended to the supervisor.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn child_strategy_override(
        mut self,
        child_strategy_override: ChildStrategyOverride,
    ) -> Self {
        self.spec
            .child_strategy_overrides
            .push(child_strategy_override);
        self
    }

    /// Sets the dynamic supervisor policy.
    ///
    /// # Arguments
    ///
    /// - `dynamic_supervisor_policy`: Runtime child addition policy.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn dynamic_supervisor_policy(
        mut self,
        dynamic_supervisor_policy: DynamicSupervisorPolicy,
    ) -> Self {
        self.spec.dynamic_supervisor_policy = dynamic_supervisor_policy;
        self
    }

    /// Sets the control command channel capacity.
    ///
    /// # Arguments
    ///
    /// - `control_channel_capacity`: Control command channel capacity.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn control_channel_capacity(mut self, control_channel_capacity: usize) -> Self {
        self.spec.control_channel_capacity = control_channel_capacity;
        self
    }

    /// Sets the event broadcast channel capacity.
    ///
    /// # Arguments
    ///
    /// - `event_channel_capacity`: Event broadcast channel capacity.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn event_channel_capacity(mut self, event_channel_capacity: usize) -> Self {
        self.spec.event_channel_capacity = event_channel_capacity;
        self
    }

    /// Sets the backpressure configuration.
    ///
    /// # Arguments
    ///
    /// - `backpressure_config`: Backpressure configuration for event subscribers.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn backpressure_config(mut self, backpressure_config: BackpressureConfig) -> Self {
        self.spec.backpressure_config = backpressure_config;
        self
    }

    /// Sets the meltdown policy.
    ///
    /// # Arguments
    ///
    /// - `meltdown_policy`: Failure fuse policy.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn meltdown_policy(mut self, meltdown_policy: MeltdownPolicy) -> Self {
        self.spec.meltdown_policy = meltdown_policy;
        self
    }

    /// Sets the failure window configuration.
    ///
    /// # Arguments
    ///
    /// - `failure_window_config`: Failure accumulation window configuration.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn failure_window_config(mut self, failure_window_config: FailureWindowConfig) -> Self {
        self.spec.failure_window_config = failure_window_config;
        self
    }

    /// Sets the restart budget configuration.
    ///
    /// # Arguments
    ///
    /// - `restart_budget_config`: Restart budget configuration.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn restart_budget_config(mut self, restart_budget_config: RestartBudgetConfig) -> Self {
        self.spec.restart_budget_config = restart_budget_config;
        self
    }

    /// Sets the event journal capacity used by the supervision pipeline.
    ///
    /// # Arguments
    ///
    /// - `pipeline_journal_capacity`: Event journal capacity.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn pipeline_journal_capacity(mut self, pipeline_journal_capacity: usize) -> Self {
        self.spec.pipeline_journal_capacity = pipeline_journal_capacity;
        self
    }

    /// Sets the subscriber queue capacity used by the supervision pipeline.
    ///
    /// # Arguments
    ///
    /// - `pipeline_subscriber_capacity`: Subscriber queue capacity.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn pipeline_subscriber_capacity(mut self, pipeline_subscriber_capacity: usize) -> Self {
        self.spec.pipeline_subscriber_capacity = pipeline_subscriber_capacity;
        self
    }

    /// Sets the concurrent restart limit.
    ///
    /// # Arguments
    ///
    /// - `concurrent_restart_limit`: Maximum concurrent restarts for this supervisor.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn concurrent_restart_limit(mut self, concurrent_restart_limit: u32) -> Self {
        self.spec.concurrent_restart_limit = concurrent_restart_limit;
        self
    }

    /// Sets whether metrics recording is enabled.
    ///
    /// # Arguments
    ///
    /// - `metrics_enabled`: Whether metrics recording is enabled.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn metrics_enabled(mut self, metrics_enabled: bool) -> Self {
        self.spec.metrics_enabled = metrics_enabled;
        self
    }

    /// Sets whether audit event recording is enabled.
    ///
    /// # Arguments
    ///
    /// - `audit_enabled`: Whether audit event recording is enabled.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn audit_enabled(mut self, audit_enabled: bool) -> Self {
        self.spec.audit_enabled = audit_enabled;
        self
    }

    /// Sets the force-kill margin after graceful and abort shutdown windows.
    ///
    /// # Arguments
    ///
    /// - `force_kill_margin`: Extra hard-deadline margin.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn force_kill_margin(mut self, force_kill_margin: Duration) -> Self {
        self.spec.force_kill_margin = force_kill_margin;
        self
    }

    /// Sets the maximum orphaned child task threshold.
    ///
    /// # Arguments
    ///
    /// - `max_orphan_threshold`: Maximum orphaned child task count.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn max_orphan_threshold(mut self, max_orphan_threshold: u32) -> Self {
        self.spec.max_orphan_threshold = max_orphan_threshold;
        self
    }

    /// Builds and validates the supervisor specification.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the constructed [`SupervisorSpec`] when local invariants pass.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError`] when validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::spec::supervisor_builder::SupervisorSpecBuilder;
    ///
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// let spec = SupervisorSpecBuilder::root(Vec::new())
    ///     .config_version("demo")
    ///     .build()?;
    /// assert_eq!(spec.config_version, "demo");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// use rust_supervisor::spec::supervisor::SupervisorSpec;
    /// use rust_supervisor::spec::supervisor_builder::SupervisorSpecBuilder;
    ///
    /// let builder = SupervisorSpecBuilder::root(Vec::new());
    /// let _spec: SupervisorSpec = builder;
    /// ```
    pub fn build(self) -> Result<SupervisorSpec, SupervisorError> {
        let spec = self.spec;
        spec.validate()?;
        Ok(spec)
    }
}
