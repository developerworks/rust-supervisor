# Data Model(数据模型): IPC 安全控制点

**Feature(功能)**: 006-1-platform-docs-ipc-security
**Phase(阶段)**: 1 (设计)
**Date(日期)**: 2026-05-17

## 概述

本文档定义 9 项 IPC 控制点 (C1-C9) 的配置结构与运行时数据结构. 所有结构体以 Rust(编程语言) 类型定义, 配置部分支持 serde(序列化) 反序列化.

## 配置层 (Config Layer)

### IpcSecurityConfig (IPC 安全配置)

顶层配置, 聚合所有 9 项控制点的可配置参数. 存放在 `src/config/ipc_security.rs`.

```rust
/// Aggregated IPC security configuration loaded from YAML.
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
```

### PeerIdentityConfig (C1-C2)

```rust
/// Peer identity verification configuration.
///
/// C1: socket owner(套接字所有者) 校验 — the process that bound the socket
/// (this process) is the only allowed owner by definition.
/// C2: peer credentials(对端身份) 校验 — connecting process must match
/// configured identity expectations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerIdentityConfig {
    /// Whether peer credential checks are enabled. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Require peer uid(用户标识) to match this process uid. Default: true.
    #[serde(default = "default_true")]
    pub require_uid_match: bool,

    /// Allowed gid(组标识) list. Empty means gid check is disabled.
    /// Default: empty.
    #[serde(default)]
    pub allowed_gids: Vec<u32>,

    /// Allowed pid(进程标识) list. Empty means pid check is disabled.
    /// Pid checks are inherently racy and only useful in container
    /// environments with deterministic pids. Default: empty.
    #[serde(default)]
    pub allowed_pids: Vec<u32>,
}

fn default_true() -> bool { true }
```

### AuthorizationConfig (C3)

```rust
/// Command authorization matrix.
///
/// Maps each risk category to an allowed identity set.
/// Write commands (restart, shutdown, etc.) require authorized peer identity.
/// Read commands (hello, state) are always allowed when peer identity passes C1-C2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Whether authorization checks are enabled. Default: true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Commands classified as high-risk that require explicit authorization.
    /// Default: ["command.restart_child", "command.pause_child",
    ///           "command.resume_child", "command.quarantine_child",
    ///           "command.remove_child", "command.add_child",
    ///           "command.shutdown_tree"]
    #[serde(default = "default_high_risk_commands")]
    pub high_risk_commands: Vec<String>,

    /// Allowed peer uids for high-risk commands. Empty means deny all.
    /// Default: [0] (root only).
    #[serde(default = "default_root_only")]
    pub allowed_uids: Vec<u32>,
}

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

fn default_root_only() -> Vec<u32> { vec![0] }
```

### ReplayProtectionConfig (C4)

```rust
/// Replay protection configuration.
///
/// Uses a sliding window of seen request identifiers with a TTL(存活时间).
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

fn default_1024() -> usize { 1024 }
fn default_60() -> u64 { 60 }
```

### RequestSizeLimitConfig (C5)

```rust
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

fn default_65536() -> usize { 65536 }
```

### RateLimitConfig (C6)

```rust
/// Rate limit configuration.
///
/// Uses token bucket(令牌桶) algorithm per connection.
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

fn default_100_0() -> f64 { 100.0 }
fn default_20() -> u32 { 20 }
```

### AuditConfig (C7)

```rust
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
    /// - "defer_bounded": defer audit writes with bounded queue (max 1000 entries).
    /// Default: "fail_closed".
    #[serde(default = "default_fail_closed")]
    pub failure_strategy: String,

    /// Max queue size for "defer_bounded" strategy. Default: 1000.
    #[serde(default = "default_1000")]
    pub max_defer_queue: usize,
}

fn default_audit_backend() -> String { "memory".into() }
fn default_fail_closed() -> String { "fail_closed".into() }
fn default_1000() -> usize { 1000 }
```

### IdempotencyConfig (C8)

```rust
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

    /// TTL(存活时间) for cached command results in seconds. Default: 60.
    #[serde(default = "default_60")]
    pub result_cache_ttl_seconds: u64,

    /// Maximum cached results. Default: 1024.
    #[serde(default = "default_1024")]
    pub max_cached_results: usize,
}
```

### AllowlistConfig (C9)

```rust
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
    #[serde(default)]
    pub allowed_paths: Vec<String>,
}
```

## 运行时层 (Runtime Layer)

### PeerIdentity (C1-C2 运行时快照)

```rust
/// Snapshot of peer identity taken from a connected Unix socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdentity {
    /// Process identifier of the peer.
    pub pid: u32,
    /// User identifier of the peer.
    pub uid: u32,
    /// Group identifier of the peer.
    pub gid: u32,
}
```

### IpcRiskAction (C3 行索引)

```rust
/// IPC actions classified by risk level for authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IpcRiskAction {
    /// Read-only: hello, state, subscribe.
    Read,
    /// Write: restart, pause, resume, quarantine.
    WriteChild,
    /// Destructive: remove, shutdown.
    Destructive,
}

impl IpcRiskAction {
    /// Classifies an IPC method into its risk category.
    pub fn classify(method: &str) -> Self {
        match method {
            "hello" | "state" | "events.subscribe" | "logs.tail" => Self::Read,
            "command.restart_child" | "command.pause_child"
            | "command.resume_child" | "command.quarantine_child"
            | "command.add_child" => Self::WriteChild,
            "command.remove_child" | "command.shutdown_tree" => Self::Destructive,
            _ => Self::WriteChild, // Unknown commands: treat as write
        }
    }
}
```

### AuditRecord (C7)

```rust
/// Immutable audit record for a single IPC write request.
///
/// Carries at least: UTC timestamp, command enum, initiator identity hash,
/// optional correlation id(关联标识), adjudication boolean,
/// and structured error code on denial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    /// UTC timestamp with millisecond precision.
    pub timestamp: String,

    /// IPC method name.
    pub method: String,

    /// SHA256 hash of the initiator's peer identity (hex-encoded).
    pub initiator_hash: String,

    /// Optional correlation identifier for tracing.
    pub correlation_id: Option<String>,

    /// Whether the request was allowed.
    pub allowed: bool,

    /// Adjudication reason code when denied.
    pub denial_code: Option<String>,

    /// The control point that denied the request (C1-C9).
    pub denial_control_point: Option<String>,
}
```

### ReplayWindow (C4/C8 运行时状态)

```rust
use std::collections::HashMap;
use std::time::Instant;

/// Sliding window tracking seen request_ids with expiry.
pub struct ReplayWindow {
    entries: HashMap<String, Instant>,
    max_size: usize,
    ttl: std::time::Duration,
}

impl ReplayWindow {
    /// Tests whether a request_id is a replay.
    /// Returns true if already seen and not expired.
    pub fn is_replay(&self, request_id: &str) -> bool { /* ... */ }

    /// Records a new request_id, evicting oldest if at capacity.
    pub fn record(&mut self, request_id: String) { /* ... */ }

    /// Purges expired entries.
    pub fn purge_expired(&mut self) { /* ... */ }
}
```

### TokenBucket (C6 运行时状态)

```rust
/// Token bucket rate limiter.
pub struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    /// Attempts to consume one token. Returns true if allowed.
    pub fn try_consume(&mut self) -> bool { /* ... */ }
}
```

## 配置默认值汇总

| 控制点 | 字段                                 | 出厂默认      |
| ------ | ------------------------------------ | ------------- |
| C1     | peer_identity.enabled                | true          |
| C1     | peer_identity.require_uid_match      | true          |
| C2     | peer_identity.allowed_gids           | []            |
| C2     | peer_identity.allowed_pids           | []            |
| C3     | authorization.enabled                | true          |
| C3     | authorization.allowed_uids           | [0]           |
| C4     | replay_protection.enabled            | true          |
| C4     | replay_protection.window_size        | 1024          |
| C4     | replay_protection.ttl_seconds        | 60            |
| C5     | request_size_limit.enabled           | true          |
| C5     | request_size_limit.max_bytes         | 65536         |
| C6     | rate_limit.enabled                   | true          |
| C6     | rate_limit.refill_rate               | 100.0         |
| C6     | rate_limit.burst_capacity            | 20            |
| C7     | audit.enabled                        | true          |
| C7     | audit.backend                        | "memory"      |
| C7     | audit.failure_strategy               | "fail_closed" |
| C7     | audit.max_defer_queue                | 1000          |
| C8     | idempotency.enabled                  | true          |
| C8     | idempotency.result_cache_ttl_seconds | 60            |
| C8     | idempotency.max_cached_results       | 1024          |
| C9     | allowlist.enabled                    | true          |
| C9     | allowlist.allowed_paths              | []            |
