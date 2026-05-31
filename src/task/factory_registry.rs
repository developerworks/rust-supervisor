//! Task factory registry for declarative worker configuration.
//!
//! This module owns the mapping from stable configuration keys to executable
//! [`TaskFactory`](crate::task::factory::TaskFactory) values. Configuration
//! code uses the same registry to validate `factory_key` declarations and to
//! generate editor completion metadata.

use crate::error::types::SupervisorError;
use crate::spec::child::TaskKind;
use crate::task::factory::TaskFactory;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// Metadata and executable factory for one registered task kind.
#[derive(Clone)]
pub struct TaskFactoryDescriptor {
    /// Stable key used in YAML `factory_key` fields.
    pub key: String,
    /// Short display title used by schema completion.
    pub title: String,
    /// Human-readable description used by schema completion.
    pub description: String,
    /// Task kinds that may use this factory.
    pub allowed_kinds: Vec<TaskKind>,
    /// Factory used to build each child attempt.
    pub factory: Arc<dyn TaskFactory>,
}

impl TaskFactoryDescriptor {
    /// Creates a task factory descriptor.
    ///
    /// # Arguments
    ///
    /// - `key`: Stable key used in YAML `factory_key` fields.
    /// - `title`: Short display title used by schema completion.
    /// - `description`: Human-readable description used by schema completion.
    /// - `allowed_kinds`: Task kinds that may use this factory.
    /// - `factory`: Factory used to build each child attempt.
    ///
    /// # Returns
    ///
    /// Returns a [`TaskFactoryDescriptor`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::spec::child::TaskKind;
    /// use rust_supervisor::task::factory::{TaskResult, service_fn};
    /// use rust_supervisor::task::factory_registry::TaskFactoryDescriptor;
    /// use std::sync::Arc;
    ///
    /// let descriptor = TaskFactoryDescriptor::new(
    ///     "worker",
    ///     "Worker",
    ///     "Runs one worker task.",
    ///     [TaskKind::AsyncWorker],
    ///     Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
    /// );
    /// assert_eq!(descriptor.key, "worker");
    /// ```
    pub fn new(
        key: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        allowed_kinds: impl IntoIterator<Item = TaskKind>,
        factory: Arc<dyn TaskFactory>,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            description: description.into(),
            allowed_kinds: allowed_kinds.into_iter().collect(),
            factory,
        }
    }
}

impl Debug for TaskFactoryDescriptor {
    /// Formats descriptor metadata without printing the executable factory.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TaskFactoryDescriptor")
            .field("key", &self.key)
            .field("title", &self.title)
            .field("description", &self.description)
            .field("allowed_kinds", &self.allowed_kinds)
            .finish()
    }
}

/// Registry that resolves declarative task factory keys.
#[derive(Clone, Default)]
pub struct TaskFactoryRegistry {
    /// Factory descriptors indexed by key.
    entries: BTreeMap<String, TaskFactoryDescriptor>,
}

impl Debug for TaskFactoryRegistry {
    /// Formats registry keys without printing executable factories.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TaskFactoryRegistry")
            .field("keys", &self.keys())
            .finish()
    }
}

impl TaskFactoryRegistry {
    /// Creates an empty task factory registry.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns an empty [`TaskFactoryRegistry`].
    ///
    /// # Examples
    ///
    /// ```
    /// let registry = rust_supervisor::task::factory_registry::TaskFactoryRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one task factory descriptor.
    ///
    /// # Arguments
    ///
    /// - `descriptor`: Descriptor that owns the factory and completion metadata.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the key is valid and unused.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError`] when the key is invalid, duplicate, or has no
    /// allowed task kind.
    pub fn register(&mut self, descriptor: TaskFactoryDescriptor) -> Result<(), SupervisorError> {
        if !is_valid_factory_key(&descriptor.key) {
            return Err(SupervisorError::fatal_config(format!(
                "task factory key '{}' must match ^[a-zA-Z_][a-zA-Z0-9_-]*$",
                descriptor.key
            )));
        }
        if descriptor.allowed_kinds.is_empty() {
            return Err(SupervisorError::fatal_config(format!(
                "task factory key '{}' must allow at least one task kind",
                descriptor.key
            )));
        }
        if self.entries.contains_key(&descriptor.key) {
            return Err(SupervisorError::fatal_config(format!(
                "duplicate task factory key '{}'",
                descriptor.key
            )));
        }

        self.entries.insert(descriptor.key.clone(), descriptor);
        Ok(())
    }

    /// Resolves a registered factory for a task kind.
    ///
    /// # Arguments
    ///
    /// - `key`: Factory key loaded from configuration.
    /// - `kind`: Task kind declared by the child.
    ///
    /// # Returns
    ///
    /// Returns the registered factory when the key exists and supports `kind`.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError`] when the key is unknown or cannot be used for
    /// the requested task kind.
    pub fn resolve(
        &self,
        key: &str,
        kind: TaskKind,
    ) -> Result<Arc<dyn TaskFactory>, SupervisorError> {
        let descriptor = self.entries.get(key).ok_or_else(|| {
            SupervisorError::fatal_config(format!("unknown task factory key '{key}'"))
        })?;
        if !descriptor.allowed_kinds.contains(&kind) {
            return Err(SupervisorError::fatal_config(format!(
                "task factory key '{key}' does not support task kind {kind:?}"
            )));
        }
        Ok(descriptor.factory.clone())
    }

    /// Returns descriptors in stable key order.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns registered descriptors sorted by key.
    pub fn descriptors(&self) -> Vec<&TaskFactoryDescriptor> {
        self.entries.values().collect()
    }

    /// Returns registered keys in stable order.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns registered factory keys sorted by key.
    pub fn keys(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Returns whether the registry has no entries.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when no factory is registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Returns whether a factory key is valid for configuration use.
///
/// # Arguments
///
/// - `key`: Candidate key text.
///
/// # Returns
///
/// Returns `true` when the key matches the supported identifier surface.
pub fn is_valid_factory_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    let first = key.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    key.chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
}
