//! Supervisor tree builder.
//!
//! This module converts declarations into indexed tree nodes while preserving
//! declaration order.

use crate::error::types::SupervisorError;
use crate::id::types::{ChildId, SupervisorPath};
use crate::spec::child::{ChildSpec, TaskKind};
use crate::spec::supervisor::SupervisorSpec;
use std::collections::HashSet;

/// Node in a supervisor tree.
#[derive(Debug, Clone)]
pub struct SupervisorTreeNode {
    /// Path of the node in the tree.
    pub path: SupervisorPath,
    /// Child declaration attached to this node.
    pub child: ChildSpec,
    /// Zero-based declaration order under the parent.
    pub declaration_index: usize,
}

/// Built supervisor tree with stable declaration order.
#[derive(Debug, Clone)]
pub struct SupervisorTree {
    /// Root supervisor path.
    pub root_path: SupervisorPath,
    /// Nodes in declaration order.
    pub nodes: Vec<SupervisorTreeNode>,
}

impl SupervisorTree {
    /// Builds a tree from a supervisor specification.
    ///
    /// # Arguments
    ///
    /// - `spec`: Supervisor declaration to index.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorTree`] when child identifiers and paths are valid.
    ///
    /// # Examples
    ///
    /// ```
    /// let spec = rust_supervisor::spec::supervisor::SupervisorSpec::root(Vec::new());
    /// let tree = rust_supervisor::tree::builder::SupervisorTree::build(&spec).unwrap();
    /// assert!(tree.nodes.is_empty());
    /// ```
    pub fn build(spec: &SupervisorSpec) -> Result<Self, SupervisorError> {
        spec.validate()?;
        let mut seen = HashSet::new();
        let mut nodes = Vec::with_capacity(spec.children.len());
        for (index, child) in spec.children.iter().enumerate() {
            validate_child_path(&spec.path, &child.id, &mut seen)?;
            nodes.push(SupervisorTreeNode {
                path: spec.path.join(&child.id.value),
                child: child.clone(),
                declaration_index: index,
            });
        }
        Ok(Self {
            root_path: spec.path.clone(),
            nodes,
        })
    }

    /// Returns the path for a child identifier.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier to locate.
    ///
    /// # Returns
    ///
    /// Returns the child path when the child exists.
    pub fn child_path(&self, child_id: &ChildId) -> Option<SupervisorPath> {
        self.nodes
            .iter()
            .find(|node| node.child.id == *child_id)
            .map(|node| node.path.clone())
    }

    /// Returns supervisor nodes declared as nested supervisors.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns nested supervisor nodes in declaration order.
    pub fn nested_supervisors(&self) -> Vec<&SupervisorTreeNode> {
        self.nodes
            .iter()
            .filter(|node| node.child.kind == TaskKind::Supervisor)
            .collect()
    }
}

/// Validates path uniqueness for a child under a parent path.
///
/// # Arguments
///
/// - `parent`: Parent supervisor path.
/// - `child_id`: Child identifier being appended to the path.
/// - `seen`: Set of paths already declared under the parent.
///
/// # Returns
///
/// Returns `Ok(())` when the path is unique.
fn validate_child_path(
    parent: &SupervisorPath,
    child_id: &ChildId,
    seen: &mut HashSet<String>,
) -> Result<(), SupervisorError> {
    let path = parent.join(&child_id.value).to_string();
    if seen.insert(path) {
        Ok(())
    } else {
        Err(SupervisorError::fatal_config(format!(
            "duplicate child path for {child_id}"
        )))
    }
}
