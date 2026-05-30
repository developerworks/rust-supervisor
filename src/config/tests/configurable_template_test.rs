//! Template generation tests for public supervisor configuration.

use rust_config_tree::config::template_targets_for_paths;
use rust_supervisor::config::configurable::SupervisorConfig;
use std::path::Path;

/// Verifies that official template generation produces root, groups, and children targets.
#[test]
fn supervisor_config_generates_split_template_targets() {
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
}

/// Verifies that the generated children split template includes a sample entry.
#[test]
fn generated_children_template_includes_sample_child() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let targets = template_targets_for_paths::<SupervisorConfig>(
        root.join("examples/config/supervisor.yaml"),
        root.join("examples/config/supervisor.template.yaml"),
    )
    .expect("generate template targets");

    let children_target = targets
        .iter()
        .find(|target| target.path.ends_with("children.yaml"))
        .expect("children template target");

    assert!(
        children_target.content.contains("name: worker"),
        "children template should include a sample child declaration"
    );
    assert!(
        !children_target.content.contains("[{"),
        "children template must not use flow-style arrays"
    );
    assert!(
        !children_target.content.contains("\nchildren:\n"),
        "children split template must be body-only"
    );
    assert!(children_target.content.contains("- name: worker"));
}

/// Verifies that the generated children split template uses block YAML body-only shape.
#[test]
fn generated_children_template_uses_block_yaml_body() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let targets = template_targets_for_paths::<SupervisorConfig>(
        root.join("examples/config/supervisor.yaml"),
        root.join("examples/config/supervisor.template.yaml"),
    )
    .expect("generate template targets");

    let children_target = targets
        .iter()
        .find(|target| target.path.ends_with("children.yaml"))
        .expect("children template target");

    assert!(
        !children_target.content.contains("[{"),
        "children template must not use flow-style arrays"
    );
    assert!(
        !children_target.content.contains("\nchildren:\n"),
        "children split template must be body-only"
    );
}

/// Verifies that the generated root template covers all runtime tunables.
#[test]
fn generated_template_contains_all_runtime_tunables() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let targets = template_targets_for_paths::<SupervisorConfig>(
        root.join("examples/config/supervisor.yaml"),
        root.join("examples/config/supervisor.template.yaml"),
    )
    .expect("generate template targets");
    let content = &targets[0].content;

    for field in [
        "strategy",
        "escalation_policy",
        "dynamic_supervisor",
        "enabled",
        "child_limit",
        "child_restart_limit",
        "child_restart_window_ms",
        "supervisor_failure_limit",
        "supervisor_failure_window_ms",
        "initial_backoff_ms",
        "max_backoff_ms",
        "jitter_ratio",
        "heartbeat_interval_ms",
        "stale_after_ms",
        "restart_budget",
        "max_burst",
        "recovery_rate_per_sec",
        "failure_window",
        "mode",
        "max_count",
        "threshold",
        "meltdown",
        "child_max_restarts",
        "child_window_secs",
        "group_max_failures",
        "group_window_secs",
        "supervisor_max_failures",
        "supervisor_window_secs",
        "reset_after_secs",
        "supervision_pipeline",
        "journal_capacity",
        "subscriber_capacity",
        "concurrent_restart_limit",
        "graceful_timeout_ms",
        "abort_wait_ms",
        "event_journal_capacity",
        "metrics_enabled",
        "audit_enabled",
        "audit",
        "backend",
        "file_path",
        "failure_strategy",
        "max_defer_queue",
        "backpressure",
        "warn_threshold_pct",
        "critical_threshold_pct",
        "window_secs",
        "audit_channel_capacity",
        "groups",
        "group_strategies",
        "group_dependencies",
        "child_strategy_overrides",
        "severity_defaults",
        "children",
        "dashboard",
    ] {
        assert!(content.contains(field), "template is missing {field}");
    }
}

/// Verifies that generated templates write schema-valid runtime defaults.
#[test]
fn generated_template_writes_runtime_default_values() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let targets = template_targets_for_paths::<SupervisorConfig>(
        root.join("examples/config/supervisor.yaml"),
        root.join("examples/config/supervisor.template.yaml"),
    )
    .expect("generate template targets");
    let content = &targets[0].content;

    for field in [
        "strategy: OneForAll",
        "child_restart_limit: 10",
        "child_restart_window_ms: 60000",
        "supervisor_failure_limit: 30",
        "supervisor_failure_window_ms: 60000",
        "initial_backoff_ms: 100",
        "max_backoff_ms: 5000",
        "jitter_ratio: 0.1",
        "heartbeat_interval_ms: 1000",
        "stale_after_ms: 3000",
        "graceful_timeout_ms: 5000",
        "abort_wait_ms: 1000",
        "event_journal_capacity: 256",
        "metrics_enabled: true",
        "audit_enabled: true",
    ] {
        assert!(content.contains(field), "template is missing {field}");
    }
}
