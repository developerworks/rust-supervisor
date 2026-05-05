//! Supervisor tree tests.
//!
//! These tests verify tree construction and ordering utilities.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::{SupervisionStrategy, SupervisorSpec};
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::tree::builder::SupervisorTree;
use rust_supervisor::tree::order::{restart_scope, shutdown_order, startup_order};
use std::sync::Arc;

#[test]
fn tree_preserves_declaration_and_reverse_shutdown_order() {
    let first = child("first");
    let second = child("second");
    let spec = SupervisorSpec::root(vec![first.clone(), second.clone()]);
    let tree = SupervisorTree::build(&spec).unwrap();

    assert_eq!(startup_order(&tree)[0].child.id, first.id);
    assert_eq!(shutdown_order(&tree)[0].child.id, second.id);
}

#[test]
fn rest_for_one_selects_failed_child_and_following_children() {
    let first = child("first");
    let second = child("second");
    let spec = SupervisorSpec::root(vec![first, second.clone()]);
    let tree = SupervisorTree::build(&spec).unwrap();

    let scope = restart_scope(&tree, SupervisionStrategy::RestForOne, &second.id);

    assert_eq!(scope, vec![second.id]);
}

fn child(id: &str) -> ChildSpec {
    let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    ChildSpec::worker(
        ChildId::new(id),
        id,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
}
