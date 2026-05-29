//! Builder for [`ChildSpec`](crate::spec::child::ChildSpec).
//!
//! Use this module when constructing child specifications in code. YAML and
//! RPC inputs should still flow through [`ChildDeclaration`](crate::spec::child_declaration::ChildDeclaration)
//! and `TryFrom` conversion.

use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::policy::task_role_defaults::{SeverityClass, SidecarConfig, TaskRole};
use crate::readiness::signal::ReadinessPolicy;
use crate::spec::child::{
    BackoffPolicy, ChildSpec, CommandPermissions, Criticality, EnvVar, HealthCheckConfig,
    HealthPolicy, Isolation, RestartPolicy, SecretRef, ShutdownPolicy, TaskKind,
};
use crate::task::factory::TaskFactory;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Default policy values shared by worker child construction.
struct WorkerPolicyDefaults {
    /// Restart policy for worker children.
    restart_policy: RestartPolicy,
    /// Shutdown policy for worker children.
    shutdown_policy: ShutdownPolicy,
    /// Health policy for worker children.
    health_policy: HealthPolicy,
    /// Readiness policy for worker children.
    readiness_policy: ReadinessPolicy,
    /// Backoff policy for worker children.
    backoff_policy: BackoffPolicy,
}

/// Returns the default policy bundle used by [`ChildSpecBuilder::worker`].
fn worker_policy_defaults() -> WorkerPolicyDefaults {
    WorkerPolicyDefaults {
        restart_policy: RestartPolicy::Transient,
        shutdown_policy: ShutdownPolicy::new(Duration::from_secs(5), Duration::from_secs(1)),
        health_policy: HealthPolicy::new(Duration::from_secs(1), Duration::from_secs(3)),
        readiness_policy: ReadinessPolicy::Immediate,
        backoff_policy: BackoffPolicy::new(Duration::from_millis(10), Duration::from_secs(1), 0.0),
    }
}

/// Returns baseline policy values for minimal or supervisor child construction.
fn baseline_policy_defaults() -> WorkerPolicyDefaults {
    WorkerPolicyDefaults {
        restart_policy: RestartPolicy::Permanent,
        shutdown_policy: ShutdownPolicy::new(Duration::from_secs(5), Duration::from_secs(1)),
        health_policy: HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(5)),
        readiness_policy: ReadinessPolicy::Immediate,
        backoff_policy: BackoffPolicy::new(Duration::from_millis(10), Duration::from_secs(1), 0.0),
    }
}

/// Applies a policy bundle to a child specification.
fn apply_policy_defaults(spec: &mut ChildSpec, defaults: WorkerPolicyDefaults) {
    spec.restart_policy = defaults.restart_policy;
    spec.shutdown_policy = defaults.shutdown_policy;
    spec.health_policy = defaults.health_policy;
    spec.readiness_policy = defaults.readiness_policy;
    spec.backoff_policy = defaults.backoff_policy;
}

/// Builder for [`ChildSpec`](crate::spec::child::ChildSpec).
#[derive(Debug, Clone)]
pub struct ChildSpecBuilder {
    /// Child specification under construction.
    spec: ChildSpec,
}

impl ChildSpecBuilder {
    /// Creates a minimal child specification builder.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier.
    /// - `name`: Human-readable child name.
    ///
    /// # Returns
    ///
    /// Returns a builder with baseline policy defaults. Callers must set `kind`
    /// and, for worker children, `factory` before validation.
    pub fn new(id: ChildId, name: impl Into<String>) -> Self {
        let mut spec = ChildSpec {
            id,
            name: name.into(),
            kind: TaskKind::default(),
            isolation: Isolation::default(),
            factory: None,
            restart_policy: RestartPolicy::default(),
            shutdown_policy: ShutdownPolicy::new(Duration::from_secs(5), Duration::from_secs(1)),
            health_policy: HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(5)),
            readiness_policy: ReadinessPolicy::Immediate,
            backoff_policy: BackoffPolicy::new(
                Duration::from_millis(10),
                Duration::from_secs(1),
                0.0,
            ),
            dependencies: Vec::new(),
            tags: Vec::new(),
            criticality: Criticality::default(),
            task_role: None,
            sidecar_config: None,
            severity: None,
            group: None,
            health_check: None,
            command_permissions: CommandPermissions::default(),
            environment: Vec::new(),
            secrets: Vec::new(),
            cleanup_paths: Vec::new(),
        };
        apply_policy_defaults(&mut spec, baseline_policy_defaults());
        Self { spec }
    }

    /// Creates a worker child specification builder.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier.
    /// - `name`: Human-readable child name.
    /// - `kind`: Worker task kind.
    /// - `factory`: Task factory used to build each child attempt.
    ///
    /// # Returns
    ///
    /// Returns a builder seeded with the same defaults as [`ChildSpec::worker`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::ChildId;
    /// use rust_supervisor::policy::task_role_defaults::TaskRole;
    /// use rust_supervisor::spec::child::TaskKind;
    /// use rust_supervisor::spec::child_builder::ChildSpecBuilder;
    /// use rust_supervisor::task::factory::{TaskResult, service_fn};
    /// use std::sync::Arc;
    ///
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    /// let spec = ChildSpecBuilder::worker(
    ///     ChildId::new("worker"),
    ///     "worker",
    ///     TaskKind::AsyncWorker,
    ///     Arc::new(factory),
    /// )
    /// .task_role(TaskRole::Worker)
    /// .tag("invoice")
    /// .build()?;
    /// assert_eq!(spec.name, "worker");
    /// # Ok(())
    /// # }
    /// ```
    pub fn worker(
        id: ChildId,
        name: impl Into<String>,
        kind: TaskKind,
        factory: Arc<dyn TaskFactory>,
    ) -> Self {
        let defaults = worker_policy_defaults();
        let spec = ChildSpec {
            id,
            name: name.into(),
            kind,
            isolation: Isolation::AsyncWorker,
            factory: Some(factory),
            restart_policy: defaults.restart_policy,
            shutdown_policy: defaults.shutdown_policy,
            health_policy: defaults.health_policy,
            readiness_policy: defaults.readiness_policy,
            backoff_policy: defaults.backoff_policy,
            dependencies: Vec::new(),
            tags: Vec::new(),
            criticality: Criticality::Critical,
            task_role: Some(TaskRole::Worker),
            sidecar_config: None,
            severity: None,
            group: None,
            health_check: None,
            command_permissions: CommandPermissions::default(),
            environment: Vec::new(),
            secrets: Vec::new(),
            cleanup_paths: Vec::new(),
        };
        Self { spec }
    }

    /// Creates a nested supervisor child specification builder.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier.
    /// - `name`: Human-readable child name.
    ///
    /// # Returns
    ///
    /// Returns a builder with supervisor kind, no factory, and critical role.
    pub fn supervisor(id: ChildId, name: impl Into<String>) -> Self {
        let defaults = baseline_policy_defaults();
        let spec = ChildSpec {
            id,
            name: name.into(),
            kind: TaskKind::Supervisor,
            isolation: Isolation::default(),
            factory: None,
            restart_policy: defaults.restart_policy,
            shutdown_policy: defaults.shutdown_policy,
            health_policy: defaults.health_policy,
            readiness_policy: defaults.readiness_policy,
            backoff_policy: defaults.backoff_policy,
            dependencies: Vec::new(),
            tags: Vec::new(),
            criticality: Criticality::Critical,
            task_role: Some(TaskRole::Supervisor),
            sidecar_config: None,
            severity: None,
            group: None,
            health_check: None,
            command_permissions: CommandPermissions::default(),
            environment: Vec::new(),
            secrets: Vec::new(),
            cleanup_paths: Vec::new(),
        };
        Self { spec }
    }

    /// Sets the child task kind.
    ///
    /// # Arguments
    ///
    /// - `kind`: Child task kind.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn kind(mut self, kind: TaskKind) -> Self {
        self.spec.kind = kind;
        self
    }

    /// Sets the runtime isolation strategy.
    ///
    /// # Arguments
    ///
    /// - `isolation`: Runtime isolation strategy.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn isolation(mut self, isolation: Isolation) -> Self {
        self.spec.isolation = isolation;
        self
    }

    /// Sets the task factory for worker children.
    ///
    /// # Arguments
    ///
    /// - `factory`: Task factory used to build each child attempt.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn factory(mut self, factory: Arc<dyn TaskFactory>) -> Self {
        self.spec.factory = Some(factory);
        self
    }

    /// Clears the task factory.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn without_factory(mut self) -> Self {
        self.spec.factory = None;
        self
    }

    /// Sets the restart policy.
    ///
    /// # Arguments
    ///
    /// - `restart_policy`: Restart policy for this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn restart_policy(mut self, restart_policy: RestartPolicy) -> Self {
        self.spec.restart_policy = restart_policy;
        self
    }

    /// Sets the shutdown policy.
    ///
    /// # Arguments
    ///
    /// - `shutdown_policy`: Shutdown policy for this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn shutdown_policy(mut self, shutdown_policy: ShutdownPolicy) -> Self {
        self.spec.shutdown_policy = shutdown_policy;
        self
    }

    /// Sets the health policy.
    ///
    /// # Arguments
    ///
    /// - `health_policy`: Health policy for this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn health_policy(mut self, health_policy: HealthPolicy) -> Self {
        self.spec.health_policy = health_policy;
        self
    }

    /// Sets the readiness policy.
    ///
    /// # Arguments
    ///
    /// - `readiness_policy`: Readiness policy for this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn readiness_policy(mut self, readiness_policy: ReadinessPolicy) -> Self {
        self.spec.readiness_policy = readiness_policy;
        self
    }

    /// Sets the backoff policy.
    ///
    /// # Arguments
    ///
    /// - `backoff_policy`: Backoff policy for this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn backoff_policy(mut self, backoff_policy: BackoffPolicy) -> Self {
        self.spec.backoff_policy = backoff_policy;
        self
    }

    /// Replaces child dependencies.
    ///
    /// # Arguments
    ///
    /// - `dependencies`: Child identifiers that must become ready first.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn dependencies(mut self, dependencies: Vec<ChildId>) -> Self {
        self.spec.dependencies = dependencies;
        self
    }

    /// Appends one child dependency.
    ///
    /// # Arguments
    ///
    /// - `dependency`: Child identifier that must become ready first.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn dependency(mut self, dependency: ChildId) -> Self {
        self.spec.dependencies.push(dependency);
        self
    }

    /// Replaces diagnostic tags.
    ///
    /// # Arguments
    ///
    /// - `tags`: Low-cardinality tags for grouping and diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.spec.tags = tags;
        self
    }

    /// Appends one diagnostic tag.
    ///
    /// # Arguments
    ///
    /// - `tag`: Low-cardinality tag for grouping and diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.spec.tags.push(tag.into());
        self
    }

    /// Sets child criticality.
    ///
    /// # Arguments
    ///
    /// - `criticality`: Criticality used by parent policy decisions.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn criticality(mut self, criticality: Criticality) -> Self {
        self.spec.criticality = criticality;
        self
    }

    /// Sets the task role.
    ///
    /// # Arguments
    ///
    /// - `task_role`: Role that selects default lifecycle policy semantics.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn task_role(mut self, task_role: TaskRole) -> Self {
        self.spec.task_role = Some(task_role);
        self
    }

    /// Clears the task role.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn without_task_role(mut self) -> Self {
        self.spec.task_role = None;
        self
    }

    /// Sets the sidecar binding configuration.
    ///
    /// # Arguments
    ///
    /// - `sidecar_config`: Sidecar binding used when the role is sidecar.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn sidecar_config(mut self, sidecar_config: SidecarConfig) -> Self {
        self.spec.sidecar_config = Some(sidecar_config);
        self
    }

    /// Clears the sidecar binding configuration.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn without_sidecar_config(mut self) -> Self {
        self.spec.sidecar_config = None;
        self
    }

    /// Sets the severity classification override.
    ///
    /// # Arguments
    ///
    /// - `severity`: Explicit severity classification.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn severity(mut self, severity: SeverityClass) -> Self {
        self.spec.severity = Some(severity);
        self
    }

    /// Clears the severity classification override.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn without_severity(mut self) -> Self {
        self.spec.severity = None;
        self
    }

    /// Sets the group name.
    ///
    /// # Arguments
    ///
    /// - `group`: Group name for group-level isolation and budget tracking.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.spec.group = Some(group.into());
        self
    }

    /// Clears the group name.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn without_group(mut self) -> Self {
        self.spec.group = None;
        self
    }

    /// Sets the optional health check configuration block.
    ///
    /// # Arguments
    ///
    /// - `health_check`: Health check configuration.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn health_check(mut self, health_check: HealthCheckConfig) -> Self {
        self.spec.health_check = Some(health_check);
        self
    }

    /// Clears the optional health check configuration block.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn without_health_check(mut self) -> Self {
        self.spec.health_check = None;
        self
    }

    /// Sets command permissions.
    ///
    /// # Arguments
    ///
    /// - `command_permissions`: Command permissions granted to this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn command_permissions(mut self, command_permissions: CommandPermissions) -> Self {
        self.spec.command_permissions = command_permissions;
        self
    }

    /// Replaces environment variables.
    ///
    /// # Arguments
    ///
    /// - `environment`: Environment variables for this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn environment(mut self, environment: Vec<EnvVar>) -> Self {
        self.spec.environment = environment;
        self
    }

    /// Appends one environment variable.
    ///
    /// # Arguments
    ///
    /// - `env_var`: Environment variable for this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn env_var(mut self, env_var: EnvVar) -> Self {
        self.spec.environment.push(env_var);
        self
    }

    /// Replaces secret references.
    ///
    /// # Arguments
    ///
    /// - `secrets`: Secret references for this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn secrets(mut self, secrets: Vec<SecretRef>) -> Self {
        self.spec.secrets = secrets;
        self
    }

    /// Appends one secret reference.
    ///
    /// # Arguments
    ///
    /// - `secret`: Secret reference for this child.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn secret(mut self, secret: SecretRef) -> Self {
        self.spec.secrets.push(secret);
        self
    }

    /// Replaces cleanup paths.
    ///
    /// # Arguments
    ///
    /// - `cleanup_paths`: Paths to clean up before every spawn attempt.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn cleanup_paths(mut self, cleanup_paths: Vec<PathBuf>) -> Self {
        self.spec.cleanup_paths = cleanup_paths;
        self
    }

    /// Appends one cleanup path.
    ///
    /// # Arguments
    ///
    /// - `cleanup_path`: Path to clean up before every spawn attempt.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn cleanup_path(mut self, cleanup_path: impl Into<PathBuf>) -> Self {
        self.spec.cleanup_paths.push(cleanup_path.into());
        self
    }

    /// Builds and validates the child specification.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the constructed [`ChildSpec`] when local invariants pass.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError`] when validation fails.
    pub fn build(self) -> Result<ChildSpec, SupervisorError> {
        let spec = self.spec;
        spec.validate()?;
        Ok(spec)
    }
}
