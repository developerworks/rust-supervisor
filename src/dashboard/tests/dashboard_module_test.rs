//! Module-owned tests for dashboard public entry points.

use rust_supervisor::config::configurable::{
    DashboardIpcBindMode, DashboardIpcConfig, DashboardRegistrationConfig,
};
use rust_supervisor::dashboard::config::validate_dashboard_ipc_config;
use rust_supervisor::dashboard::registration::build_registration_payload;

/// Verifies that dashboard registration keeps target identity stable.
#[test]
fn dashboard_registration_payload_uses_validated_target_identity() {
    let config = DashboardIpcConfig {
        enabled: true,
        target_id: Some("orders-supervisor".to_string()),
        path: Some("/tmp/orders-supervisor.sock".into()),
        permissions: Some("0600".to_string()),
        bind_mode: Some(DashboardIpcBindMode::ReplaceStale),
        registration: Some(DashboardRegistrationConfig {
            enabled: true,
            relay_registration_path: Some("/tmp/rust-supervisor-relay/register.sock".into()),
            display_name: Some("Orders Supervisor".to_string()),
            authorization_scope: Some("ops:orders".to_string()),
            lease_seconds: Some(30),
        }),
    };

    let validated = validate_dashboard_ipc_config(Some(&config))
        .expect("validate dashboard IPC")
        .expect("enabled dashboard IPC");
    let registration = build_registration_payload(&validated).expect("build registration payload");

    assert_eq!(registration.target_id, "orders-supervisor");
    assert_eq!(registration.ipc_path, "/tmp/orders-supervisor.sock");
    assert_eq!(registration.lease_seconds, 30);
}

/// Verifies that module-owned validation accepts an absolute IPC path.
#[test]
fn dashboard_validation_accepts_absolute_socket_paths() {
    let config = DashboardIpcConfig {
        enabled: true,
        target_id: Some("billing-supervisor".to_string()),
        path: Some("/tmp/billing-supervisor.sock".into()),
        permissions: None,
        bind_mode: None,
        registration: None,
    };

    let result = validate_dashboard_ipc_config(Some(&config));

    assert!(result.is_ok());
}
