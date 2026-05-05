//! Documentation synchronization integration tests.
//!
//! These tests keep README and user docs aligned with current examples.

use std::fs;
use std::path::Path;

/// Verifies that public docs mention the active configuration and examples.
#[test]
fn docs_reference_current_config_and_examples() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let docs = [
        "README.md",
        "README.zh.md",
        "manual/zh/index.md",
        "manual/zh/getting-started.md",
        "manual/zh/configuration.md",
        "manual/zh/examples.md",
        "manual/en/index.md",
        "manual/en/getting-started.md",
        "manual/en/configuration.md",
        "manual/en/examples.md",
        "docs/zh/index.md",
        "docs/en/index.md",
    ]
    .into_iter()
    .map(|path| fs::read_to_string(root.join(path)).expect("read doc"))
    .collect::<Vec<_>>()
    .join("\n");

    assert!(docs.contains("rust-config-tree(集中配置树) v0.1.9"));
    assert!(docs.contains("supervisor_quickstart"));
    assert!(docs.contains("supervisor_tree_story"));
    assert!(docs.contains("runtime_control_story"));
    assert!(docs.contains("policy_failure_matrix"));
    assert!(docs.contains("diagnostic_replay"));
    assert!(docs.contains("Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务)"));
}

/// Verifies that configuration schema documentation mentions every public field.
#[test]
fn docs_reference_supervisor_config_fields() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let docs = [
        "README.md",
        "README.zh.md",
        "manual/zh/configuration.md",
        "manual/en/configuration.md",
        "examples/config/supervisor.yaml",
        "examples/config/supervisor.template.yaml",
    ]
    .into_iter()
    .map(|path| fs::read_to_string(root.join(path)).expect("read doc"))
    .collect::<Vec<_>>()
    .join("\n");

    for field in [
        "supervisor",
        "strategy",
        "policy",
        "child_restart_limit",
        "child_restart_window_ms",
        "supervisor_failure_limit",
        "supervisor_failure_window_ms",
        "initial_backoff_ms",
        "max_backoff_ms",
        "jitter_ratio",
        "heartbeat_interval_ms",
        "stale_after_ms",
        "shutdown",
        "graceful_timeout_ms",
        "abort_wait_ms",
        "observability",
        "event_journal_capacity",
        "metrics_enabled",
        "audit_enabled",
    ] {
        assert!(docs.contains(field), "docs are missing {field}");
    }

    assert!(docs.contains("SupervisorConfig"));
    assert!(docs.contains("x-tree-split"));
}

/// Verifies that the public documentation sync target remains callable by name.
#[test]
fn documentation_sync_matches_public_api() {
    docs_reference_current_config_and_examples();
    docs_reference_supervisor_config_fields();
}
