//! JSON Schema completion support for declarative task factories.
//!
//! This module enriches the generated supervisor configuration schema with
//! `factory_key` values from the caller-provided task factory registry.

use crate::config::configurable::SupervisorConfig;
use crate::error::types::SupervisorError;
use crate::task::factory_registry::TaskFactoryRegistry;
use rust_config_tree::config::{ConfigSchemaTarget, config_schema_targets_for_path};
use serde_json::{Value, json};
use std::path::Path;

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

/// Builds root and split-section schemas with registry-backed completion values.
///
/// # Arguments
///
/// - `output_path`: Root JSON Schema output path.
/// - `registry`: Task factory registry that supplies valid completion keys.
///
/// # Returns
///
/// Returns all generated schema targets with `factory_key` completions injected
/// where the target exposes that property.
///
/// # Errors
///
/// Returns [`SupervisorError`] when schema target generation, JSON parsing, or
/// JSON serialization fails.
///
/// # Examples
///
/// ```
/// use rust_supervisor::config::factory_schema::supervisor_schema_targets_with_factory_registry;
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
/// let targets = supervisor_schema_targets_with_factory_registry(
///     "config/supervisor_config.schema.json",
///     &registry,
/// )?;
/// assert!(targets.iter().any(|target| target.path.ends_with("children.schema.json")));
/// # Ok(())
/// # }
/// ```
pub fn supervisor_schema_targets_with_factory_registry(
    output_path: impl AsRef<Path>,
    registry: &TaskFactoryRegistry,
) -> Result<Vec<ConfigSchemaTarget>, SupervisorError> {
    let targets =
        config_schema_targets_for_path::<SupervisorConfig>(output_path).map_err(|error| {
            SupervisorError::fatal_config(format!(
                "failed to generate supervisor schema targets: {error}"
            ))
        })?;
    let mut enriched_targets = Vec::with_capacity(targets.len());

    for mut target in targets {
        let mut schema = serde_json::from_str::<Value>(&target.content).map_err(|error| {
            SupervisorError::fatal_config(format!(
                "failed to parse generated supervisor schema '{}': {error}",
                target.path.display()
            ))
        })?;
        inject_factory_key_completions_if_present(&mut schema, registry);
        target.content = serde_json::to_string_pretty(&schema).map_err(|error| {
            SupervisorError::fatal_config(format!(
                "failed to serialize generated supervisor schema '{}': {error}",
                target.path.display()
            ))
        })?;
        target.content.push('\n');
        enriched_targets.push(target);
    }

    Ok(enriched_targets)
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
    if !inject_factory_key_completions_if_present(schema, registry) {
        return Err(SupervisorError::fatal_config(
            "supervisor schema is missing ChildDeclaration.factory_key",
        ));
    }

    Ok(())
}

/// Injects completion values when a schema contains a `factory_key` property.
///
/// # Arguments
///
/// - `schema`: Root or split-section JSON Schema value.
/// - `registry`: Task factory registry that supplies valid completion keys.
///
/// # Returns
///
/// Returns `true` when the schema was updated. Returns `false` when the schema
/// does not contain a known `factory_key` location.
///
/// # Examples
///
/// ```
/// use rust_supervisor::config::factory_schema::inject_factory_key_completions_if_present;
/// use rust_supervisor::spec::child::TaskKind;
/// use rust_supervisor::task::factory::{TaskResult, service_fn};
/// use rust_supervisor::task::factory_registry::{
///     TaskFactoryDescriptor, TaskFactoryRegistry,
/// };
/// use serde_json::json;
/// use std::sync::Arc;
///
/// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
/// let mut schema = json!({
///     "items": {
///         "properties": {
///             "factory_key": { "type": ["string", "null"] }
///         }
///     }
/// });
/// let mut registry = TaskFactoryRegistry::new();
/// registry.register(TaskFactoryDescriptor::new(
///     "worker",
///     "Worker",
///     "Runs one worker.",
///     [TaskKind::AsyncWorker],
///     Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
/// ))?;
/// assert!(inject_factory_key_completions_if_present(&mut schema, &registry));
/// assert!(schema.to_string().contains("worker"));
/// # Ok(())
/// # }
/// ```
pub fn inject_factory_key_completions_if_present(
    schema: &mut Value,
    registry: &TaskFactoryRegistry,
) -> bool {
    let Some(pointer) = factory_key_schema_pointer(schema) else {
        return false;
    };
    let Some(factory_key_schema) = schema.pointer_mut(pointer) else {
        return false;
    };

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
    true
}

/// Returns the pointer for the supported `factory_key` schema location.
///
/// # Arguments
///
/// - `schema`: Root or split-section JSON Schema value.
///
/// # Returns
///
/// Returns a JSON pointer when the schema exposes `factory_key`.
fn factory_key_schema_pointer(schema: &Value) -> Option<&'static str> {
    [
        "/definitions/ChildDeclaration/properties/factory_key",
        "/$defs/ChildDeclaration/properties/factory_key",
        "/items/properties/factory_key",
        "/properties/factory_key",
    ]
    .into_iter()
    .find(|pointer| schema.pointer(pointer).is_some())
}
