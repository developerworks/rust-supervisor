//! ChildSpec builder tests.
//!
//! These tests verify builder defaults, entry points, and validation behavior.

use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::task_role_defaults::{SidecarConfig, TaskRole};
use rust_supervisor::readiness::signal::ReadinessPolicy;
use rust_supervisor::spec::child::{ChildSpec, Criticality, Isolation, RestartPolicy, TaskKind};
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;
use std::time::Duration;

/// Returns a no-op worker factory for builder tests.
fn test_factory() -> Arc<dyn rust_supervisor::task::factory::TaskFactory> {
    Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded }))
}

/// Compares builder output with `ChildSpec::worker` for every field except factory.
fn assert_worker_fields_match(builder_spec: &ChildSpec, worker_spec: &ChildSpec) {
    assert_eq!(builder_spec.id, worker_spec.id);
    assert_eq!(builder_spec.name, worker_spec.name);
    assert_eq!(builder_spec.kind, worker_spec.kind);
    assert_eq!(builder_spec.isolation, worker_spec.isolation);
    assert!(builder_spec.factory.is_some());
    assert!(worker_spec.factory.is_some());
    assert_eq!(builder_spec.factory_key, worker_spec.factory_key);
    assert_eq!(builder_spec.restart_policy, worker_spec.restart_policy);
    assert_eq!(builder_spec.shutdown_policy, worker_spec.shutdown_policy);
    assert_eq!(builder_spec.health_policy, worker_spec.health_policy);
    assert_eq!(builder_spec.readiness_policy, worker_spec.readiness_policy);
    assert_eq!(builder_spec.backoff_policy, worker_spec.backoff_policy);
    assert_eq!(builder_spec.dependencies, worker_spec.dependencies);
    assert_eq!(builder_spec.tags, worker_spec.tags);
    assert_eq!(builder_spec.criticality, worker_spec.criticality);
    assert_eq!(builder_spec.task_role, worker_spec.task_role);
    assert_eq!(builder_spec.sidecar_config, worker_spec.sidecar_config);
    assert_eq!(builder_spec.severity, worker_spec.severity);
    assert_eq!(builder_spec.group, worker_spec.group);
    assert_eq!(builder_spec.health_check, worker_spec.health_check);
    assert_eq!(
        builder_spec.command_permissions,
        worker_spec.command_permissions
    );
    assert_eq!(builder_spec.environment, worker_spec.environment);
    assert_eq!(builder_spec.secrets, worker_spec.secrets);
    assert_eq!(builder_spec.cleanup_paths, worker_spec.cleanup_paths);
}

/// Verifies worker builder defaults match `ChildSpec::worker`.
#[test]
fn worker_builder_matches_child_spec_worker_defaults() -> Result<(), SupervisorError> {
    let factory = test_factory();
    let id = ChildId::new("worker");
    let builder_spec =
        ChildSpecBuilder::worker(id.clone(), "worker", TaskKind::AsyncWorker, factory.clone())
            .build()?;
    let worker_spec = ChildSpec::worker(id, "worker", TaskKind::AsyncWorker, factory)?;

    assert_worker_fields_match(&builder_spec, &worker_spec);
    assert!(builder_spec.validate().is_ok());
    Ok(())
}

/// Verifies worker builder preserves role-specific policy defaults.
#[test]
fn worker_builder_applies_worker_policy_defaults() -> Result<(), SupervisorError> {
    let spec = ChildSpecBuilder::worker(
        ChildId::new("worker"),
        "worker",
        TaskKind::AsyncWorker,
        test_factory(),
    )
    .build()?;

    assert_eq!(spec.isolation, Isolation::AsyncWorker);
    assert_eq!(spec.restart_policy, RestartPolicy::Transient);
    assert_eq!(
        spec.shutdown_policy.graceful_timeout,
        Duration::from_secs(5)
    );
    assert_eq!(spec.shutdown_policy.abort_wait, Duration::from_secs(1));
    assert_eq!(
        spec.health_policy.heartbeat_interval,
        Duration::from_secs(1)
    );
    assert_eq!(spec.health_policy.stale_after, Duration::from_secs(3));
    assert_eq!(spec.readiness_policy, ReadinessPolicy::Immediate);
    assert_eq!(spec.backoff_policy.initial_delay, Duration::from_millis(10));
    assert_eq!(spec.backoff_policy.max_delay, Duration::from_secs(1));
    assert_eq!(spec.backoff_policy.jitter_ratio, 0.0);
    Ok(())
}

/// Verifies supervisor builder produces a valid nested supervisor child.
#[test]
fn supervisor_builder_produces_valid_supervisor_child() -> Result<(), SupervisorError> {
    let spec = ChildSpecBuilder::supervisor(ChildId::new("nested"), "Nested Supervisor").build()?;

    assert_eq!(spec.kind, TaskKind::Supervisor);
    assert!(spec.factory.is_none());
    assert_eq!(spec.task_role, Some(TaskRole::Supervisor));
    assert_eq!(spec.criticality, Criticality::Critical);
    assert!(spec.validate().is_ok());
    Ok(())
}

/// Verifies supervisor builder preserves baseline policy defaults.
#[test]
fn supervisor_builder_applies_baseline_policy_defaults() -> Result<(), SupervisorError> {
    let spec = ChildSpecBuilder::supervisor(ChildId::new("nested"), "Nested Supervisor").build()?;

    assert_eq!(spec.isolation, Isolation::AsyncWorker);
    assert_eq!(spec.restart_policy, RestartPolicy::Permanent);
    assert_eq!(
        spec.shutdown_policy.graceful_timeout,
        Duration::from_secs(5)
    );
    assert_eq!(spec.shutdown_policy.abort_wait, Duration::from_secs(1));
    assert_eq!(
        spec.health_policy.heartbeat_interval,
        Duration::from_secs(10)
    );
    assert_eq!(spec.health_policy.stale_after, Duration::from_secs(5));
    assert_eq!(spec.readiness_policy, ReadinessPolicy::Immediate);
    assert_eq!(spec.backoff_policy.initial_delay, Duration::from_millis(10));
    assert_eq!(spec.backoff_policy.max_delay, Duration::from_secs(1));
    assert_eq!(spec.backoff_policy.jitter_ratio, 0.0);
    Ok(())
}

/// Verifies service builder applies service role defaults.
#[test]
fn service_builder_sets_service_role() -> Result<(), SupervisorError> {
    let spec = ChildSpecBuilder::service(
        ChildId::new("api-service"),
        "API Service",
        TaskKind::AsyncWorker,
        test_factory(),
    )
    .build()?;

    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Service));
    assert_eq!(spec.criticality, Criticality::Critical);
    assert!(spec.sidecar_config.is_none());
    assert!(spec.validate().is_ok());
    Ok(())
}

/// Verifies job builder applies job role defaults.
#[test]
fn job_builder_sets_job_role_and_optional_criticality() -> Result<(), SupervisorError> {
    let spec = ChildSpecBuilder::job(
        ChildId::new("daily-report"),
        "Daily Report",
        TaskKind::AsyncWorker,
        test_factory(),
    )
    .build()?;

    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Job));
    assert_eq!(spec.criticality, Criticality::Optional);
    assert!(spec.sidecar_config.is_none());
    assert!(spec.validate().is_ok());
    Ok(())
}

/// Verifies sidecar builder applies sidecar binding and dependency.
#[test]
fn sidecar_builder_sets_sidecar_role_binding_and_dependency() -> Result<(), SupervisorError> {
    let primary_id = ChildId::new("api-service");
    let spec = ChildSpecBuilder::sidecar(
        ChildId::new("metrics-sidecar"),
        "Metrics Sidecar",
        TaskKind::AsyncWorker,
        test_factory(),
        SidecarConfig::new(primary_id.clone(), true),
    )
    .build()?;

    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Sidecar));
    assert_eq!(spec.dependencies, vec![primary_id]);
    assert_eq!(spec.criticality, Criticality::Critical);
    assert!(spec.sidecar_config.is_some());
    assert!(spec.validate().is_ok());
    Ok(())
}

/// Verifies fluent setters update the constructed child specification.
#[test]
fn builder_setters_apply_expected_fields() -> Result<(), SupervisorError> {
    let primary_id = ChildId::new("primary");
    let spec = ChildSpecBuilder::worker(
        ChildId::new("sidecar"),
        "sidecar",
        TaskKind::AsyncWorker,
        test_factory(),
    )
    .task_role(TaskRole::Sidecar)
    .sidecar_config(SidecarConfig::new(primary_id.clone(), true))
    .dependency(primary_id)
    .restart_policy(RestartPolicy::Permanent)
    .tag("metrics")
    .build()?;

    assert_eq!(spec.task_role, Some(TaskRole::Sidecar));
    assert_eq!(spec.dependencies, vec![ChildId::new("primary")]);
    assert_eq!(spec.restart_policy, RestartPolicy::Permanent);
    assert_eq!(spec.tags, vec!["metrics".to_owned()]);
    assert!(spec.sidecar_config.is_some());
    assert!(spec.validate().is_ok());
    Ok(())
}

/// Verifies `build` rejects invalid sidecar combinations.
#[test]
fn build_rejects_invalid_sidecar_combination() -> Result<(), SupervisorError> {
    let error = ChildSpecBuilder::worker(
        ChildId::new("sidecar"),
        "sidecar",
        TaskKind::AsyncWorker,
        test_factory(),
    )
    .task_role(TaskRole::Sidecar)
    .build()
    .expect_err("sidecar without sidecar_config should fail validation");

    assert!(error.to_string().contains("sidecar_config"));
    Ok(())
}

/// Verifies minimal builder can be completed into a valid worker child.
#[test]
fn new_builder_can_build_valid_worker_with_factory() -> Result<(), SupervisorError> {
    let spec = ChildSpecBuilder::new(ChildId::new("custom"), "custom")
        .kind(TaskKind::AsyncWorker)
        .factory(test_factory())
        .readiness_policy(ReadinessPolicy::Immediate)
        .build()?;

    assert_eq!(spec.name, "custom");
    assert!(spec.factory.is_some());
    Ok(())
}
