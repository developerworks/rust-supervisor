//! Role template contract tests.
//!
//! These tests verify the lightweight template entries that wrap explicit role
//! contracts without introducing new lifecycle names.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::task_role_defaults::{SidecarConfig, TaskRole};
use rust_supervisor::role::context::job::JobContext;
use rust_supervisor::role::context::service::ServiceContext;
use rust_supervisor::role::context::sidecar::SidecarContext;
use rust_supervisor::role::context::supervisor::SupervisorContext;
use rust_supervisor::role::context::worker::WorkerContext;
use rust_supervisor::role::result::job::JobResult;
use rust_supervisor::role::result::service::ServiceResult;
use rust_supervisor::role::result::sidecar::SidecarResult;
use rust_supervisor::role::result::supervisor::SupervisorResult;
use rust_supervisor::role::result::worker::WorkerResult;
use rust_supervisor::role::templates::job::JobTemplate;
use rust_supervisor::role::templates::service::ServiceTemplate;
use rust_supervisor::role::templates::sidecar::SidecarTemplate;
use rust_supervisor::role::templates::supervisor::SupervisorTemplate;
use rust_supervisor::role::templates::worker::WorkerTemplate;
use rust_supervisor::role::traits::job::JobRole;
use rust_supervisor::role::traits::service::ServiceRole;
use rust_supervisor::role::traits::sidecar::SidecarRole;
use rust_supervisor::role::traits::supervisor::SupervisorRole;
use rust_supervisor::role::traits::worker::WorkerRole;
use rust_supervisor::spec::child::{Criticality, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;

/// Service fixture for template tests.
struct TemplateService {
    /// Mutable value used to verify accessors.
    value: usize,
}

impl ServiceRole for TemplateService {
    /// Runs the template service body.
    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Worker fixture for template tests.
struct TemplateWorker;

impl WorkerRole for TemplateWorker {
    /// Runs the template worker body.
    async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Job fixture for template tests.
struct TemplateJob;

impl JobRole for TemplateJob {
    /// Runs the template job body.
    async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Sidecar fixture for template tests.
struct TemplateSidecar;

impl SidecarRole for TemplateSidecar {
    /// Runs the template sidecar body.
    async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Supervisor fixture for template tests.
struct TemplateSupervisor;

impl SupervisorRole for TemplateSupervisor {
    /// Builds an empty nested supervisor tree.
    async fn build_tree(&mut self, ctx: &SupervisorContext) -> SupervisorResult<SupervisorSpec> {
        let _ = ctx;
        Ok(SupervisorSpec::root(Vec::new()))
    }
}

/// Verifies that service template accessors and child spec generation work.
#[test]
fn service_template_builds_service_child_spec() {
    let mut template = ServiceTemplate::new(TemplateService { value: 1 });
    assert_eq!(template.inner().value, 1);
    template.inner_mut().value = 2;
    assert_eq!(template.inner().value, 2);

    let spec = template
        .child_spec(ChildId::new("template-service"), "Template Service")
        .expect("service child spec");

    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Service));
    assert!(spec.factory.is_some());
}

/// Verifies that worker template builds a worker child spec.
#[test]
fn worker_template_builds_worker_child_spec() {
    let spec = WorkerTemplate::new(TemplateWorker)
        .child_spec(ChildId::new("template-worker"), "Template Worker")
        .expect("worker child spec");

    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Worker));
    assert!(spec.factory.is_some());
}

/// Verifies that job template builds a job child spec.
#[test]
fn job_template_builds_job_child_spec() {
    let spec = JobTemplate::new(TemplateJob)
        .child_spec(ChildId::new("template-job"), "Template Job")
        .expect("job child spec");

    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Job));
    assert!(spec.factory.is_some());
}

/// Verifies that sidecar template builds a sidecar child spec.
#[test]
fn sidecar_template_builds_sidecar_child_spec() {
    let primary = ChildId::new("template-primary");
    let spec = SidecarTemplate::new(TemplateSidecar, primary.clone())
        .child_spec(ChildId::new("template-sidecar"), "Template Sidecar")
        .expect("sidecar child spec");

    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Sidecar));
    assert_eq!(spec.sidecar_config, Some(SidecarConfig::new(primary, true)));
    assert!(spec.factory.is_some());
}

/// Verifies that supervisor template builds an executable supervisor role spec.
#[test]
fn supervisor_template_builds_supervisor_role_child_spec() {
    let spec = SupervisorTemplate::new(TemplateSupervisor)
        .child_spec(ChildId::new("template-supervisor"), "Template Supervisor")
        .expect("supervisor child spec");

    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Supervisor));
    assert_eq!(spec.criticality, Criticality::Critical);
    assert!(spec.factory.is_some());
}
