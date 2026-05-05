//! Supervisor tree integration tests.
//!
//! These tests verify tree build and traversal behavior.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::{SupervisionStrategy, SupervisorSpec};
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::tree::builder::SupervisorTree;
use rust_supervisor::tree::order::{restart_scope, shutdown_order, startup_order};
use std::sync::Arc;

/// Verifies that declaration order drives startup and shutdown traversal.
#[test]
fn supervisor_tree_preserves_declaration_order() {
    let first = worker("first");
    let second = worker("second");
    let spec = SupervisorSpec::root(vec![first.clone(), second.clone()]);
    let tree = SupervisorTree::build(&spec).expect("build tree");

    assert_eq!(startup_order(&tree)[0].child.id, first.id);
    assert_eq!(shutdown_order(&tree)[0].child.id, second.id);
    assert_eq!(
        restart_scope(&tree, SupervisionStrategy::RestForOne, &first.id),
        vec![first.id, second.id]
    );
}

/// Builds a deterministic worker specification.
fn worker(id: &str) -> ChildSpec {
    let factory = service_fn(|_context| async { TaskResult::Succeeded });
    ChildSpec::worker(
        ChildId::new(id),
        id,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
}
