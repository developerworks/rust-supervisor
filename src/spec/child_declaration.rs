//! Child declaration model for YAML loading and add_child RPC.
//!
//! This module owns the declarative representation of child declarations as
//! they appear in YAML configuration files or runtime add_child payloads. It
//! also defines the transaction phase enum, pending child state, and
//! compensating records used by the add_child transaction pipeline.

use crate::id::types::ChildId;
use crate::spec::child::{
    ChildSpec, CommandPermissions, Criticality, EnvVar, HealthCheckConfig, ReadinessConfig,
    ResourceLimits, RestartPolicy, SecretRef, TaskKind,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Valid characters for child names and secret names: alphanumeric, underscore, hyphen.
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Validates a `${SECRET_NAME}` placeholder syntax.
fn is_valid_secret_placeholder(s: &str) -> bool {
    if !s.starts_with("${") || !s.ends_with('}') || s.len() < 4 {
        return false;
    }
    let inner = &s[2..s.len() - 1];
    if inner.is_empty() {
        return false;
    }
    let first = inner.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Declarative child specification loaded from YAML or received via add_child RPC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChildDeclaration {
    /// Unique child name used for ChildId generation.
    pub name: String,
    /// Task kind.
    #[serde(default)]
    pub kind: TaskKind,
    /// Child criticality.
    #[serde(default)]
    pub criticality: Criticality,
    /// Restart policy.
    #[serde(default)]
    pub restart_policy: RestartPolicy,
    /// Child dependencies by name.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Optional health check configuration.
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,
    /// Optional readiness check configuration.
    #[serde(default)]
    pub readiness: Option<ReadinessConfig>,
    /// Optional resource limits.
    #[serde(default)]
    pub resource_limits: Option<ResourceLimits>,
    /// Optional command permissions.
    #[serde(default)]
    pub command_permissions: Option<CommandPermissions>,
    /// Environment variables.
    #[serde(default)]
    pub environment: Vec<EnvVar>,
    /// Secret references.
    #[serde(default)]
    pub secrets: Vec<SecretRef>,
}

/// Phase of an add_child transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Parsing completed.
    Parsed,
    /// Validation passed.
    Validated,
    /// Registered in the topology.
    Registered,
    /// Child has been started.
    Started,
    /// Audit has been persisted.
    Audited,
    /// Transaction committed successfully.
    Committed,
    /// Transaction failed, compensation in progress.
    Compensating,
    /// Compensation completed.
    Compensated,
}

/// Pending child entry in the add_child transaction staging area.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChild {
    /// Unique transaction identifier.
    pub transaction_id: Uuid,
    /// Original child declaration.
    pub declaration: ChildDeclaration,
    /// Converted runtime child specification.
    pub child_spec: Box<ChildSpec>,
    /// Current transaction phase.
    pub phase: Phase,
    /// Creation timestamp in Unix nanoseconds.
    pub created_at_unix_nanos: u128,
}

// Manual PartialEq — skips child_spec because ChildSpec contains
// Arc<dyn TaskFactory> which does not implement PartialEq.
impl PartialEq for PendingChild {
    /// Compares two PendingChild values, skipping `child_spec`.
    fn eq(&self, other: &Self) -> bool {
        self.transaction_id == other.transaction_id
            && self.declaration == other.declaration
            && self.phase == other.phase
            && self.created_at_unix_nanos == other.created_at_unix_nanos
    }
}

/// Compensating record stored in the audit channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompensatingRecord {
    /// Unique transaction identifier.
    pub transaction_id: Uuid,
    /// Operation type (e.g. "add_child").
    pub operation: String,
    /// Compensation state: "pending", "committed", or "compensated".
    pub state: String,
    /// Child name.
    pub child_name: String,
    /// SHA-256 hash of the ChildDeclaration.
    pub declaration_hash: String,
    /// Optional error reason.
    pub error: Option<String>,
    /// Optional correlation id for linking to 006-5 event chains.
    pub correlation_id: Option<String>,
    /// Optional runtime ChildId, if assigned.
    pub child_id: Option<String>,
    /// Creation timestamp in Unix nanoseconds.
    pub created_at_unix_nanos: u128,
}

/// Validation error for a child declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationError {
    /// JSON Pointer field path.
    pub field_path: String,
    /// Human-readable failure reason.
    pub reason: String,
    /// Optional actionable hint.
    pub hint: Option<String>,
}

/// Converts a ChildDeclaration into a ChildSpec.
impl TryFrom<ChildDeclaration> for ChildSpec {
    type Error = ValidationError;

    /// Converts a child declaration into a runtime child specification.
    ///
    /// # Arguments
    ///
    /// - `decl`: The child declaration to convert.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildSpec`] with mapped fields.
    ///
    /// # Errors
    ///
    /// Returns a [`ValidationError`] when the declaration cannot be converted.
    fn try_from(decl: ChildDeclaration) -> Result<Self, Self::Error> {
        let child_id = ChildId::new(&decl.name);
        let kind = decl.kind;
        let criticality = decl.criticality;
        let restart_policy = decl.restart_policy;

        // Convert dependency names to ChildIds.
        let dependencies: Vec<ChildId> = decl.dependencies.iter().map(ChildId::new).collect();

        // Map health_check to health_policy.
        let health_policy = match &decl.health_check {
            Some(hc) => crate::spec::child::HealthPolicy::new(
                std::time::Duration::from_secs(hc.check_interval_secs),
                std::time::Duration::from_secs(hc.timeout_secs),
            ),
            None => crate::spec::child::HealthPolicy::new(
                std::time::Duration::from_secs(10),
                std::time::Duration::from_secs(5),
            ),
        };

        // Map readiness using the existing ReadinessPolicy::Immediate as default.
        let readiness_policy = crate::readiness::signal::ReadinessPolicy::Immediate;

        let command_permissions = decl.command_permissions.unwrap_or_default();

        Ok(Self {
            id: child_id,
            name: decl.name,
            kind,
            factory: None,
            restart_policy,
            shutdown_policy: crate::spec::child::ShutdownPolicy::new(
                std::time::Duration::from_secs(5),
                std::time::Duration::from_secs(1),
            ),
            health_policy,
            readiness_policy,
            backoff_policy: crate::spec::child::BackoffPolicy::new(
                std::time::Duration::from_millis(10),
                std::time::Duration::from_secs(1),
                0.0,
            ),
            dependencies,
            tags: Vec::new(),
            criticality,
            work_role: None,
            sidecar_config: None,
            severity: None,
            group: None,
            health_check: decl.health_check,
            readiness: decl.readiness,
            resource_limits: decl.resource_limits,
            command_permissions,
            environment: decl.environment,
            secrets: decl.secrets,
        })
    }
}

/// Validates a child declaration against the given set of existing child names.
///
/// # Arguments
///
/// - `declaration`: The child declaration to validate.
/// - `all_names`: Set of existing child names for dependency existence checks.
///
/// # Returns
///
/// Returns `Ok(())` when all validation rules pass.
///
/// # Errors
///
/// Returns a [`ValidationError`] describing the first rule violation found.
pub fn validate_child_declaration(
    declaration: &ChildDeclaration,
    all_names: &HashSet<String>,
) -> Result<(), ValidationError> {
    // Rule 1: name is non-empty and matches identifier pattern.
    if !is_valid_identifier(&declaration.name) {
        return Err(ValidationError {
            field_path: "name".to_string(),
            reason: format!(
                "Child name '{}' contains invalid characters",
                declaration.name
            ),
            hint: Some("Names must match ^[a-zA-Z_][a-zA-Z0-9_-]*$".to_string()),
        });
    }

    // Rule 2: dependencies exist in all_names.
    for dep in &declaration.dependencies {
        if !all_names.contains(dep) {
            return Err(ValidationError {
                field_path: format!("dependencies[{dep}]"),
                reason: format!("Dependency '{dep}' does not exist in the children list"),
                hint: Some(format!(
                    "Add a child named '{dep}' or remove the dependency"
                )),
            });
        }
    }

    // Rule 4: secret placeholder syntax validation.
    for secret in &declaration.secrets {
        let placeholder = format!("${{{}}}", secret.name);
        if !is_valid_secret_placeholder(&placeholder) {
            return Err(ValidationError {
                field_path: format!("secrets[{}].name", secret.name),
                reason: format!(
                    "Secret name '{}' contains invalid characters for placeholder",
                    secret.name
                ),
                hint: Some("Secret names must match ^[A-Za-z_][A-Za-z0-9_]*$".to_string()),
            });
        }
    }
    for env in &declaration.environment {
        if let Some(ref secret_ref) = env.secret_ref
            && !is_valid_secret_placeholder(secret_ref)
        {
            return Err(ValidationError {
                field_path: format!("environment[{}].secret_ref", env.name),
                reason: format!("Secret reference '{}' has invalid syntax", secret_ref),
                hint: Some(
                    "Secret references must match ^\\$\\{[A-Za-z_][A-Za-z0-9_]*\\}$".to_string(),
                ),
            });
        }
    }

    // Rule 5: value and secret_ref are mutually exclusive.
    for env in &declaration.environment {
        if env.value.is_some() && env.secret_ref.is_some() {
            return Err(ValidationError {
                field_path: format!("environment[{}]", env.name),
                reason: format!(
                    "Environment variable '{}' has both value and secret_ref set",
                    env.name
                ),
                hint: Some("Set either 'value' or 'secret_ref', not both".to_string()),
            });
        }
    }

    Ok(())
}
