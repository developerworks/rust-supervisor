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

/// Default policy values shared by child construction.
struct PolicyDefaults {
    /// Restart policy for child construction.
    restart_policy: RestartPolicy,
    /// Shutdown policy for child construction.
    shutdown_policy: ShutdownPolicy,
    /// Health policy for child construction.
    health_policy: HealthPolicy,
    /// Readiness policy for child construction.
    readiness_policy: ReadinessPolicy,
    /// Backoff policy for child construction.
    backoff_policy: BackoffPolicy,
}

/// Returns the default policy bundle used by [`ChildSpecBuilder::worker`].
fn worker_policy_defaults() -> PolicyDefaults {
    PolicyDefaults {
        restart_policy: RestartPolicy::Transient,
        shutdown_policy: ShutdownPolicy::new(Duration::from_secs(5), Duration::from_secs(1)),
        health_policy: HealthPolicy::new(Duration::from_secs(1), Duration::from_secs(3)),
        readiness_policy: ReadinessPolicy::Immediate,
        backoff_policy: BackoffPolicy::new(Duration::from_millis(10), Duration::from_secs(1), 0.0),
    }
}

/// Returns baseline policy values for minimal or supervisor child construction.
fn baseline_policy_defaults() -> PolicyDefaults {
    PolicyDefaults {
        restart_policy: RestartPolicy::Permanent,
        shutdown_policy: ShutdownPolicy::new(Duration::from_secs(5), Duration::from_secs(1)),
        health_policy: HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(5)),
        readiness_policy: ReadinessPolicy::Immediate,
        backoff_policy: BackoffPolicy::new(Duration::from_millis(10), Duration::from_secs(1), 0.0),
    }
}

/// Applies a policy bundle to a child specification.
fn apply_policy_defaults(spec: &mut ChildSpec, defaults: PolicyDefaults) {
    spec.restart_policy = defaults.restart_policy;
    spec.shutdown_policy = defaults.shutdown_policy;
    spec.health_policy = defaults.health_policy;
    spec.readiness_policy = defaults.readiness_policy;
    spec.backoff_policy = defaults.backoff_policy;
}

/// Builder for [`ChildSpec`](crate::spec::child::ChildSpec).
///
/// Public constructors and setters keep returning [`ChildSpecBuilder`] for
/// chaining. Call [`build`](ChildSpecBuilder::build) to consume the builder,
/// validate local invariants, and receive the final [`ChildSpec`].
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
        let spec = ChildSpec {
            id,
            name: name.into(),
            kind: TaskKind::default(),
            isolation: Isolation::default(),
            factory: None,
            factory_key: None,
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
        Self { spec }.with_policy_defaults(baseline_policy_defaults())
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
        Self::new(id, name)
            .kind(kind)
            .isolation(Isolation::AsyncWorker)
            .factory(factory)
            .criticality(Criticality::Critical)
            .task_role(TaskRole::Worker)
            .with_policy_defaults(worker_policy_defaults())
    }

    /// Creates a service role child specification builder.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier.
    /// - `name`: Human-readable child name.
    /// - `kind`: Worker task kind used to run the service body.
    /// - `factory`: Task factory used to build each child attempt.
    ///
    /// # Returns
    ///
    /// Returns a builder with worker execution defaults and service role
    /// classification.
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
    /// let spec = ChildSpecBuilder::service(
    ///     ChildId::new("api-service"),
    ///     "API Service",
    ///     TaskKind::AsyncWorker,
    ///     Arc::new(factory),
    /// )
    /// .build()?;
    /// assert_eq!(spec.task_role, Some(TaskRole::Service));
    /// # Ok(())
    /// # }
    /// ```
    pub fn service(
        id: ChildId,
        name: impl Into<String>,
        kind: TaskKind,
        factory: Arc<dyn TaskFactory>,
    ) -> Self {
        Self::worker(id, name, kind, factory)
            .task_role(TaskRole::Service)
            .criticality(Criticality::Critical)
    }

    /// Creates a job role child specification builder.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier.
    /// - `name`: Human-readable child name.
    /// - `kind`: Worker task kind used to run the job body.
    /// - `factory`: Task factory used to build each child attempt.
    ///
    /// # Returns
    ///
    /// Returns a builder with worker execution defaults, job role
    /// classification, and optional criticality.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::ChildId;
    /// use rust_supervisor::policy::task_role_defaults::TaskRole;
    /// use rust_supervisor::spec::child::{Criticality, TaskKind};
    /// use rust_supervisor::spec::child_builder::ChildSpecBuilder;
    /// use rust_supervisor::task::factory::{TaskResult, service_fn};
    /// use std::sync::Arc;
    ///
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    /// let spec = ChildSpecBuilder::job(
    ///     ChildId::new("daily-report"),
    ///     "Daily Report",
    ///     TaskKind::AsyncWorker,
    ///     Arc::new(factory),
    /// )
    /// .build()?;
    /// assert_eq!(spec.task_role, Some(TaskRole::Job));
    /// assert_eq!(spec.criticality, Criticality::Optional);
    /// # Ok(())
    /// # }
    /// ```
    pub fn job(
        id: ChildId,
        name: impl Into<String>,
        kind: TaskKind,
        factory: Arc<dyn TaskFactory>,
    ) -> Self {
        Self::worker(id, name, kind, factory)
            .task_role(TaskRole::Job)
            .criticality(Criticality::Optional)
    }

    /// Creates a sidecar role child specification builder.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier.
    /// - `name`: Human-readable child name.
    /// - `kind`: Worker task kind used to run the sidecar body.
    /// - `factory`: Task factory used to build each child attempt.
    /// - `sidecar_config`: Binding that points at the primary child.
    ///
    /// # Returns
    ///
    /// Returns a builder with worker execution defaults, sidecar role
    /// classification, sidecar binding, and a primary child dependency.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::ChildId;
    /// use rust_supervisor::policy::task_role_defaults::{SidecarConfig, TaskRole};
    /// use rust_supervisor::spec::child::TaskKind;
    /// use rust_supervisor::spec::child_builder::ChildSpecBuilder;
    /// use rust_supervisor::task::factory::{TaskResult, service_fn};
    /// use std::sync::Arc;
    ///
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    /// let primary = ChildId::new("api-service");
    /// let spec = ChildSpecBuilder::sidecar(
    ///     ChildId::new("metrics-sidecar"),
    ///     "Metrics Sidecar",
    ///     TaskKind::AsyncWorker,
    ///     Arc::new(factory),
    ///     SidecarConfig::new(primary.clone(), true),
    /// )
    /// .build()?;
    /// assert_eq!(spec.task_role, Some(TaskRole::Sidecar));
    /// assert_eq!(spec.dependencies, vec![primary]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn sidecar(
        id: ChildId,
        name: impl Into<String>,
        kind: TaskKind,
        factory: Arc<dyn TaskFactory>,
        sidecar_config: SidecarConfig,
    ) -> Self {
        let primary_child_id = sidecar_config.primary_child_id.clone();
        Self::worker(id, name, kind, factory)
            .task_role(TaskRole::Sidecar)
            .sidecar_config(sidecar_config)
            .dependency(primary_child_id)
            .criticality(Criticality::Critical)
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
        Self::new(id, name)
            .kind(TaskKind::Supervisor)
            .without_factory()
            .criticality(Criticality::Critical)
            .task_role(TaskRole::Supervisor)
    }

    /// Applies policy defaults to the inner child specification.
    ///
    /// # Arguments
    ///
    /// - `defaults`: Policy defaults applied to this builder.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    fn with_policy_defaults(mut self, defaults: PolicyDefaults) -> Self {
        apply_policy_defaults(&mut self.spec, defaults);
        self
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

    /// Sets the task factory registry key for declarative worker children.
    ///
    /// # Arguments
    ///
    /// - `factory_key`: Registry key used to resolve a task factory before startup.
    ///
    /// # Returns
    ///
    /// Returns the builder for chaining.
    pub fn factory_key(mut self, factory_key: impl Into<String>) -> Self {
        self.spec.factory_key = Some(factory_key.into());
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
    /// Public constructors and setters return [`ChildSpecBuilder`]. This method
    /// is the public exit that consumes the builder and returns the final
    /// [`ChildSpec`].
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
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::ChildId;
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
    /// .build()?;
    /// assert_eq!(spec.name, "worker");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// use rust_supervisor::id::types::ChildId;
    /// use rust_supervisor::spec::child::ChildSpec;
    /// use rust_supervisor::spec::child_builder::ChildSpecBuilder;
    ///
    /// let builder = ChildSpecBuilder::new(ChildId::new("worker"), "worker");
    /// let _spec: ChildSpec = builder;
    /// ```
    pub fn build(self) -> Result<ChildSpec, SupervisorError> {
        let spec = self.spec;
        spec.validate()?;
        Ok(spec)
    }
}
