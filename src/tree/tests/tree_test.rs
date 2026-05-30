//! Supervisor tree tests.
//!
//! These tests verify tree construction and ordering utilities.

use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::id::types::ChildId;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::{
    ChildStrategyOverride, EscalationPolicy, GroupConfig, GroupStrategy, RestartLimit,
    SupervisionStrategy, SupervisorSpec,
};
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::tree::builder::SupervisorTree;
use rust_supervisor::tree::order::{
    restart_execution_plan, restart_scope, shutdown_order, startup_order,
};
use std::sync::Arc;
use std::time::Duration;

/// Verifies declaration order and reverse shutdown order.
#[test]
fn tree_preserves_declaration_and_reverse_shutdown_order() -> Result<(), SupervisorError> {
    let first = child("first")?;
    let second = child("second")?;
    let spec = SupervisorSpec::root(vec![first.clone(), second.clone()]);
    let tree = SupervisorTree::build(&spec).unwrap();

    assert_eq!(startup_order(&tree)[0].child.id, first.id);
    assert_eq!(shutdown_order(&tree)[0].child.id, second.id);
    Ok(())
}

/// Verifies that RestForOne selects the failed child and following children.
#[test]
fn rest_for_one_selects_failed_child_and_following_children() -> Result<(), SupervisorError> {
    let first = child("first")?;
    let second = child("second")?;
    let spec = SupervisorSpec::root(vec![first, second.clone()]);
    let tree = SupervisorTree::build(&spec).unwrap();

    let scope = restart_scope(&tree, SupervisionStrategy::RestForOne, &second.id);

    assert_eq!(scope, vec![second.id]);
    Ok(())
}

/// Verifies that group strategies limit restart plans to group members.
#[test]
fn group_strategy_limits_restart_plan_to_group_members() -> Result<(), SupervisorError> {
    let first = child("first")?;
    let mut second = child("second")?;
    let mut third = child("third")?;
    let mut fourth = child("fourth")?;
    second.group = Some("pipeline".to_owned());
    third.group = Some("pipeline".to_owned());
    fourth.group = Some("other".to_owned());
    let mut spec = SupervisorSpec::root(vec![first, second.clone(), third.clone(), fourth]);
    spec.group_configs = vec![
        GroupConfig::new(
            "pipeline",
            vec![second.id.clone(), third.id.clone()],
            None,
        ),
        GroupConfig::new("other", vec![ChildId::new("fourth")], None),
    ];
    spec.group_strategies = vec![GroupStrategy::new(
        "pipeline",
        SupervisionStrategy::RestForOne,
    )];
    let tree = SupervisorTree::build(&spec).unwrap();

    let plan = restart_execution_plan(&tree, &spec, &second.id);

    assert_eq!(plan.group, Some("pipeline".to_owned()));
    assert_eq!(plan.strategy, SupervisionStrategy::RestForOne);
    assert_eq!(plan.scope, vec![second.id, third.id]);
    Ok(())
}

/// Verifies that a child override takes precedence over group strategy.
#[test]
fn child_override_wins_over_group_strategy_and_selects_limit() -> Result<(), SupervisorError> {
    let mut first = child("first")?;
    let second = child("second")?;
    first.group = Some("pipeline".to_owned());
    let limit = RestartLimit::new(3, Duration::from_secs(10));
    let mut override_strategy =
        ChildStrategyOverride::new(first.id.clone(), SupervisionStrategy::OneForAll);
    override_strategy.restart_limit = Some(limit);
    override_strategy.escalation_policy = Some(EscalationPolicy::ShutdownTree);
    let mut spec = SupervisorSpec::root(vec![first.clone(), second.clone()]);
    spec.group_configs = vec![GroupConfig::new(
        "pipeline",
        vec![first.id.clone()],
        None,
    )];
    spec.group_strategies = vec![GroupStrategy::new(
        "pipeline",
        SupervisionStrategy::OneForOne,
    )];
    spec.child_strategy_overrides = vec![override_strategy];
    let tree = SupervisorTree::build(&spec).unwrap();

    let plan = restart_execution_plan(&tree, &spec, &first.id);

    assert_eq!(plan.group, None);
    assert_eq!(plan.strategy, SupervisionStrategy::OneForAll);
    assert_eq!(plan.scope, vec![first.id, second.id]);
    assert_eq!(plan.restart_limit, Some(limit));
    assert_eq!(plan.escalation_policy, Some(EscalationPolicy::ShutdownTree));
    Ok(())
}

/// Verifies that unused strategy groups are rejected.
#[test]
fn validation_rejects_unused_strategy_group() -> Result<(), SupervisorError> {
    let child = child("worker")?;
    let mut spec = SupervisorSpec::root(vec![child]);
    spec.group_strategies = vec![GroupStrategy::new(
        "missing",
        SupervisionStrategy::OneForOne,
    )];

    let error = spec.validate().unwrap_err();

    assert!(error.to_string().contains("unused group"));
    Ok(())
}

/// Verifies that overrides targeting unknown children are rejected.
#[test]
fn validation_rejects_unknown_child_override() -> Result<(), SupervisorError> {
    let child = child("worker")?;
    let mut spec = SupervisorSpec::root(vec![child]);
    spec.child_strategy_overrides = vec![ChildStrategyOverride::new(
        ChildId::new("missing"),
        SupervisionStrategy::OneForOne,
    )];

    let error = spec.validate().unwrap_err();

    assert!(error.to_string().contains("unknown child"));
    Ok(())
}

/// Verifies that invalid restart limits are rejected.
#[test]
fn validation_rejects_invalid_restart_limit() -> Result<(), SupervisorError> {
    let mut spec = SupervisorSpec::root(vec![child("worker")?]);
    spec.restart_limit = Some(RestartLimit::new(0, Duration::from_secs(1)));

    let error = spec.validate().unwrap_err();

    assert!(error.to_string().contains("max_restarts"));
    Ok(())
}

/// Builds one worker child specification for tree tests.
fn child(id: &str) -> Result<ChildSpec, SupervisorError> {
    let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    ChildSpec::worker(
        ChildId::new(id),
        id,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
}
