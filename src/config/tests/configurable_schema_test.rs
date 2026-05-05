//! Schema generation tests for public supervisor configuration.

use rust_supervisor::config::configurable::SupervisorConfig;
use serde_json::Value;

/// Verifies that the public root configuration struct can generate JSON Schema.
#[test]
fn supervisor_config_generates_schema_for_all_public_fields() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_value = serde_json::to_value(&schema).expect("serialize schema");
    let schema_text = serde_json::to_string(&schema_value).expect("stringify schema");

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
        assert!(schema_text.contains(field), "schema is missing {field}");
    }
}

/// Verifies that the root schema exposes the expected top-level sections.
#[test]
fn supervisor_config_schema_contains_top_level_sections() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_value = serde_json::to_value(&schema).expect("serialize schema");
    let properties = schema_value
        .get("properties")
        .and_then(Value::as_object)
        .expect("root properties");

    for section in ["supervisor", "policy", "shutdown", "observability"] {
        assert!(
            properties.contains_key(section),
            "missing section {section}"
        );
    }
}
