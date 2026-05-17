//! Acceptance tests for work role default policy behavior.
//!
//! These tests verify that role defaults affect pipeline decisions, sidecar
//! bindings are validated with sibling context, and emitted events carry
//! effective policy attribution.

use rust_supervisor::event::payload::{ProtectionAction, SupervisorEvent, What, Where};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::policy::decision::{PolicyFailureKind, TaskExit};
use rust_supervisor::policy::failure_window::{FailureWindow, FailureWindowConfig};
use rust_supervisor::policy::meltdown::{MeltdownPolicy, MeltdownTracker};
use rust_supervisor::policy::role_defaults::{
    EffectivePolicy, OnFailureAction, OnSuccessAction, PolicySource, RoleDefaultPolicy,
    SidecarConfig, WorkRole, semantic_conflicts_for_child,
};
use rust_supervisor::runtime::pipeline::{PipelineContext, SupervisionPipeline};
use rust_supervisor::spec::child::{ChildSpec, RestartPolicy, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::tree::builder::SupervisorTree;
use rust_supervisor::tree::order::restart_execution_plan;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Creates a test child with the requested work role.
fn child_with_role(id: &str, role: WorkRole) -> ChildSpec {
    let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    let mut child = ChildSpec::worker(
        ChildId::new(id),
        id,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    );
    child.work_role = Some(role);
    child
}

/// Creates a pipeline with high protection limits for role behavior tests.
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
    SupervisionPipeline::new(100, 10, meltdown_tracker, failure_window)
}

/// Runs a successful exit through the production pipeline with a role policy.
fn run_success_with_role(role: WorkRole) -> PipelineContext {
    let mut pipeline = create_pipeline();
    let child = child_with_role("role-child", role);
    let spec = SupervisorSpec::root(vec![child.clone()]);
    let tree = SupervisorTree::build(&spec).expect("build supervisor tree");
    let mut ctx = PipelineContext::new(child.id.clone(), SupervisorPath::root(), 1, "role-success");
    ctx.effective_policy = Some(EffectivePolicy::for_child(&child));

    pipeline.execute_pipeline(ctx, TaskExit::Succeeded, &spec, &tree)
}

#[test]
fn job_success_exit_does_not_request_restart() {
    let ctx = run_success_with_role(WorkRole::Job);
    let decision = ctx.action_decision.expect("action decision");

    assert_eq!(decision.action, ProtectionAction::SupervisedStop);
    assert_eq!(decision.reason, "role_success_stop");
}

#[test]
fn service_success_exit_allows_restart() {
    let ctx = run_success_with_role(WorkRole::Service);
    let decision = ctx.action_decision.expect("action decision");

    assert_eq!(decision.action, ProtectionAction::RestartAllowed);
    assert_eq!(decision.reason, "role_success_restart");
}

#[test]
fn worker_failure_default_uses_bounded_retry() {
    let pack = RoleDefaultPolicy::for_role(WorkRole::Worker);

    assert_eq!(pack.on_failure_exit, OnFailureAction::RestartWithBackoff);
    assert!(pack.default_restart_limit.is_some());
}

#[test]
fn worker_default_restart_limit_feeds_budget_evaluation() {
    let mut pipeline = create_pipeline();
    let child = child_with_role("worker-budget", WorkRole::Worker);
    let spec = SupervisorSpec::root(vec![child.clone()]);
    let tree = SupervisorTree::build(&spec).expect("build supervisor tree");
    let mut final_ctx = None;

    for sequence in 1..=4 {
        let mut ctx = PipelineContext::new(
            child.id.clone(),
            SupervisorPath::root(),
            sequence,
            format!("worker-budget-{sequence}"),
        );
        ctx.effective_policy = Some(EffectivePolicy::for_child(&child));
        final_ctx = Some(pipeline.execute_pipeline(
            ctx,
            TaskExit::Failed {
                kind: PolicyFailureKind::Recoverable,
            },
            &spec,
            &tree,
        ));
    }

    let budget = final_ctx
        .expect("final pipeline context")
        .budget_evaluation
        .expect("budget evaluation");

    assert_eq!(budget.remaining_restarts, Some(0));
    assert!(budget.limit_exhausted);
    assert_eq!(
        budget.escalation_policy,
        Some("EscalateToParent".to_string())
    );
}

#[test]
fn sidecar_failure_default_restarts_only_sidecar_scope() {
    let primary = child_with_role("primary-service", WorkRole::Service);
    let mut sidecar = child_with_role("metrics-sidecar", WorkRole::Sidecar);
    sidecar.sidecar_config = Some(SidecarConfig::new(primary.id.clone(), false));
    let spec = SupervisorSpec::root(vec![primary, sidecar.clone()]);
    let tree = SupervisorTree::build(&spec).expect("build supervisor tree");

    let plan = restart_execution_plan(&tree, &spec, &sidecar.id);

    assert_eq!(plan.scope, vec![sidecar.id]);
    assert_eq!(
        RoleDefaultPolicy::for_role(WorkRole::Sidecar).on_failure_exit,
        OnFailureAction::RestartWithBackoff
    );
}

#[test]
fn supervisor_role_default_uses_outer_unit_restart_budget() {
    let pack = RoleDefaultPolicy::for_role(WorkRole::Supervisor);

    assert_eq!(pack.on_success_exit, OnSuccessAction::Restart);
    assert!(pack.default_restart_limit.is_some());
}

#[test]
fn missing_role_uses_worker_fallback_attribution() {
    let policy = EffectivePolicy::merge(None, Vec::new());

    assert_eq!(policy.work_role, WorkRole::Worker);
    assert_eq!(policy.source, PolicySource::FallbackDefault);
    assert!(policy.used_fallback);
}

#[test]
fn child_spec_deserialization_defaults_role_fields() {
    let child = child_with_role("serde-child", WorkRole::Worker);
    let mut value = serde_json::to_value(child).expect("serialize child spec");
    let object = value.as_object_mut().expect("child spec object");
    object.remove("work_role");
    object.remove("sidecar_config");

    let decoded: ChildSpec = serde_json::from_value(value).expect("deserialize child spec");

    assert_eq!(decoded.work_role, None);
    assert_eq!(decoded.sidecar_config, None);
}

#[test]
fn unknown_work_role_is_rejected_by_deserialization() {
    let child = child_with_role("unknown-role-child", WorkRole::Worker);
    let mut value = serde_json::to_value(child).expect("serialize child spec");
    let object = value.as_object_mut().expect("child spec object");
    object.insert(
        "work_role".to_string(),
        serde_json::Value::String("unknown_role".to_string()),
    );

    let error = serde_json::from_value::<ChildSpec>(value)
        .expect_err("unknown work_role should be rejected");

    assert!(error.to_string().contains("unknown variant"));
}

#[test]
fn sidecar_missing_config_is_rejected() {
    let sidecar = child_with_role("metrics-sidecar", WorkRole::Sidecar);
    let spec = SupervisorSpec::root(vec![sidecar]);

    let error = spec
        .validate()
        .expect_err("sidecar config should be required");

    assert!(error.to_string().contains("sidecar_config"));
}

#[test]
fn sidecar_unknown_primary_is_rejected() {
    let mut sidecar = child_with_role("metrics-sidecar", WorkRole::Sidecar);
    sidecar.sidecar_config = Some(SidecarConfig::new(ChildId::new("missing-primary"), true));
    let spec = SupervisorSpec::root(vec![sidecar]);

    let error = spec
        .validate()
        .expect_err("primary child should be required");

    assert!(error.to_string().contains("primary_child_id"));
}

#[test]
fn sidecar_chain_is_rejected() {
    let mut first = child_with_role("first-sidecar", WorkRole::Sidecar);
    first.sidecar_config = Some(SidecarConfig::new(ChildId::new("primary"), true));
    let mut second = child_with_role("second-sidecar", WorkRole::Sidecar);
    second.sidecar_config = Some(SidecarConfig::new(ChildId::new("first-sidecar"), true));
    let primary = child_with_role("primary", WorkRole::Service);
    let spec = SupervisorSpec::root(vec![primary, first, second]);

    let error = spec
        .validate()
        .expect_err("sidecar chains should be rejected");

    assert!(error.to_string().contains("must not use another sidecar"));
}

#[test]
fn job_permanent_restart_conflict_is_reported() {
    let mut child = child_with_role("job-child", WorkRole::Job);
    child.restart_policy = RestartPolicy::Permanent;

    let conflicts = semantic_conflicts_for_child(&child);

    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].conflicting_field, "restart_policy");
}

#[test]
fn emitted_pipeline_event_carries_policy_attribution() {
    let mut pipeline = create_pipeline();
    let child = child_with_role("service-child", WorkRole::Service);
    let spec = SupervisorSpec::root(vec![child.clone()]);
    let tree = SupervisorTree::build(&spec).expect("build supervisor tree");
    let mut ctx = PipelineContext::new(child.id.clone(), SupervisorPath::root(), 1, "role-event");
    ctx.effective_policy = Some(EffectivePolicy::for_child(&child));

    pipeline.execute_pipeline(ctx, TaskExit::Succeeded, &spec, &tree);

    let event = pipeline
        .observability
        .test_recorder
        .events
        .last()
        .expect("pipeline event");
    assert_eq!(event.work_role, Some(WorkRole::Service));
    assert_eq!(
        event.effective_policy_source,
        Some(PolicySource::RoleDefault)
    );
    assert!(!event.used_fallback_default);
}

#[test]
fn supervisor_event_fields_exist_for_policy_source() {
    let mut event = SupervisorEvent::new(
        When::new(EventTime::deterministic(
            1,
            1,
            0,
            Generation::initial(),
            ChildStartCount::first(),
        )),
        Where::new(SupervisorPath::root()),
        What::ChildRunning { transition: None },
        EventSequence::new(1),
        CorrelationId::from_uuid(Uuid::nil()),
        1,
    );

    event.work_role = Some(WorkRole::Worker);
    event.used_fallback_default = true;
    event.effective_policy_source = Some(PolicySource::FallbackDefault);

    assert_eq!(event.work_role, Some(WorkRole::Worker));
    assert_eq!(
        event.effective_policy_source,
        Some(PolicySource::FallbackDefault)
    );
    assert!(event.used_fallback_default);
}
