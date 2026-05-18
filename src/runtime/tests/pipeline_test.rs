//! Supervision pipeline unit tests.
//!
//! These tests verify exit classification, budget evaluation,
//! and action decision stages of the six-stage supervision pipeline.

use crate::event::payload::ProtectionAction;
use crate::id::types::{ChildId, SupervisorPath};
use crate::policy::budget::RestartBudgetConfig;
use crate::policy::decision::{PolicyFailureKind, TaskExit};
use crate::policy::failure_window::{FailureWindow, FailureWindowConfig};
use crate::policy::meltdown::{MeltdownOutcome, MeltdownPolicy, MeltdownTracker};
use crate::runtime::pipeline::{
    BudgetEvaluation, ExitClassification, PipelineContext, SupervisionPipeline,
};
use std::time::Duration;

/// Creates a test supervision pipeline with default meltdown policy configuration.
fn test_pipeline() -> SupervisionPipeline {
    let meltdown_policy = MeltdownPolicy::new(
        3,
        Duration::from_secs(10),
        5,
        Duration::from_secs(30),
        10,
        Duration::from_secs(60),
        Duration::from_secs(120),
    );
    let meltdown_tracker = MeltdownTracker::new(meltdown_policy);

    let failure_config = FailureWindowConfig::time_sliding(60, 5);
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

/// Tests that successful task exits are classified correctly by the pipeline.
#[test]
fn test_exit_classification_success() {
    let pipeline = test_pipeline();
    let child_id = ChildId::new("test".to_string());
    let path = SupervisorPath::root();
    let ctx = PipelineContext::new(child_id, path, 1, "test-correlation");

    let exit = TaskExit::Succeeded;
    let ctx = pipeline.stage_classify_exit(ctx, &exit, 1);

    assert_eq!(ctx.exit_classification, Some(ExitClassification::Success));
    assert!(!ctx.exit_classification.unwrap().should_restart());
}

/// Tests that failed task exits are classified correctly with restart decision.
#[test]
fn test_exit_classification_failure() {
    let pipeline = test_pipeline();
    let child_id = ChildId::new("test".to_string());
    let path = SupervisorPath::root();
    let ctx = PipelineContext::new(child_id, path, 1, "test-correlation");

    let exit = TaskExit::Failed {
        kind: PolicyFailureKind::Recoverable,
    };
    let ctx = pipeline.stage_classify_exit(ctx, &exit, 1);

    assert!(matches!(
        ctx.exit_classification,
        Some(ExitClassification::NonZeroExit { .. })
    ));
    assert!(ctx.exit_classification.unwrap().should_restart());
}

/// Tests that cancel exits have priority over budget evaluation in action decision.
#[test]
fn test_cancel_has_priority() {
    let pipeline = test_pipeline();
    let child_id = ChildId::new("test".to_string());
    let path = SupervisorPath::root();
    let mut ctx = PipelineContext::new(child_id, path, 1, "test-correlation");

    // Set up budget evaluation showing restarts allowed
    ctx.budget_evaluation = Some(BudgetEvaluation {
        remaining_restarts: Some(3),
        limit_exhausted: false,
        escalation_policy: None,
        meltdown_outcome: MeltdownOutcome::Continue,
        budget_verdict: None,
    });

    // Classify as external cancel
    ctx.exit_classification = Some(ExitClassification::ExternalCancel);

    // Decide action should prioritize cancel over budget
    let ctx = pipeline.stage_decide_action(ctx, 1);

    assert_eq!(
        ctx.action_decision.unwrap().action,
        ProtectionAction::SupervisedStop
    );
}
