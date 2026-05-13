//! Example suite integration tests.
//!
//! These tests keep the learning examples present and wired to YAML config.

use std::fs;
use std::path::Path;

/// Verifies that the expected examples exist and reference the public API.
#[test]
fn example_suite_contains_learning_programs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for example in [
        "supervisor_quickstart.rs",
        "config_tree_supervisor.rs",
        "restart_policy_lab.rs",
        "shutdown_tree.rs",
        "observability_probe.rs",
        "supervisor_tree_story.rs",
        "runtime_control_story.rs",
        "policy_failure_matrix.rs",
        "diagnostic_replay.rs",
    ] {
        let text = fs::read_to_string(root.join("examples").join(example)).expect("read example");
        assert!(text.contains("rust_supervisor::"));
    }

    assert!(root.join("examples/config/supervisor.yaml").is_file());
}

/// Verifies that the demo entry point uses configuration startup.
#[test]
fn demo_example_starts_from_config_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let demo = root.join("examples/demo/main.rs");
    let text = fs::read_to_string(&demo).expect("read demo example");

    assert!(text.contains("Supervisor::start_from_config_file"));
    assert!(!text.contains("to_supervisor_spec"));
    assert!(!root.join("src/bin").exists());

    let readme = fs::read_to_string(root.join("README.md")).expect("read README");
    assert!(
        readme.contains("cargo run --example demo -- --config examples/config/supervisor.yaml")
    );
}
