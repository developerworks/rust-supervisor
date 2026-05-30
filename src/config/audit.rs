//! Command audit configuration model.
//!
//! This module owns the single public audit configuration used by the
//! supervisor root configuration and by IPC security audit persistence.

use confique::Config;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Audit persistence configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config, JsonSchema)]
pub struct AuditConfig {
    /// Whether audit logging is enabled. Default: true.
    #[config(default = true)]
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Audit storage backend. Default: "memory".
    /// - "memory": ring buffer only, not persisted.
    /// - "file": append-only JSON lines file.
    #[config(default = "memory")]
    #[serde(default = "default_audit_backend")]
    pub backend: String,

    /// File path for file backend. Required when backend is "file".
    #[config(default = "/tmp/rust-supervisor-demo/audit.jsonl")]
    #[schemars(!default)]
    #[serde(default = "default_file_path")]
    pub file_path: String,

    /// Failure strategy when audit backend is unavailable.
    /// - "fail_closed": reject write commands when audit cannot be written.
    /// - "defer_bounded": defer audit writes with bounded queue.
    ///   Default: "fail_closed".
    #[config(default = "fail_closed")]
    #[serde(default = "default_fail_closed")]
    pub failure_strategy: String,

    /// Max queue size for "defer_bounded" strategy. Default: 1000.
    #[config(default = 1000)]
    #[serde(default = "default_1000")]
    pub max_defer_queue: usize,
}

impl Default for AuditConfig {
    /// Returns default audit config: memory backend, fail_closed strategy.
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "memory".into(),
            file_path: "/tmp/rust-supervisor-demo/audit.jsonl".to_string(),
            failure_strategy: "fail_closed".into(),
            max_defer_queue: 1000,
        }
    }
}

/// Serde default helper: returns true.
fn default_true() -> bool {
    true
}

/// Serde default helper: returns "memory".
fn default_audit_backend() -> String {
    "memory".into()
}

/// Serde default helper: returns the demo audit file path.
fn default_file_path() -> String {
    "/tmp/rust-supervisor-demo/audit.jsonl".to_string()
}

/// Serde default helper: returns "fail_closed".
fn default_fail_closed() -> String {
    "fail_closed".into()
}

/// Serde default helper: returns 1000.
fn default_1000() -> usize {
    1000
}
