//! Template generation tests for public supervisor configuration.

use rust_supervisor::config::configurable::SupervisorConfig;
use std::path::Path;

/// Verifies that official template generation produces one root YAML target.
#[test]
fn supervisor_config_generates_single_root_template_target() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let targets = rust_config_tree::template_targets_for_paths::<SupervisorConfig>(
        root.join("examples/config/supervisor.yaml"),
        root.join("examples/config/supervisor.template.yaml"),
    )
    .expect("generate template targets");

    assert_eq!(targets.len(), 1);
    assert_eq!(
        targets[0].path,
        root.join("examples/config/supervisor.template.yaml")
    );
}

/// Verifies that the generated root template covers all runtime tunables.
#[test]
fn generated_template_contains_all_runtime_tunables() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let targets = rust_config_tree::template_targets_for_paths::<SupervisorConfig>(
        root.join("examples/config/supervisor.yaml"),
        root.join("examples/config/supervisor.template.yaml"),
    )
    .expect("generate template targets");
    let content = &targets[0].content;

    for field in [
        "strategy",
        "child_restart_limit",
        "child_restart_window_ms",
        "supervisor_failure_limit",
        "supervisor_failure_window_ms",
        "initial_backoff_ms",
        "max_backoff_ms",
        "jitter_ratio",
        "heartbeat_interval_ms",
        "stale_after_ms",
        "graceful_timeout_ms",
        "abort_wait_ms",
        "event_journal_capacity",
        "metrics_enabled",
        "audit_enabled",
    ] {
        assert!(content.contains(field), "template is missing {field}");
    }
}
