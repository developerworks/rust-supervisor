//! Registry store for child runtime records.
//!
//! This module owns the in-memory index that maps child identifiers and paths
//! to runtime records.

use crate::error::types::SupervisorError;
use crate::id::types::{ChildId, SupervisorPath};
use crate::registry::entry::ChildRuntime;
use crate::tree::builder::SupervisorTree;
use std::collections::HashMap;

/// In-memory registry for supervisor children.
#[derive(Debug, Clone, Default)]
pub struct RegistryStore {
    /// Runtime records keyed by child identifier.
    by_child_id: HashMap<ChildId, ChildRuntime>,
    /// Child identifiers keyed by path string.
    by_path: HashMap<String, ChildId>,
    /// Child identifiers in declaration order.
    declaration_order: Vec<ChildId>,
}

impl RegistryStore {
    /// Creates an empty registry store.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns an empty [`RegistryStore`].
    ///
    /// # Examples
    ///
    /// ```
    /// let store = rust_supervisor::registry::store::RegistryStore::new();
    /// assert!(store.declaration_order().is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers all tree nodes in declaration order.
    ///
    /// # Arguments
    ///
    /// - `tree`: Built supervisor tree.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when every child is registered once.
    pub fn register_tree(&mut self, tree: &SupervisorTree) -> Result<(), SupervisorError> {
        for node in &tree.nodes {
            self.register(ChildRuntime::new(node.child.clone(), node.path.clone()))?;
        }
        Ok(())
    }

    /// Registers a single child runtime record.
    ///
    /// # Arguments
    ///
    /// - `runtime`: Child runtime record to insert.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when no child or path collision exists.
    pub fn register(&mut self, runtime: ChildRuntime) -> Result<(), SupervisorError> {
        if self.by_child_id.contains_key(&runtime.id) {
            return Err(SupervisorError::fatal_config(format!(
                "duplicate child id: {}",
                runtime.id
            )));
        }
        let path = runtime.path.to_string();
        if self.by_path.contains_key(&path) {
            return Err(SupervisorError::fatal_config(format!(
                "duplicate child path: {path}"
            )));
        }
        self.declaration_order.push(runtime.id.clone());
        self.by_path.insert(path, runtime.id.clone());
        self.by_child_id.insert(runtime.id.clone(), runtime);
        Ok(())
    }

    /// Returns a runtime record by child identifier.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier to find.
    ///
    /// # Returns
    ///
    /// Returns the runtime record when it exists.
    pub fn child(&self, child_id: &ChildId) -> Option<&ChildRuntime> {
        self.by_child_id.get(child_id)
    }

    /// Returns a mutable runtime record by child identifier.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier to find.
    ///
    /// # Returns
    ///
    /// Returns the mutable runtime record when it exists.
    pub fn child_mut(&mut self, child_id: &ChildId) -> Option<&mut ChildRuntime> {
        self.by_child_id.get_mut(child_id)
    }

    /// Returns a runtime record by supervisor path.
    ///
    /// # Arguments
    ///
    /// - `path`: Child path to find.
    ///
    /// # Returns
    ///
    /// Returns the runtime record when it exists.
    pub fn child_by_path(&self, path: &SupervisorPath) -> Option<&ChildRuntime> {
        self.by_path
            .get(&path.to_string())
            .and_then(|child_id| self.by_child_id.get(child_id))
    }

    /// Returns child identifiers in declaration order.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns declaration-order child identifiers.
    pub fn declaration_order(&self) -> &[ChildId] {
        &self.declaration_order
    }
}
