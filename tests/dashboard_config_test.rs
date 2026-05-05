use rust_supervisor::config::configurable::SupervisorConfig;
use rust_supervisor::config::state::ConfigState;
use rust_supervisor::config::yaml::parse_config_state;

fn dashboard_yaml(path: &str) -> String {
    format!(
        r#"
supervisor:
  strategy: OneForAll
policy:
  child_restart_limit: 10
  child_restart_window_ms: 60000
  supervisor_failure_limit: 30
  supervisor_failure_window_ms: 60000
  initial_backoff_ms: 100
  max_backoff_ms: 5000
  jitter_ratio: 0.10
  heartbeat_interval_ms: 1000
  stale_after_ms: 3000
shutdown:
  graceful_timeout_ms: 5000
  abort_wait_ms: 1000
observability:
  event_journal_capacity: 256
  metrics_enabled: true
  audit_enabled: true
ipc:
  enabled: true
  target_id: payments-worker-a
  path: {path}
  permissions: "0600"
  bind_mode: create_new
  registration:
    enabled: true
    relay_registration_path: /run/rust-supervisor/dashboard-relay-registration.sock
    display_name: "payments worker a"
    authorization_scope: "payments:operate"
    lease_seconds: 30
"#
    )
}

#[test]
fn dashboard_ipc_config_loads_and_validates_absolute_paths() {
    let state = parse_config_state(&dashboard_yaml("/run/rust-supervisor/payments.sock"))
        .expect("dashboard IPC config should load");

    let ipc = state.ipc.expect("ipc section");
    assert!(ipc.enabled);
    assert_eq!(ipc.target_id.as_deref(), Some("payments-worker-a"));
    assert!(ipc.path.expect("ipc path").is_absolute());
}

#[test]
fn dashboard_ipc_config_rejects_relative_ipc_path() {
    let result = parse_config_state(&dashboard_yaml("relative.sock"));

    assert!(result.is_err());
}

#[test]
fn dashboard_ipc_schema_exposes_optional_ipc_section() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let text = serde_json::to_string(&schema).expect("schema string");

    for field in [
        "ipc",
        "target_id",
        "relay_registration_path",
        "authorization_scope",
        "lease_seconds",
    ] {
        assert!(text.contains(field), "schema missing {field}");
    }
}

#[test]
fn current_repository_does_not_define_relay_or_ui_production_paths() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    assert!(
        !root
            .join("src/bin/rust-supervisor-dashboard-sidecar.rs")
            .exists()
    );
    assert!(!root.join("dashboard").exists());
}

#[test]
fn disabled_ipc_keeps_existing_configs_valid() {
    let yaml = dashboard_yaml("/run/rust-supervisor/payments.sock")
        .replace("enabled: true", "enabled: false");
    let config: SupervisorConfig = serde_yaml::from_str(&yaml).expect("deserialize config");
    let state = ConfigState::try_from(config).expect("disabled IPC should validate");

    assert!(state.ipc.is_some());
}
