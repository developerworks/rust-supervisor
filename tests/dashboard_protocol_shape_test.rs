use rust_supervisor::dashboard::config::ValidatedDashboardIpcConfig;
use rust_supervisor::dashboard::ipc_server::DashboardIpcService;
use rust_supervisor::dashboard::ipc_server::validate_command;
use rust_supervisor::dashboard::model::{
    ControlCommandKind, ControlCommandRequest, ControlCommandTarget,
};
use rust_supervisor::dashboard::protocol::{
    IpcMethod, IpcRequest, IpcResponse, IpcResult, parse_request_line, response_to_line,
};
use rust_supervisor::dashboard::state::declared_state_from_spec;
use rust_supervisor::journal::ring::EventJournal;
use rust_supervisor::spec::supervisor::SupervisorSpec;

#[test]
fn dashboard_protocol_accepts_only_current_methods() {
    for method in [
        "hello",
        "snapshot",
        "events.subscribe",
        "logs.tail",
        "command.restart_child",
        "command.pause_child",
        "command.resume_child",
        "command.quarantine_child",
        "command.remove_child",
        "command.add_child",
        "command.shutdown_tree",
    ] {
        assert!(IpcMethod::parse(method).is_ok(), "{method} should parse");
    }

    for alias in ["restart", "stop_child", "dashboard.snapshot", "tailLogs"] {
        assert!(
            IpcMethod::parse(alias).is_err(),
            "{alias} should be rejected"
        );
    }
}

#[test]
fn dashboard_protocol_parses_newline_json_request() {
    let request = parse_request_line(
        r#"{"request_id":"r1","method":"snapshot","params":{"target_id":"payments"}}"#,
    )
    .expect("valid request");

    assert_eq!(request.request_id, "r1");
    assert_eq!(request.method, "snapshot");
}

#[test]
fn dashboard_protocol_serializes_structured_error_response() {
    let response = IpcResponse::error(
        "r2",
        rust_supervisor::dashboard::error::DashboardError::unsupported_method("old.restart"),
    );
    let line = response_to_line(&response).expect("response line");

    assert!(line.ends_with('\n'));
    assert!(line.contains("unsupported_method"));
}

#[test]
fn dashboard_protocol_serializes_subscription_result() {
    let response = IpcResponse::ok(
        "r3",
        IpcResult::Subscription {
            target_id: "payments".to_owned(),
            subscription: "events".to_owned(),
        },
    );
    let line = response_to_line(&response).expect("response line");

    assert!(line.contains("subscription"));
    assert!(line.contains("payments"));
}

#[test]
fn command_validation_rejects_empty_reason_and_missing_confirmation() {
    let command = ControlCommandRequest {
        command_id: "cmd-1".to_owned(),
        target_id: "payments".to_owned(),
        command: ControlCommandKind::ShutdownTree,
        target: ControlCommandTarget {
            child_path: None,
            child_manifest: None,
        },
        reason: " ".to_owned(),
        requested_by: "operator".to_owned(),
        confirmed: false,
        requested_at_unix_nanos: 1,
    };

    assert!(validate_command(&command).is_err());
}

#[tokio::test]
async fn target_ipc_rejects_command_for_different_target_id() {
    let config = ValidatedDashboardIpcConfig {
        target_id: "payments-worker-a".to_owned(),
        path: "/tmp/payments-worker-a.sock".into(),
        permissions: "0600".to_owned(),
        bind_mode: rust_supervisor::config::configurable::DashboardIpcBindMode::CreateNew,
        registration: None,
    };
    let spec = SupervisorSpec::root(Vec::new());
    let state = declared_state_from_spec(&spec);
    let service = DashboardIpcService::new(config, spec, state, EventJournal::new(16));

    let response = service
        .handle_request(IpcRequest {
            request_id: "r4".to_owned(),
            method: "command.pause_child".to_owned(),
            params: serde_json::json!({
                "command_id": "cmd-1",
                "target_id": "orders-worker-b",
                "command": "pause_child",
                "target": {"child_path": "/root/payment_loop"},
                "reason": "operator supplied reason",
                "requested_by": "operator@example.test",
                "confirmed": false,
                "requested_at_unix_nanos": 1
            }),
        })
        .await;

    let error = response.error.expect("target mismatch should fail");
    assert!(!response.ok);
    assert_eq!(error.code, "validation_failed");
    assert_eq!(error.stage, "command_validate");
    assert_eq!(error.target_id.as_deref(), Some("payments-worker-a"));
}
