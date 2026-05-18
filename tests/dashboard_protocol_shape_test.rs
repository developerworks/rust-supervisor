use rust_supervisor::control::command::{CommandResult, CurrentState};
use rust_supervisor::control::outcome::{
    ChildAttemptStatus, ChildControlOperation, ChildControlResult, ChildLivenessState,
    ChildRuntimeRecord, ChildStopState, GenerationFencePhase, RestartLimitState,
};
use rust_supervisor::dashboard::config::ValidatedDashboardIpcConfig;
use rust_supervisor::dashboard::ipc_server::DashboardIpcService;
use rust_supervisor::dashboard::ipc_server::validate_command;
use rust_supervisor::dashboard::model::{
    ControlCommandKind, ControlCommandRequest, ControlCommandTarget, DashboardManagedChildState,
    dashboard_command_result_value,
};
use rust_supervisor::dashboard::protocol::{
    IpcMethod, IpcRequest, IpcResponse, IpcResult, parse_request_line, response_to_line,
};
use rust_supervisor::dashboard::state::declared_state_from_spec;
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::journal::ring::EventJournal;
use rust_supervisor::readiness::signal::ReadinessState;
use rust_supervisor::spec::supervisor::SupervisorSpec;
use std::time::Duration;

#[test]
fn dashboard_protocol_accepts_only_current_methods() {
    for method in [
        "hello",
        "state",
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

    let old_state_query = ["snap", "shot"].concat();
    let old_dashboard_alias = ["dashboard.", &old_state_query].concat();
    for alias in [
        "restart",
        "stop_child",
        old_dashboard_alias.as_str(),
        "tailLogs",
    ] {
        assert!(
            IpcMethod::parse(alias).is_err(),
            "{alias} should be rejected"
        );
    }
}

#[test]
fn dashboard_protocol_parses_newline_json_request() {
    let request = parse_request_line(
        r#"{"request_id":"r1","method":"state","params":{"target_id":"payments"}}"#,
    )
    .expect("valid request");

    assert_eq!(request.request_id, "r1");
    assert_eq!(request.method, "state");
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

#[test]
fn shutdown_tree_command_request_shape_stays_stable() {
    let request = ControlCommandRequest {
        command_id: "cmd-2".to_owned(),
        target_id: "payments".to_owned(),
        command: ControlCommandKind::ShutdownTree,
        target: ControlCommandTarget {
            child_path: None,
            child_manifest: None,
        },
        reason: "operator requested shutdown".to_owned(),
        requested_by: "operator@example.test".to_owned(),
        confirmed: true,
        requested_at_unix_nanos: 42,
    };

    let value = serde_json::to_value(&request).expect("command request should serialize");

    assert_eq!(value["command"], "shutdown_tree");
    assert_eq!(value["target"]["child_path"], serde_json::Value::Null);
    assert_eq!(value["target"]["child_manifest"], serde_json::Value::Null);
    assert!(value.get("report").is_none());
    assert!(value.get("shutdown_result").is_none());
}

#[test]
fn child_control_command_request_shape_stays_stable() {
    for (command, expected_name) in [
        (ControlCommandKind::PauseChild, "pause_child"),
        (ControlCommandKind::RemoveChild, "remove_child"),
        (ControlCommandKind::QuarantineChild, "quarantine_child"),
    ] {
        let request = ControlCommandRequest {
            command_id: format!("cmd-{expected_name}"),
            target_id: "payments".to_owned(),
            command,
            target: ControlCommandTarget {
                child_path: Some("/root/payment_loop".to_owned()),
                child_manifest: None,
            },
            reason: "operator requested child control".to_owned(),
            requested_by: "operator@example.test".to_owned(),
            confirmed: true,
            requested_at_unix_nanos: 42,
        };

        let value = serde_json::to_value(&request).expect("command request should serialize");

        assert_eq!(value["command"], expected_name);
        assert_eq!(value["target"]["child_path"], "/root/payment_loop");
        assert_eq!(value["target"]["child_manifest"], serde_json::Value::Null);
        assert!(value.get("report").is_none());
        assert!(value.get("outcome").is_none());
        assert!(value.get("child_runtime_records").is_none());
    }
}

#[test]
fn dashboard_model_maps_operation_to_managed_child_state() {
    assert_eq!(
        DashboardManagedChildState::from(ChildControlOperation::Active),
        DashboardManagedChildState::Running
    );
    assert_eq!(
        DashboardManagedChildState::from(ChildControlOperation::Paused),
        DashboardManagedChildState::Paused
    );
    assert_eq!(
        DashboardManagedChildState::from(ChildControlOperation::Quarantined),
        DashboardManagedChildState::Quarantined
    );
    assert_eq!(
        DashboardManagedChildState::from(ChildControlOperation::Removed),
        DashboardManagedChildState::Removed
    );
}

#[test]
fn dashboard_command_result_model_serializes_child_control_shape() {
    let result = CommandResult::ChildControl {
        outcome: child_control_result(),
    };

    let value = dashboard_command_result_value(&result).expect("dashboard command result");

    assert_eq!(value["type"], "child_control");
    assert_eq!(value["outcome"]["child_id"], "payment_loop");
    assert_eq!(value["outcome"]["operation_before"], "active");
    assert_eq!(value["outcome"]["operation_after"], "paused");
    assert_eq!(value["outcome"]["managed_child_state_before"], "running");
    assert_eq!(value["outcome"]["managed_child_state_after"], "paused");
    assert_eq!(value["outcome"]["status"], "cancelling");
    assert_eq!(value["outcome"]["cancel_delivered"], true);
    assert_eq!(value["outcome"]["stop_state"], "cancel_delivered");
    assert_eq!(value["outcome"]["restart_limit"]["remaining"], 2);
    assert_eq!(value["outcome"]["liveness"]["readiness"], "ready");
    assert!(value["outcome"].get("generation_fence").is_some());
    assert_eq!(
        value["outcome"]["generation_fence"],
        serde_json::Value::Null
    );
    assert!(value.get("ChildState").is_none());
}

#[test]
fn dashboard_command_result_model_serializes_current_state_runtime_records() {
    let result = CommandResult::CurrentState {
        state: CurrentState {
            child_count: 1,
            shutdown_completed: false,
            child_runtime_records: vec![child_runtime_record(ChildControlOperation::Active)],
        },
    };

    let value = dashboard_command_result_value(&result).expect("dashboard command result");
    let record = &value["state"]["child_runtime_records"][0];

    assert_eq!(value["type"], "current_state");
    assert_eq!(value["state"]["child_count"], 1);
    assert_eq!(record["child_id"], "payment_loop");
    assert_eq!(record["child_path"], "/payment_loop");
    assert_eq!(record["operation"], "active");
    assert_eq!(record["managed_child_state"], "running");
    assert_eq!(record["status"], "running");
    assert_eq!(record["generation"], 0);
    assert_eq!(record["attempt"], 1);
    assert_eq!(record["restart_limit"]["remaining"], 2);
    assert_eq!(record["liveness"]["readiness"], "ready");
    assert_eq!(record["generation_fence_phase"], "open");
    assert_eq!(record["pending_restart"], serde_json::Value::Null);
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

fn child_control_result() -> ChildControlResult {
    ChildControlResult::new(
        ChildId::new("payment_loop"),
        Some(ChildStartCount::first()),
        Some(Generation::initial()),
        ChildControlOperation::Active,
        ChildControlOperation::Paused,
        Some(ChildAttemptStatus::Cancelling),
        true,
        ChildStopState::CancelDelivered,
        restart_limit(),
        liveness(),
        false,
        None,
        None,
    )
}

fn child_runtime_record(operation: ChildControlOperation) -> ChildRuntimeRecord {
    ChildRuntimeRecord::new(
        ChildId::new("payment_loop"),
        SupervisorPath::root().join("payment_loop"),
        Some(Generation::initial()),
        Some(ChildStartCount::first()),
        Some(ChildAttemptStatus::Running),
        operation,
        liveness(),
        restart_limit(),
        ChildStopState::Idle,
        None,
        GenerationFencePhase::Open,
        None,
    )
}

fn liveness() -> ChildLivenessState {
    ChildLivenessState::new(Some(100), false, ReadinessState::Ready)
}

fn restart_limit() -> RestartLimitState {
    RestartLimitState::new(Duration::from_secs(60), 3, 1, 100)
}
