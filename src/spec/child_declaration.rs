//! Child declaration model for YAML loading and add_child RPC.
//!
//! This module owns the declarative representation of child declarations as
//! they appear in YAML configuration files or runtime add_child payloads. It
//! also defines the transaction phase enum, pending child state, and
//! compensating records used by the add_child transaction pipeline.

use crate::id::types::ChildId;
use crate::policy::task_role_defaults::{SeverityClass, SidecarConfig, TaskRole};
use crate::readiness::signal::ReadinessPolicy;
use crate::spec::child::{
    BackoffPolicy, ChildSpec, CommandPermissions, Criticality, EnvVar, HealthCheckConfig,
    HealthPolicy, RestartPolicy, SecretRef, ShutdownPolicy, TaskKind,
};
use confique::Config;
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct ChildDeclaration {
    /// Unique child name used for ChildId generation.
    pub name: String,
    /// Task kind.
    #[config(default = "async_worker")]
    #[serde(default)]
    pub kind: TaskKind,
    /// Child criticality.
    #[config(default = "optional")]
    #[serde(default)]
    pub criticality: Criticality,
    /// Low-cardinality tags used for grouping and diagnostics.
    #[config(default = [])]
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional task role that selects default lifecycle semantics.
    ///
    /// Defaults to `worker` when omitted.
    #[serde(default)]
    #[schemars(default = "default_task_role_for_schema")]
    pub task_role: Option<TaskRole>,
    /// Optional sidecar binding used when the role is `sidecar`.
    #[schemars(!default)]
    #[serde(default)]
    pub sidecar_config: Option<SidecarConfig>,
    /// Optional severity classification that overrides the role default.
    #[schemars(!default)]
    #[serde(default)]
    pub severity: Option<SeverityClass>,
    /// Optional group name for group-level isolation and budget tracking.
    #[schemars(!default)]
    #[serde(default)]
    pub group: Option<String>,
    /// Restart policy.
    #[config(default = "permanent")]
    #[serde(default)]
    pub restart_policy: RestartPolicy,
    /// Child dependencies by name.
    #[config(default = [])]
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Optional health check configuration.
    #[schemars(!default)]
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,
    /// Optional readiness policy; defaults to immediate readiness.
    #[schemars(!default)]
    #[serde(default)]
    pub readiness: Option<ReadinessPolicy>,
    /// Optional command permissions.
    #[schemars(!default)]
    #[serde(default)]
    pub command_permissions: Option<CommandPermissions>,
    /// Environment variables.
    #[config(default = [])]
    #[serde(default)]
    pub environment: Vec<EnvVar>,
    /// Secret references.
    #[config(default = [])]
    #[serde(default)]
    pub secrets: Vec<SecretRef>,
}

/// Returns the schema default shown for omitted child `task_role` values.
fn default_task_role_for_schema() -> Option<TaskRole> {
    Some(TaskRole::Worker)
}

/// Split-friendly nested section for child declarations.
///
/// Single-file configs use `children: [...]`. Split `children.yaml` files contain
/// only the child declaration sequence for this section.
#[derive(Debug, Clone, PartialEq, Config)]
pub struct ChildrenConfigSection {
    /// Child declarations loaded from the `children` configuration section.
    #[config(default = [{ "name": "worker" }])]
    pub items: Vec<ChildDeclaration>,
}

impl Default for ChildrenConfigSection {
    /// Returns an empty section for omitted runtime configuration values.
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl JsonSchema for ChildrenConfigSection {
    /// Returns the schema name for this transparent section wrapper.
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("ChildrenConfigSection")
    }

    /// Delegates schema generation to the child declaration sequence.
    fn json_schema(generator: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
        Vec::<ChildDeclaration>::json_schema(generator)
    }
}

impl Serialize for ChildrenConfigSection {
    /// Serializes this section as a child declaration sequence.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.items.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChildrenConfigSection {
    /// Deserializes a child declaration sequence into this section wrapper.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            items: Vec::<ChildDeclaration>::deserialize(deserializer)?,
        })
    }
}

impl ChildrenConfigSection {
    /// Returns child declarations as a slice.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the child declarations declared in this section.
    pub fn as_slice(&self) -> &[ChildDeclaration] {
        &self.items
    }

    /// Returns the number of child declarations in this section.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the child declaration count.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns whether this section contains no child declarations.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when this section contains no child declarations.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl From<ChildrenConfigSection> for Vec<ChildDeclaration> {
    /// Converts a split-friendly section into a plain child declaration vector.
    fn from(section: ChildrenConfigSection) -> Self {
        section.items
    }
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
            Some(hc) => HealthPolicy::new(
                std::time::Duration::from_secs(hc.check_interval_secs),
                std::time::Duration::from_secs(hc.timeout_secs),
            ),
            None => HealthPolicy::new(
                std::time::Duration::from_secs(10),
                std::time::Duration::from_secs(5),
            ),
        };

        let readiness_policy = decl.readiness.unwrap_or(ReadinessPolicy::Immediate);

        let command_permissions = decl.command_permissions.unwrap_or_default();

        Ok(Self {
            id: child_id,
            name: decl.name,
            kind,
            factory: None,
            restart_policy,
            shutdown_policy: ShutdownPolicy::new(
                std::time::Duration::from_secs(5),
                std::time::Duration::from_secs(1),
            ),
            health_policy,
            readiness_policy,
            backoff_policy: BackoffPolicy::new(
                std::time::Duration::from_millis(10),
                std::time::Duration::from_secs(1),
                0.0,
            ),
            dependencies,
            tags: decl.tags,
            criticality,
            task_role: decl.task_role,
            sidecar_config: decl.sidecar_config,
            severity: decl.severity,
            group: decl.group,
            health_check: decl.health_check,
            command_permissions,
            environment: decl.environment,
            secrets: decl.secrets,
            isolation: crate::spec::child::Isolation::AsyncWorker,
            cleanup_paths: Vec::new(),
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
                reason: format!("Secret reference '{secret_ref}' has invalid syntax"),
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
