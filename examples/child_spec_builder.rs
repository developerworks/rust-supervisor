//! Demonstrates [`ChildSpecBuilder`](rust_supervisor::spec::child_builder::ChildSpecBuilder)
//! for constructing [`ChildSpec`](rust_supervisor::spec::child::ChildSpec) values in code.
//!
//! YAML configuration and `add_child` RPC payloads should still use
//! [`ChildDeclaration`](rust_supervisor::spec::child_declaration::ChildDeclaration)
//! and `TryFrom` conversion. This example is print-only and does not start a supervisor.

use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::task_role_defaults::{SidecarConfig, TaskRole};
use rust_supervisor::spec::child::{Criticality, RestartPolicy, ShutdownPolicy, TaskKind};
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;
use std::time::Duration;

/// Runs the ChildSpecBuilder demonstration.
fn main() -> Result<(), SupervisorError> {
    println!("=== ChildSpecBuilder Demo ===");
    println!();

    // Build and print a worker child with fluent setters.
    demo_worker_builder()?;
    // Build and print a one-shot job override on a worker base.
    demo_job_builder()?;
    // Build and print a nested supervisor child.
    demo_supervisor_builder()?;
    // Build and print a sidecar from the minimal `new` entry point.
    demo_minimal_new_builder()?;
    // Show that `build` rejects invalid sidecar combinations.
    demo_build_failure()?;
    Ok(())
}

/// Builds a worker child with the `worker` entry point and fluent setters.
fn demo_worker_builder() -> Result<(), SupervisorError> {
    // Create a no-op async worker factory.
    let factory = Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded }));

    // Build a worker child with tags, group, and shutdown policy overrides.
    let spec = ChildSpecBuilder::worker(
        ChildId::new("invoice-worker"),
        "Invoice Worker",
        TaskKind::AsyncWorker,
        factory,
    )
    .criticality(Criticality::Critical)
    .tag("worker")
    .tag("invoice")
    .group("billing")
    .shutdown_policy(ShutdownPolicy::new(
        Duration::from_millis(150),
        Duration::from_millis(50),
    ))
    .build()?;

    // Print the worker summary.
    println!("--- worker entry ---");
    print_spec_summary(&spec);
    println!();
    Ok(())
}

/// Builds a one-shot job child by overriding role and restart policy on a worker base.
fn demo_job_builder() -> Result<(), SupervisorError> {
    // Create a no-op async worker factory.
    let factory = Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded }));

    // Build a job child by overriding role and restart policy on a worker base.
    let spec = ChildSpecBuilder::worker(
        ChildId::new("nightly-export"),
        "Nightly Export",
        TaskKind::AsyncWorker,
        factory,
    )
    .task_role(TaskRole::Job)
    .restart_policy(RestartPolicy::Temporary)
    .tag("job")
    .build()?;

    // Print the job summary.
    println!("--- job override on worker base ---");
    print_spec_summary(&spec);
    println!();
    Ok(())
}

/// Builds a nested supervisor child with the `supervisor` entry point.
fn demo_supervisor_builder() -> Result<(), SupervisorError> {
    // Build a nested supervisor child with the supervisor entry point.
    let spec = ChildSpecBuilder::supervisor(ChildId::new("nested-tree"), "Nested Tree")
        .tag("nested")
        .build()?;

    // Print the supervisor summary.
    println!("--- supervisor entry ---");
    print_spec_summary(&spec);
    println!();
    Ok(())
}

/// Builds a sidecar from the minimal `new` entry point plus required fields.
fn demo_minimal_new_builder() -> Result<(), SupervisorError> {
    // Identify the primary child that the sidecar follows.
    let primary_id = ChildId::new("api");
    // Create a no-op async worker factory.
    let factory = Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded }));

    // Build a sidecar from the minimal `new` entry point plus required fields.
    let spec = ChildSpecBuilder::new(ChildId::new("metrics-sidecar"), "Metrics Sidecar")
        .kind(TaskKind::AsyncWorker)
        .factory(factory)
        .task_role(TaskRole::Sidecar)
        .sidecar_config(SidecarConfig::new(primary_id.clone(), false))
        .dependency(primary_id)
        .tag("sidecar")
        .build()?;

    // Print the sidecar summary including dependency ids.
    println!("--- new entry + setters ---");
    print_spec_summary(&spec);
    println!("  dependencies      = {:?}", spec.dependencies);
    println!();
    Ok(())
}

/// Shows that `build` rejects invalid sidecar combinations.
fn demo_build_failure() -> Result<(), SupervisorError> {
    // Create a no-op async worker factory.
    let factory = Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded }));

    // Attempt to build a sidecar without sidecar_config and capture the error.
    let error = ChildSpecBuilder::worker(
        ChildId::new("broken-sidecar"),
        "Broken Sidecar",
        TaskKind::AsyncWorker,
        factory,
    )
    .task_role(TaskRole::Sidecar)
    .build()
    .expect_err("sidecar without sidecar_config should fail validation");

    // Print the validation error.
    println!("--- build failure ---");
    println!("  error = {error}");
    println!();
    Ok(())
}

/// Prints a compact summary of the fields learners usually inspect first.
fn print_spec_summary(spec: &rust_supervisor::spec::child::ChildSpec) {
    println!("  name              = {}", spec.name);
    println!("  kind              = {:?}", spec.kind);
    println!(
        "  task_role         = {:?}",
        spec.task_role.map(|role| role.as_str())
    );
    println!("  restart_policy    = {:?}", spec.restart_policy);
    println!("  criticality       = {:?}", spec.criticality);
    println!("  group             = {:?}", spec.group);
    println!("  tags              = {:?}", spec.tags);
    println!("  has_factory       = {}", spec.factory.is_some());
}
