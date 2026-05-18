//! Acceptance tests for six-stage supervision pipeline order.
//!
//! This test verifies SC-001: In fixed acceptance scenarios, at least 100% of simulated failure
//! samples trigger supervision results consistent with the six-stage pipeline sequence, and
//! reviewers can verify the order from events or diagnostic exports without reading source code.
//!
//! Tests drive the real SupervisionPipeline and assert that each sample produces 6 stage
//! diagnostics in order: classify_exit → record_failure_window → evaluate_budget →
//! decide_action → emit_typed_event → execute_action.

use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::observe::pipeline::{
    ObservabilityPipeline, PipelineStage, PipelineStageDiagnostic,
};
use rust_supervisor::policy::budget::RestartBudgetConfig;
use rust_supervisor::policy::decision::{PolicyFailureKind, TaskExit};
use rust_supervisor::policy::failure_window::{FailureWindow, FailureWindowConfig, WindowMode};
use rust_supervisor::policy::meltdown::{MeltdownPolicy, MeltdownTracker};
use rust_supervisor::runtime::pipeline::{
    ExitClassification, PipelineContext, SupervisionPipeline,
};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::tree::builder::SupervisorTree;
use std::time::{Duration, SystemTime};

/// Creates a test supervision pipeline with default configuration
fn create_test_pipeline() -> SupervisionPipeline {
    let meltdown_policy = MeltdownPolicy::new(
        3,                        // child_restart_limit
        Duration::from_secs(10),  // child_window
        5,                        // group_restart_limit
        Duration::from_secs(30),  // group_window
        10,                       // supervisor_restart_limit
        Duration::from_secs(60),  // supervisor_window
        Duration::from_secs(120), // cooldown
    );
    let meltdown_tracker = MeltdownTracker::new(meltdown_policy);

    let failure_config = FailureWindowConfig {
        mode: WindowMode::TimeSliding { window_secs: 60 },
        threshold: 5,
    };
    let failure_window = FailureWindow::new(failure_config);

    SupervisionPipeline::new(
        100,
        10,
        meltdown_tracker,
        failure_window,
        RestartBudgetConfig::new(Duration::from_secs(60), 10, 0.5),
        vec![],
    )
}

/// Creates a minimal supervisor spec for testing
fn create_test_spec() -> SupervisorSpec {
    SupervisorSpec::root(vec![]) // Empty children list for pipeline testing
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
    // SC-001: Verify non-zero exit goes through all six pipeline stages in order

    // Simulate all minimum exit types for comprehensive testing.
    let exits = vec![
        (
            TaskExit::Succeeded,
            None,
            ExitClassification::Success,
            "success",
        ),
        (
            TaskExit::Failed {
                kind: PolicyFailureKind::Recoverable,
            },
            None,
            ExitClassification::NonZeroExit { exit_code: -1 },
            "nonzero_exit",
        ),
        (
            TaskExit::Failed {
                kind: PolicyFailureKind::Panic,
            },
            None,
            ExitClassification::Crash {
                reason: "panic".to_string(),
            },
            "panic",
        ),
        (
            TaskExit::Failed {
                kind: PolicyFailureKind::Timeout,
            },
            None,
            ExitClassification::Timeout,
            "timeout",
        ),
        (
            TaskExit::Failed {
                kind: PolicyFailureKind::Cancelled,
            },
            None,
            ExitClassification::ExternalCancel,
            "external_cancel",
        ),
        (
            TaskExit::Failed {
                kind: PolicyFailureKind::Cancelled,
            },
            Some(ExitClassification::ManualStop),
            ExitClassification::ManualStop,
            "manual_stop",
        ),
    ];

    for (exit, preclassified, expected_classification, name) in exits {
        let mut pipeline = create_test_pipeline();
        let spec = create_test_spec();
        let tree = SupervisorTree::build(&spec).expect("build tree");

        let child_id = ChildId::new(format!("test-child-{name}"));
        let supervisor_path = SupervisorPath::root();

        let mut ctx = PipelineContext::new(
            child_id.clone(),
            supervisor_path.clone(),
            1,
            format!("correlation-{name}"),
        );
        ctx.exit_classification = preclassified;

        let result_ctx = pipeline.execute_pipeline(ctx, exit, &spec, &tree);

        assert!(
            result_ctx.exit_classification.is_some(),
            "Exit kind '{name}' should go through classify_exit"
        );

        let actual = result_ctx.exit_classification.unwrap();
        assert_eq!(
            actual.as_str(),
            expected_classification.as_str(),
            "Exit kind '{name}' should be classified correctly"
        );

        let stages = result_ctx
            .stage_diagnostics
            .iter()
            .map(|diagnostic| diagnostic.stage)
            .collect::<Vec<_>>();
        assert_eq!(
            stages,
            vec![
                PipelineStage::ClassifyExit,
                PipelineStage::RecordFailureWindow,
                PipelineStage::EvaluateBudget,
                PipelineStage::DecideAction,
                PipelineStage::EmitTypedEvent,
                PipelineStage::ExecuteAction,
            ],
            "Exit kind '{name}' should produce six ordered diagnostics"
        );
    }
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

#[test]
fn test_pipeline_stage_diagnostics_export_through_observability() {
    let mut observability = ObservabilityPipeline::new(8, 2);
    let diagnostics = vec![
        PipelineStageDiagnostic::new(1, "shared-correlation", PipelineStage::ClassifyExit, 1),
        PipelineStageDiagnostic::new(
            1,
            "shared-correlation",
            PipelineStage::RecordFailureWindow,
            2,
        ),
        PipelineStageDiagnostic::new(1, "shared-correlation", PipelineStage::EvaluateBudget, 3),
        PipelineStageDiagnostic::new(1, "shared-correlation", PipelineStage::DecideAction, 4),
        PipelineStageDiagnostic::new(1, "shared-correlation", PipelineStage::EmitTypedEvent, 5),
        PipelineStageDiagnostic::new(1, "shared-correlation", PipelineStage::ExecuteAction, 6),
    ];

    observability.record_pipeline_stage_diagnostics(&diagnostics);

    let stages = observability
        .test_recorder
        .pipeline_stage_diagnostics
        .iter()
        .map(|diagnostic| diagnostic.stage)
        .collect::<Vec<_>>();

    assert_eq!(
        stages,
        vec![
            PipelineStage::ClassifyExit,
            PipelineStage::RecordFailureWindow,
            PipelineStage::EvaluateBudget,
            PipelineStage::DecideAction,
            PipelineStage::EmitTypedEvent,
            PipelineStage::ExecuteAction,
        ]
    );
}
