//! Tests for intentional split configuration sections.

use rust_config_tree::config::template_targets_for_paths;
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

/// Verifies that split sections carry the tree split marker in generated schema.
#[test]
fn generated_schema_marks_split_sections_for_tree_split() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_text =
        serde_json::to_string(&schema).expect("serialize generated supervisor schema");

    assert!(schema_text.contains("x-tree-split"));
    assert!(schema_text.contains("x-tree-transparent-array"));
    assert!(schema_text.contains("GroupsConfigSection"));
    assert!(schema_text.contains("ChildrenConfigSection"));
}

/// Verifies that template generation emits root, groups, and children targets.
#[test]
fn generated_template_emits_split_section_targets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let targets = template_targets_for_paths::<SupervisorConfig>(
        root.join("examples/config/supervisor.yaml"),
        root.join("examples/config/supervisor.template.yaml"),
    )
    .expect("generate template targets");

    assert_eq!(targets.len(), 3);
    let file_names = targets
        .iter()
        .map(|target| {
            target
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("template target file name")
        })
        .collect::<Vec<_>>();
    assert!(file_names.contains(&"supervisor.template.yaml"));
    assert!(file_names.contains(&"groups.yaml"));
    assert!(file_names.contains(&"children.yaml"));

    let root_target = targets
        .iter()
        .find(|target| target.path.ends_with("supervisor.template.yaml"))
        .expect("root template target");
    assert!(root_target.content.contains("include:"));
    assert!(root_target.content.contains("groups.yaml"));
    assert!(root_target.content.contains("children.yaml"));
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
