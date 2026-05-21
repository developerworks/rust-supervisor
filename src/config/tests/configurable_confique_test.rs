//! `confique::Config` trait tests for public configuration structs.

use confique::Config;
use rust_supervisor::config::{
    audit::AuditConfig,
    configurable::{
        DashboardIpcConfig, DashboardRegistrationConfig, ObservabilityConfig, PolicyConfig,
        ShutdownConfig, SupervisorConfig, SupervisorRootConfig,
    },
    policy::{
        ChildStrategyOverrideConfig, DynamicSupervisorConfig, FailureWindowConfig, GroupConfig,
        GroupDependencyConfig, GroupStrategyConfig, MeltdownConfig, RestartBudgetConfig,
        RestartLimitConfig, SeverityDefaultConfig, SupervisionPipelineConfig,
    },
};
use rust_supervisor::spec::supervisor::BackpressureConfig;

/// Accepts any type that implements `confique::Config`.
fn assert_confique_config<T: confique::Config>() {}

/// Verifies that the root configuration struct supports `confique::Config`.
#[test]
fn supervisor_config_implements_confique_config() {
    assert_confique_config::<SupervisorConfig>();
}

/// Verifies that every nested configuration struct supports `confique::Config`.
#[test]
fn nested_config_structs_implement_confique_config() {
    assert_confique_config::<SupervisorRootConfig>();
    assert_confique_config::<PolicyConfig>();
    assert_confique_config::<ShutdownConfig>();
    assert_confique_config::<ObservabilityConfig>();
    assert_confique_config::<AuditConfig>();
    assert_confique_config::<BackpressureConfig>();
    assert_confique_config::<DashboardIpcConfig>();
    assert_confique_config::<DashboardRegistrationConfig>();
    assert_confique_config::<RestartBudgetConfig>();
    assert_confique_config::<FailureWindowConfig>();
    assert_confique_config::<MeltdownConfig>();
    assert_confique_config::<SupervisionPipelineConfig>();
    assert_confique_config::<DynamicSupervisorConfig>();
    assert_confique_config::<RestartLimitConfig>();
    assert_confique_config::<GroupConfig>();
    assert_confique_config::<GroupStrategyConfig>();
    assert_confique_config::<GroupDependencyConfig>();
    assert_confique_config::<ChildStrategyOverrideConfig>();
    assert_confique_config::<SeverityDefaultConfig>();
}

/// Verifies that the root configuration metadata contains all public sections.
#[test]
fn confique_metadata_contains_public_sections() {
    let field_names = SupervisorConfig::META
        .fields
        .iter()
        .map(|field| field.name)
        .collect::<Vec<_>>();

    assert_eq!(
        field_names,
        [
            "include",
            "supervisor",
            "policy",
            "shutdown",
            "observability",
            "audit",
            "backpressure",
            "groups",
            "group_strategies",
            "group_dependencies",
            "child_strategy_overrides",
            "severity_defaults",
            "dashboard",
            "children"
        ]
    );
}
