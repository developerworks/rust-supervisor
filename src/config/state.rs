//! Immutable configuration state for supervisor runtime values.
//!
//! Raw YAML input belongs to [`crate::config::configurable`]. This module owns
//! semantic validation and conversion into supervisor runtime declarations.

use crate::config::configurable::{
    ObservabilityConfig, PolicyConfig, ShutdownConfig, SupervisorConfig, SupervisorRootConfig,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Immutable validated configuration state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigState {
    /// Root supervisor declaration values.
    pub supervisor: SupervisorRootConfig,
    /// Runtime policy values.
    pub policy: PolicyConfig,
    /// Shutdown budget values.
    pub shutdown: ShutdownConfig,
    /// Observability switches and capacities.
    pub observability: ObservabilityConfig,
}

impl TryFrom<SupervisorConfig> for ConfigState {
    type Error = crate::error::types::SupervisorError;

    /// Converts a deserialized supervisor config into validated state.
    fn try_from(config: SupervisorConfig) -> Result<Self, Self::Error> {
        validate_policy(&config.policy)?;
        validate_shutdown(&config.shutdown)?;
        validate_observability(&config.observability)?;
        Ok(Self {
            supervisor: config.supervisor,
            policy: config.policy,
            shutdown: config.shutdown,
            observability: config.observability,
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
