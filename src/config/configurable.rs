//! Public configuration input model for supervisor users.
//!
//! The structs in this module are the single raw configuration surface used for
//! YAML loading, template rendering, and JSON Schema generation.

use confique::Config;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration file shape loaded from YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct SupervisorConfig {
    /// Root supervisor declaration values.
    #[config(nested)]
    pub supervisor: SupervisorRootConfig,
    /// Runtime policy values.
    #[config(nested)]
    pub policy: PolicyConfig,
    /// Shutdown budget values.
    #[config(nested)]
    pub shutdown: ShutdownConfig,
    /// Observability switches and capacities.
    #[config(nested)]
    pub observability: ObservabilityConfig,
    /// Optional target-side dashboard IPC configuration.
    pub ipc: Option<DashboardIpcConfig>,
}

impl rust_config_tree::ConfigSchema for SupervisorConfig {
    /// Returns child configuration paths declared by one loaded layer.
    ///
    /// # Arguments
    ///
    /// - `layer`: Partially loaded supervisor configuration layer.
    ///
    /// # Returns
    ///
    /// Returns an empty list because official supervisor templates stay in one
    /// root YAML file unless crate users wrap this type in their own project.
    fn include_paths(layer: &<Self as Config>::Layer) -> Vec<PathBuf> {
        let _ = layer;
        Vec::new()
    }
}

/// Root supervisor configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct SupervisorRootConfig {
    /// Restart scope strategy for child failures.
    pub strategy: crate::spec::supervisor::SupervisionStrategy,
}

/// Restart, backoff, and fuse configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct PolicyConfig {
    /// Maximum child restarts within the child restart window.
    pub child_restart_limit: u32,
    /// Child restart window in milliseconds.
    pub child_restart_window_ms: u64,
    /// Maximum supervisor failures within the supervisor failure window.
    pub supervisor_failure_limit: u32,
    /// Supervisor failure window in milliseconds.
    pub supervisor_failure_window_ms: u64,
    /// Initial backoff in milliseconds.
    pub initial_backoff_ms: u64,
    /// Maximum backoff in milliseconds.
    pub max_backoff_ms: u64,
    /// Jitter ratio expressed as a fraction between zero and one.
    pub jitter_ratio: f64,
    /// Heartbeat interval in milliseconds.
    pub heartbeat_interval_ms: u64,
    /// Stale heartbeat threshold in milliseconds.
    pub stale_after_ms: u64,
}

/// Shutdown coordination configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct ShutdownConfig {
    /// Graceful drain timeout in milliseconds.
    pub graceful_timeout_ms: u64,
    /// Abort wait timeout in milliseconds.
    pub abort_wait_ms: u64,
}

/// Observability configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct ObservabilityConfig {
    /// Event journal capacity.
    pub event_journal_capacity: usize,
    /// Whether metrics recording is enabled.
    pub metrics_enabled: bool,
    /// Whether command audit recording is enabled.
    pub audit_enabled: bool,
}

/// Optional target-side dashboard IPC configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct DashboardIpcConfig {
    /// Whether the target process opens the local IPC endpoint.
    pub enabled: bool,
    /// Stable target process identifier sent to relay and UI.
    pub target_id: Option<String>,
    /// Local Unix domain socket path used by the target process.
    pub path: Option<PathBuf>,
    /// Socket file permission string such as `0600`.
    pub permissions: Option<String>,
    /// Socket bind behavior when the path already exists.
    pub bind_mode: Option<DashboardIpcBindMode>,
    /// Dynamic registration settings used after IPC is ready.
    pub registration: Option<DashboardRegistrationConfig>,
}

/// Socket bind behavior for target-side dashboard IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DashboardIpcBindMode {
    /// Fail when the socket path already exists.
    CreateNew,
    /// Remove a stale socket path before binding.
    ReplaceStale,
}

/// Dynamic registration settings for a target process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Config, JsonSchema)]
pub struct DashboardRegistrationConfig {
    /// Whether the target process registers with relay after IPC is ready.
    pub enabled: bool,
    /// Local relay registration socket path.
    pub relay_registration_path: Option<PathBuf>,
    /// Human-readable name shown in the dashboard.
    pub display_name: Option<String>,
    /// Registration lease duration in seconds.
    pub lease_seconds: Option<u64>,
    /// Registration heartbeat interval in seconds.
    pub registration_heartbeat_interval_seconds: Option<u64>,
}
