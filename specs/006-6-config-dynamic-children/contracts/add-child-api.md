# Contract(契约): Add Child API(追加子任务接口)

本文件定义 `add_child` 动态追加子任务的 RPC 接口契约.

## 1. 调用签名

```rust
/// Adds a child to the current supervisor's topology at runtime.
///
/// # Arguments
/// - `declaration`: ChildDeclaration parsed from the RPC payload.
///
/// # Returns
/// - `Ok(AddChildResponse)` on successful completion.
/// - `Err(AddChildError)` on any failure (including compensating).
pub async fn add_child(
    &mut self,
    declaration: ChildDeclaration,
) -> Result<AddChildResponse, AddChildError>;
```

## 2. 响应类型

```rust
pub struct AddChildResponse {
    /// Transaction ID for audit tracing.
    pub transaction_id: Uuid,
    /// Current phase after successful execution (always `Committed`).
    pub phase: Phase,
    /// Child ID assigned by the runtime.
    pub child_id: ChildId,
    /// Supervisor spec hash after this operation.
    pub spec_hash: String,
}
```

## 3. 错误类型

```rust
pub enum AddChildError {
    /// Another add_child transaction is in progress.
    TransactionInProgress,
    /// Child name conflicts with an existing child.
    ChildNameConflict { name: String },
    /// Dependency refers to a non-existent child.
    DependencyNotFound { dependency: String },
    /// Dependency graph contains a cycle.
    DependencyCycle { nodes: Vec<String> },
    /// Secret reference syntax is invalid (validation_failed).
    SecretSyntaxError { field_path: String, detail: String },
    /// Secret reference is syntactically valid but vault is offline or
    /// the referenced key is missing (runtime_secret_miss).
    SecretRuntimeMissing { field_path: String, secret_name: String },
    /// Resource limit is not supported by the host kernel.
    ResourceLimitNotSupported { field: String },
    /// Supervisor is shutting down; add_child is not allowed.
    SupervisorShuttingDown,
    /// Child limit (max 1000) would be exceeded.
    ChildLimitExceeded {
        /// Maximum allowed children.
        max: u32,
        /// Current child count.
        current: u32,
    },
    /// Audit storage write failure (ring buffer full or I/O error).
    AuditStorageFailure {
        /// Human-readable failure detail.
        detail: String,
    },
    /// The add_child transaction failed and was compensated.
    TransactionCompensated {
        transaction_id: Uuid,
        phase: String,
        error: String,
    },
}
```

## 4. 事务边界

add_child 在 `ConfigState` 级别持有互斥锁期间执行. 调用方应认为 add_child 返回前所有副作用(注册、拉起、审计)已按以下顺序发生:

1. 解析 ChildDeclaration → JSON/YAML 输入格式校验
2. 校验字段合法性 → 返回结构化错误(含 field_path)
3. 注册到拓扑 → ConfigState.registry 更新
4. 启动 child → TaskFactory 创建 + spawn
5. 审计持久化 → 写入 CompensatingRecord + 最终审计条目

## 5. 幂等性

当前不支持幂等. 重复提交同一 declaration 会导致 `ChildNameConflict` 错误(如果 child 已注册)或重复注册(如果第一次事务被补偿). 调用方应通过 `transaction_id` 去重.
