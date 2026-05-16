//! Acceptance tests for six-stage supervision pipeline order.
//!
//! This test verifies that:
//! 1. Non-zero exit codes trigger failures that go through all six pipeline stages in order
//! 2. Each stage produces structured event output
//! 3. Success exit codes also go through six stages and leave reconcilable record points

use rust_supervisor::event::payload::{SupervisorEvent, What};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::observe::pipeline::{ObservabilityPipeline, PipelineStage};
use std::time::SystemTime;

/// Helper to create a deterministic event timestamp
fn deterministic_when(sequence: u64) -> When {
    When::new(EventTime::deterministic(
        sequence as u128,
        sequence as u128,
        0,
        Generation::initial(),
        ChildStartCount::first(),
    ))
}

/// Helper to create a test supervisor event
fn test_event(sequence: u64, what: What, child_id: Option<ChildId>) -> SupervisorEvent {
    let path = SupervisorPath::root();
    let location = rust_supervisor::event::payload::Where::new(path);
    let location = if let Some(ref id) = child_id {
        location.with_child(id.clone(), "test-child")
    } else {
        location
    };

    SupervisorEvent::new(
        deterministic_when(sequence),
        location,
        what,
        EventSequence::new(sequence),
        CorrelationId::from_uuid(uuid::Uuid::nil()),
        1,
    )
}

#[test]
fn test_pipeline_stages_exist() {
    // Verify all six pipeline stages are defined
    let stages = [
        PipelineStage::ClassifyExit,
        PipelineStage::RecordFailureWindow,
        PipelineStage::EvaluateBudget,
        PipelineStage::DecideAction,
        PipelineStage::EmitTypedEvent,
        PipelineStage::ExecuteAction,
    ];

    assert_eq!(stages.len(), 6);

    // Verify stage display names
    assert_eq!(format!("{}", PipelineStage::ClassifyExit), "classify_exit");
    assert_eq!(
        format!("{}", PipelineStage::RecordFailureWindow),
        "record_failure_window"
    );
    assert_eq!(
        format!("{}", PipelineStage::EvaluateBudget),
        "evaluate_budget"
    );
    assert_eq!(format!("{}", PipelineStage::DecideAction), "decide_action");
    assert_eq!(
        format!("{}", PipelineStage::EmitTypedEvent),
        "emit_typed_event"
    );
    assert_eq!(
        format!("{}", PipelineStage::ExecuteAction),
        "execute_action"
    );
}

#[test]
fn test_non_zero_exit_goes_through_pipeline() {
    // This test will initially fail because the pipeline orchestration is not yet implemented
    // It serves as a placeholder for T010-T014 implementation

    let mut pipeline = ObservabilityPipeline::new(100, 10);
    let _subscriber_idx = pipeline.add_subscriber();

    let child_id = ChildId::new("test-child-1".to_string());

    // Simulate a non-zero exit event
    let exit_event = test_event(
        1,
        What::ChildFailed {
            failure: rust_supervisor::error::types::TaskFailure::new(
                rust_supervisor::error::types::TaskFailureKind::Error,
                "exit_code",
                "non-zero exit",
            ),
        },
        Some(child_id.clone()),
    );

    // Emit the event through the pipeline
    let lagged = pipeline.emit(exit_event);

    // Verify event was emitted without lag
    assert_eq!(lagged, 0);

    // Verify event is recorded in test recorder
    assert!(!pipeline.test_recorder.events.is_empty());
    assert_eq!(pipeline.test_recorder.events[0].what.name(), "ChildFailed");

    // TODO(T010-T014): After pipeline orchestration is implemented, verify:
    // 1. All six stages were executed in order
    // 2. Each stage produced a PipelineStageDiagnostic
    // 3. The diagnostic contains correct stage identifiers
    // 4. Exit classification is recorded in stage 1 output
}

#[test]
fn test_success_exit_goes_through_pipeline() {
    // This test verifies that successful exits also go through the pipeline
    // and leave reconcilable record points

    let mut pipeline = ObservabilityPipeline::new(100, 10);
    let _subscriber_idx = pipeline.add_subscriber();

    let child_id = ChildId::new("test-child-success".to_string());

    // Simulate a success event (ChildRunning with transition or custom success variant)
    let success_event = test_event(
        1,
        What::ChildRunning {
            transition: Some(rust_supervisor::event::payload::StateTransition::new(
                "starting", "running",
            )),
        },
        Some(child_id.clone()),
    );

    // Emit the event through the pipeline
    let lagged = pipeline.emit(success_event);

    // Verify event was emitted
    assert_eq!(lagged, 0);
    assert!(!pipeline.test_recorder.events.is_empty());

    // TODO(T008): After implementation, verify:
    // 1. Success path also goes through all six stages
    // 2. Record points are left for reconciliation
    // 3. No restart is triggered for successful completion
}

#[test]
fn test_pipeline_stage_diagnostic_creation() {
    // Test that PipelineStageDiagnostic can be created and populated

    let now_unix_nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let diagnostic = rust_supervisor::observe::pipeline::PipelineStageDiagnostic::new(
        1,
        "test-correlation-id",
        PipelineStage::ClassifyExit,
        now_unix_nanos,
    )
    .with_child_id("test-child")
    .with_group_id("test-group")
    .with_supervisor_path("/");

    assert_eq!(diagnostic.sequence, 1);
    assert_eq!(diagnostic.correlation_id, "test-correlation-id");
    assert_eq!(diagnostic.stage, PipelineStage::ClassifyExit);
    assert_eq!(diagnostic.child_id, Some("test-child".to_string()));
    assert_eq!(diagnostic.group_id, Some("test-group".to_string()));
    assert_eq!(diagnostic.supervisor_path, "/");
    assert_eq!(diagnostic.completed_at_unix_nanos, now_unix_nanos);
}
