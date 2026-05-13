use rust_supervisor::config::configurable::{
    DashboardIpcBindMode, DashboardIpcConfig, DashboardRegistrationConfig,
};
use rust_supervisor::dashboard::config::{
    ValidatedDashboardIpcConfig, validate_dashboard_ipc_config,
};
use rust_supervisor::dashboard::registration::{
    run_registration_heartbeat, send_registration_upsert,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

fn test_directory(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = PathBuf::from("/tmp").join(format!("rsdash-{name}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&path).expect("create temp directory");
    path
}

fn dashboard_config(
    relay_registration_path: std::path::PathBuf,
    ipc_path: std::path::PathBuf,
) -> ValidatedDashboardIpcConfig {
    let config = DashboardIpcConfig {
        enabled: true,
        target_id: Some("payments-worker-a".to_owned()),
        path: Some(ipc_path),
        permissions: Some("0600".to_owned()),
        bind_mode: Some(DashboardIpcBindMode::CreateNew),
        registration: Some(DashboardRegistrationConfig {
            enabled: true,
            relay_registration_path: Some(relay_registration_path),
            display_name: Some("payments worker a".to_owned()),
            lease_seconds: Some(30),
            registration_heartbeat_interval_seconds: Some(15),
        }),
    };
    validate_dashboard_ipc_config(Some(&config))
        .expect("config should validate")
        .expect("IPC should be enabled")
}

#[tokio::test]
async fn registration_upsert_writes_payload_and_reads_ack() {
    let directory = test_directory("registration-upsert");
    let register_path = directory.join("register.sock");
    let ipc_path = directory.join("target.sock");
    let listener = UnixListener::bind(&register_path).expect("bind registration listener");
    let config = dashboard_config(register_path, ipc_path);

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept registration");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read registration");
        let payload: serde_json::Value =
            serde_json::from_str(line.trim()).expect("registration json");
        assert_eq!(payload["target_id"], "payments-worker-a");
        let old_key = ["authorization", "_scope"].concat();
        assert!(payload.get(&old_key).is_none());
        assert!(payload["supported_commands"].as_array().is_some());
        reader
            .get_mut()
            .write_all(
                br#"{"ok":true,"target_id":"payments-worker-a","status":"registered","retryable":false}
"#,
            )
            .await
            .expect("write ack");
    });

    let ack = send_registration_upsert(&config)
        .await
        .expect("registration should send");
    server.await.expect("server task");

    assert!(ack.ok);
    assert_eq!(ack.target_id.as_deref(), Some("payments-worker-a"));
    std::fs::remove_dir_all(directory).expect("remove temp directory");
}

#[tokio::test]
async fn registration_heartbeat_stops_on_non_retryable_ack() {
    let directory = test_directory("registration-heartbeat");
    let register_path = directory.join("register.sock");
    let ipc_path = directory.join("target.sock");
    let listener = UnixListener::bind(&register_path).expect("bind registration listener");
    let config = dashboard_config(register_path, ipc_path);

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept registration");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read registration");
        reader
            .get_mut()
            .write_all(
                br#"{"ok":false,"error":{"code":"ipc_path_conflict","message":"ipc path is already used"},"retryable":false}
"#,
            )
            .await
            .expect("write ack");
    });

    let error = run_registration_heartbeat(config)
        .await
        .expect_err("non-retryable ack should stop heartbeat");
    server.await.expect("server task");

    assert_eq!(error.code, "registration_failed");
    assert!(!error.retryable);
    std::fs::remove_dir_all(directory).expect("remove temp directory");
}
