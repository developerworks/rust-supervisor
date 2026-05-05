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
    ] {
        let text = fs::read_to_string(root.join("examples").join(example)).expect("read example");
        assert!(text.contains("rust_supervisor::"));
    }

    assert!(root.join("examples/config/supervisor.yaml").is_file());
}
