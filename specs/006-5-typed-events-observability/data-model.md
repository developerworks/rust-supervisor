# Data Model(数据模型): 类型化事件与端到端可追溯闭环

**Branch(分支)**: `006-5-typed-events-observability` | **Date(日期)**: 2026-05-18
**Source(来源)**: `specs/006-5-typed-events-observability/spec.md` + `research.md`

## Entities(实体)

### SupervisorEvent(监督器事件)

结构化事件, 控制循环中每条监督弧段对应一个 `SupervisorEvent` 实例.

| Field(字段)      | Type(类型)  | Required(必填) | Description(说明)                           |
| ---------------- | ----------- | -------------- | ------------------------------------------- |
| `schema_id`      | `u64`       | 是             | 事件 schema 版本号, 单调递增                |
| `sequence`       | `u64`       | 是             | 单调递增事件序号, 每个 SupervisorEvent 唯一 |
| `correlation_id` | `Uuid`      | 是             | 关联标识, UUID v4, 追踪同一起因的跨阶段记录 |
| `what`           | `What`      | 是             | 事件变体枚举, 携带该弧段的类型化字段        |
| `where`          | `Where`     | 是             | 位置元数据(路径, child_id, 主机等)          |
| `when`           | `EventTime` | 是             | 时间戳(unix_nanos + monotonic_nanos)        |
| `config_version` | `u64`       | 否             | 事件发射时生效的配置版本                    |

### What(事件变体枚举)

已有 30+ 变体(见 `src/event/payload.rs`), 本切片扩展以下新变体:

| Variant(变体)             | Fields(字段)                                                       | Arc(监督弧段) |
| ------------------------- | ------------------------------------------------------------------ | ------------- |
| `BudgetDenied`            | `group: Option<String>`, `reason: String`, `budget_remaining: f64` | 预算拒绝      |
| `GenerationFenced`        | `old_generation: u64`, `new_generation: u64`, `reason: String`     | 代次隔离      |
| `HealthCheckPassed`       | `age_ms: u64`, `healthy_since_unix_nanos: u128`                    | 健康检查通过  |
| `HealthCheckFailed`       | `reason: String`, `consecutive_failures: u32`                      | 健康检查失败  |
| `Paused`                  | `reason: String`, `paused_by: String`                              | 暂停监督      |
| `Resumed`                 | `reason: String`                                                   | 恢复监督      |
| `Quarantined`             | `scope: MeltdownScope`, `reason: String`, `duration_secs: u64`     | 隔离          |
| `BackpressureAlert`       | `subscriber: String`, `buffer_pct: u8`, `threshold_pct: u8`        | 背压告警      |
| `BackpressureDegradation` | `subscriber: String`, `strategy: String`, `sample_ratio: f64`      | 背压降级/采样 |
| `AuditRecorded`           | `command_id: String`, `event_type: String`, `sample_ratio: f64`    | 审计记录      |

### CorrelationHandle(关联句柄)

串联跨阶段记录的 correlation id(关联标识) 包装类型, 对人 API 暴露.

| Field(字段)      | Type(类型)  | Required(必填) | Description(说明) |
| ---------------- | ----------- | -------------- | ----------------- |
| `correlation_id` | `Uuid`      | 是             | UUID v4, 稳定标识 |
| `child_id`       | `ChildId`   | 否             | 关联的子任务标识  |
| `created_at`     | `EventTime` | 是             | 创建时间          |
| `event_count`    | `u64`       | 是             | 已关联事件数      |

### BackpressureConfig(背压配置)

| Field(字段)              | Type(类型)             | Required(必填) | Default(默认值) | Description(说明)  |
| ------------------------ | ---------------------- | -------------- | --------------- | ------------------ |
| `strategy`               | `BackpressureStrategy` | 是             | `AlertAndBlock` | 背压策略选择       |
| `warn_threshold_pct`     | `u8`                   | 否             | 80              | 告警软阈值(0-100)  |
| `critical_threshold_pct` | `u8`                   | 否             | 95              | 降级硬阈值(0-100)  |
| `window_secs`            | `u64`                  | 否             | 30              | 滑动窗口时间(秒)   |
| `audit_channel_capacity` | `usize`                | 否             | 1024            | audit 独立通道容量 |

#### BackpressureStrategy(背压策略枚举)

| Variant(变体)    | Description(说明)                    |
| ---------------- | ------------------------------------ |
| `AlertAndBlock`  | 告警并阻塞生产者, 不丢事件           |
| `SampleAndAudit` | 按采样率丢弃事件, audit 记录采样比例 |

## Relationships(关系)

```
CorrelationHandle
    │
    ├── 1:N ──► SupervisorEvent(按时间排序, 同 correlation_id)
    │
    ├── 0:1 ──► ChildId(关联的子任务标识, 可选)
    │
    └── 0:N ──► AuditRecord(高风险事件的审计记录)

SupervisorEvent
    ├── 1:1 ──► What(事件变体, 不可为空)
    ├── 1:1 ──► Where(位置元数据)
    ├── 1:1 ──► EventTime(时间戳)
    └── 1:1 ──► CorrelationId(关联标识)

BackpressureConfig
    └── 1:1 ──► BackpressureStrategy(策略选择)
```

## Validation Rules(校验规则)

1. **What 枚举完整性**: 控制循环中每条迁移弧必须对应至少一个 `What` 变体. 新增弧段时必须同步新增 `What` 变体或在已有变体上追加字段.
2. **CorrelationId 非空**: 每个 `SupervisorEvent` 的 `correlation_id` 不得为 nil UUID(全零).
3. **Schema ID 单调性**: `schema_id` 必须严格递增, 不得回退. 新增字段时 `schema_id+1`, 废弃字段时 `schema_id+1`.
4. **背压阈值范围**: `warn_threshold_pct` < `critical_threshold_pct`; 两者均必须在 [1, 100] 范围内.
5. **Audit 禁止采样**: 当 `audit_enabled: true` 时, `strategy` 为 `SampleAndAudit` 也不得对 audit 通道采样.
6. **序列化失败处理**: 若 `SupervisorEvent` 序列化失败, 控制循环不得 panic. 必须记录结构化错误到 stderr 并继续执行. 审计记录必须包含原始事件的 child_id 和 what 变体名.

## State Transitions(状态迁移)

本切片不改变监督生命周期状态机. 事件模型是状态迁移的投影(projection), 每条迁移弧发射一个对应的事件变体.

```
迁移弧                     ──►  What 变体
─────────────────────────────────────────────
spawn                      ──►  ChildStarting
spawn_failed               ──►  ChildFailed
ready                      ──►  ChildReady
health_check_passed        ──►  HealthCheckPassed(新增)
health_check_failed        ──►  HealthCheckFailed(新增)
failure_detected           ──►  ChildFailed / ChildPanicked
budget_denied              ──►  BudgetDenied(新增)
restart_scheduled          ──►  BackoffScheduled
restarting                 ──►  ChildRestarting
restarted                  ──►  ChildRestarted
generation_fenced          ──►  GenerationFenced(新增)
paused                     ──►  Paused(新增)
resumed                    ──►  Resumed(新增)
quarantined                ──►  Quarantined(新增)
stopped                    ──►  ChildStopped
shutdown_requested         ──►  ShutdownRequested
shutdown_completed         ──►  ShutdownCompleted
backpressure_alert         ──►  BackpressureAlert(新增)
backpressure_degradation   ──►  BackpressureDegradation(新增)
```
