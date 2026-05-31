//! JSON Schema completion support for declarative task factories.
//!
//! This module enriches the generated supervisor configuration schema with
//! `factory_key` values from the caller-provided task factory registry.

use crate::config::configurable::SupervisorConfig;
use crate::error::types::SupervisorError;
use crate::task::factory_registry::TaskFactoryRegistry;
use serde_json::{Value, json};

/// Builds a supervisor configuration schema with `factory_key` completion values.
///
/// # Arguments
///
/// - `registry`: Task factory registry that supplies valid completion keys.
///
/// # Returns
///
/// Returns a JSON Schema value whose `ChildDeclaration.factory_key` field
/// contains registry-backed completion candidates.
///
/// # Errors
///
/// Returns [`SupervisorError`] when the generated schema does not expose the
/// expected `ChildDeclaration.factory_key` property.
///
/// # Examples
///
/// ```
/// use rust_supervisor::config::factory_schema::supervisor_schema_with_factory_registry;
/// use rust_supervisor::spec::child::TaskKind;
/// use rust_supervisor::task::factory::{TaskResult, service_fn};
/// use rust_supervisor::task::factory_registry::{
///     TaskFactoryDescriptor, TaskFactoryRegistry,
/// };
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
/// let schema = supervisor_schema_with_factory_registry(&registry)?;
/// let schema_text = serde_json::to_string(&schema).unwrap();
/// assert!(schema_text.contains("worker"));
/// # Ok(())
/// # }
/// ```
pub fn supervisor_schema_with_factory_registry(
    registry: &TaskFactoryRegistry,
) -> Result<Value, SupervisorError> {
    let schema = schemars::schema_for!(SupervisorConfig);
    let mut value = serde_json::to_value(&schema).map_err(|error| {
        SupervisorError::fatal_config(format!("failed to serialize supervisor schema: {error}"))
    })?;
    inject_factory_key_completions(&mut value, registry)?;
    Ok(value)
}

/// Injects registry-backed completion values into an existing schema.
///
/// # Arguments
///
/// - `schema`: Schema value generated from [`SupervisorConfig`].
/// - `registry`: Task factory registry that supplies valid completion keys.
///
/// # Returns
///
/// Returns `Ok(())` after completion candidates have been injected.
///
/// # Errors
///
/// Returns [`SupervisorError`] when the schema does not expose the expected
/// `ChildDeclaration.factory_key` property.
pub fn inject_factory_key_completions(
    schema: &mut Value,
    registry: &TaskFactoryRegistry,
) -> Result<(), SupervisorError> {
    let pointer = if schema
        .pointer("/definitions/ChildDeclaration/properties/factory_key")
        .is_some()
    {
        "/definitions/ChildDeclaration/properties/factory_key"
    } else if schema
        .pointer("/$defs/ChildDeclaration/properties/factory_key")
        .is_some()
    {
        "/$defs/ChildDeclaration/properties/factory_key"
    } else {
        return Err(SupervisorError::fatal_config(
            "supervisor schema is missing ChildDeclaration.factory_key",
        ));
    };
    let factory_key_schema = schema.pointer_mut(pointer).ok_or_else(|| {
        SupervisorError::fatal_config("supervisor schema is missing ChildDeclaration.factory_key")
    })?;

    let choices = registry
        .descriptors()
        .into_iter()
        .map(|descriptor| {
            json!({
                "const": descriptor.key,
                "title": descriptor.title,
                "description": descriptor.description,
            })
        })
        .collect::<Vec<_>>();

    factory_key_schema["oneOf"] = Value::Array(choices);
    factory_key_schema["description"] = Value::String(
        "TaskFactory registry key used to bind worker children before startup.".to_owned(),
    );
    Ok(())
}
