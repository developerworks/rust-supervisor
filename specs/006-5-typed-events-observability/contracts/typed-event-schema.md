# Contract(契约): Typed Event Schema(类型化事件方案)

本文件约束 `006-5-typed-events-observability` 交付时调用方或验收夹具能够依赖的稳定事件 schema(方案) 语义. Rust 类型实现必须与本契约字段同名或在本契约末尾 `Alias mapping(别名映射)` 表中登记.

## 1. SupervisorEvent(监督器事件) 顶层字段

所有 `SupervisorEvent` 实例必须包含下列顶层字段:

| Canonical field(标准字段) | Type(类型)  | Serialized format(序列化格式)                              | Required(必填) |
| ------------------------- | ----------- | ---------------------------------------------------------- | -------------- |
| `schema_id`               | `u64`       | JSON number                                                | 是             |
| `sequence`                | `u64`       | JSON number                                                | 是             |
| `correlation_id`          | `Uuid`      | JSON string(e.g. `"550e8400-e29b-41d4-a716-446655440000"`) | 是             |
| `what`                    | `What`      | JSON object(type + payload fields)                         | 是             |
| `where`                   | `Where`     | JSON object                                                | 是             |
| `when`                    | `EventTime` | JSON object(unix_nanos + monotonic_nanos)                  | 是             |
| `config_version`          | `u64`       | JSON number                                                | 否(缺省为 0)   |

## 2. Correlation ID(关联标识) 契约

`correlation_id` 字段必须使用 UUID v4(随机 UUID) 格式. 不得使用 nil UUID(全零) 作为有效值. 非空断言是验收测试的必过项.

### 2.1 传播契约

| 出口(Export)  | 位置(Location)                   | 格式(Format)   | 必需(Mandatory)    |
| ------------- | -------------------------------- | -------------- | ------------------ |
| Event journal | `SupervisorEvent.correlation_id` | UUID v4 string | 是                 |
| Tracing span  | `correlation.id` 标签            | UUID v4 string | 是                 |
| Metrics       | `correlation_id` label           | UUID v4 string | 是(仅事件计数指标) |

### 2.2 查询契约

`CorrelationHandle` 查询 API 必须支持:

- **Input**: `correlation_id: Uuid`(必填), `child_id: Option<ChildId>`(可选过滤)
- **Output**: 按时间升序排列的 `Vec<SupervisorEvent>`, 或 `CorrelationQueryError`(结构化错误)
- **错误变体**:
  - `CorrelationNotFound` — 指定 ID 不存在
  - `CorrelationTruncated` — 日志轮转或 journal 容量限制导致记录不完整
  - `CorrelationGapDetected { missing_stages: Vec<String> }` — 五段中有缺失

## 3. What(事件变体) 枚举契约

### 3.1 已有变体(保持向后兼容)

已有变体列表见 `src/event/payload.rs` 中 `What` 枚举定义. 本契约不重述.

### 3.2 本切片新增变体

| Variant(变体)             | Payload fields(载荷字段)                                           | 对应监督弧段 |
| ------------------------- | ------------------------------------------------------------------ | ------------ |
| `BudgetDenied`            | `group: Option<String>`, `reason: String`, `budget_remaining: f64` | 预算拒绝     |
| `GenerationFenced`        | `old_generation: u64`, `new_generation: u64`, `reason: String`     | 代次隔离     |
| `HealthCheckPassed`       | `age_ms: u64`, `healthy_since_unix_nanos: u128`                    | 健康检查通过 |
| `HealthCheckFailed`       | `reason: String`, `consecutive_failures: u32`                      | 健康检查失败 |
| `Paused`                  | `reason: String`, `paused_by: String`                              | 暂停监督     |
| `Resumed`                 | `reason: String`                                                   | 恢复监督     |
| `Quarantined`             | `scope: MeltdownScope`, `reason: String`, `duration_secs: u64`     | 隔离         |
| `BackpressureAlert`       | `subscriber: String`, `buffer_pct: u8`, `threshold_pct: u8`        | 背压告警     |
| `BackpressureDegradation` | `subscriber: String`, `strategy: String`, `sample_ratio: f64`      | 背压降级     |
| `AuditRecorded`           | `command_id: String`, `event_type: String`, `sample_ratio: f64`    | 审计记录     |

### 3.3 What 枚举演化规则

- **追加式演化**: 新增变体只追加在枚举末尾, 不插入中间位置.
- **禁止重命名**: 已有变体名称和字段名称不得重命名.
- **禁止移除**: 已有变体不得移除(可废弃但保留定义).
- **废弃标注**: 废弃变体使用 `#[deprecated]` 属性标注, 定义保留.
- **字段冻结时间线**: 本契约中列出的字段名从 spec 进入 Draft(草稿) 状态起冻结. 任何字段名变更必须: (a) 更新本契约; (b) 递增 `schema_id`; (c) 在 `CHANGELOG.md` 中记录迁移脚注.

## 4. 序列化格式

默认序列化格式为 JSON(simd-json 或 serde_json). 后续可扩展 MessagePack.

### 4.1 时间戳序列化

`EventTime.unix_nanos` 序列化为 JSON number(nanoseconds since Unix epoch). `monotonic_nanos` 序列化为 JSON number(monotonic clock nanoseconds since unspecified epoch).

**排序约定**: 事件排序优先使用 `monotonic_nanos`(单调时钟), 避免 NTP 时钟跳变导致的排序错误. 仅当两个事件的 `monotonic_nanos` 无法比较(如来自不同进程)时回退到 `unix_nanos`.

### 4.3 反序列化兼容性

- **未知字段处理**: 默认静默忽略(兼容未来版本). 验证/测试场景可使用 `#[serde(deny_unknown_fields)]` 严格模式.
- **缺失可选字段**: 使用 Rust 的 `Option` 类型默认值填充, 不报错.
- **类型不匹配**: 返回 `serde` 反序列化错误, 调用方必须处理.

**排序约定**: 事件排序优先使用 `monotonic_nanos`(单调时钟), 避免 NTP 时钟跳变导致的排序错误. 仅当两个事件的 `monotonic_nanos` 无法比较(如来自不同进程)时回退到 `unix_nanos`.

### 4.2 枚举序列化

`What` 变体序列化为 JSON object, 包含一个 `type` 字段(变体名, snake_case)和一个 `payload` 字段(变体特有的字段). 例:

```json
{
  "schema_id": 1,
  "sequence": 42,
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
  "what": {
    "type": "budget_denied",
    "payload": {
      "group": "worker-pool-a",
      "reason": "budget exhausted in 60s window",
      "budget_remaining": 0.0
    }
  },
  "where": { ... },
  "when": {
    "unix_nanos": 1716019200000000000,
    "monotonic_nanos": 123456789
  },
  "config_version": 3
}
```

## 5. 背压策略契约

配置文件必须定义一个 `backpressure_strategy` 字段, 类型为 `string`, 可选值:

| Value(取值)          | Meaning(含义)                                                        |
| -------------------- | -------------------------------------------------------------------- |
| `"alert_and_block"`  | 缓冲满时告警并阻塞生产者, 不丢事件                                   |
| `"sample_and_audit"` | 缓冲满时按 `sample_ratio`(默认 0.5) 丢弃事件, audit 记录实际采样比例 |

**采样率范围**: `sample_ratio` 值必须在 `[0.01, 1.0]` 范围内, 步长 0.01. 低于 0.01 视为未配置(使用默认值).

## 6. Alias mapping(别名映射)

| Contract term(契约术语) | Current code anchor(当前代码锚点)                        | Migration note(迁移说明)                |
| ----------------------- | -------------------------------------------------------- | --------------------------------------- |
| `SupervisorEvent`       | `SupervisorEvent` in `src/event/payload.rs`              | 追加 `schema_id`, `config_version` 字段 |
| `What`                  | `What` enum in `src/event/payload.rs`                    | 追加 10 个新变体                        |
| `CorrelationId`         | `CorrelationId` in `src/event/time.rs`                   | 保持 UUID v4                            |
| `CorrelationHandle`     | `CorrelationHandle`(新增) in `src/event/correlation.rs`  | 新增模块                                |
| `BackpressureConfig`    | `BackpressureConfig`(新增) in `src/spec/supervisor.rs`   | 新增配置类型                            |
| `BackpressureStrategy`  | `BackpressureStrategy`(新增) in `src/spec/supervisor.rs` | 新增枚举                                |
