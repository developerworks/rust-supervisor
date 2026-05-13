use rust_supervisor::config::yaml::parse_config_state;
use rust_supervisor::runtime::supervisor::Supervisor;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::{sleep, timeout};

fn test_directory(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = PathBuf::from("/tmp").join(format!("rsdash-{name}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&path).expect("create temp directory");
    path
}

fn dashboard_yaml(
    ipc_path: &Path,
    register_path: &Path,
    bind_mode: &str,
    registration_enabled: bool,
    heartbeat_seconds: u64,
    lease_seconds: u64,
) -> String {
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
  path: {}
  permissions: "0600"
  bind_mode: {bind_mode}
  registration:
    enabled: {registration_enabled}
    relay_registration_path: {}
    display_name: "payments worker a"
    lease_seconds: {lease_seconds}
    registration_heartbeat_interval_seconds: {heartbeat_seconds}
"#,
        ipc_path.display(),
        register_path.display(),
    )
}

async fn request_state(ipc_path: &Path) -> Value {
    let stream = UnixStream::connect(ipc_path).await.expect("connect IPC");
    let mut reader = BufReader::new(stream);
    reader
        .get_mut()
        .write_all(
            br#"{"request_id":"r1","method":"state","params":{"target_id":"payments-worker-a"}}
"#,
        )
        .await
        .expect("write state request");
    let mut line = String::new();
    reader.read_line(&mut line).await.expect("read response");
    serde_json::from_str(line.trim()).expect("response json")
}

async fn wait_until_removed(path: &Path) {
    for _ in 0..20 {
        if !path.exists() {
            return;
        }
        sleep(Duration::from_millis(10)).await;
    }
    assert!(!path.exists(), "socket path should be removed");
}

#[tokio::test]
async fn start_from_config_state_starts_ipc_and_registration_heartbeat() {
    let directory = test_directory("runtime-start");
    let ipc_path = directory.join("target.sock");
    let register_path = directory.join("register.sock");
    let listener = UnixListener::bind(&register_path).expect("bind registration listener");
    let state = parse_config_state(&dashboard_yaml(
        &ipc_path,
        &register_path,
        "create_new",
        true,
        10,
        30,
    ))
    .expect("parse config");
    let expected_ipc_path = ipc_path.clone();

    let relay = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept registration");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read registration");
        let payload: Value = serde_json::from_str(line.trim()).expect("registration json");
        assert_eq!(payload["target_id"], "payments-worker-a");
        assert_eq!(
            payload["ipc_path"],
            expected_ipc_path.to_string_lossy().as_ref()
        );
        assert_eq!(payload["lease_seconds"], 30);
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

    let handle = Supervisor::start_from_config_state(state)
        .await
        .expect("start from config");
    relay.await.expect("relay task");

    let response = request_state(&ipc_path).await;
    assert_eq!(response["request_id"], "r1");
    assert_eq!(response["ok"], true);
    assert_eq!(response["result"]["type"], "state");
    assert_eq!(response["result"]["target_id"], "payments-worker-a");

    drop(handle);
    wait_until_removed(&ipc_path).await;
    std::fs::remove_dir_all(directory).expect("remove temp directory");
}

#[tokio::test]
async fn non_retryable_registration_ack_stops_fixed_heartbeat() {
    let directory = test_directory("runtime-nonretryable");
    let ipc_path = directory.join("target.sock");
    let register_path = directory.join("register.sock");
    let listener = UnixListener::bind(&register_path).expect("bind registration listener");
    let state = parse_config_state(&dashboard_yaml(
        &ipc_path,
        &register_path,
        "create_new",
        true,
        1,
        4,
    ))
    .expect("parse config");

    let relay = tokio::spawn(async move {
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
        assert!(
            timeout(Duration::from_millis(1200), listener.accept())
                .await
                .is_err(),
            "heartbeat should stop after non-retryable ack"
        );
    });

    let handle = Supervisor::start_from_config_state(state)
        .await
        .expect("start from config");
    relay.await.expect("relay task");
    drop(handle);
    wait_until_removed(&ipc_path).await;
    std::fs::remove_dir_all(directory).expect("remove temp directory");
}

#[tokio::test]
async fn unavailable_registration_socket_does_not_fail_startup() {
    let directory = test_directory("runtime-unavailable-relay");
    let ipc_path = directory.join("target.sock");
    let register_path = directory.join("missing-register.sock");
    let state = parse_config_state(&dashboard_yaml(
        &ipc_path,
        &register_path,
        "create_new",
        true,
        10,
        30,
    ))
    .expect("parse config");

    let handle = Supervisor::start_from_config_state(state)
        .await
        .expect("start from config despite missing relay");
    let response = request_state(&ipc_path).await;
    assert_eq!(response["ok"], true);

    drop(handle);
    wait_until_removed(&ipc_path).await;
    std::fs::remove_dir_all(directory).expect("remove temp directory");
}

#[tokio::test]
async fn create_new_rejects_existing_live_socket() {
    let directory = test_directory("runtime-create-new-live");
    let ipc_path = directory.join("target.sock");
    let register_path = directory.join("register.sock");
    let _live_listener = UnixListener::bind(&ipc_path).expect("bind live IPC socket");
    let state = parse_config_state(&dashboard_yaml(
        &ipc_path,
        &register_path,
        "create_new",
        false,
        10,
        30,
    ))
    .expect("parse config");

    let result = Supervisor::start_from_config_state(state).await;

    assert!(result.is_err());
    std::fs::remove_dir_all(directory).expect("remove temp directory");
}

#[tokio::test]
async fn replace_stale_deletes_only_unserved_unix_socket() {
    let directory = test_directory("runtime-replace-stale");
    let ipc_path = directory.join("target.sock");
    let register_path = directory.join("register.sock");
    let stale_listener = UnixListener::bind(&ipc_path).expect("bind stale IPC socket");
    drop(stale_listener);
    let state = parse_config_state(&dashboard_yaml(
        &ipc_path,
        &register_path,
        "replace_stale",
        false,
        10,
        30,
    ))
    .expect("parse config");

    let handle = Supervisor::start_from_config_state(state)
        .await
        .expect("replace stale socket");
    let response = request_state(&ipc_path).await;
    assert_eq!(response["ok"], true);

    drop(handle);
    wait_until_removed(&ipc_path).await;
    std::fs::remove_dir_all(directory).expect("remove temp directory");
}

#[tokio::test]
async fn replace_stale_rejects_live_socket() {
    let directory = test_directory("runtime-replace-live");
    let ipc_path = directory.join("target.sock");
    let register_path = directory.join("register.sock");
    let _live_listener = UnixListener::bind(&ipc_path).expect("bind live IPC socket");
    let state = parse_config_state(&dashboard_yaml(
        &ipc_path,
        &register_path,
        "replace_stale",
        false,
        10,
        30,
    ))
    .expect("parse config");

    let result = Supervisor::start_from_config_state(state).await;

    assert!(result.is_err());
    std::fs::remove_dir_all(directory).expect("remove temp directory");
}

#[tokio::test]
async fn replace_stale_rejects_symlink_path() {
    let directory = test_directory("runtime-replace-symlink");
    let ipc_path = directory.join("target.sock");
    let linked_path = directory.join("linked.sock");
    let register_path = directory.join("register.sock");
    std::os::unix::fs::symlink(&linked_path, &ipc_path).expect("create symlink");
    let state = parse_config_state(&dashboard_yaml(
        &ipc_path,
        &register_path,
        "replace_stale",
        false,
        10,
        30,
    ))
    .expect("parse config");

    let result = Supervisor::start_from_config_state(state).await;

    assert!(result.is_err());
    std::fs::remove_dir_all(directory).expect("remove temp directory");
}
