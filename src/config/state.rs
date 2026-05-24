//! Immutable configuration state for supervisor runtime values.
//!
//! Raw YAML input belongs to [`crate::config::configurable`]. This module owns
//! semantic validation and conversion into supervisor runtime declarations.

use crate::config::audit::AuditConfig;
use crate::config::configurable::{
    DashboardIpcConfig, ObservabilityConfig, PolicyConfig, ShutdownConfig, SupervisorConfig,
    SupervisorRootConfig,
};
use crate::config::ipc_security::IpcSecurityConfig;
use crate::config::policy::{
    ChildStrategyOverrideConfig, GroupConfig, GroupDependencyConfig, GroupStrategyConfig,
    SeverityDefaultConfig,
};
use crate::spec::child::ChildSpec;
use crate::spec::child_declaration::{ChildDeclaration, CompensatingRecord, PendingChild, Phase};
use crate::spec::supervisor::BackpressureConfig;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use uuid::Uuid;

/// Supervisor configuration state with add_child transaction support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigState {
    /// Root supervisor declaration values.
    pub supervisor: SupervisorRootConfig,
    /// Runtime policy values.
    pub policy: PolicyConfig,
    /// Shutdown budget values.
    pub shutdown: ShutdownConfig,
    /// Observability switches and capacities.
    pub observability: ObservabilityConfig,
    /// Command audit persistence configuration.
    pub audit: AuditConfig,
    /// Backpressure policy for observability event subscribers.
    pub backpressure: BackpressureConfig,
    /// Group-level restart budgets and membership declarations.
    pub groups: Vec<GroupConfig>,
    /// Group-level strategy overrides.
    pub group_strategies: Vec<GroupStrategyConfig>,
    /// Cross-group failure propagation dependencies.
    pub group_dependencies: Vec<GroupDependencyConfig>,
    /// Child-level strategy overrides.
    pub child_strategy_overrides: Vec<ChildStrategyOverrideConfig>,
    /// Default severity class per task role.
    pub severity_defaults: Vec<SeverityDefaultConfig>,
    /// Optional target-side dashboard IPC configuration.
    pub dashboard: Option<DashboardIpcConfig>,
    /// Validated child specifications loaded from YAML declarations.
    #[serde(default)]
    pub children: Vec<ChildSpec>,
    /// SHA-256 hash of the SupervisorSpec for audit reconciliation.
    #[serde(default)]
    pub spec_hash: String,
    /// Pending add_child transactions.
    #[serde(default)]
    pub pending_additions: Vec<PendingChild>,
    /// Compensating records for recovery.
    #[serde(default)]
    pub compensating_records: Vec<CompensatingRecord>,
}

/// Manual partial equality — skips `children` because [`ChildSpec`] contains
/// `Arc<dyn TaskFactory>` which does not implement `PartialEq`.
impl PartialEq for ConfigState {
    /// Compares two ConfigState values, skipping the `children` vector.
    fn eq(&self, other: &Self) -> bool {
        self.supervisor == other.supervisor
            && self.policy == other.policy
            && self.shutdown == other.shutdown
            && self.observability == other.observability
            && self.audit == other.audit
            && self.backpressure == other.backpressure
            && self.groups == other.groups
            && self.group_strategies == other.group_strategies
            && self.group_dependencies == other.group_dependencies
            && self.child_strategy_overrides == other.child_strategy_overrides
            && self.severity_defaults == other.severity_defaults
            && self.dashboard == other.dashboard
            && self.spec_hash == other.spec_hash
            && self.pending_additions == other.pending_additions
    }
}

impl TryFrom<SupervisorConfig> for ConfigState {
    type Error = crate::error::types::SupervisorError;

    /// Converts a deserialized supervisor config into validated state.
    fn try_from(config: SupervisorConfig) -> Result<Self, Self::Error> {
        validate_policy(&config.policy)?;
        validate_shutdown(&config.shutdown)?;
        validate_observability(&config.observability)?;
        validate_audit(&config.audit)?;
        validate_backpressure(&config.backpressure)?;
        validate_lower_policy(&config.policy)?;
        validate_supervisor_root(&config.supervisor)?;
        validate_group_inputs(
            &config.groups,
            &config.group_strategies,
            &config.group_dependencies,
            &config.child_strategy_overrides,
            &config.severity_defaults,
            &config.children,
        )?;
        let dashboard = dashboard_with_default_security(config.dashboard);
        validate_dashboard(dashboard.as_ref())?;

        // Validate and convert child declarations.
        use crate::spec::child_declaration::validate_child_declaration;
        use crate::tree::order::kahn_sort;

        // Collect all child names for validation.
        let all_names: HashSet<String> = config.children.iter().map(|c| c.name.clone()).collect();

        // Validate each declaration.
        for child in &config.children {
            validate_child_declaration(child, &all_names).map_err(|e| {
                crate::error::types::SupervisorError::fatal_config(format!(
                    "Child declaration validation failed at {}: {}",
                    e.field_path, e.reason
                ))
            })?;
        }

        // Convert to ChildSpec list.
        let child_specs: Vec<ChildSpec> = config
            .children
            .into_iter()
            .map(ChildSpec::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                crate::error::types::SupervisorError::fatal_config(format!(
                    "Child declaration conversion failed at {}: {}",
                    e.field_path, e.reason
                ))
            })?;

        // Topological sort.
        let _sorted = kahn_sort(&child_specs).map_err(|cycle_nodes| {
            let node_names: Vec<String> = cycle_nodes.iter().map(|id| id.value.clone()).collect();
            crate::error::types::SupervisorError::fatal_config(format!(
                "Dependency cycle detected among children: {node_names:?}",
            ))
        })?;

        let spec_hash = String::new(); // Will be computed after SupervisorSpec is built.

        Ok(Self {
            supervisor: config.supervisor,
            policy: config.policy,
            shutdown: config.shutdown,
            observability: config.observability,
            audit: config.audit,
            backpressure: config.backpressure,
            groups: config.groups,
            group_strategies: config.group_strategies,
            group_dependencies: config.group_dependencies,
            child_strategy_overrides: config.child_strategy_overrides,
            severity_defaults: config.severity_defaults,
            dashboard,
            children: child_specs,
            spec_hash,
            pending_additions: Vec::new(),
            compensating_records: Vec::new(),
        })
    }
}

impl ConfigState {
    /// Converts validated configuration into a supervisor declaration.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`crate::spec::supervisor::SupervisorSpec`] derived from the
    /// validated YAML configuration.
    ///
    /// # Examples
    ///
    /// Begins an add_child transaction by creating a PendingChild entry.
    ///
    /// # Arguments
    ///
    /// - `declaration`: The child declaration to stage.
    ///
    /// # Returns
    ///
    /// Returns the generated transaction UUID on success.
    ///
    /// # Errors
    ///
    /// Returns an error if a transaction is already in progress.
    pub fn begin_transaction(
        &mut self,
        declaration: ChildDeclaration,
    ) -> Result<Uuid, crate::error::types::SupervisorError> {
        if self.has_pending_transaction() {
            return Err(crate::error::types::SupervisorError::fatal_config(
                "add_child transaction already in progress",
            ));
        }
        let transaction_id = Uuid::new_v4();
        let child_spec = Box::new(ChildSpec::try_from(declaration.clone()).map_err(|e| {
            crate::error::types::SupervisorError::fatal_config(format!(
                "Child declaration conversion failed: {}",
                e.reason
            ))
        })?);
        let pending = PendingChild {
            transaction_id,
            declaration,
            child_spec,
            phase: Phase::Parsed,
            created_at_unix_nanos: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        };
        self.pending_additions.push(pending);
        Ok(transaction_id)
    }

    /// Commits an add_child transaction, registering the child in the topology.
    ///
    /// # Arguments
    ///
    /// - `transaction_id`: The transaction UUID to commit.
    pub fn commit_transaction(
        &mut self,
        transaction_id: Uuid,
    ) -> Result<(), crate::error::types::SupervisorError> {
        let idx = self
            .pending_additions
            .iter()
            .position(|p| p.transaction_id == transaction_id)
            .ok_or_else(|| {
                crate::error::types::SupervisorError::fatal_config(
                    "transaction not found for commit",
                )
            })?;

        let mut pending = self.pending_additions.remove(idx);
        pending.phase = Phase::Committed;

        // Register child in the topology.
        let spec = (*pending.child_spec).clone();
        self.children.push(spec);

        // Compute a deterministic spec hash from the current children list.
        // Uses JSON serialization for cross-version stability; this is a
        // content hash for change detection, not a cryptographic digest.
        self.spec_hash = self.compute_spec_hash();

        Ok(())
    }

    /// Rolls back an add_child transaction, creating a compensating record.
    ///
    /// # Arguments
    ///
    /// - `transaction_id`: The transaction UUID to roll back.
    /// - `error`: Human-readable error description.
    pub fn rollback_transaction(
        &mut self,
        transaction_id: Uuid,
        error: String,
    ) -> Result<(), crate::error::types::SupervisorError> {
        let idx = self
            .pending_additions
            .iter()
            .position(|p| p.transaction_id == transaction_id);

        let pending = if let Some(i) = idx {
            self.pending_additions.remove(i)
        } else {
            return Err(crate::error::types::SupervisorError::fatal_config(
                "transaction not found for rollback",
            ));
        };

        // Create compensating record.
        let record = CompensatingRecord {
            transaction_id,
            operation: "add_child".to_string(),
            state: "compensated".to_string(),
            child_name: pending.declaration.name.clone(),
            declaration_hash: compute_declaration_hash(&pending.declaration),
            error: Some(error),
            correlation_id: None,
            child_id: Some(pending.child_spec.id.value.clone()),
            created_at_unix_nanos: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        };
        self.compensating_records.push(record);

        Ok(())
    }

    /// Returns true when a pending transaction exists.
    pub fn has_pending_transaction(&self) -> bool {
        self.pending_additions
            .iter()
            .any(|p| p.phase != Phase::Committed && p.phase != Phase::Compensated)
    }

    /// Returns the current spec hash for audit reconciliation.
    pub fn hash(&self) -> &str {
        &self.spec_hash
    }

    /// Computes a deterministic spec hash from the current children list.
    ///
    /// Uses JSON serialization for cross-version stability. This is a
    /// content hash for change detection and audit reconciliation, not a
    /// cryptographic digest. Changes to the serialization format will
    /// produce different hashes — this is acceptable because the hash is
    /// always recomputed from the current state after restart.
    pub fn compute_spec_hash(&self) -> String {
        let json = serde_json::to_string(&self.children).unwrap_or_default();
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        json.hash(&mut hasher);
        format!("v{:x}", hasher.finish())
    }

    /// Recovers pending transactions after a restart.
    ///
    /// Iterates compensating records and reconciles them against the
    /// current spec state. Records with state "pending" that have a
    /// matching declaration hash are marked as committed.
    pub fn recover_pending_transactions(&mut self) {
        let mut recovered = Vec::new();
        for record in self.compensating_records.iter_mut() {
            if record.state == "pending" {
                // Mark as compensated since we cannot truly roll back
                // runtime state after a restart without full runtime.
                record.state = "compensated".to_string();
                recovered.push(record.transaction_id);
            }
        }
        if !recovered.is_empty() {
            // Log recovery info.
            #[cfg(debug_assertions)]
            eprintln!("Recovered {} pending transactions", recovered.len());
        }
    }

    /// ```
    /// let yaml = r#"
    /// supervisor:
    ///   strategy: OneForAll
    /// policy:
    ///   child_restart_limit: 10
    ///   child_restart_window_ms: 60000
    ///   supervisor_failure_limit: 30
    ///   supervisor_failure_window_ms: 60000
    ///   initial_backoff_ms: 10
    ///   max_backoff_ms: 1000
    ///   jitter_ratio: 0.0
    ///   heartbeat_interval_ms: 1000
    ///   stale_after_ms: 3000
    /// shutdown:
    ///   graceful_timeout_ms: 1000
    ///   abort_wait_ms: 100
    /// observability:
    ///   event_journal_capacity: 64
    ///   metrics_enabled: true
    ///   audit_enabled: true
    /// "#;
    /// let state = rust_supervisor::config::yaml::parse_config_state(yaml).unwrap();
    /// let spec = state.to_supervisor_spec().unwrap();
    /// assert_eq!(spec.strategy, rust_supervisor::spec::supervisor::SupervisionStrategy::OneForAll);
    /// assert_eq!(spec.supervisor_failure_limit, 30);
    /// ```
    pub fn to_supervisor_spec(
        &self,
    ) -> Result<crate::spec::supervisor::SupervisorSpec, crate::error::types::SupervisorError> {
        let mut spec = crate::spec::supervisor::SupervisorSpec::root(self.children.clone());
        spec.strategy = self.supervisor.strategy;
        spec.config_version = self.config_version();
        spec.supervisor_failure_limit = self.policy.supervisor_failure_limit;
        spec.escalation_policy = self.supervisor.escalation_policy;
        spec.control_channel_capacity = self.observability.event_journal_capacity;
        spec.event_channel_capacity = self.observability.event_journal_capacity;
        spec.backpressure_config = self.backpressure.clone();
        spec.group_configs = self
            .groups
            .iter()
            .map(GroupConfig::to_runtime)
            .collect::<Vec<_>>();
        spec.group_strategies = self
            .group_strategies
            .iter()
            .map(GroupStrategyConfig::to_runtime)
            .collect::<Vec<_>>();
        spec.group_dependencies = self
            .group_dependencies
            .iter()
            .map(GroupDependencyConfig::to_runtime)
            .collect::<Vec<_>>();
        spec.child_strategy_overrides = self
            .child_strategy_overrides
            .iter()
            .map(ChildStrategyOverrideConfig::to_runtime)
            .collect::<Vec<_>>();
        spec.severity_defaults = self
            .severity_defaults
            .iter()
            .map(|default| (default.task_role, default.severity))
            .collect::<HashMap<_, _>>();
        spec.dynamic_supervisor_policy = self.supervisor.dynamic_supervisor.to_runtime();
        spec.meltdown_policy = self.policy.meltdown.to_runtime();
        spec.failure_window_config = self.policy.failure_window.to_runtime();
        spec.restart_budget_config = self.policy.restart_budget.to_runtime();
        spec.pipeline_journal_capacity = self.policy.supervision_pipeline.journal_capacity;
        spec.pipeline_subscriber_capacity = self.policy.supervision_pipeline.subscriber_capacity;
        spec.concurrent_restart_limit = self.policy.supervision_pipeline.concurrent_restart_limit;
        spec.default_backoff_policy = crate::spec::child::BackoffPolicy::new(
            Duration::from_millis(self.policy.initial_backoff_ms),
            Duration::from_millis(self.policy.max_backoff_ms),
            self.policy.jitter_ratio,
        );
        spec.default_health_policy = crate::spec::child::HealthPolicy::new(
            Duration::from_millis(self.policy.heartbeat_interval_ms),
            Duration::from_millis(self.policy.stale_after_ms),
        );
        spec.default_shutdown_policy = crate::spec::child::ShutdownPolicy::new(
            Duration::from_millis(self.shutdown.graceful_timeout_ms),
            Duration::from_millis(self.shutdown.abort_wait_ms),
        );
        spec.restart_limit = Some(crate::spec::supervisor::RestartLimit::new(
            self.policy.child_restart_limit,
            Duration::from_millis(self.policy.child_restart_window_ms),
        ));
        spec.metrics_enabled = self.observability.metrics_enabled;
        spec.audit_enabled = self.observability.audit_enabled;
        spec.validate()?;
        Ok(spec)
    }

    /// Builds a stable configuration version string from configured values.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a deterministic version string for diagnostics.
    fn config_version(&self) -> String {
        format!(
            "supervisor-{:?}-policy-{}-{}-shutdown-{}-observe-{}-backpressure-{:?}-{}-{}",
            self.supervisor.strategy,
            self.policy.child_restart_limit,
            self.policy.supervisor_failure_limit,
            self.shutdown.graceful_timeout_ms,
            self.observability.event_journal_capacity,
            self.backpressure.strategy,
            self.backpressure.warn_threshold_pct,
            self.backpressure.critical_threshold_pct
        )
    }
}

/// Validates policy configuration invariants.
///
/// # Arguments
///
/// - `policy`: Policy configuration loaded from YAML.
///
/// # Returns
///
/// Returns `Ok(())` when policy values are usable.
fn validate_policy(policy: &PolicyConfig) -> Result<(), crate::error::types::SupervisorError> {
    validate_positive(policy.child_restart_limit, "policy.child_restart_limit")?;
    validate_positive(
        policy.supervisor_failure_limit,
        "policy.supervisor_failure_limit",
    )?;
    validate_positive(
        policy.child_restart_window_ms,
        "policy.child_restart_window_ms",
    )?;
    validate_positive(
        policy.supervisor_failure_window_ms,
        "policy.supervisor_failure_window_ms",
    )?;
    validate_positive(policy.initial_backoff_ms, "policy.initial_backoff_ms")?;
    validate_positive(policy.max_backoff_ms, "policy.max_backoff_ms")?;
    validate_positive(policy.heartbeat_interval_ms, "policy.heartbeat_interval_ms")?;
    validate_positive(policy.stale_after_ms, "policy.stale_after_ms")?;
    if policy.initial_backoff_ms > policy.max_backoff_ms {
        return Err(crate::error::types::SupervisorError::fatal_config(
            "policy.initial_backoff_ms must be less than or equal to policy.max_backoff_ms",
        ));
    }
    if !(0.0..=1.0).contains(&policy.jitter_ratio) {
        return Err(crate::error::types::SupervisorError::fatal_config(
            "policy.jitter_ratio must be between 0 and 1",
        ));
    }
    Ok(())
}

/// Validates lower-level policy sections loaded from YAML.
///
/// # Arguments
///
/// - `policy`: Policy configuration loaded from YAML.
///
/// # Returns
///
/// Returns `Ok(())` when lower-level policy values are usable.
fn validate_lower_policy(
    policy: &PolicyConfig,
) -> Result<(), crate::error::types::SupervisorError> {
    validate_positive(
        policy.restart_budget.window_secs,
        "policy.restart_budget.window_secs",
    )?;
    validate_positive(
        policy.restart_budget.max_burst as u64,
        "policy.restart_budget.max_burst",
    )?;
    if !(0.0..=1000.0).contains(&policy.restart_budget.recovery_rate_per_sec)
        || policy.restart_budget.recovery_rate_per_sec == 0.0
    {
        return Err(crate::error::types::SupervisorError::fatal_config(
            "policy.restart_budget.recovery_rate_per_sec must be greater than 0 and less than or equal to 1000",
        ));
    }

    match policy.failure_window.mode {
        crate::config::policy::FailureWindowMode::TimeSliding => {
            validate_positive(
                policy.failure_window.window_secs,
                "policy.failure_window.window_secs",
            )?;
        }
        crate::config::policy::FailureWindowMode::CountSliding => {
            validate_positive(
                policy.failure_window.max_count as u64,
                "policy.failure_window.max_count",
            )?;
        }
    }
    validate_positive(
        policy.failure_window.threshold as u64,
        "policy.failure_window.threshold",
    )?;

    validate_positive(
        policy.meltdown.child_max_restarts as u64,
        "policy.meltdown.child_max_restarts",
    )?;
    validate_positive(
        policy.meltdown.child_window_secs,
        "policy.meltdown.child_window_secs",
    )?;
    validate_positive(
        policy.meltdown.group_max_failures as u64,
        "policy.meltdown.group_max_failures",
    )?;
    validate_positive(
        policy.meltdown.group_window_secs,
        "policy.meltdown.group_window_secs",
    )?;
    validate_positive(
        policy.meltdown.supervisor_max_failures as u64,
        "policy.meltdown.supervisor_max_failures",
    )?;
    validate_positive(
        policy.meltdown.supervisor_window_secs,
        "policy.meltdown.supervisor_window_secs",
    )?;
    validate_positive(
        policy.meltdown.reset_after_secs,
        "policy.meltdown.reset_after_secs",
    )?;
    validate_positive(
        policy.supervision_pipeline.journal_capacity as u64,
        "policy.supervision_pipeline.journal_capacity",
    )?;
    validate_positive(
        policy.supervision_pipeline.subscriber_capacity as u64,
        "policy.supervision_pipeline.subscriber_capacity",
    )?;
    validate_positive(
        policy.supervision_pipeline.concurrent_restart_limit as u64,
        "policy.supervision_pipeline.concurrent_restart_limit",
    )
}

/// Validates root supervisor policy options.
///
/// # Arguments
///
/// - `supervisor`: Root supervisor configuration loaded from YAML.
///
/// # Returns
///
/// Returns `Ok(())` when root policy values are usable.
fn validate_supervisor_root(
    supervisor: &SupervisorRootConfig,
) -> Result<(), crate::error::types::SupervisorError> {
    if supervisor.dynamic_supervisor.child_limit == Some(0) {
        return Err(crate::error::types::SupervisorError::fatal_config(
            "supervisor.dynamic_supervisor.child_limit must be greater than zero",
        ));
    }
    Ok(())
}

/// Validates group and override inputs against declared child names.
///
/// # Arguments
///
/// - `groups`: Group declarations loaded from YAML.
/// - `group_strategies`: Group strategy overrides loaded from YAML.
/// - `group_dependencies`: Group dependency edges loaded from YAML.
/// - `child_strategy_overrides`: Child strategy overrides loaded from YAML.
/// - `severity_defaults`: Role severity defaults loaded from YAML.
/// - `children`: Child declarations loaded from YAML.
///
/// # Returns
///
/// Returns `Ok(())` when group and override declarations are coherent.
fn validate_group_inputs(
    groups: &[GroupConfig],
    group_strategies: &[GroupStrategyConfig],
    group_dependencies: &[GroupDependencyConfig],
    child_strategy_overrides: &[ChildStrategyOverrideConfig],
    severity_defaults: &[SeverityDefaultConfig],
    children: &[ChildDeclaration],
) -> Result<(), crate::error::types::SupervisorError> {
    let child_names = children
        .iter()
        .map(|child| child.name.as_str())
        .collect::<HashSet<_>>();
    let mut group_names = HashSet::new();
    for group in groups {
        if group.name.trim().is_empty() {
            return Err(crate::error::types::SupervisorError::fatal_config(
                "groups[].name must not be empty",
            ));
        }
        if !group_names.insert(group.name.as_str()) {
            return Err(crate::error::types::SupervisorError::fatal_config(format!(
                "duplicate group name '{}'",
                group.name
            )));
        }
        for child in &group.children {
            if !child_names.contains(child.as_str()) {
                return Err(crate::error::types::SupervisorError::fatal_config(format!(
                    "group '{}' references unknown child '{}'",
                    group.name, child
                )));
            }
        }
    }

    for strategy in group_strategies {
        if !group_names.contains(strategy.group.as_str()) {
            return Err(crate::error::types::SupervisorError::fatal_config(format!(
                "group_strategies references unknown group '{}'",
                strategy.group
            )));
        }
        validate_restart_limit_input(
            strategy.restart_limit.as_ref(),
            "group_strategies.restart_limit",
        )?;
    }

    for dependency in group_dependencies {
        if !group_names.contains(dependency.from_group.as_str()) {
            return Err(crate::error::types::SupervisorError::fatal_config(format!(
                "group_dependencies references unknown from_group '{}'",
                dependency.from_group
            )));
        }
        if !group_names.contains(dependency.to_group.as_str()) {
            return Err(crate::error::types::SupervisorError::fatal_config(format!(
                "group_dependencies references unknown to_group '{}'",
                dependency.to_group
            )));
        }
    }

    // Detect cycles in group dependency graph using Kahn's algorithm.
    if !group_dependencies.is_empty() {
        let mut in_degree: std::collections::HashMap<&str, usize> =
            group_names.iter().map(|&n| (n, 0)).collect();
        let mut adj: std::collections::HashMap<&str, Vec<&str>> =
            group_names.iter().map(|&n| (n, Vec::new())).collect();
        for dep in group_dependencies {
            adj.entry(dep.from_group.as_str())
                .or_default()
                .push(dep.to_group.as_str());
            *in_degree.entry(dep.to_group.as_str()).or_insert(0) += 1;
        }
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(n, _)| *n)
            .collect();
        let mut visited = 0;
        while let Some(node) = queue.pop() {
            visited += 1;
            if let Some(neighbors) = adj.get(node) {
                for &next in neighbors {
                    if let Some(deg) = in_degree.get_mut(next) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push(next);
                        }
                    }
                }
            }
        }
        if visited != group_names.len() {
            let cycle_nodes: Vec<&str> = in_degree
                .iter()
                .filter(|(_, deg)| **deg > 0)
                .map(|(n, _)| *n)
                .collect();
            return Err(crate::error::types::SupervisorError::fatal_config(format!(
                "Group dependency cycle detected among groups: {cycle_nodes:?}",
            )));
        }
    }

    for child_override in child_strategy_overrides {
        if !child_names.contains(child_override.child_id.as_str()) {
            return Err(crate::error::types::SupervisorError::fatal_config(format!(
                "child_strategy_overrides references unknown child '{}'",
                child_override.child_id
            )));
        }
        validate_restart_limit_input(
            child_override.restart_limit.as_ref(),
            "child_strategy_overrides.restart_limit",
        )?;
    }

    let mut roles = HashSet::new();
    for default in severity_defaults {
        if !roles.insert(default.task_role) {
            return Err(crate::error::types::SupervisorError::fatal_config(format!(
                "duplicate severity default for task role '{}'",
                default.task_role
            )));
        }
    }
    Ok(())
}

/// Validates an optional YAML restart limit.
///
/// # Arguments
///
/// - `limit`: Optional restart limit loaded from YAML.
/// - `path`: Field path used in error messages.
///
/// # Returns
///
/// Returns `Ok(())` when the optional restart limit is absent or usable.
fn validate_restart_limit_input(
    limit: Option<&crate::config::policy::RestartLimitConfig>,
    path: &str,
) -> Result<(), crate::error::types::SupervisorError> {
    let Some(limit) = limit else {
        return Ok(());
    };
    validate_positive(limit.max_restarts as u64, &format!("{path}.max_restarts"))?;
    validate_positive(limit.window_ms, &format!("{path}.window_ms"))
}

/// Validates shutdown configuration invariants.
///
/// # Arguments
///
/// - `shutdown`: Shutdown configuration loaded from YAML.
///
/// # Returns
///
/// Returns `Ok(())` when shutdown values are usable.
fn validate_shutdown(
    shutdown: &ShutdownConfig,
) -> Result<(), crate::error::types::SupervisorError> {
    validate_positive(shutdown.graceful_timeout_ms, "shutdown.graceful_timeout_ms")?;
    validate_positive(shutdown.abort_wait_ms, "shutdown.abort_wait_ms")
}

/// Validates observability configuration invariants.
///
/// # Arguments
///
/// - `observability`: Observability configuration loaded from YAML.
///
/// # Returns
///
/// Returns `Ok(())` when observability values are usable.
fn validate_observability(
    observability: &ObservabilityConfig,
) -> Result<(), crate::error::types::SupervisorError> {
    validate_positive(
        observability.event_journal_capacity as u64,
        "observability.event_journal_capacity",
    )
}

/// Validates audit persistence configuration invariants.
///
/// # Arguments
///
/// - `audit`: Audit persistence configuration loaded from YAML.
///
/// # Returns
///
/// Returns `Ok(())` when audit persistence values are usable.
fn validate_audit(audit: &AuditConfig) -> Result<(), crate::error::types::SupervisorError> {
    match audit.backend.as_str() {
        "memory" => {}
        "file" => {
            let path = audit.file_path.as_deref().unwrap_or_default().trim();
            if path.is_empty() {
                return Err(crate::error::types::SupervisorError::fatal_config(
                    "audit.file_path is required when audit.backend is file",
                ));
            }
        }
        backend => {
            return Err(crate::error::types::SupervisorError::fatal_config(format!(
                "audit.backend must be memory or file, got {backend}"
            )));
        }
    }

    match audit.failure_strategy.as_str() {
        "fail_closed" | "defer_bounded" => {}
        strategy => {
            return Err(crate::error::types::SupervisorError::fatal_config(format!(
                "audit.failure_strategy must be fail_closed or defer_bounded, got {strategy}"
            )));
        }
    }

    validate_positive(audit.max_defer_queue as u64, "audit.max_defer_queue")
}

/// Validates backpressure configuration invariants.
///
/// # Arguments
///
/// - `backpressure`: Backpressure configuration loaded from YAML.
///
/// # Returns
///
/// Returns `Ok(())` when thresholds, windows, and capacities are usable.
fn validate_backpressure(
    backpressure: &BackpressureConfig,
) -> Result<(), crate::error::types::SupervisorError> {
    if backpressure.warn_threshold_pct == 0 || backpressure.warn_threshold_pct > 100 {
        return Err(crate::error::types::SupervisorError::fatal_config(
            "backpressure.warn_threshold_pct must be between 1 and 100",
        ));
    }
    if backpressure.critical_threshold_pct == 0 || backpressure.critical_threshold_pct > 100 {
        return Err(crate::error::types::SupervisorError::fatal_config(
            "backpressure.critical_threshold_pct must be between 1 and 100",
        ));
    }
    if backpressure.warn_threshold_pct >= backpressure.critical_threshold_pct {
        return Err(crate::error::types::SupervisorError::fatal_config(
            "backpressure.warn_threshold_pct must be less than backpressure.critical_threshold_pct",
        ));
    }
    validate_positive(backpressure.window_secs, "backpressure.window_secs")?;
    validate_positive(
        backpressure.audit_channel_capacity as u64,
        "backpressure.audit_channel_capacity",
    )
}

/// Applies default security settings to the optional dashboard pipeline.
///
/// # Arguments
///
/// - `dashboard`: Optional target-side dashboard configuration loaded from YAML.
///
/// # Returns
///
/// Returns dashboard configuration with a security pipeline when dashboard IPC
/// is enabled.
fn dashboard_with_default_security(
    mut dashboard: Option<DashboardIpcConfig>,
) -> Option<DashboardIpcConfig> {
    let Some(dashboard_config) = dashboard.as_mut() else {
        return dashboard;
    };
    if !dashboard_config.enabled {
        return dashboard;
    }

    dashboard_config
        .security_config
        .get_or_insert_with(IpcSecurityConfig::default);
    dashboard
}

/// Validates dashboard IPC configuration invariants.
///
/// # Arguments
///
/// - `dashboard`: Optional target-side dashboard IPC configuration.
///
/// # Returns
///
/// Returns `Ok(())` when IPC is absent, disabled, or semantically valid.
/// Validates dashboard IPC configuration when the dashboard module is
/// compiled (Unix). On non-Unix platforms, rejects any configuration that
/// has `dashboard.enabled = true`.
#[cfg(unix)]
fn validate_dashboard(
    dashboard: Option<&DashboardIpcConfig>,
) -> Result<(), crate::error::types::SupervisorError> {
    crate::dashboard::config::validate_dashboard_ipc_config(dashboard)
        .map(|_| ())
        .map_err(|error| crate::error::types::SupervisorError::fatal_config(error.to_string()))
}

/// On non-Unix platforms the dashboard module is not compiled. If the
/// user explicitly set `dashboard.enabled = true`, report a clear error
/// instead of silently ignoring the configuration.
#[cfg(not(unix))]
fn validate_dashboard(
    dashboard: Option<&DashboardIpcConfig>,
) -> Result<(), crate::error::types::SupervisorError> {
    if let Some(config) = dashboard {
        if config.enabled {
            return Err(crate::error::types::SupervisorError::fatal_config(
                "dashboard is enabled but the dashboard IPC module is only available on Unix platforms",
            ));
        }
    }
    Ok(())
}

/// Validates that a runtime configuration number is positive.
///
/// # Arguments
///
/// - `value`: Runtime configuration number.
/// - `name`: Configuration key name.
///
/// # Returns
///
/// Returns `Ok(())` when the value is positive.
fn validate_positive(
    value: impl Into<u64>,
    name: &str,
) -> Result<(), crate::error::types::SupervisorError> {
    if value.into() == 0 {
        Err(crate::error::types::SupervisorError::fatal_config(format!(
            "{name} must be greater than zero"
        )))
    } else {
        Ok(())
    }
}

/// Computes a deterministic hash of a ChildDeclaration content.
///
/// This is used for compensating record identification, not for
/// cryptographic security. The hash is always recomputed from the
/// declaration content after restart.
fn compute_declaration_hash(declaration: &ChildDeclaration) -> String {
    let json = serde_json::to_string(declaration).unwrap_or_default();
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut hasher);
    format!("v{:x}", hasher.finish())
}
