//! Tree ordering utilities.
//!
//! This module provides pure traversal helpers for startup, shutdown, and
//! restart scope calculations.

use crate::id::types::ChildId;
use crate::spec::supervisor::SupervisionStrategy;
use crate::tree::builder::{SupervisorTree, SupervisorTreeNode};

/// Returns nodes in declaration order for startup.
///
/// # Arguments
///
/// - `tree`: Tree whose nodes should be traversed.
///
/// # Returns
///
/// Returns node references in declaration order.
///
/// # Examples
///
/// ```
/// let spec = rust_supervisor::spec::supervisor::SupervisorSpec::root(Vec::new());
/// let tree = rust_supervisor::tree::builder::SupervisorTree::build(&spec).unwrap();
/// assert!(rust_supervisor::tree::order::startup_order(&tree).is_empty());
/// ```
pub fn startup_order(tree: &SupervisorTree) -> Vec<&SupervisorTreeNode> {
    tree.nodes.iter().collect()
}

/// Returns nodes in reverse declaration order for shutdown.
///
/// # Arguments
///
/// - `tree`: Tree whose nodes should be traversed.
///
/// # Returns
///
/// Returns node references in reverse declaration order.
pub fn shutdown_order(tree: &SupervisorTree) -> Vec<&SupervisorTreeNode> {
    tree.nodes.iter().rev().collect()
}

/// Selects the restart scope for a failed child.
///
/// # Arguments
///
/// - `tree`: Tree that owns the child declarations.
/// - `strategy`: Supervisor restart strategy.
/// - `failed_child`: Identifier of the failed child.
///
/// # Returns
///
/// Returns child identifiers that belong to the restart scope.
pub fn restart_scope(
    tree: &SupervisorTree,
    strategy: SupervisionStrategy,
    failed_child: &ChildId,
) -> Vec<ChildId> {
    match strategy {
        SupervisionStrategy::OneForOne => vec![failed_child.clone()],
        SupervisionStrategy::OneForAll => all_children(tree),
        SupervisionStrategy::RestForOne => rest_for_one(tree, failed_child),
    }
}

/// Returns all child identifiers in declaration order.
///
/// # Arguments
///
/// - `tree`: Tree that owns the child declarations.
///
/// # Returns
///
/// Returns every child identifier in declaration order.
fn all_children(tree: &SupervisorTree) -> Vec<ChildId> {
    tree.nodes
        .iter()
        .map(|node| node.child.id.clone())
        .collect()
}

/// Returns the failed child and all children declared after it.
///
/// # Arguments
///
/// - `tree`: Tree that owns the child declarations.
/// - `failed_child`: Identifier where the restart scope starts.
///
/// # Returns
///
/// Returns child identifiers in restart order.
fn rest_for_one(tree: &SupervisorTree, failed_child: &ChildId) -> Vec<ChildId> {
    let Some(index) = tree
        .nodes
        .iter()
        .position(|node| node.child.id == *failed_child)
    else {
        return Vec::new();
    };
    tree.nodes[index..]
        .iter()
        .map(|node| node.child.id.clone())
        .collect()
}
