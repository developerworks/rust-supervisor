//! Child declaration model.
//!
//! This module owns declarative child specifications and validates local child
//! invariants before the runtime registers or starts work.

use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::readiness::signal::ReadinessPolicy;
use crate::task::factory::TaskFactory;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;

/// Kind of task represented by a child declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    /// Asynchronous worker that can be cancelled through its context.
    AsyncWorker,
    /// Blocking worker with explicit shutdown and escalation boundaries.
    BlockingWorker,
    /// Nested supervisor node.
    Supervisor,
}

/// Importance of a child to its parent supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Criticality {
    /// The child is required for the supervisor to remain healthy.
    Critical,
    /// The child can fail without forcing parent shutdown.
    Optional,
}

/// Restart behavior attached to a child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Restart regardless of the exit result.
    Permanent,
    /// Restart only when the task failed.
    Transient,
    /// Do not restart after any exit.
    Temporary,
}

/// Shutdown behavior attached to a child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Backoff behavior attached to a child.
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Clone)]
pub struct ChildSpec {
    /// Stable child identifier.
    pub id: ChildId,
    /// Human-readable child name.
    pub name: String,
    /// Child task kind.
    pub kind: TaskKind,
    /// Optional factory for worker children.
    pub factory: Option<Arc<dyn TaskFactory>>,
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
}

impl Debug for ChildSpec {
    /// Formats the child specification without printing the task factory.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChildSpec")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("restart_policy", &self.restart_policy)
            .field("shutdown_policy", &self.shutdown_policy)
            .field("health_policy", &self.health_policy)
            .field("readiness_policy", &self.readiness_policy)
            .field("backoff_policy", &self.backoff_policy)
            .field("dependencies", &self.dependencies)
            .field("tags", &self.tags)
            .field("criticality", &self.criticality)
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
    /// # Examples
    ///
    /// ```
    /// let factory = rust_supervisor::task::factory::service_fn(|_ctx| async {
    ///     rust_supervisor::task::factory::TaskResult::Succeeded
    /// });
    /// let spec = rust_supervisor::spec::child::ChildSpec::worker(
    ///     rust_supervisor::id::types::ChildId::new("worker"),
    ///     "worker",
    ///     rust_supervisor::spec::child::TaskKind::AsyncWorker,
    ///     std::sync::Arc::new(factory),
    /// );
    /// assert_eq!(spec.name, "worker");
    /// ```
    pub fn worker(
        id: ChildId,
        name: impl Into<String>,
        kind: TaskKind,
        factory: Arc<dyn TaskFactory>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            factory: Some(factory),
            restart_policy: RestartPolicy::Transient,
            shutdown_policy: ShutdownPolicy::new(Duration::from_secs(5), Duration::from_secs(1)),
            health_policy: HealthPolicy::new(Duration::from_secs(1), Duration::from_secs(3)),
            readiness_policy: ReadinessPolicy::Immediate,
            backoff_policy: BackoffPolicy::new(
                Duration::from_millis(10),
                Duration::from_secs(1),
                0.0,
            ),
            dependencies: Vec::new(),
            tags: Vec::new(),
            criticality: Criticality::Critical,
        }
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
        validate_factory(self.kind, self.factory.is_some())
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
