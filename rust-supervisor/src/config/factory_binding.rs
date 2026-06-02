//! Factory binding from configuration keys to runtime child specs.
//!
//! The functions in this module keep executable task factories out of raw YAML
//! loading while still providing a single startup-time binding step.

use crate::error::types::SupervisorError;
use crate::spec::child::{ChildSpec, TaskKind};
use crate::task::factory_registry::TaskFactoryRegistry;

/// Binds all worker child factory keys to executable task factories.
///
/// # Arguments
///
/// - `children`: Child specifications converted from configuration.
/// - `registry`: Registry that owns the executable factories.
///
/// # Returns
///
/// Returns `Ok(())` when every worker child has a usable factory.
///
/// # Errors
///
/// Returns [`SupervisorError`] when a worker is missing `factory_key`, a key is
/// unknown, a key does not support the declared task kind, or a supervisor child
/// declares a factory key.
///
/// # Examples
///
/// ```
/// use rust_supervisor::config::factory_binding::bind_task_factories;
/// use rust_supervisor::id::types::ChildId;
/// use rust_supervisor::spec::child_builder::ChildSpecBuilder;
/// use rust_supervisor::task::factory::{TaskResult, service_fn};
/// use rust_supervisor::task::factory_registry::{
///     TaskFactoryDescriptor, TaskFactoryRegistry,
/// };
/// use rust_supervisor::spec::child::TaskKind;
/// use std::sync::Arc;
///
/// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
/// let mut registry = TaskFactoryRegistry::new();
/// registry.register(TaskFactoryDescriptor::new(
///     "worker",
///     "Worker",
///     "Runs one worker.",
///     [TaskKind::AsyncWorker],
///     Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
/// ))?;
/// let child = ChildSpecBuilder::new(ChildId::new("worker"), "worker")
///     .kind(TaskKind::AsyncWorker)
///     .factory_key("worker")
///     .factory(Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })))
///     .build()?;
/// let mut children = vec![child];
/// bind_task_factories(&mut children, &registry)?;
/// assert!(children[0].factory.is_some());
/// # Ok(())
/// # }
/// ```
pub fn bind_task_factories(
    children: &mut [ChildSpec],
    registry: &TaskFactoryRegistry,
) -> Result<(), SupervisorError> {
    for child in children {
        bind_child_factory(child, registry)?;
    }
    Ok(())
}

/// Binds one child factory key to an executable task factory.
///
/// # Arguments
///
/// - `child`: Child specification converted from configuration.
/// - `registry`: Registry that owns executable task factories.
///
/// # Returns
///
/// Returns `Ok(())` after the child has a valid factory assignment or does not
/// require one.
///
/// # Errors
///
/// Returns [`SupervisorError`] when the child factory declaration is invalid.
pub fn bind_child_factory(
    child: &mut ChildSpec,
    registry: &TaskFactoryRegistry,
) -> Result<(), SupervisorError> {
    match child.kind {
        TaskKind::AsyncWorker | TaskKind::BlockingWorker => {
            let key = child
                .factory_key
                .as_deref()
                .filter(|key| !key.trim().is_empty())
                .ok_or_else(|| {
                    SupervisorError::fatal_config(format!(
                        "worker child '{}' requires factory_key",
                        child.id
                    ))
                })?;
            child.factory = Some(registry.resolve(key, child.kind)?);
            Ok(())
        }
        TaskKind::Supervisor => {
            if child
                .factory_key
                .as_deref()
                .is_some_and(|key| !key.trim().is_empty())
            {
                return Err(SupervisorError::fatal_config(format!(
                    "supervisor child '{}' must not declare factory_key",
                    child.id
                )));
            }
            child.factory = None;
            Ok(())
        }
    }
}
