//! Tree ordering utilities.
//!
//! This module provides pure traversal helpers for startup, shutdown, and
//! restart scope calculations.

use crate::id::types::ChildId;
use crate::spec::supervisor::{
    GroupStrategy, StrategyExecutionPlan, SupervisionStrategy, SupervisorSpec,
};
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

/// Builds the restart plan for a failed child.
///
/// # Arguments
///
/// - `tree`: Tree that owns child declaration order.
/// - `spec`: Supervisor specification that owns strategy overrides.
/// - `failed_child`: Identifier of the failed child.
///
/// # Returns
///
/// Returns a [`StrategyExecutionPlan`] with the selected scope and governance.
pub fn restart_execution_plan(
    tree: &SupervisorTree,
    spec: &SupervisorSpec,
    failed_child: &ChildId,
) -> StrategyExecutionPlan {
    if let Some(override_strategy) = child_override(spec, failed_child) {
        return StrategyExecutionPlan {
            failed_child: failed_child.clone(),
            strategy: override_strategy.strategy,
            scope: restart_scope(tree, override_strategy.strategy, failed_child),
            group: None,
            restart_budget: override_strategy.restart_budget.or(spec.restart_budget),
            escalation_policy: override_strategy
                .escalation_policy
                .or(spec.escalation_policy),
            dynamic_supervisor_enabled: spec.dynamic_supervisor_policy.enabled,
        };
    }

    if let Some(group_strategy) = group_strategy(tree, spec, failed_child) {
        return StrategyExecutionPlan {
            failed_child: failed_child.clone(),
            strategy: group_strategy.strategy,
            scope: group_restart_scope(
                tree,
                &group_strategy.group,
                group_strategy.strategy,
                failed_child,
            ),
            group: Some(group_strategy.group.clone()),
            restart_budget: group_strategy.restart_budget.or(spec.restart_budget),
            escalation_policy: group_strategy.escalation_policy.or(spec.escalation_policy),
            dynamic_supervisor_enabled: spec.dynamic_supervisor_policy.enabled,
        };
    }

    StrategyExecutionPlan {
        failed_child: failed_child.clone(),
        strategy: spec.strategy,
        scope: restart_scope(tree, spec.strategy, failed_child),
        group: None,
        restart_budget: spec.restart_budget,
        escalation_policy: spec.escalation_policy,
        dynamic_supervisor_enabled: spec.dynamic_supervisor_policy.enabled,
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

/// Returns a child override for the failed child.
///
/// # Arguments
///
/// - `spec`: Supervisor specification that owns overrides.
/// - `failed_child`: Identifier of the failed child.
///
/// # Returns
///
/// Returns the matching override when one is declared.
fn child_override<'a>(
    spec: &'a SupervisorSpec,
    failed_child: &ChildId,
) -> Option<&'a crate::spec::supervisor::ChildStrategyOverride> {
    spec.child_strategy_overrides
        .iter()
        .find(|override_strategy| override_strategy.child_id == *failed_child)
}

/// Returns the group strategy for the failed child.
///
/// # Arguments
///
/// - `tree`: Tree that owns child tags.
/// - `spec`: Supervisor specification that owns group strategies.
/// - `failed_child`: Identifier of the failed child.
///
/// # Returns
///
/// Returns the matching group strategy when the child belongs to one group.
fn group_strategy<'a>(
    tree: &SupervisorTree,
    spec: &'a SupervisorSpec,
    failed_child: &ChildId,
) -> Option<&'a GroupStrategy> {
    let child = tree
        .nodes
        .iter()
        .find(|node| node.child.id == *failed_child)?;
    spec.group_strategies
        .iter()
        .find(|strategy| child.child.tags.contains(&strategy.group))
}

/// Selects a restart scope constrained to one group.
///
/// # Arguments
///
/// - `tree`: Tree that owns declaration order.
/// - `group`: Group tag that constrains the scope.
/// - `strategy`: Strategy applied inside the group.
/// - `failed_child`: Identifier of the failed child.
///
/// # Returns
///
/// Returns child identifiers selected inside the group.
fn group_restart_scope(
    tree: &SupervisorTree,
    group: &str,
    strategy: SupervisionStrategy,
    failed_child: &ChildId,
) -> Vec<ChildId> {
    let group_nodes = group_nodes(tree, group);
    match strategy {
        SupervisionStrategy::OneForOne => vec![failed_child.clone()],
        SupervisionStrategy::OneForAll => group_nodes
            .iter()
            .map(|node| node.child.id.clone())
            .collect(),
        SupervisionStrategy::RestForOne => group_rest_for_one(&group_nodes, failed_child),
    }
}

/// Returns nodes that belong to a group in declaration order.
///
/// # Arguments
///
/// - `tree`: Tree that owns child tags.
/// - `group`: Group tag to match.
///
/// # Returns
///
/// Returns matching nodes in declaration order.
fn group_nodes<'a>(tree: &'a SupervisorTree, group: &str) -> Vec<&'a SupervisorTreeNode> {
    tree.nodes
        .iter()
        .filter(|node| node.child.tags.iter().any(|tag| tag == group))
        .collect()
}

/// Returns the failed group child and later group children.
///
/// # Arguments
///
/// - `nodes`: Group nodes in declaration order.
/// - `failed_child`: Identifier where the restart scope starts.
///
/// # Returns
///
/// Returns child identifiers selected by group-local `RestForOne`.
fn group_rest_for_one(nodes: &[&SupervisorTreeNode], failed_child: &ChildId) -> Vec<ChildId> {
    let Some(index) = nodes.iter().position(|node| node.child.id == *failed_child) else {
        return Vec::new();
    };
    nodes[index..]
        .iter()
        .map(|node| node.child.id.clone())
        .collect()
}
