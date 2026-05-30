//! Tests for transparent split-section loading.

use rust_config_tree::config::load_config;
use rust_supervisor::config::{
    configurable::SupervisorConfig,
    loader::load_config_from_yaml_file,
};
use std::fs;
use std::path::PathBuf;

/// Builds a unique temporary directory for split-section tests.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rust-supervisor-split-section-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Verifies that body-only split files load through the section field name.
#[test]
fn load_supervisor_config_accepts_body_only_split_files() {
    let dir = temp_dir("body-only");
    let root = dir.join("root.yaml");
    let children = dir.join("children.yaml");

    fs::write(
        &root,
        "include:\n  - children.yaml\nsupervisor:\n  strategy: OneForAll\n",
    )
    .expect("write root");
    fs::write(
        &children,
        "- name: api\n  kind: async_worker\n",
    )
    .expect("write children");

    let config = load_config::<SupervisorConfig>(&root).expect("load split config");
    assert_eq!(config.children.len(), 1);
    assert_eq!(config.children.as_slice()[0].name, "api");

    let _ = fs::remove_dir_all(dir);
}

/// Verifies that wrapped split files still load through flat include merge.
#[test]
fn load_supervisor_config_accepts_wrapped_split_files() {
    let dir = temp_dir("wrapped");
    let root = dir.join("root.yaml");
    let children = dir.join("children.yaml");

    fs::write(
        &root,
        "include:\n  - children.yaml\nsupervisor:\n  strategy: OneForAll\n",
    )
    .expect("write root");
    fs::write(
        &children,
        "children:\n  - name: api\n    kind: async_worker\n",
    )
    .expect("write children");

    let config = load_config::<SupervisorConfig>(&root).expect("load wrapped split config");
    assert_eq!(config.children.len(), 1);

    let _ = fs::remove_dir_all(dir);
}

/// Verifies that the public YAML loader accepts body-only split includes end to end.
#[test]
fn yaml_loader_accepts_body_only_split_includes() {
    let dir = temp_dir("yaml-loader");
    let root = dir.join("root.yaml");
    let children = dir.join("children.yaml");

    fs::write(
        &root,
        "include:\n  - children.yaml\nsupervisor:\n  strategy: OneForAll\npolicy:\n  child_restart_limit: 10\n  child_restart_window_ms: 60000\n  supervisor_failure_limit: 30\n  supervisor_failure_window_ms: 60000\n  initial_backoff_ms: 100\n  max_backoff_ms: 5000\n  jitter_ratio: 0.1\n  heartbeat_interval_ms: 1000\n  stale_after_ms: 3000\n",
    )
    .expect("write root");
    fs::write(
        &children,
        "- name: api\n  kind: async_worker\n  group: core\n",
    )
    .expect("write children");
    fs::write(
        dir.join("groups.yaml"),
        "- name: core\n  budget:\n    window_secs: 60\n    max_burst: 10\n    recovery_rate_per_sec: 0.5\n",
    )
    .expect("write groups");

    let state = load_config_from_yaml_file(&root).expect("load validated config");
    assert_eq!(state.children.len(), 1);
    assert_eq!(state.children[0].name, "api");

    let _ = fs::remove_dir_all(dir);
}
