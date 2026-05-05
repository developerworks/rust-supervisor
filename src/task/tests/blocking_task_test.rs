//! Blocking task specification tests.
//!
//! These tests keep blocking worker behavior outside inline module tests.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;

/// Verifies that blocking workers require an owned task factory.
#[test]
fn blocking_worker_spec_accepts_factory() {
    let factory = service_fn(|_context| async { TaskResult::Succeeded });
    let child = ChildSpec::worker(
        ChildId::new("blocking-worker"),
        "blocking-worker",
        TaskKind::BlockingWorker,
        Arc::new(factory),
    );

    assert!(child.validate().is_ok());
    assert_eq!(child.kind, TaskKind::BlockingWorker);
}
