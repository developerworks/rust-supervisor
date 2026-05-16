//! Configuration boundary integration tests.
//!
//! These tests keep YAML configuration and rust-config-tree ownership visible.

use rust_supervisor::config::loader::load_config_from_yaml_file;
use rust_supervisor::config::yaml::parse_config_state;
use rust_supervisor::spec::supervisor::SupervisionStrategy;
use std::fs;
use std::path::Path;

/// Verifies that the declared dependency version is rust-config-tree v0.1.9.
#[test]
fn cargo_uses_rust_config_tree_v019() {
    let cargo = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read Cargo.toml");

    assert!(cargo.contains("rust-config-tree = \"0.1.9\""));
}

/// Verifies that the example YAML configuration loads and derives a spec.
#[test]
fn yaml_config_loads_into_config_state() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config_path = root.join("examples/config/supervisor.yaml");
    let yaml = fs::read_to_string(&config_path).expect("read YAML config");
    let state = parse_config_state(&yaml).expect("parse YAML config");
    let loaded = load_config_from_yaml_file(&config_path).expect("load YAML config");
    let spec = loaded.to_supervisor_spec().expect("derive supervisor spec");

    assert_eq!(state, loaded);
    assert_eq!(loaded.supervisor.strategy, SupervisionStrategy::OneForAll);
    assert_eq!(spec.strategy, loaded.supervisor.strategy);
    assert_eq!(
        spec.supervisor_failure_limit,
        loaded.policy.supervisor_failure_limit
    );
}
