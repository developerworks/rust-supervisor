//! Role macro compile-pass integration tests.
//!
//! These tests compile and exercise role attribute macros through the public
//! runtime role contracts.

use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
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
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult};
use rust_supervisor_macros::{job, service, sidecar, supervisor_role, worker};
use std::sync::{Arc, Mutex};

/// Creates a task context for service macro tests.
///
/// # Arguments
///
/// - `id`: Stable child identifier used by the test.
///
/// # Returns
///
/// Returns a runtime task context.
fn task_context(id: &str) -> TaskContext {
    let (ctx, _heartbeat) = TaskContext::new(
        ChildId::new(id),
        SupervisorPath::root().join(id),
        Generation::initial(),
        ChildStartCount::first(),
    );
    ctx
}

/// Service used to verify generated macro bridges.
struct MacroService {
    /// Shared event list that records lifecycle order.
    events: Arc<Mutex<Vec<&'static str>>>,
}

// Attach the service role macro to the lifecycle implementation.
#[service(id = "macro-service", name = "Macro Service")]
impl MacroService {
    /// Records service initialization.
    async fn init(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("init");
        Ok(())
    }

    /// Records service execution.
    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("run");
        Ok(())
    }

    /// Records service shutdown.
    async fn shutdown(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("shutdown");
        Ok(())
    }
}

/// Verifies that the generated adapter calls lifecycle methods in order.
#[tokio::test]
async fn service_macro_generates_runtime_adapter() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = MacroService {
        events: events.clone(),
    }
    .service_adapter();

    let result = TaskFactory::build(&adapter, task_context("macro-service")).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *events.lock().expect("events lock"),
        vec!["init", "run", "shutdown"]
    );
}

/// Verifies that the generated child spec uses service role defaults.
#[test]
fn service_macro_generates_child_spec() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let spec = MacroService { events }.child_spec().expect("child spec");

    assert_eq!(spec.id, ChildId::new("macro-service"));
    assert_eq!(spec.name, "Macro Service");
    assert_eq!(spec.task_role, Some(TaskRole::Service));
}

/// Worker used to verify generated macro bridges.
struct MacroWorker {
    /// Shared event list that records lifecycle order.
    events: Arc<Mutex<Vec<&'static str>>>,
}

// Attach the worker role macro to the lifecycle implementation.
#[worker(id = "macro-worker", name = "Macro Worker")]
impl MacroWorker {
    /// Records worker initialization.
    async fn init(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("init");
        Ok(())
    }

    /// Records worker execution.
    async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("work");
        Ok(())
    }

    /// Records worker completion.
    async fn complete(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("complete");
        Ok(())
    }
}

/// Verifies that the generated worker adapter calls lifecycle methods in order.
#[tokio::test]
async fn worker_macro_generates_runtime_adapter() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = MacroWorker {
        events: events.clone(),
    }
    .worker_adapter();

    let result = TaskFactory::build(&adapter, task_context("macro-worker")).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *events.lock().expect("events lock"),
        vec!["init", "work", "complete"]
    );
}

/// Verifies that the generated worker child spec uses worker role defaults.
#[test]
fn worker_macro_generates_child_spec() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let spec = MacroWorker { events }.child_spec().expect("child spec");

    assert_eq!(spec.id, ChildId::new("macro-worker"));
    assert_eq!(spec.name, "Macro Worker");
    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Worker));
}

/// Job used to verify generated macro bridges.
struct MacroJob {
    /// Shared event list that records lifecycle order.
    events: Arc<Mutex<Vec<&'static str>>>,
}

// Attach the job role macro to the lifecycle implementation.
#[job(id = "macro-job", name = "Macro Job")]
impl MacroJob {
    /// Records job initialization.
    async fn init(&mut self, ctx: &JobContext) -> JobResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("init");
        Ok(())
    }

    /// Records job execution.
    async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("run");
        Ok(())
    }

    /// Records job completion.
    async fn complete(&mut self, ctx: &JobContext) -> JobResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("complete");
        Ok(())
    }
}

/// Verifies that the generated job adapter calls lifecycle methods in order.
#[tokio::test]
async fn job_macro_generates_runtime_adapter() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = MacroJob {
        events: events.clone(),
    }
    .job_adapter();

    let result = TaskFactory::build(&adapter, task_context("macro-job")).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *events.lock().expect("events lock"),
        vec!["init", "run", "complete"]
    );
}

/// Verifies that the generated job child spec uses job role defaults.
#[test]
fn job_macro_generates_child_spec() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let spec = MacroJob { events }.child_spec().expect("child spec");

    assert_eq!(spec.id, ChildId::new("macro-job"));
    assert_eq!(spec.name, "Macro Job");
    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Job));
}

/// Sidecar used to verify generated macro bridges.
struct MacroSidecar {
    /// Shared event list that records lifecycle order.
    events: Arc<Mutex<Vec<&'static str>>>,
}

// Attach the sidecar role macro to the lifecycle implementation.
#[sidecar(
    id = "macro-sidecar",
    name = "Macro Sidecar",
    primary = "macro-service"
)]
impl MacroSidecar {
    /// Records sidecar initialization.
    async fn init(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("init");
        Ok(())
    }

    /// Records sidecar execution.
    async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("run");
        Ok(())
    }

    /// Records sidecar shutdown.
    async fn shutdown(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("shutdown");
        Ok(())
    }
}

/// Verifies that the generated sidecar adapter calls lifecycle methods in order.
#[tokio::test]
async fn sidecar_macro_generates_runtime_adapter() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = MacroSidecar {
        events: events.clone(),
    }
    .sidecar_adapter();

    let result = TaskFactory::build(&adapter, task_context("macro-sidecar")).await;

    assert_eq!(result, TaskResult::Succeeded);
    assert_eq!(
        *events.lock().expect("events lock"),
        vec!["init", "run", "shutdown"]
    );
}

/// Verifies that the generated sidecar child spec uses sidecar role defaults.
#[test]
fn sidecar_macro_generates_child_spec() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let spec = MacroSidecar { events }.child_spec().expect("child spec");

    assert_eq!(spec.id, ChildId::new("macro-sidecar"));
    assert_eq!(spec.name, "Macro Sidecar");
    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Sidecar));
    assert_eq!(
        spec.sidecar_config,
        Some(SidecarConfig::new(ChildId::new("macro-service"), true))
    );
}

/// Supervisor role used to verify generated macro bridges.
struct MacroSupervisor {
    /// Shared event list that records lifecycle order.
    events: Arc<Mutex<Vec<&'static str>>>,
}

// Attach the supervisor role macro to the lifecycle implementation.
#[supervisor_role(id = "macro-supervisor", name = "Macro Supervisor")]
impl MacroSupervisor {
    /// Records nested supervisor tree construction.
    async fn build_tree(&mut self, ctx: &SupervisorContext) -> SupervisorResult<SupervisorSpec> {
        let _ = ctx;
        self.events.lock().expect("events lock").push("build_tree");
        Ok(SupervisorSpec::root(Vec::new()))
    }
}

/// Verifies that the generated supervisor adapter builds the nested tree.
#[tokio::test]
async fn supervisor_macro_generates_runtime_adapter() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let adapter = MacroSupervisor {
        events: events.clone(),
    }
    .supervisor_adapter();
    let ctx = task_context("macro-supervisor");
    ctx.cancel();

    let result = TaskFactory::build(&adapter, ctx).await;

    assert_eq!(result, TaskResult::Cancelled);
    assert_eq!(*events.lock().expect("events lock"), vec!["build_tree"]);
}

/// Verifies that the generated supervisor child spec uses supervisor role defaults.
#[test]
fn supervisor_macro_generates_child_spec() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let spec = MacroSupervisor { events }.child_spec().expect("child spec");

    assert_eq!(spec.id, ChildId::new("macro-supervisor"));
    assert_eq!(spec.name, "Macro Supervisor");
    assert_eq!(spec.kind, TaskKind::AsyncWorker);
    assert_eq!(spec.task_role, Some(TaskRole::Supervisor));
    assert!(spec.factory.is_some());
}
