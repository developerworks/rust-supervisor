//! SupervisorSpec builder tests.
//!
//! These tests verify supervisor builder defaults, setters, topology changes,
//! and validation behavior.

use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::budget::RestartBudgetConfig;
use rust_supervisor::policy::failure_window::FailureWindowConfig;
use rust_supervisor::policy::group::{GroupDependencyEdge, PropagationPolicy};
use rust_supervisor::policy::meltdown::MeltdownPolicy;
use rust_supervisor::policy::task_role_defaults::{SeverityClass, TaskRole};
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::spec::child::{BackoffPolicy, ChildSpec, HealthPolicy, RestartPolicy};
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
use rust_supervisor::spec::shutdown::{ShutdownBudget, TreeShutdownPolicy};
use rust_supervisor::spec::supervisor::{
    BackpressureConfig, BackpressureStrategy, ChildStrategyOverride, DynamicSupervisorPolicy,
    EscalationPolicy, GroupConfig, GroupStrategy, RECOMMENDED_CHANNEL_CAPACITY, RestartLimit,
    SupervisionStrategy, SupervisorSpec,
};
use rust_supervisor::spec::supervisor_builder::SupervisorSpecBuilder;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Returns a no-op service child for supervisor builder tests.
fn service_child(id: &str) -> Result<ChildSpec, SupervisorError> {
    let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    ChildSpecBuilder::service(
        ChildId::new(id),
        id,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
    .build()
}

/// Returns a grouped service child for supervisor builder tests.
fn grouped_service_child(id: &str, group: &str) -> Result<ChildSpec, SupervisorError> {
    let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    ChildSpecBuilder::service(
        ChildId::new(id),
        id,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
    .group(group)
    .build()
}

/// Verifies root builder defaults match `SupervisorSpec::root`.
#[test]
fn root_builder_matches_supervisor_spec_root_defaults() -> Result<(), SupervisorError> {
    let builder_spec = SupervisorSpecBuilder::root(Vec::new()).build()?;
    let root_spec = SupervisorSpec::root(Vec::new());

    assert_eq!(builder_spec.path, root_spec.path);
    assert_eq!(builder_spec.strategy, root_spec.strategy);
    assert_eq!(builder_spec.children.len(), root_spec.children.len());
    assert_eq!(builder_spec.config_version, root_spec.config_version);
    assert_eq!(
        builder_spec.default_restart_policy,
        root_spec.default_restart_policy
    );
    assert_eq!(
        builder_spec.default_backoff_policy,
        root_spec.default_backoff_policy
    );
    assert_eq!(
        builder_spec.default_health_policy,
        root_spec.default_health_policy
    );
    assert_eq!(builder_spec.tree_shutdown, root_spec.tree_shutdown);
    assert_eq!(
        builder_spec.supervisor_failure_limit,
        root_spec.supervisor_failure_limit
    );
    assert_eq!(builder_spec.restart_limit, root_spec.restart_limit);
    assert_eq!(builder_spec.escalation_policy, root_spec.escalation_policy);
    assert_eq!(builder_spec.group_strategies, root_spec.group_strategies);
    assert_eq!(builder_spec.group_configs, root_spec.group_configs);
    assert_eq!(
        builder_spec.group_dependencies,
        root_spec.group_dependencies
    );
    assert_eq!(builder_spec.severity_defaults, root_spec.severity_defaults);
    assert_eq!(
        builder_spec.child_strategy_overrides,
        root_spec.child_strategy_overrides
    );
    assert_eq!(
        builder_spec.dynamic_supervisor_policy,
        root_spec.dynamic_supervisor_policy
    );
    assert_eq!(
        builder_spec.control_channel_capacity,
        root_spec.control_channel_capacity
    );
    assert_eq!(
        builder_spec.event_channel_capacity,
        root_spec.event_channel_capacity
    );
    assert_eq!(
        builder_spec.backpressure_config,
        root_spec.backpressure_config
    );
    assert_eq!(builder_spec.meltdown_policy, root_spec.meltdown_policy);
    assert_eq!(
        builder_spec.failure_window_config,
        root_spec.failure_window_config
    );
    assert_eq!(
        builder_spec.restart_budget_config,
        root_spec.restart_budget_config
    );
    assert_eq!(
        builder_spec.pipeline_journal_capacity,
        root_spec.pipeline_journal_capacity
    );
    assert_eq!(
        builder_spec.pipeline_subscriber_capacity,
        root_spec.pipeline_subscriber_capacity
    );
    assert_eq!(
        builder_spec.concurrent_restart_limit,
        root_spec.concurrent_restart_limit
    );
    assert_eq!(builder_spec.metrics_enabled, root_spec.metrics_enabled);
    assert_eq!(builder_spec.audit_enabled, root_spec.audit_enabled);
    Ok(())
}

/// Verifies appending children keeps the recommended channel capacities.
#[test]
fn child_setter_keeps_recommended_channel_capacities() -> Result<(), SupervisorError> {
    let spec = SupervisorSpecBuilder::root(Vec::new())
        .child(service_child("api")?)
        .build()?;

    assert_eq!(spec.children.len(), 1);
    assert_eq!(spec.control_channel_capacity, RECOMMENDED_CHANNEL_CAPACITY);
    assert_eq!(spec.event_channel_capacity, RECOMMENDED_CHANNEL_CAPACITY);
    Ok(())
}

/// Verifies explicit channel capacities survive child topology changes.
#[test]
fn explicit_channel_capacities_are_preserved_after_child_changes() -> Result<(), SupervisorError> {
    let spec = SupervisorSpecBuilder::root(Vec::new())
        .control_channel_capacity(32)
        .event_channel_capacity(64)
        .child(service_child("api")?)
        .build()?;

    assert_eq!(spec.children.len(), 1);
    assert_eq!(spec.control_channel_capacity, 32);
    assert_eq!(spec.event_channel_capacity, 64);
    Ok(())
}

/// Fixture for fluent supervisor builder setter coverage.
struct FluentSetterFixture {
    /// Child mounted on the built supervisor.
    child: ChildSpec,
    /// Restart limit applied at supervisor and group scope.
    restart_limit: RestartLimit,
    /// Supervisor specification built through fluent setters.
    spec: SupervisorSpec,
}

/// Applies fluent builder setters and returns the constructed supervisor specification.
fn build_supervisor_with_fluent_setters(
    child: ChildSpec,
    restart_limit: RestartLimit,
    group_config: GroupConfig,
    group_strategy: GroupStrategy,
    child_override: ChildStrategyOverride,
    backpressure_config: BackpressureConfig,
    severity_defaults: HashMap<TaskRole, SeverityClass>,
) -> Result<SupervisorSpec, SupervisorError> {
    SupervisorSpecBuilder::root(Vec::new())
        .path(rust_supervisor::id::types::SupervisorPath::root())
        .child(child.clone())
        .config_version("builder-demo")
        .strategy(SupervisionStrategy::OneForAll)
        .default_restart_policy(RestartPolicy::Permanent)
        .default_backoff_policy(BackoffPolicy::new(
            Duration::from_millis(20),
            Duration::from_secs(2),
            0.0,
        ))
        .default_health_policy(HealthPolicy::new(
            Duration::from_secs(2),
            Duration::from_secs(6),
        ))
        .tree_shutdown(TreeShutdownPolicy::new(
            ShutdownBudget::new(Duration::from_secs(4), Duration::from_secs(1)),
            true,
            Duration::from_secs(3),
            2,
        ))
        .supervisor_failure_limit(2)
        .restart_limit(restart_limit)
        .escalation_policy(EscalationPolicy::ShutdownTree)
        .group_config(group_config)
        .group_strategy(group_strategy)
        .group_dependency(GroupDependencyEdge {
            from_group: "pipeline".to_owned(),
            to_group: "storage".to_owned(),
            propagation: PropagationPolicy::EscalateOnly,
        })
        .severity_defaults(severity_defaults)
        .severity_default(TaskRole::Worker, SeverityClass::Standard)
        .without_severity_default(TaskRole::Worker)
        .child_strategy_override(child_override)
        .dynamic_supervisor_policy(DynamicSupervisorPolicy::limited(4))
        .control_channel_capacity(16)
        .event_channel_capacity(32)
        .backpressure_config(backpressure_config)
        .meltdown_policy(MeltdownPolicy::new(
            2,
            Duration::from_secs(8),
            4,
            Duration::from_secs(20),
            8,
            Duration::from_secs(40),
            Duration::from_secs(80),
        ))
        .failure_window_config(FailureWindowConfig::count_sliding(4, 2))
        .restart_budget_config(RestartBudgetConfig::new(Duration::from_secs(20), 2, 1.0))
        .pipeline_journal_capacity(64)
        .pipeline_subscriber_capacity(8)
        .concurrent_restart_limit(2)
        .metrics_enabled(false)
        .audit_enabled(false)
        .build()
}

/// Builds a supervisor specification that exercises fluent builder setters.
fn fluent_setter_fixture() -> Result<FluentSetterFixture, SupervisorError> {
    let child = grouped_service_child("api", "pipeline")?;
    let restart_limit = RestartLimit::new(3, Duration::from_secs(30));
    let group_config = GroupConfig::new(
        "pipeline",
        vec![child.id.clone()],
        Some(RestartBudgetConfig::new(Duration::from_secs(30), 3, 1.0)),
    );
    let mut group_strategy = GroupStrategy::new("pipeline", SupervisionStrategy::OneForAll);
    group_strategy.restart_limit = Some(restart_limit);
    group_strategy.escalation_policy = Some(EscalationPolicy::EscalateToParent);
    let child_override =
        ChildStrategyOverride::new(child.id.clone(), SupervisionStrategy::RestForOne);
    let backpressure_config = BackpressureConfig {
        strategy: BackpressureStrategy::SampleAndAudit,
        warn_threshold_pct: 70,
        critical_threshold_pct: 90,
        window_secs: 10,
        audit_channel_capacity: 16,
    };
    let mut severity_defaults = HashMap::new();
    severity_defaults.insert(TaskRole::Service, SeverityClass::Critical);
    let spec = build_supervisor_with_fluent_setters(
        child.clone(),
        restart_limit,
        group_config,
        group_strategy,
        child_override,
        backpressure_config,
        severity_defaults,
    )?;

    Ok(FluentSetterFixture {
        child,
        restart_limit,
        spec,
    })
}

/// Verifies fluent setters update the constructed supervisor specification.
#[test]
fn builder_setters_apply_expected_fields() -> Result<(), SupervisorError> {
    let FluentSetterFixture {
        child,
        restart_limit,
        spec,
    } = fluent_setter_fixture()?;

    assert_eq!(spec.children.len(), 1);
    assert_eq!(spec.children[0].id, child.id);
    assert_eq!(spec.config_version, "builder-demo");
    assert_eq!(spec.strategy, SupervisionStrategy::OneForAll);
    assert_eq!(spec.default_restart_policy, RestartPolicy::Permanent);
    assert_eq!(spec.supervisor_failure_limit, 2);
    assert_eq!(spec.restart_limit, Some(restart_limit));
    assert_eq!(spec.escalation_policy, Some(EscalationPolicy::ShutdownTree));
    assert_eq!(spec.group_configs.len(), 1);
    assert_eq!(spec.group_strategies.len(), 1);
    assert_eq!(spec.group_dependencies.len(), 1);
    assert_eq!(
        spec.severity_defaults.get(&TaskRole::Service),
        Some(&SeverityClass::Critical)
    );
    assert!(!spec.severity_defaults.contains_key(&TaskRole::Worker));
    assert_eq!(spec.child_strategy_overrides.len(), 1);
    assert_eq!(spec.dynamic_supervisor_policy.child_limit, Some(4));
    assert_eq!(spec.control_channel_capacity, 16);
    assert_eq!(spec.event_channel_capacity, 32);
    assert_eq!(
        spec.backpressure_config.strategy,
        BackpressureStrategy::SampleAndAudit
    );
    assert_eq!(spec.pipeline_journal_capacity, 64);
    assert_eq!(spec.pipeline_subscriber_capacity, 8);
    assert_eq!(spec.concurrent_restart_limit, 2);
    assert!(!spec.metrics_enabled);
    assert!(!spec.audit_enabled);
    assert_eq!(spec.tree_shutdown.force_kill_margin, Duration::from_secs(3));
    assert_eq!(spec.tree_shutdown.max_orphan_threshold, 2);
    assert_eq!(
        spec.tree_shutdown.budget.graceful_timeout,
        Duration::from_secs(4)
    );
    Ok(())
}

/// Verifies `build` rejects invalid supervisor specifications.
#[test]
fn build_rejects_invalid_supervisor_spec() {
    let error = SupervisorSpecBuilder::root(Vec::new())
        .config_version(" ")
        .build()
        .expect_err("empty config version should fail validation");

    assert!(error.to_string().contains("config version"));
}
