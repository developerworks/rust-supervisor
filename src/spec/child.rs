//! Child declaration model.
//!
//! This module owns declarative child specifications and validates local child
//! invariants before the runtime registers or starts work.

use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::policy::task_role_defaults::{SeverityClass, SidecarConfig, TaskRole};
use crate::readiness::signal::ReadinessPolicy;
use crate::task::factory::TaskFactory;
use confique::Config;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;

/// Kind of task represented by a child declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Asynchronous worker that can be cancelled through its context.
    AsyncWorker,
    /// Blocking worker with explicit shutdown and escalation boundaries.
    BlockingWorker,
    /// Nested supervisor node.
    Supervisor,
}

impl Default for TaskKind {
    /// Returns the default task kind: [`AsyncWorker`](TaskKind::AsyncWorker).
    fn default() -> Self {
        Self::AsyncWorker
    }
}

/// Runtime isolation strategy for a child task.
///
/// Controls which tokio runtime the child's future is spawned into. Tasks
/// that are prone to blocking (CPU-heavy loops, synchronous I/O) should use
/// `BlockingPool` to avoid starving the async worker threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Isolation {
    /// Spawn on the default multi-threaded async worker pool.
    /// Suitable for well-behaved async tasks that always yield at `.await`.
    AsyncWorker,
    /// Spawn on the dedicated `spawn_blocking` thread pool (up to 500 threads).
    /// Use for tasks that may block or perform CPU-heavy work without `.await`.
    BlockingPool,
}

impl Default for Isolation {
    /// Returns the default isolation: [`AsyncWorker`](Isolation::AsyncWorker).
    fn default() -> Self {
        Self::AsyncWorker
    }
}

/// Importance of a child to its parent supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Criticality {
    /// The child is required for the supervisor to remain healthy.
    Critical,
    /// The child can fail without forcing parent shutdown.
    Optional,
}

impl Default for Criticality {
    /// Returns the default criticality: [`Optional`](Criticality::Optional).
    fn default() -> Self {
        Self::Optional
    }
}

/// Restart behavior attached to a child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Restart regardless of the exit result.
    Permanent,
    /// Restart only when the task failed.
    Transient,
    /// Do not restart after any exit.
    Temporary,
}

impl Default for RestartPolicy {
    /// Returns the default restart policy: [`Permanent`](RestartPolicy::Permanent).
    fn default() -> Self {
        Self::Permanent
    }
}

/// Shutdown behavior attached to a child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ShutdownPolicy {
    /// Graceful stop budget for cooperative shutdown.
    pub graceful_timeout: Duration,
    /// Wait budget after an abort request.
    pub abort_wait: Duration,
}

impl ShutdownPolicy {
    /// Creates a shutdown policy.
    ///
    /// # Arguments
    ///
    /// - `graceful_timeout`: Cooperative shutdown budget.
    /// - `abort_wait`: Wait budget after abort escalation.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownPolicy`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let policy = rust_supervisor::spec::child::ShutdownPolicy::new(
    ///     std::time::Duration::from_secs(1),
    ///     std::time::Duration::from_millis(100),
    /// );
    /// assert_eq!(policy.graceful_timeout.as_secs(), 1);
    /// ```
    pub fn new(graceful_timeout: Duration, abort_wait: Duration) -> Self {
        Self {
            graceful_timeout,
            abort_wait,
        }
    }
}

/// Health behavior attached to a child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HealthPolicy {
    /// Expected heartbeat interval.
    pub heartbeat_interval: Duration,
    /// Maximum age for the last heartbeat before the child is stale.
    pub stale_after: Duration,
}

impl HealthPolicy {
    /// Creates a health policy.
    ///
    /// # Arguments
    ///
    /// - `heartbeat_interval`: Expected heartbeat interval.
    /// - `stale_after`: Maximum heartbeat age.
    ///
    /// # Returns
    ///
    /// Returns a [`HealthPolicy`] value.
    pub fn new(heartbeat_interval: Duration, stale_after: Duration) -> Self {
        Self {
            heartbeat_interval,
            stale_after,
        }
    }
}

/// Health check configuration for a child declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct HealthCheckConfig {
    /// Interval between health checks in seconds.
    #[config(default = 10)]
    #[serde(default = "default_health_check_interval_secs")]
    pub check_interval_secs: u64,
    /// Timeout for each health check in seconds.
    #[config(default = 5)]
    #[serde(default = "default_health_check_timeout_secs")]
    pub timeout_secs: u64,
    /// Maximum retries before marking the child as unhealthy.
    #[config(default = 3)]
    #[serde(default = "default_health_check_max_retries")]
    pub max_retries: u32,
}

impl Default for HealthCheckConfig {
    /// Returns the default health check config: 10s interval, 5s timeout, 3 retries.
    fn default() -> Self {
        Self {
            check_interval_secs: default_health_check_interval_secs(),
            timeout_secs: default_health_check_timeout_secs(),
            max_retries: default_health_check_max_retries(),
        }
    }
}

/// Returns the default health check interval in seconds.
fn default_health_check_interval_secs() -> u64 {
    10
}

/// Returns the default health check timeout in seconds.
fn default_health_check_timeout_secs() -> u64 {
    5
}

/// Returns the default health check retry count.
fn default_health_check_max_retries() -> u32 {
    3
}

/// Command permissions granted to a child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct CommandPermissions {
    /// Whether the child may trigger supervisor shutdown.
    #[config(default = false)]
    #[serde(default)]
    pub allow_shutdown: bool,
    /// Whether the child may request its own restart.
    #[config(default = false)]
    #[serde(default)]
    pub allow_restart: bool,
    /// Signals the child is allowed to send.
    #[config(default = [])]
    #[serde(default = "default_command_permission_signals")]
    pub allowed_signals: Vec<String>,
}

impl Default for CommandPermissions {
    /// Returns the default command permissions: no shutdown, no restart, SIGTERM only.
    fn default() -> Self {
        Self {
            allow_shutdown: false,
            allow_restart: false,
            allowed_signals: default_command_permission_signals(),
        }
    }
}

/// Returns the default allowed signal list for command permissions.
fn default_command_permission_signals() -> Vec<String> {
    vec!["SIGTERM".to_string()]
}

/// Environment variable for a child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct EnvVar {
    /// Environment variable name.
    pub name: String,
    /// Plain-text value (mutually exclusive with secret_ref).
    pub value: Option<String>,
    /// Secret reference in `${SECRET_NAME}` format (mutually exclusive with value).
    pub secret_ref: Option<String>,
}

/// Secret reference for a child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct SecretRef {
    /// Secret name used as an identifier.
    pub name: String,
    /// Key path within the vault.
    pub key: String,
    /// Whether the secret is required (vault offline treated as rejection when true).
    #[config(default = false)]
    #[serde(default)]
    pub required: bool,
}

/// Backoff behavior attached to a child.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BackoffPolicy {
    /// Initial delay before the first restart.
    pub initial_delay: Duration,
    /// Maximum restart delay.
    pub max_delay: Duration,
    /// Jitter ratio between zero and one.
    pub jitter_ratio: f64,
}

impl BackoffPolicy {
    /// Creates a backoff policy.
    ///
    /// # Arguments
    ///
    /// - `initial_delay`: Initial restart delay.
    /// - `max_delay`: Maximum restart delay.
    /// - `jitter_ratio`: Jitter ratio between zero and one.
    ///
    /// # Returns
    ///
    /// Returns a [`BackoffPolicy`] value.
    pub fn new(initial_delay: Duration, max_delay: Duration, jitter_ratio: f64) -> Self {
        Self {
            initial_delay,
            max_delay,
            jitter_ratio,
        }
    }
}

/// Declarative specification for a child task or nested supervisor.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChildSpec {
    /// Stable child identifier.
    pub id: ChildId,
    /// Human-readable child name.
    pub name: String,
    /// Child task kind.
    pub kind: TaskKind,
    /// Runtime isolation strategy (default: AsyncWorker).
    #[serde(default)]
    pub isolation: Isolation,
    /// Optional factory for worker children.
    #[serde(skip)]
    #[schemars(skip)]
    pub factory: Option<Arc<dyn TaskFactory>>,
    /// Optional registry key used to resolve the worker factory before startup.
    #[serde(default)]
    pub factory_key: Option<String>,
    /// Restart policy for this child.
    pub restart_policy: RestartPolicy,
    /// Shutdown policy for this child.
    pub shutdown_policy: ShutdownPolicy,
    /// Health policy for this child.
    pub health_policy: HealthPolicy,
    /// Readiness policy for this child.
    pub readiness_policy: ReadinessPolicy,
    /// Backoff policy for this child.
    pub backoff_policy: BackoffPolicy,
    /// Child identifiers that must become ready before this child starts.
    pub dependencies: Vec<ChildId>,
    /// Low-cardinality tags used for grouping and diagnostics.
    pub tags: Vec<String>,
    /// Criticality used by parent policy decisions.
    pub criticality: Criticality,
    /// Optional role that selects default lifecycle policy semantics.
    #[serde(default)]
    pub task_role: Option<TaskRole>,
    /// Optional sidecar binding used when the role is [`TaskRole::Sidecar`].
    #[serde(default)]
    pub sidecar_config: Option<SidecarConfig>,
    /// Optional explicit severity classification that overrides the role default (US3).
    #[serde(default)]
    pub severity: Option<SeverityClass>,
    /// Optional group name for group-level isolation and budget tracking (US2).
    #[serde(default)]
    pub group: Option<String>,
    /// Optional health check configuration.
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,
    /// Command permissions granted to this child.
    #[serde(default)]
    pub command_permissions: CommandPermissions,
    /// Environment variables for this child.
    #[serde(default)]
    pub environment: Vec<EnvVar>,
    /// Secret references for this child.
    #[serde(default)]
    pub secrets: Vec<SecretRef>,
    /// File system paths (sockets, PID files, temp dirs) to clean up before
    /// every spawn attempt. Prevents "Address already in use" errors when a
    /// previous instance was orphaned by emergency force-kill.
    #[serde(default)]
    pub cleanup_paths: Vec<std::path::PathBuf>,
}

impl Debug for ChildSpec {
    /// Formats the child specification without printing the task factory.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChildSpec")
            .field("isolation", &self.isolation)
            .field("id", &self.id)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("restart_policy", &self.restart_policy)
            .field("factory_key", &self.factory_key)
            .field("shutdown_policy", &self.shutdown_policy)
            .field("health_policy", &self.health_policy)
            .field("readiness_policy", &self.readiness_policy)
            .field("backoff_policy", &self.backoff_policy)
            .field("dependencies", &self.dependencies)
            .field("tags", &self.tags)
            .field("criticality", &self.criticality)
            .field("task_role", &self.task_role)
            .field("sidecar_config", &self.sidecar_config)
            .field("severity", &self.severity)
            .field("group", &self.group)
            .field("health_check", &self.health_check)
            .field("command_permissions", &self.command_permissions)
            .field("environment", &self.environment)
            .field("secrets", &self.secrets)
            .finish()
    }
}

impl ChildSpec {
    /// Creates a worker child specification.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier.
    /// - `name`: Human-readable child name.
    /// - `kind`: Worker task kind.
    /// - `factory`: Task factory used to build each child_start_count.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildSpec`] with conservative policy values.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError`] when local invariants fail validation.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// let factory = rust_supervisor::task::factory::service_fn(|_ctx| async {
    ///     rust_supervisor::task::factory::TaskResult::Succeeded
    /// });
    /// let spec = rust_supervisor::spec::child::ChildSpec::worker(
    ///     rust_supervisor::id::types::ChildId::new("worker"),
    ///     "worker",
    ///     rust_supervisor::spec::child::TaskKind::AsyncWorker,
    ///     std::sync::Arc::new(factory),
    /// )?;
    /// assert_eq!(spec.name, "worker");
    /// # Ok(())
    /// # }
    /// ```
    pub fn worker(
        id: ChildId,
        name: impl Into<String>,
        kind: TaskKind,
        factory: Arc<dyn TaskFactory>,
    ) -> Result<Self, SupervisorError> {
        crate::spec::child_builder::ChildSpecBuilder::worker(id, name, kind, factory).build()
    }

    /// Validates local child specification invariants.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the child can be registered.
    pub fn validate(&self) -> Result<(), SupervisorError> {
        validate_non_empty(&self.id.value, "child id")?;
        validate_non_empty(&self.name, "child name")?;
        validate_tags(&self.tags)?;
        validate_backoff(self.backoff_policy)?;
        validate_factory(self.kind, self.factory.is_some())?;
        validate_sidecar_local(self)
    }
}

/// Validates a non-empty string invariant.
///
/// # Arguments
///
/// - `value`: String value being validated.
/// - `label`: Field label used in the error message.
///
/// # Returns
///
/// Returns `Ok(())` when the string is not empty.
fn validate_non_empty(value: &str, label: &str) -> Result<(), SupervisorError> {
    if value.trim().is_empty() {
        Err(SupervisorError::fatal_config(format!(
            "{label} must not be empty"
        )))
    } else {
        Ok(())
    }
}

/// Validates tag invariants.
///
/// # Arguments
///
/// - `tags`: Tags attached to the child.
///
/// # Returns
///
/// Returns `Ok(())` when every tag is non-empty.
fn validate_tags(tags: &[String]) -> Result<(), SupervisorError> {
    for tag in tags {
        validate_non_empty(tag, "child tag")?;
    }
    Ok(())
}

/// Validates backoff invariants.
///
/// # Arguments
///
/// - `policy`: Backoff policy attached to the child.
///
/// # Returns
///
/// Returns `Ok(())` when delay and jitter values are valid.
fn validate_backoff(policy: BackoffPolicy) -> Result<(), SupervisorError> {
    if policy.initial_delay > policy.max_delay {
        return Err(SupervisorError::fatal_config(
            "initial backoff must not exceed max backoff",
        ));
    }
    if !(0.0..=1.0).contains(&policy.jitter_ratio) {
        return Err(SupervisorError::fatal_config(
            "jitter ratio must be between zero and one",
        ));
    }
    Ok(())
}

/// Validates factory presence for the child kind.
///
/// # Arguments
///
/// - `kind`: Child task kind.
/// - `has_factory`: Whether a factory was supplied.
///
/// # Returns
///
/// Returns `Ok(())` when factory presence matches the task kind.
fn validate_factory(kind: TaskKind, has_factory: bool) -> Result<(), SupervisorError> {
    match (kind, has_factory) {
        (TaskKind::Supervisor, true) => Err(SupervisorError::fatal_config(
            "supervisor child must not own a task factory",
        )),
        (TaskKind::AsyncWorker | TaskKind::BlockingWorker, false) => Err(
            SupervisorError::fatal_config("worker child requires a task factory"),
        ),
        _ => Ok(()),
    }
}

/// Validates local sidecar fields without inspecting sibling children.
///
/// # Arguments
///
/// - `child`: Child specification to validate.
///
/// # Returns
///
/// Returns `Ok(())` when the local sidecar declaration is coherent.
fn validate_sidecar_local(child: &ChildSpec) -> Result<(), SupervisorError> {
    match (child.task_role, child.sidecar_config.as_ref()) {
        (Some(TaskRole::Sidecar), None) => Err(SupervisorError::fatal_config(
            "sidecar task_role requires sidecar_config",
        )),
        (role, Some(_)) if role != Some(TaskRole::Sidecar) => Err(SupervisorError::fatal_config(
            "sidecar_config requires sidecar task_role",
        )),
        _ => Ok(()),
    }
}
