//! `confique::Config` trait tests for public configuration structs.

use confique::Config;
use rust_supervisor::config::configurable::{
    DashboardIpcConfig, DashboardRegistrationConfig, ObservabilityConfig, PolicyConfig,
    ShutdownConfig, SupervisorConfig, SupervisorRootConfig,
};

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
    assert_confique_config::<DashboardIpcConfig>();
    assert_confique_config::<DashboardRegistrationConfig>();
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
            "supervisor",
            "policy",
            "shutdown",
            "observability",
            "ipc",
            "children"
        ]
    );
}
