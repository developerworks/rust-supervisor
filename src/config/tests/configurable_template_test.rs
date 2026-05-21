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
