//! Supervisor specification tests.
//!
//! These tests verify child and supervisor declaration validation.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;

/// Verifies worker child specification field validation.
#[test]
fn child_spec_validates_worker_fields() {
    let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    let spec = ChildSpec::worker(
        ChildId::new("worker"),
        "worker",
        TaskKind::AsyncWorker,
        Arc::new(factory),
    );

    assert!(spec.validate().is_ok());
}

/// Verifies supervisor specification validation for child declarations.
#[test]
fn supervisor_spec_validates_children() {
    let spec = SupervisorSpec::root(Vec::new());

    assert!(spec.validate().is_ok());
}
