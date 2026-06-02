//! Schema generation tests for public supervisor configuration.

use rust_supervisor::config::configurable::SupervisorConfig;
use rust_supervisor::config::factory_schema::{
    supervisor_schema_targets_with_factory_registry, supervisor_schema_with_factory_registry,
};
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::task::factory_registry::{TaskFactoryDescriptor, TaskFactoryRegistry};
use serde_json::Value;
use std::sync::Arc;

/// Public root schema field names that must appear in generated JSON Schema text.
const SUPERVISOR_CONFIG_SCHEMA_FIELDS: &[&str] = &[
    "supervisor",
    "strategy",
    "escalation_policy",
    "control_channel_capacity",
    "event_channel_capacity",
    "dynamic_supervisor",
    "enabled",
    "child_limit",
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
    "shutdown",
    "graceful_timeout_ms",
    "abort_wait_ms",
    "observability",
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
    "dependencies",
    "health_check",
    "readiness",
    "command_permissions",
    "environment",
    "secrets",
    "tags",
    "task_role",
    "factory_key",
    "sidecar_config",
    "severity",
    "group",
    "dashboard",
];

/// Verifies that the public root configuration struct can generate JSON Schema.
#[test]
fn supervisor_config_generates_schema_for_all_public_fields() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_value = serde_json::to_value(&schema).expect("serialize schema");
    let schema_text = serde_json::to_string(&schema_value).expect("stringify schema");

    for field in SUPERVISOR_CONFIG_SCHEMA_FIELDS {
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

    for section in [
        "supervisor",
        "policy",
        "shutdown",
        "observability",
        "audit",
        "backpressure",
        "groups",
        "group_strategies",
        "group_dependencies",
        "child_strategy_overrides",
        "severity_defaults",
        "dashboard",
        "children",
    ] {
        assert!(
            properties.contains_key(section),
            "missing section {section}"
        );
    }
}

/// Verifies that optional child fields do not leak serde defaults into YAML completion.
#[test]
fn child_optional_fields_do_not_emit_null_defaults() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_value = serde_json::to_value(&schema).expect("serialize schema");
    let child_properties = schema_value
        .pointer("/definitions/ChildDeclaration/properties")
        .or_else(|| schema_value.pointer("/$defs/ChildDeclaration/properties"))
        .and_then(Value::as_object)
        .expect("child field schemas");

    for field in [
        "factory_key",
        "sidecar_config",
        "severity",
        "group",
        "health_check",
        "readiness",
        "command_permissions",
    ] {
        let field_schema = child_properties.get(field).expect("child field schema");

        assert!(
            field_schema.get("default").is_none(),
            "{field} schema must not emit default: null"
        );
    }

    let task_role_schema = child_properties
        .get("task_role")
        .expect("child task_role schema");
    assert_eq!(
        task_role_schema.get("default"),
        Some(&Value::String("worker".into())),
        "task_role schema should document the worker fallback default"
    );
}

/// Builds a test task factory registry for schema completion tests.
fn registry() -> TaskFactoryRegistry {
    let mut registry = TaskFactoryRegistry::new();
    registry
        .register(TaskFactoryDescriptor::new(
            "api_server",
            "API Server",
            "Runs the API service.",
            [TaskKind::AsyncWorker],
            Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
        ))
        .expect("register api factory");
    registry
        .register(TaskFactoryDescriptor::new(
            "report_exporter",
            "Report Exporter",
            "Runs blocking export work.",
            [TaskKind::BlockingWorker],
            Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
        ))
        .expect("register exporter factory");
    registry
}

/// Verifies that registry-backed schema completion exposes factory keys.
#[test]
fn supervisor_schema_with_factory_registry_injects_factory_key_completion() {
    let schema_value =
        supervisor_schema_with_factory_registry(&registry()).expect("schema enrichment");
    let factory_key_schema = schema_value
        .pointer("/definitions/ChildDeclaration/properties/factory_key")
        .or_else(|| schema_value.pointer("/$defs/ChildDeclaration/properties/factory_key"))
        .expect("factory_key schema");
    let completion_text =
        serde_json::to_string(factory_key_schema).expect("stringify factory_key schema");

    assert!(completion_text.contains("api_server"));
    assert!(completion_text.contains("API Server"));
    assert!(completion_text.contains("report_exporter"));
    assert!(completion_text.contains("Report Exporter"));
}

/// Verifies that split children schemas receive factory key completion.
#[test]
fn split_children_schema_receives_factory_key_completion() {
    let targets = supervisor_schema_targets_with_factory_registry(
        "config/supervisor_config.schema.json",
        &registry(),
    )
    .expect("schema targets");
    let children_target = targets
        .into_iter()
        .find(|target| target.path.ends_with("children.schema.json"))
        .expect("children schema target");
    let schema_value =
        serde_json::from_str::<Value>(&children_target.content).expect("children schema json");
    let factory_key_schema = schema_value
        .pointer("/items/properties/factory_key")
        .expect("factory_key schema");
    let completion_text =
        serde_json::to_string(factory_key_schema).expect("stringify factory_key schema");

    assert!(completion_text.contains("api_server"));
    assert!(completion_text.contains("report_exporter"));
}

/// Verifies that health check completion stays field-by-field.
#[test]
fn child_health_check_schema_uses_field_by_field_completion() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_value = serde_json::to_value(&schema).expect("serialize schema");
    let health_check_schema = schema_value
        .pointer("/definitions/ChildDeclaration/properties/health_check")
        .or_else(|| schema_value.pointer("/$defs/ChildDeclaration/properties/health_check"))
        .expect("child health check schema");

    assert!(
        health_check_schema.get("default").is_none(),
        "health_check schema must not emit default: null"
    );

    assert!(
        health_check_schema.get("defaultSnippets").is_none(),
        "health_check schema must not emit aggregate snippets"
    );

    let health_check_fields = schema_value
        .pointer("/definitions/HealthCheckConfig/properties")
        .or_else(|| schema_value.pointer("/$defs/HealthCheckConfig/properties"))
        .and_then(Value::as_object)
        .expect("health check field schemas");

    for field in ["check_interval_secs", "timeout_secs", "max_retries"] {
        assert!(
            health_check_fields.contains_key(field),
            "health_check nested completion is missing {field}"
        );
    }
}

/// Verifies that readiness completion exposes policy values, not unused check settings.
#[test]
fn child_readiness_schema_uses_policy_enum() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_value = serde_json::to_value(&schema).expect("serialize schema");
    let readiness_schema = schema_value
        .pointer("/definitions/ChildDeclaration/properties/readiness")
        .or_else(|| schema_value.pointer("/$defs/ChildDeclaration/properties/readiness"))
        .expect("child readiness schema");
    let readiness_text = serde_json::to_string(readiness_schema).expect("stringify readiness");

    assert!(
        readiness_schema.get("default").is_none(),
        "readiness schema must not emit default: null"
    );
    assert!(
        readiness_text.contains("ReadinessPolicy"),
        "readiness must reference the readiness policy enum"
    );

    let definitions = schema_value
        .get("definitions")
        .or_else(|| schema_value.get("$defs"))
        .and_then(Value::as_object)
        .expect("schema definitions");

    assert!(
        definitions.get("ReadinessConfig").is_none(),
        "unused readiness check config must not be public schema"
    );
    assert!(
        definitions.get("ResourceLimits").is_none(),
        "unused resource limits must not be public schema"
    );
}

/// Verifies that the public schema never advertises null as a completion default.
#[test]
fn supervisor_config_schema_does_not_emit_null_defaults() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_value = serde_json::to_value(&schema).expect("serialize schema");
    let mut null_default_paths = Vec::new();

    collect_null_default_paths(&schema_value, "$", &mut null_default_paths);

    assert!(
        null_default_paths.is_empty(),
        "schema must not emit default: null at {}",
        null_default_paths.join(", ")
    );
}

/// Collects schema object paths whose default value is explicit null.
fn collect_null_default_paths(value: &Value, path: &str, paths: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if object.get("default") == Some(&Value::Null) {
                paths.push(path.to_string());
            }

            for (key, child) in object {
                let child_path = format!("{path}.{key}");
                collect_null_default_paths(child, &child_path, paths);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let child_path = format!("{path}[{index}]");
                collect_null_default_paths(child, &child_path, paths);
            }
        }
        _ => {}
    }
}
