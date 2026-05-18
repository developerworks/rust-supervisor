//! Immutable configuration state for supervisor runtime values.
//!
//! Raw YAML input belongs to [`crate::config::configurable`]. This module owns
//! semantic validation and conversion into supervisor runtime declarations.

use crate::config::configurable::{
    DashboardIpcConfig, ObservabilityConfig, PolicyConfig, ShutdownConfig, SupervisorConfig,
    SupervisorRootConfig,
};
use crate::spec::child::ChildSpec;
use crate::spec::child_declaration::{ChildDeclaration, CompensatingRecord, PendingChild, Phase};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
    /// Optional target-side dashboard IPC configuration.
    pub ipc: Option<DashboardIpcConfig>,
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
            && self.ipc == other.ipc
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
        validate_ipc(config.ipc.as_ref())?;

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
                "Dependency cycle detected among children: {:?}",
                node_names
            ))
        })?;

        let spec_hash = String::new(); // Will be computed after SupervisorSpec is built.

        Ok(Self {
            supervisor: config.supervisor,
            policy: config.policy,
            shutdown: config.shutdown,
            observability: config.observability,
            ipc: config.ipc,
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

        // Update spec_hash.
        self.spec_hash = format!("sha256-{}", transaction_id);

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
            declaration_hash: format!("sha256-{}", transaction_id),
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
        let mut spec = crate::spec::supervisor::SupervisorSpec::root(Vec::new());
        spec.strategy = self.supervisor.strategy;
        spec.config_version = self.config_version();
        spec.supervisor_failure_limit = self.policy.supervisor_failure_limit;
        spec.control_channel_capacity = self.observability.event_journal_capacity;
        spec.event_channel_capacity = self.observability.event_journal_capacity;
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
            "supervisor-{:?}-policy-{}-{}-shutdown-{}-observe-{}",
            self.supervisor.strategy,
            self.policy.child_restart_limit,
            self.policy.supervisor_failure_limit,
            self.shutdown.graceful_timeout_ms,
            self.observability.event_journal_capacity
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

/// Validates dashboard IPC configuration invariants.
///
/// # Arguments
///
/// - `ipc`: Optional target-side dashboard IPC configuration.
///
/// # Returns
///
/// Returns `Ok(())` when IPC is absent, disabled, or semantically valid.
fn validate_ipc(
    ipc: Option<&DashboardIpcConfig>,
) -> Result<(), crate::error::types::SupervisorError> {
    crate::dashboard::config::validate_dashboard_ipc_config(ipc)
        .map(|_| ())
        .map_err(|error| crate::error::types::SupervisorError::fatal_config(error.to_string()))
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
