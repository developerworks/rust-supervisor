//! Registry store tests.
//!
//! These tests verify declaration-order registration and child lookup.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::registry::store::RegistryStore;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::tree::builder::SupervisorTree;
use std::sync::Arc;

#[test]
fn registry_indexes_tree_nodes_by_child_and_path() {
    let child = worker("worker");
    let spec = SupervisorSpec::root(vec![child.clone()]);
    let tree = SupervisorTree::build(&spec).unwrap();
    let mut store = RegistryStore::new();

    store.register_tree(&tree).unwrap();

    assert!(store.child(&child.id).is_some());
    assert_eq!(store.declaration_order(), &[child.id]);
    assert!(store.child_by_path(&tree.nodes[0].path).is_some());
}

fn worker(id: &str) -> ChildSpec {
    let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    ChildSpec::worker(
        ChildId::new(id),
        id,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
}
