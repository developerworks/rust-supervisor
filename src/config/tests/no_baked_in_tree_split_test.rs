//! Tests that keep tree split decisions outside this crate.

use rust_supervisor::config::configurable::SupervisorConfig;
use std::fs;
use std::path::Path;

/// User-owned wrapper type that can choose its own schema extensions.
#[derive(schemars::JsonSchema)]
#[allow(dead_code)]
struct UserProjectConfig {
    /// Reused supervisor configuration model.
    #[schemars(extend("x-tree-split" = true))]
    supervisor: SupervisorConfig,
}

/// Verifies that the generated public schema has no built-in tree split marker.
#[test]
fn generated_schema_does_not_contain_tree_split_marker() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_text =
        serde_json::to_string(&schema).expect("serialize generated supervisor schema");

    assert!(!schema_text.contains("x-tree-split"));
}

/// Verifies that the generated official template has no tree split marker.
#[test]
fn generated_template_does_not_contain_tree_split_marker() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let targets = rust_config_tree::template_targets_for_paths::<SupervisorConfig>(
        root.join("examples/config/supervisor.yaml"),
        root.join("examples/config/supervisor.template.yaml"),
    )
    .expect("generate template targets");

    assert_eq!(targets.len(), 1);
    assert!(!targets[0].content.contains("x-tree-split"));
}

/// Verifies that the checked-in official template has no tree split marker.
#[test]
fn checked_in_template_does_not_contain_tree_split_marker() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let template =
        fs::read_to_string(root.join("examples/config/supervisor.template.yaml")).expect("read");

    assert!(!template.contains("x-tree-split"));
}

/// Verifies that users can decide tree splitting in their own wrapper type.
#[test]
fn user_project_wrapper_can_choose_tree_split_marker() {
    let schema = schemars::schema_for!(UserProjectConfig);
    let schema_text = serde_json::to_string(&schema).expect("serialize wrapper schema");

    assert!(schema_text.contains("x-tree-split"));
    assert!(schema_text.contains("SupervisorConfig"));
}
