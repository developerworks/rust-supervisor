//! IPC security configuration model.
//!
//! Defines the nine control point (C1-C9) configuration structs with serde
//! deserialization support and secure-by-default values. All control points
//! are independently configurable via YAML.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Top-level config aggregator
// ---------------------------------------------------------------------------

/// Aggregated IPC security configuration loaded from YAML.
///
/// Holds all nine control-point sub-configs. Each sub-config is independently
/// gated by its own `enabled` flag so that partial adoption is possible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcSecurityConfig {
    /// C1-C2: Peer identity verification settings.
    #[serde(default)]
    pub peer_identity: PeerIdentityConfig,

    /// C3: Command authorization matrix.
    #[serde(default)]
    pub authorization: AuthorizationConfig,

    /// C4: Replay protection settings.
    #[serde(default)]
    pub replay_protection: ReplayProtectionConfig,

    /// C5: Request size limit.
    #[serde(default)]
    pub request_size_limit: RequestSizeLimitConfig,

    /// C6: Rate limiting settings.
    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    /// C7: Audit persistence settings.
    #[serde(default)]
    pub audit: AuditConfig,

    /// C8: Command idempotency settings.
    #[serde(default)]
    pub idempotency: IdempotencyConfig,

    /// C9: External command allowlist.
    #[serde(default)]
    pub allowlist: AllowlistConfig,
}

impl Default for IpcSecurityConfig {
    /// Returns the default IPC security configuration with all control
    /// points enabled and set to secure-by-default values.
    fn default() -> Self {
        Self {
            peer_identity: PeerIdentityConfig::default(),
            authorization: AuthorizationConfig::default(),
            replay_protection: ReplayProtectionConfig::default(),
            request_size_limit: RequestSizeLimitConfig::default(),
            rate_limit: RateLimitConfig::default(),
            audit: AuditConfig::default(),
            idempotency: IdempotencyConfig::default(),
            allowlist: AllowlistConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// C1-C2: Peer identity verification
// ---------------------------------------------------------------------------

/// Peer identity verification configuration.
///
/// C1: socket owner verification — the process that bound the socket
/// (this process) is the only allowed owner by definition.
/// C2: peer credentials verification — connecting process must match
/// configured identity expectations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerIdentityConfig {
    /// Whether peer credential checks are enabled. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Require peer uid to match this process uid. Default: true.
    #[serde(default = "default_true")]
    pub require_uid_match: bool,

    /// Allowed gid list. Empty means gid check is disabled.
    /// Default: empty.
    #[serde(default)]
    pub allowed_gids: Vec<u32>,

    /// Allowed pid list. Empty means pid check is disabled.
    /// Pid checks are inherently racy and only useful in container
    /// environments with deterministic pids. Default: empty.
    #[serde(default)]
    pub allowed_pids: Vec<u32>,
}

impl Default for PeerIdentityConfig {
    /// Returns secure-by-default peer identity config: uid match required,
    /// gid and pid checks disabled.
    fn default() -> Self {
        Self {
            enabled: true,
            require_uid_match: true,
            allowed_gids: Vec::new(),
            allowed_pids: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// C3: Command authorization
// ---------------------------------------------------------------------------

/// Command authorization matrix.
///
/// Maps each risk category to an allowed identity set.
/// Write commands (restart, shutdown, etc.) require authorized peer identity.
/// Read commands (hello, state) are always allowed when peer identity passes
/// C1-C2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Whether authorization checks are enabled. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Commands classified as high-risk that require explicit authorization.
    /// Default: all write/destructive commands.
    #[serde(default = "default_high_risk_commands")]
    pub high_risk_commands: Vec<String>,

    /// Allowed peer uids for high-risk commands. Empty means deny all.
    /// Default: [0] (root only).
    #[serde(default = "default_root_only")]
    pub allowed_uids: Vec<u32>,
}

impl Default for AuthorizationConfig {
    /// Returns default authorization config: enabled, root-only uid whitelist.
    fn default() -> Self {
        Self {
            enabled: true,
            high_risk_commands: default_high_risk_commands(),
            allowed_uids: default_root_only(),
        }
    }
}

/// Returns the default high-risk command list: all write/destructive
/// IPC methods that require explicit authorization.
fn default_high_risk_commands() -> Vec<String> {
    vec![
        "command.restart_child".into(),
        "command.pause_child".into(),
        "command.resume_child".into(),
        "command.quarantine_child".into(),
        "command.remove_child".into(),
        "command.add_child".into(),
        "command.shutdown_tree".into(),
    ]
}

/// Returns the default allowed uid list: root only ([0]).
fn default_root_only() -> Vec<u32> {
    vec![0]
}

// ---------------------------------------------------------------------------
// C4: Replay protection
// ---------------------------------------------------------------------------

/// Replay protection configuration.
///
/// Uses a sliding window of seen request identifiers with a TTL (time-to-live).
/// A request_id appearing twice within the window is rejected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayProtectionConfig {
    /// Whether replay protection is enabled. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Sliding window size in number of request_ids. Default: 1024.
    #[serde(default = "default_1024")]
    pub window_size: usize,

    /// Time-to-live for each entry in seconds. Entry removed after TTL.
    /// Default: 60.
    #[serde(default = "default_60")]
    pub ttl_seconds: u64,
}

impl Default for ReplayProtectionConfig {
    /// Returns default replay protection: window size 1024, TTL 60s.
    fn default() -> Self {
        Self {
            enabled: true,
            window_size: 1024,
            ttl_seconds: 60,
        }
    }
}

/// Serde default helper: returns 1024.
fn default_1024() -> usize {
    1024
}
/// Serde default helper: returns 60.
fn default_60() -> u64 {
    60
}

// ---------------------------------------------------------------------------
// C5: Request size limit
// ---------------------------------------------------------------------------

/// Request size limit configuration.
///
/// Rejects requests whose body byte length exceeds the configured maximum.
/// Checked before JSON deserialization to prevent memory-bomb attacks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestSizeLimitConfig {
    /// Whether size limit is enforced. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum request body size in bytes. Default: 65536 (64 KiB).
    #[serde(default = "default_65536")]
    pub max_bytes: usize,
}

impl Default for RequestSizeLimitConfig {
    /// Returns default size limit: 64 KiB (65536 bytes).
    fn default() -> Self {
        Self {
            enabled: true,
            max_bytes: 65536,
        }
    }
}

/// Serde default helper: returns 65536 (64 KiB).
fn default_65536() -> usize {
    65536
}

// ---------------------------------------------------------------------------
// C6: Rate limit
// ---------------------------------------------------------------------------

/// Rate limit configuration.
///
/// Uses token bucket algorithm per connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Token refill rate per second. Default: 100.0.
    #[serde(default = "default_100_0")]
    pub refill_rate: f64,

    /// Maximum burst capacity. Default: 20.
    #[serde(default = "default_20")]
    pub burst_capacity: u32,
}

impl Default for RateLimitConfig {
    /// Returns default rate limit: 100 req/s, burst capacity 20.
    fn default() -> Self {
        Self {
            enabled: true,
            refill_rate: 100.0,
            burst_capacity: 20,
        }
    }
}
/// Serde default helper: returns 100.0.
fn default_100_0() -> f64 {
    100.0
}
/// Serde default helper: returns 20.
fn default_20() -> u32 {
    20
}

// ---------------------------------------------------------------------------
// C7: Audit persistence
// ---------------------------------------------------------------------------

/// Audit persistence configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Whether audit logging is enabled. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Audit storage backend. Default: "memory".
    /// - "memory": ring buffer only, not persisted.
    /// - "file": append-only JSON lines file.
    #[serde(default = "default_audit_backend")]
    pub backend: String,

    /// File path for file backend. Required when backend is "file".
    #[serde(default)]
    pub file_path: Option<String>,

    /// Failure strategy when audit backend is unavailable.
    /// - "fail_closed": reject write commands when audit cannot be written.
    /// - "defer_bounded": defer audit writes with bounded queue.
    ///   Default: "fail_closed".
    #[serde(default = "default_fail_closed")]
    pub failure_strategy: String,

    /// Max queue size for "defer_bounded" strategy. Default: 1000.
    #[serde(default = "default_1000")]
    pub max_defer_queue: usize,
}

impl Default for AuditConfig {
    /// Returns default audit config: memory backend, fail_closed strategy.
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "memory".into(),
            file_path: None,
            failure_strategy: "fail_closed".into(),
            max_defer_queue: 1000,
        }
    }
}

/// Serde default helper: returns "memory".
fn default_audit_backend() -> String {
    "memory".into()
}
/// Serde default helper: returns "fail_closed".
fn default_fail_closed() -> String {
    "fail_closed".into()
}
/// Serde default helper: returns 1000.
fn default_1000() -> usize {
    1000
}

// ---------------------------------------------------------------------------
// C8: Command idempotency
// ---------------------------------------------------------------------------

/// Command idempotency configuration.
///
/// Uses the same request_id from C4 replay protection.
/// If a command with a seen request_id is replayed, return the cached
/// result instead of re-executing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdempotencyConfig {
    /// Whether idempotency is enforced. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// TTL for cached command results in seconds. Default: 60.
    #[serde(default = "default_60")]
    pub result_cache_ttl_seconds: u64,

    /// Maximum cached results. Default: 1024.
    #[serde(default = "default_1024")]
    pub max_cached_results: usize,
}

impl Default for IdempotencyConfig {
    /// Returns default idempotency: TTL 60s, max 1024 cached results.
    fn default() -> Self {
        Self {
            enabled: true,
            result_cache_ttl_seconds: 60,
            max_cached_results: 1024,
        }
    }
}

// ---------------------------------------------------------------------------
// C9: External command allowlist
// ---------------------------------------------------------------------------

/// External command allowlist configuration.
///
/// Only absolute paths listed here are eligible for execution via
/// control-plane extension points. Default: empty (deny all).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllowlistConfig {
    /// Whether allowlist enforcement is enabled. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Allowed absolute executable paths. Default: empty array.
    /// Returns default allowlist: empty (deny all external commands).
    #[serde(default)]
    pub allowed_paths: Vec<String>,
}

impl Default for AllowlistConfig {
    /// Returns default allowlist: empty (deny all external commands).
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_paths: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared default helpers
// ---------------------------------------------------------------------------

/// Serde default helper: returns true.
fn default_true() -> bool {
    true
}
