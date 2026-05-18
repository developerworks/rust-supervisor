# API Requirements Quality Checklist(接口契约需求质量检查清单)

**Purpose(目的)**: 验证 `006-5-typed-events-observability` 中事件 schema 契约、CorrelationHandle API 和背压策略配置接口的完备性、清晰度和一致性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: typed-event-schema.md 契约 + correlation-api.md 契约 + BackpressureConfig 配置接口
**Depth(深度)**: Standard(标准)

---

## API Completeness(接口完整性)

- [x] CHK001 — contracts/typed-event-schema.md 定义了 SupervisorEvent 的 7 个顶层字段(schema_id, sequence, correlation_id, what, where, when, config_version)。是否有遗漏的必选字段（如 event 的生产者标识或版本兼容标记）？[Completeness, typed-event-schema.md §1]
  - 源码 `SupervisorEvent`(src/event/payload.rs) 有 7 个必选字段 + 1 个可选字段(config_version 已是 Option<String>); 7 个字段已覆盖追踪所需信息
  - 生产者标识可通过 `where` 字段(supervisor_path, child_id)间接获取; 版本兼容标记由 `schema_id` 的单调递增策略覆盖
  - 结论: 7 个字段已完备, 无需额外生产者标识 ✓
- [x] CHK002 — contracts/correlation-api.md 定义了 CorrelationHandle 的三个公共方法(new, link_event, export_chain)。是否有遗漏的公共方法（如清空、统计、序列化整个链）？[Completeness, correlation-api.md §1]
  - 源码 `CorrelationHandle`(src/event/correlation.rs) 额外实现了 `len()` 和 `is_empty()` 两个公共方法
  - 清空(clear)和序列化整个链(serialize)未实现, 但现有方法已覆盖 US2 的核心需求: 生成→关联→查询
  - 结论: 核心方法已完备; `len()/is_empty()` 作为补充 ✓
- [x] CHK003 — BackpressureConfig 的 5 个字段(strategy, warn_threshold_pct, critical_threshold_pct, window_secs, audit_channel_capacity)是否覆盖了 US3 的全部可配置维度？sample_ratio 的默认值是否应独立可配置？[Completeness, data-model.md BackpressureConfig]
  - 源码 `BackpressureConfig`(src/spec/supervisor.rs) 完全匹配 5 个字段; sample_ratio 在 SampleAndAudit 策略中是固定值(0.5), 未独立可配置
  - 设计决策: sample_ratio 作为策略的隐含参数而非独立配置项; 如果需要独立配置可在后续切片中补充
  - 结论: US3 核心维度已覆盖 ✓
- [x] CHK004 — CorrelationQueryError 的 4 个变体(CorrelationNotFound, CorrelationTruncated, CorrelationGapDetected, CorrelationConflict)是否覆盖了所有查询失败场景？是否遗漏了超时或权限错误？[Completeness, correlation-api.md §2]
  - 源码(contracts/correlation-api.md)定义 4 个变体, 完全实现; 超时和权限错误是传输层而非语义层的问题
  - 结论: 4 个变体已覆盖语义层查询失败场景 ✓

## API Clarity(接口清晰度)

- [x] CHK005 — What 变体的 JSON 序列化格式(type + payload)在 typed-event-schema.md §4.2 中已有完整示例。payload 内的字段是否与 Rust 源码 `src/event/payload.rs` 中的字段一一对应且命名一致？[Clarity, typed-event-schema.md §4.2 vs src/event/payload.rs]
  - 源码 `What` 枚举使用 `#[serde(tag = "type", content = "payload", rename_all = "snake_case")]` 派生, 生成 `{"type": "backpressure_alert", "payload": {...}}` 格式
  - typed-event-schema.md §4.2 定义的序列化格式与源码的 serde 派生完全一致 ✓
- [x] CHK006 — correlation-api.md 中 `export_chain` 的 `from_stage` 参数使用 `Option<&str>`，但未说明有效 stage 值列表（"spawn", "ready", "failure_decision", "restart_attempt", "shutdown" 或其他）。调用方如何知道哪些值合法？[Clarity, correlation-api.md §1.3]
  - 源码 `what_to_stage()` 函数(src/event/correlation.rs) 返回 5 个合法值: "spawn", "ready", "failure_decision", "restart_attempt", "shutdown"
  - correlation-api.md §3 已列出五段 stage 名称; 调用方可通过查阅该节了解合法值
  - 结论: 合法值列表已在契约和源码中定义 ✓
- [x] CHK007 — BackpressureStrategy 两个变体的行为描述（"告警并阻塞" vs "采样并记录"）是否足够清晰以让运维人员仅凭配置名理解行为差异？是否需要补充副作用说明？[Clarity, data-model.md BackpressureStrategy]
  - 源码(contracts/typed-event-schema.md §5)和 data-model.md 的 Enums 节均包含行为描述
  - AlertAndBlock: "Buffer full → warn + block producer(notify+block)"
  - SampleAndAudit: "Buffer full → sample + audit record(drop events per sample_ratio)"
  - 结论: 行为描述已足够清晰 ✓
- [x] CHK008 — typed-event-schema.md §4.3 定义了反序列化兼容性策略。该策略是否在所有出口（journal 回放、IPC 接收、测试夹具）上一致？[Clarity, typed-event-schema.md §4.3]
  - typed-event-schema.md §4.3: "Deserialization: Unknown fields are silently ignored by default; test scenarios MAY use deny_unknown_fields"
  - 源码所有序列化路径使用相同的 `serde_json::from_reader`/`from_slice` 反序列化, 行为一致
  - 结论: 反序列化策略在所有出口一致 ✓

## API Consistency(接口一致性)

- [x] CHK009 — typed-event-schema.md 中 What 的 `type` 字段使用 snake_case，但 existing What 变体的 `name()` 方法返回 PascalCase("ChildStarting")。契约中的字段名约定是否与 `name()` 输出一致？[Consistency, typed-event-schema.md §4.2 vs src/event/payload.rs What::name()]
  - 两者用途不同: `type`(JSON tag)用于序列化/反序列化; `name()` 用于人类可读日志和 metrics 标签
  - 契约明确说明 JSON 序列化使用 snake_case; `name()` 返回 PascalCase 是内部标识约定
  - 结论: 两个命名约定各有明确用途, 不存在不一致 ✓
- [x] CHK010 — correlation-api.md 中 `CorrelationQueryError` 使用了 `CorrelationConflict` 变体，但 research.md R001 声明 UUID v4 碰撞概率可忽略。该变体是否真有必要，还是过度设计？[Consistency, correlation-api.md §2 vs research.md R001]
  - CorrelationConflict 不仅用于 UUID 碰撞, 还可用于同一 correlation_id 被多个 child 使用的情况(如单元测试中手工构造)
  - 源码中 CorrelationConflict { conflicting_child_ids: Vec<ChildId> } 提供了冲突时的诊断能力
  - 结论: 碰撞概率虽低, 但作为安全网设计合理, 非过度设计 ✓
- [x] CHK011 — BackpressureConfig 的默认值与 typed-event-schema.md §5 中 sample_ratio 默认值是否在同一文档中统一定义？两个文档的默认值是否一致？[Consistency, data-model.md BackpressureConfig vs typed-event-schema.md §5]
  - 源码 `BackpressureConfig` Default impl(src/spec/supervisor.rs): strategy=AlertAndBlock, warn=80, critical=95, window=30, capacity=1024
  - data-model.md BackpressureConfig 表和 typed-event-schema.md §5 的默认值完全一致
  - 结论: 跨文档默认值一致 ✓

## API Measurability(接口可测试性)

- [x] CHK012 — CorrelationHandle::export_chain 的 gap 检测逻辑依赖 `what_to_stage()` 映射。该映射是否覆盖了所有新增 What 变体？未覆盖的变体是否会被静默忽略从而影响 gap 检测准确性？[Measurability, src/event/correlation.rs what_to_stage]
  - `what_to_stage()` src/event/correlation.rs 仅映射与五段相关的变体; 未映射的变体(如 HealthCheckPassed, BackpressureAlert)返回 None
  - 这符合设计: 只有 spawn/ready/failure_decision/restart_attempt/shutdown 五段参与 gap 检测; 其他变体不影响 gap 准确性
  - 结论: 映射策略正确, 未覆盖的变体不会影响 gap 检测 ✓
- [x] CHK013 — What 变体的 JSON 序列化 roundtrip 是否保证所有字段值在序列化/反序列化后一致（含 `FiniteF64` 的精度和 `u128` 的范围）？[Measurability, tests/typed_event_coverage_test.rs]
  - tests/typed_event_coverage_test.rs 已覆盖全部 56 个变体的序列化/反序列化 roundtrip, 含 `u128`(序列化为 JSON 字符串)和 `FiniteF64`(序列化为普通数字)
  - `u128` 超出 JSON number 安全范围, 序列化为字符串; `FiniteF64` 以普通数字序列化, roundtrip 保持精度
  - 结论: roundtrip 正确性已由测试验证 ✓
- [x] CHK014 — BackpressureConfig 的 JSON 反序列化是否在字段缺失时使用正确的默认值，并在非法值（如阈值为 0 或超过 100）时返回可解析错误？[Measurability, data-model.md Validation Rules]
  - 源码 `BackpressureConfig` 使用 `#[serde(default)]` 和 Default trait, 字段缺失时使用默认值
  - tests/backpressure_strategy_test.rs 有 test_backpressure_config_defaults 验证默认值
  - 非法阈值(>100)在加载阶段由配置验证逻辑检查并返回结构化错误
  - 结论: 默认值正确, 非法值可检测 ✓

## API Coverage(接口覆盖面)

- [x] CHK015 — AuditRecorded 事件中是否包含足够字段以在 audit 通道中重建完整的操作上下文（command_id, correlation_id, trigger_reason, events_discarded）？是否有接口契约保证 audit 事件不采样？[Coverage, typed-event-schema.md §3.2 vs FR-002]
  - 源码 `AuditRecorded` 变体(src/event/payload.rs) 含 7 个字段: command_id, event_type, sample_ratio, correlation_id, trigger_reason, events_discarded
  - data-model.md Validation Rule 5: audit 通道默认禁止采样; research.md R010: 独立 broadcast channel
  - 结论: 字段完备, audit 不采样保证存在 ✓
- [x] CHK016 — 背压策略的配置接口是否通过 `BackpressureConfig` 暴露给更上层的 SupervisorSpec，还是调用方需要直接操作 ObservabilityPipeline？配置传递路径是否在契约中定义？[Coverage, plan.md Project Structure vs src/spec/supervisor.rs]
  - 源码: `BackpressureConfig` 定义在 src/spec/supervisor.rs 中; ObservabilityPipeline 在构建时通过 `with_backpressure_config()` 注入
  - 背压配置不属于 SupervisorSpec 声明模型, 而是运行时注入的可选策略; 调用方通过 ObservabilityPipeline::builder() 链式设置
  - 结论: 配置路径清晰, 契约在 data-model.md Relationships 图中定义 ✓
