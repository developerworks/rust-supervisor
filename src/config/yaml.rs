//! YAML helpers for configuration examples and tests.
//!
//! The module exposes an explicit YAML parsing boundary for callers that already
//! hold configuration text in memory.

use crate::config::state::{ConfigState, SupervisorConfig};
use crate::error::types::SupervisorError;

/// Parses a YAML string into validated configuration state.
///
/// # Arguments
///
/// - `yaml`: YAML document containing the full supervisor configuration.
///
/// # Returns
///
/// Returns validated [`ConfigState`] when all runtime tunables are present.
///
/// # Examples
///
/// ```
/// let yaml = r#"
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
/// assert_eq!(state.policy.child_restart_limit, 10);
/// ```
pub fn parse_config_state(yaml: &str) -> Result<ConfigState, SupervisorError> {
    let config: SupervisorConfig = serde_yaml::from_str(yaml).map_err(|error| {
        SupervisorError::fatal_config(format!("failed to parse YAML config: {error}"))
    })?;
    ConfigState::try_from(config)
}
