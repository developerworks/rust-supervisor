# Contract(契约): Correlation Tracking API(关联追踪接口)

本文件定义 `CorrelationHandle`(关联句柄) 的公共 API 契约. 调用方和验收夹具依赖此契约完成 correlation id(关联标识) 的查询与校验.

## 1. CorrelationHandle(关联句柄)

### 1.1 创建

```rust
/// Creates a new correlation handle.
///
/// # Arguments
/// - `correlation_id`: UUID v4 that identifies this tracking chain.
/// - `child_id`: Optional child identifier for scoped queries.
///
/// # Returns
/// A new `CorrelationHandle`.
pub fn new(correlation_id: CorrelationId, child_id: Option<ChildId>) -> Self;
```

### 1.2 关联事件

```rust
/// Links a supervisor event to this correlation handle.
///
/// The event is stored in chronological order. Duplicate sequence
/// numbers are rejected.
///
/// # Arguments
/// - `event`: The supervisor event to associate.
///
/// # Returns
/// `Ok(())` on success, `Err(SequenceAlreadyRegistered)` if the
/// event's sequence was already linked.
pub fn link_event(&mut self, event: SupervisorEvent) -> Result<(), SequenceAlreadyRegistered>;
```

### 1.3 导出事件链

```rust
/// Exports all linked events in chronological order.
///
/// # Arguments
/// - `from_stage`: Optional stage filter (e.g., "spawn", "ready").
///
/// # Returns
/// A vector of `SupervisorEvent` sorted by `when.unix_nanos`, or
/// a `CorrelationQueryError` if gaps are detected.
pub fn export_chain(&self, from_stage: Option<&str>) -> Result<Vec<SupervisorEvent>, CorrelationQueryError>;
```

## 2. CorrelationQueryError(关联查询错误)

```rust
pub enum CorrelationQueryError {
    /// No events found for the given correlation ID.
    CorrelationNotFound { correlation_id: CorrelationId },
    /// Event chain is truncated due to log rotation or journal capacity.
    CorrelationTruncated {
        correlation_id: CorrelationId,
        total_events: u64,
        max_events: u64,
    },
    /// One or more lifecycle stages are missing from the chain.
    CorrelationGapDetected {
        correlation_id: CorrelationId,
        /// Set of lifecycle stages that are missing (e.g., "ready", "shutdown").
        missing_stages: Vec<String>,
        /// Stages that are present in the chain.
        present_stages: Vec<String>,
    },
    /// Sequence collision detected (possible UUID collision).
    CorrelationConflict {
        correlation_id: CorrelationId,
        conflicting_child_ids: Vec<ChildId>,
    },
}
```

## 3. 五段覆盖校验

调用方使用 `CorrelationHandle::export_chain()` 后, 必须验证返回的事件链是否覆盖下列五个强制阶段:

| 阶段(Stage)        | 对应 What 变体                                             | 强制(Mandatory) |
| ------------------ | ---------------------------------------------------------- | --------------- |
| `spawn`            | `ChildStarting`                                            | 是              |
| `ready`            | `ChildReady` / `HealthCheckPassed`                         | 是              |
| `failure_decision` | `ChildFailed` / `ChildPanicked` / `BudgetDenied`           | 是(若失败发生)  |
| `restart_attempt`  | `ChildRestarting` / `BackoffScheduled`                     | 是(若重启发生)  |
| `shutdown`         | `ChildStopped` / `ShutdownRequested` / `ShutdownCompleted` | 是(若关闭发生)  |

缺失阶段必须在 `CorrelationGapDetected.missing_stages` 中列出.

## 4. 使用示例

```rust
let handle = CorrelationHandle::new(correlation_id, Some(child_id));

// Simulate lifecycle events
handle.link_event(spawn_event)?;
handle.link_event(ready_event)?;
handle.link_event(failure_event)?;
handle.link_event(restart_event)?;
handle.link_event(shutdown_event)?;

// Export and verify
let chain = handle.export_chain(None)?;
assert_eq!(chain.len(), 5);
assert!(chain.windows(2).all(|w| w[0].when.unix_nanos <= w[1].when.unix_nanos));
```
