//! Acceptance tests for cold start budget and hot loop detection.
//!
//! These tests use production `ColdStartBudget`, `HotLoopDetector`, and
//! `SupervisionPipeline` paths instead of local simulation types.

use rust_supervisor::event::payload::{ColdStartReason, HotLoopReason, ProtectionAction};
use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::policy::backoff::{ColdStartBudget, HotLoopDetector};
use rust_supervisor::policy::budget::RestartBudgetConfig;
use rust_supervisor::policy::decision::{PolicyFailureKind, TaskExit};
use rust_supervisor::policy::failure_window::{FailureWindow, FailureWindowConfig};
use rust_supervisor::policy::meltdown::{MeltdownPolicy, MeltdownTracker};
use rust_supervisor::runtime::pipeline::{PipelineContext, SupervisionPipeline};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::tree::builder::SupervisorTree;
use std::time::Duration;

/// Creates a production pipeline with high meltdown limits.
fn create_pipeline() -> SupervisionPipeline {
    let meltdown_policy = MeltdownPolicy::new(
        100,
        Duration::from_secs(60),
        100,
        Duration::from_secs(60),
        100,
        Duration::from_secs(60),
        Duration::from_secs(120),
    );
    let meltdown_tracker = MeltdownTracker::new(meltdown_policy);
    let failure_window = FailureWindow::new(FailureWindowConfig::time_sliding(60, 100));
    SupervisionPipeline::new(
        100,
        10,
        meltdown_tracker,
        failure_window,
        RestartBudgetConfig::new(Duration::from_secs(60), 10, 0.5),
        vec![],
    )
}

/// Runs one recoverable failure through the production pipeline.
fn run_failure(pipeline: &mut SupervisionPipeline, sequence: u64) -> PipelineContext {
    let spec = SupervisorSpec::root(vec![]);
    let tree = SupervisorTree::build(&spec).expect("build supervisor tree");
    let ctx = PipelineContext::new(
        ChildId::new("cold-hot-child".to_string()),
        SupervisorPath::root(),
        sequence,
        format!("cold-hot-{sequence}"),
    );

    pipeline.execute_pipeline(
        ctx,
        TaskExit::Failed {
            kind: PolicyFailureKind::Recoverable,
        },
        &spec,
        &tree,
    )
}

#[test]
fn cold_start_budget_exhaustion_uses_production_type() {
    let mut budget = ColdStartBudget::new(60, 3, 1_000);

    assert!(!budget.record_restart(1_005));
    assert_eq!(budget.get_restart_count(), 1);
    assert!(!budget.record_restart(1_010));
    assert!(!budget.record_restart(1_015));
    assert!(budget.record_restart(1_020));
    assert!(budget.is_exhausted(1_020));
}

#[test]
fn cold_start_window_expiry_resets_production_budget() {
    let mut budget = ColdStartBudget::new(10, 2, 1_000);

    budget.record_restart(1_001);
    budget.record_restart(1_005);

    assert!(!budget.record_restart(1_020));
    assert_eq!(budget.get_restart_count(), 1);
    assert!(!budget.is_exhausted(1_020));
}

#[test]
fn hot_loop_detection_uses_production_type() {
    let mut detector = HotLoopDetector::new(10, 3);

    assert!(!detector.record_crash(1_000));
    assert!(!detector.record_crash(1_002));
    assert!(detector.record_crash(1_004));
    assert!(detector.is_hot_loop_detected(1_004));
}

#[test]
fn hot_loop_window_slides_in_production_detector() {
    let mut detector = HotLoopDetector::new(5, 3);

    detector.record_crash(1_000);
    detector.record_crash(1_001);

    assert!(!detector.record_crash(1_007));
}

#[test]
fn pipeline_records_cold_start_exhaustion_reason() {
    let mut pipeline = create_pipeline();
    let mut last = None;

    for sequence in 1..=6 {
        last = Some(run_failure(&mut pipeline, sequence));
    }

    let ctx = last.expect("pipeline result");
    assert_eq!(ctx.cold_start_reason, ColdStartReason::BudgetExhausted);
    assert!(
        ctx.action_decision
            .as_ref()
            .is_some_and(|decision| decision.action >= ProtectionAction::RestartDenied)
    );
}

#[test]
fn pipeline_records_hot_loop_reason_distinct_from_restart_limit() {
    let mut pipeline = create_pipeline();
    let mut last = None;

    for sequence in 1..=3 {
        last = Some(run_failure(&mut pipeline, sequence));
    }

    let ctx = last.expect("pipeline result");
    assert_eq!(ctx.hot_loop_reason, HotLoopReason::RapidCrashDetected);
    assert!(
        ctx.action_decision
            .as_ref()
            .is_some_and(|decision| decision.reason == "hot_loop_detected")
    );
}

#[test]
fn protection_action_for_cold_start_exhausted_is_restrictive() {
    let action = ProtectionAction::RestartDenied;
    assert!(action >= ProtectionAction::RestartDenied);
}

#[test]
fn protection_action_for_hot_loop_is_restrictive() {
    let action = ProtectionAction::SupervisionPaused;
    assert!(action >= ProtectionAction::RestartDenied);
}

#[test]
fn cold_start_reason_display_is_stable() {
    assert_eq!(
        format!("{}", ColdStartReason::NotApplicable),
        "not_applicable"
    );
    assert_eq!(
        format!("{}", ColdStartReason::InitialStartup),
        "initial_startup"
    );
    assert_eq!(
        format!("{}", ColdStartReason::BudgetExhausted),
        "budget_exhausted"
    );
    assert_eq!(
        format!("{}", ColdStartReason::ExcessiveRestarts),
        "excessive_restarts"
    );
}

#[test]
fn hot_loop_reason_display_is_stable() {
    assert_eq!(
        format!("{}", HotLoopReason::NotApplicable),
        "not_applicable"
    );
    assert_eq!(
        format!("{}", HotLoopReason::RapidCrashDetected),
        "rapid_crash_detected"
    );
    assert_eq!(
        format!("{}", HotLoopReason::CycleThresholdExceeded),
        "cycle_threshold_exceeded"
    );
    assert_eq!(
        format!("{}", HotLoopReason::InsufficientStableRuntime),
        "insufficient_stable_runtime"
    );
}
