//! Public configuration input model for supervisor users.
//!
//! The structs in this module are the single raw configuration surface used for
//! YAML loading, template rendering, and JSON Schema generation.

use confique::Config;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    config::{
        audit::AuditConfig,
        ipc_security::IpcSecurityConfig,
        policy::{
            ChildStrategyOverrideConfig, DynamicSupervisorConfig, FailureWindowConfig, GroupConfig,
            GroupDependencyConfig, GroupStrategyConfig, MeltdownConfig, RestartBudgetConfig,
            SeverityDefaultConfig, SupervisionPipelineConfig,
        },
    },
    spec::{
        child_declaration::ChildDeclaration,
        supervisor::{BackpressureConfig, EscalationPolicy, SupervisionStrategy},
    },
};

/// Configuration file shape loaded from YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct SupervisorConfig {
    /// Additional configuration files included by `rust-config-tree`.
    #[config(default = [])]
    #[serde(default)]
    pub include: Vec<PathBuf>,

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
    /// Command audit persistence configuration.
    #[config(nested)]
    #[serde(default)]
    pub audit: AuditConfig,
    /// Backpressure policy for observability event subscribers.
    #[config(nested)]
    #[serde(default)]
    pub backpressure: BackpressureConfig,
    /// Group-level restart budgets and membership declarations.
    #[config(default = [])]
    #[serde(default)]
    pub groups: Vec<GroupConfig>,
    /// Group-level strategy overrides.
    #[config(default = [])]
    #[serde(default)]
    pub group_strategies: Vec<GroupStrategyConfig>,
    /// Cross-group failure propagation dependencies.
    #[config(default = [])]
    #[serde(default)]
    pub group_dependencies: Vec<GroupDependencyConfig>,
    /// Child-level strategy overrides.
    #[config(default = [])]
    #[serde(default)]
    pub child_strategy_overrides: Vec<ChildStrategyOverrideConfig>,
    /// Default severity class per task role.
    #[config(default = [])]
    #[serde(default)]
    pub severity_defaults: Vec<SeverityDefaultConfig>,
    /// Optional target-side dashboard IPC configuration.
    pub dashboard: Option<DashboardIpcConfig>,
    /// Child declarations loaded from YAML children array.
    #[config(default = [])]
    #[serde(default)]
    pub children: Vec<ChildDeclaration>,
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
    /// Returns include paths declared by this configuration layer.
    fn include_paths(layer: &<Self as Config>::Layer) -> Vec<PathBuf> {
        layer.include.clone().unwrap_or_default()
    }
}

/// Root supervisor configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct SupervisorRootConfig {
    /// Restart scope strategy for child failures.
    pub strategy: SupervisionStrategy,
    /// Optional supervisor-level escalation policy.
    #[serde(default)]
    pub escalation_policy: Option<EscalationPolicy>,
    /// Runtime dynamic child acceptance policy.
    #[config(nested)]
    #[serde(default)]
    pub dynamic_supervisor: DynamicSupervisorConfig,
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
    /// Restart budget used by the supervision pipeline.
    #[config(nested)]
    #[serde(default)]
    pub restart_budget: RestartBudgetConfig,
    /// Failure window used by the supervision pipeline.
    #[config(nested)]
    #[serde(default)]
    pub failure_window: FailureWindowConfig,
    /// Meltdown fuse limits for child, group, and supervisor scopes.
    #[config(nested)]
    #[serde(default)]
    pub meltdown: MeltdownConfig,
    /// Supervision pipeline capacities and concurrent restart limit.
    #[config(nested)]
    #[serde(default)]
    pub supervision_pipeline: SupervisionPipelineConfig,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
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
    /// Optional IPC security pipeline configuration (C1-C9).
    #[serde(default)]
    pub security_config: Option<IpcSecurityConfig>,
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
