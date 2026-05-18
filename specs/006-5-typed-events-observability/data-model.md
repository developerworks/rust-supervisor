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

| Variant(变体)             | Fields(字段)                                                                                                                               | Arc(监督弧段) |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------- |
| `BudgetDenied`            | `group: Option<String>`, `reason: String`, `budget_remaining: f64`                                                                         | 预算拒绝      |
| `GenerationFenced`        | `old_generation: u64`, `new_generation: u64`, `reason: String`                                                                             | 代次隔离      |
| `HealthCheckPassed`       | `age_ms: u64`, `healthy_since_unix_nanos: u128`                                                                                            | 健康检查通过  |
| `HealthCheckFailed`       | `reason: String`, `consecutive_failures: u32`                                                                                              | 健康检查失败  |
| `Paused`                  | `reason: String`, `paused_by: String`                                                                                                      | 暂停监督      |
| `Resumed`                 | `reason: String`                                                                                                                           | 恢复监督      |
| `Quarantined`             | `scope: MeltdownScope`, `reason: String`, `duration_secs: u64`                                                                             | 隔离          |
| `BackpressureAlert`       | `subscriber: String`, `buffer_pct: u8`, `threshold_pct: u8`                                                                                | 背压告警      |
| `BackpressureDegradation` | `subscriber: String`, `strategy: String`, `sample_ratio: f64`, `buffer_peak_pct: u8`, `recovered: bool`                                    | 背压降级/采样 |
| `AuditRecorded`           | `command_id: String`, `event_type: String`, `sample_ratio: f64`, `correlation_id: Uuid`, `trigger_reason: String`, `events_discarded: u64` | 审计记录      |

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

### Backpressure Behavior(背压行为定义)

- **告警严重级别**: 软阈值(80%)触发时发射 `warn` 级别 tracing event + `BackpressureAlert` 事件; 硬阈值(95%)触发时发射 `error` 级别 tracing event + `BackpressureDegradation` 事件.
- **采样率范围**: `[0.01, 1.0]`, 步长 0.01. 默认 `sample_ratio = 0.5`.
- **降级范围**: 仅影响触发背压的单个 subscriber; 其他 subscriber 不受影响.
- **恢复机制**: 当缓冲区占用率连续 3 个 `window_secs` 周期低于 `warn_threshold_pct` 时, 自动恢复正常(停止采样或解除阻塞).
- **Broadcast 通道容量**: 默认 256. 容量满时 `AlertAndBlock` 策略阻塞生产者; `SampleAndAudit` 策略按采样率丢弃.
- **通道隔离**: audit(审计) 通道与普通 event(事件) 通道物理隔离, 有独立的 `audit_channel_capacity`(默认 1024) 配置. 背压触发时优先保障 audit(审计) 通道全量写入, 普通 event(事件) 通道按策略采样或阻塞. audit 通道满时阻塞生产者(不采样 audit), 以符合 FR-002 的高风险改写动作禁止采样的要求.
- **默认策略**: 背压策略默认配置为 `AlertAndBlock`(告警并阻塞, 不丢事件), 平台提供者可改为 `SampleAndAudit`(采样并记录审计). 此二选一的选择在默认配置文件中固化后不得在运行时由控制命令动态切换.
- **内存预算估值**: 单事件约 512 字节(含序列化开销). 256 容量 × 512 字节 ≈ 128KB 每通道. 四通道约 512KB. audit 通道独立预算: 1024 容量 × 512 字节 ≈ 512KB.

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
7. **"高风险" 判定标准**: 满足以下任一条件的事件为高风险: (a) 命令来源非本地环回地址; (b) 命令影响受监督单元的生命周期状态(启动/停止/重启/关闭); (c) 事件携带 `audit_required: true` 标记. 高风险事件禁止采样.
8. **Schema 版本治理**: schema_id 的晋升由技术负责人(tech lead)在 PR 审阅时批准. 每次晋升必须在 `CHANGELOG.md` 中记录迁移脚注, 包含: 变更摘要、变更字段列表、兼容性类型(向后兼容/不兼容).
9. **audit_enabled: false 行为**: 当 `audit_enabled: false` 时, audit 通道不发射任何事件; "禁止采样"约束自然不适用. 不提供替代防护措施, 因为 audit 禁用是管理员的有意选择.
10. **Journal 满行为**: 事件 journal(`src/journal/ring.rs`) 在容量满时丢弃最旧事件, 始终保留最新事件. `dropped_count` 计数器跟踪丢弃总数. 此行为与背压策略独立.

## Deployment Recommendations(部署推荐)

### 环境配置基线

| 环境 | backpressure_strategy   | warn_threshold_pct | critical_threshold_pct | window_secs | audit_channel_capacity |
| ---- | ----------------------- | ------------------ | ---------------------- | ----------- | ---------------------- |
| 开发 | `alert_and_block`       | 90                 | 98                     | 60          | 256                    |
| 预发 | `alert_and_block`       | 85                 | 96                     | 30          | 512                    |
| 生产 | `alert_and_block`(推荐) | 80                 | 95                     | 30          | 1024                   |

### 配置热加载

本切片不支持运行时配置热加载. 背压策略更改需要重启 supervisor 实例生效. 此项限制可在后续切片(如 006-6 动态配置)解除.

### Scope & Boundaries(范围与边界)

- **背压场景范围**: 本切片仅覆盖 event subscriber(事件订阅者) 慢消费的背压场景. command channel 满、IPC connection 风暴、event bus 内部缓冲区溢出等其他背压场景不在本切片范围, 由后续切片或基础设施层处理.
- **订阅者隔离**: 本切片不实现 per-subscriber 独立缓冲区. 一个慢订阅者可能影响其他共享同一 broadcast channel 的订阅者. 此项限制可在后续切片中通过独立广播通道或 per-subscriber 队列解除.
- **Audit channel 瓶颈**: 当 audit channel 满时, 生产者被阻塞(不采样 audit). 这保持了"禁止采样"的承诺但可能反压控制循环. 生产环境应配置充足的 audit_channel_capacity(推荐 ≥ 1024).
- **机器可读格式**: `What` 枚举定义在 `src/event/payload.rs` 中, 是 Rust 类型系统的第一等成员, 可被 `cargo doc` 和 IDE 工具解析. 虽无独立 DOT 图, 但枚举定义本身是机器可读的架构事实来源.

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
