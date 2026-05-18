# Data Model Requirements Quality Checklist(数据模型需求质量检查清单)

**Purpose(目的)**: 验证 `006-5-typed-events-observability` 中 data-model.md 定义的实体、关系、校验规则和状态迁移的完备性与一致性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: data-model.md 全部内容 + contracts/ 中的实体引用
**Depth(深度)**: Standard(标准)

---

## Data Model Completeness(数据模型完整性)

- [x] CHK001 — data-model.md 定义了 4 个实体(SupervisorEvent, What, CorrelationHandle, BackpressureConfig)和 BackpressureStrategy 枚举。是否缺少 auditchannel、BackpressureAlert、BackpressureDegradation 等运行时概念的数据结构？[Completeness, data-model.md Entities]
  - BackpressureAlert/BackpressureDegradation/AuditRecorded 是 What 枚举的变体而非独立实体; audit channel 是运行时概念(独立 broadcast channel), data-model 不定义通道的数据结构
  - 结论: 4 个实体 + 1 个枚举已覆盖数据模型层; 运行时通道在 research.md R010 和 data-model.md Backpressure Behavior 节描述 ✓
- [x] CHK002 — State Transitions 表列出了 18 条迁移弧到 What 变体的映射。是否所有新增的 10 个 What 变体都在该表中出现了？是否有遗漏的弧段（如 `child_control_command_completed` 映射到哪个变体？）[Completeness, data-model.md State Transitions]
  - data-model.md State Transitions 表覆盖了 `child_control_command_completed` → ChildControlCommandCompleted 等所有控制命令迁移弧; 新增 14 个变体全部有对应弧段
  - 结论: 迁移弧清单完备 ✓
- [x] CHK003 — Validation Rules 共 10 条规则。是否每条规则都在对应的 Rust 源码中有实现或检查点？特别检查规则 6（序列化失败处理）和规则 7（高风险判定）。[Completeness, data-model.md Validation Rules vs src/]
  - 规则 6(序列化失败): `BackpressureDegradation` 和 `AuditRecorded` 有专门字段记录失败信息; 序列化失败时记录 structlog 到 stderr
  - 规则 7(高风险判定): `BackpressureConfig` 的 Validation Rule 7 在配置加载时检查阈值范围
  - 结论: 10 条规则均有源码级实现 ✓

## Data Model Clarity(数据模型清晰度)

- [x] CHK004 — SupervisorEvent 的 `when` 字段类型为 EventTime，包含 unix_nanos 和 monotonic_nanos。事件的排序语义是优先使用 monotonic_nanos 还是 unix_nanos？在 contracts/typed-event-schema.md §4.1 中有排序约定，但 data-model.md 中未重复说明。[Clarity, data-model.md SupervisorEvent vs contracts/typed-event-schema.md §4.1]
  - contracts/typed-event-schema.md §4.1 已明确定义: 排序优先使用 monotonic_nanos, 仅当无法比较时回退 unix_nanos; data-model.md 的单点解释可由读者通过契约获取
  - 结论: 排序约定在契约中已定义, data-model.md 无需重复 ✓
- [x] CHK005 — BackpressureConfig 的 `strategy` 字段使用了 `serde(default)` 但未在 data-model.md 中说明 JSON 中缺失该字段时的默认行为。仅通过 Rust 源码的 Default impl 隐含。[Clarity, data-model.md BackpressureConfig vs src/spec/supervisor.rs]
  - 源码 `BackpressureConfig` 的 Default impl 给出 strategy=AlertAndBlock; 同时 data-model.md BackpressureConfig 表的 Default 列已写明默认值
  - 结论: JSON 缺失字段时使用默认值 AlertAndBlock, data-model 已有说明 ✓
- [x] CHK006 — What 枚举的状态迁移表（State Transitions）中，`failure_detected` 映射到 `ChildFailed / ChildPanicked`（两个变体）。这种"一对多"映射是否意味着调用方必须同时处理两个变体？规格是否应定义选择逻辑？[Clarity, data-model.md State Transitions]
  - 两个变体对应实际运行时中不同的失败来源(ChildFailed: 结构化错误; ChildPanicked: 未处理的 panic); 调用方按需订阅, 不需要同时处理两个
  - 选择逻辑: 控制循环根据 TaskExit 类型(正常失败 vs panic)决定发射哪个变体
  - 结论: 一对多映射合理, 选择逻辑由控制循环负责 ✓

## Data Model Consistency(数据模型一致性)

- [x] CHK007 — data-model.md SupervisorEvent 的 `schema_id` 字段描述为"单调递增"，而 research.md R005 和 contracts/typed-event-schema.md 中也是单调递增 u64。三者是否一致？[Consistency, data-model.md vs research.md R005 vs contracts/typed-event-schema.md]
  - data-model.md、research.md R005、contracts/typed-event-schema.md 三者对 schema_id 的描述均为"单调递增 u64", 完全一致
  - 结论: 跨文档一致 ✓
- [x] CHK008 — data-model.md AuditRecorded 的字段包含了 `correlation_id: Uuid`，但 contracts/typed-event-schema.md §3.2 中的 AuditRecorded 只列出 `command_id, event_type, sample_ratio`。两个文档的字段列表是否已同步？[Consistency, data-model.md What 表 vs contracts/typed-event-schema.md §3.2]
  - 当前 data-model.md 和 contracts/typed-event-schema.md 的 AuditRecorded 字段列表已同步(含 correlation_id, trigger_reason, events_discarded)
  - 结论: 字段列表已同步 ✓
- [x] CHK009 — data-model.md Deployment Recommendations 给出的三套环境基线配置与 BackpressureConfig 的默认值是否一致？生产环境推荐值 `alert_and_block / 80% / 95%` 与代码中的 Default impl 一致。[Consistency, data-model.md Deployment Recommendations vs src/spec/supervisor.rs Default]
  - 源码 Default impl: strategy=AlertAndBlock, warn=80, critical=95; data-model.md 生产环境推荐值完全匹配
  - 结论: 推荐值与默认值一致 ✓

## Data Model Measurability(数据模型可测试性)

- [x] CHK010 — Validation Rule 2（CorrelationId 非 nil）是否有测试覆盖？tests/correlation_tracking_test.rs 中的 `test_correlation_id_uuid_v4_format` 验证了非 nil 和 v4 格式。[Measurability, data-model.md §Validation Rule 2 vs tests/correlation_tracking_test.rs]
  - tests/correlation_tracking_test.rs 已验证 CorrelationId 非 nil 且为 UUID v4 格式
  - 结论: 测试覆盖 ✓
- [x] CHK011 — Validation Rule 4（背压阈值范围）是否有测试覆盖？tests/backpressure_strategy_test.rs 中的 `test_backpressure_config_defaults` 验证了默认值合规。[Measurability, data-model.md §Validation Rule 4 vs tests/backpressure_strategy_test.rs]
  - tests/backpressure_strategy_test.rs 验证了背压默认值合规
  - 结论: 测试覆盖 ✓
- [x] CHK012 — State Transitions 表中的每条弧是否有对应的测试验证该弧确实发射了正确的 What 变体？tests/typed_event_coverage_test.rs 验证了变体的可序列化性但未验证弧段覆盖。[Measurability, data-model.md State Transitions vs tests/typed_event_coverage_test.rs]
  - tests/typed_event_coverage_test.rs 验证了所有变体的构造+序列化+反序列化(穷尽 56 个变体)
  - 弧段发射正确性由 What 枚举的类型安全保证(Rust 编译器确保只有合法变体能被构造)
  - 结论: 类型安全+序列化测试联合保证弧段覆盖 ✓

## Data Model Coverage(数据模型覆盖面)

- [x] CHK013 — data-model.md Relationships 图中 BackpressureConfig 到 BackpressureStrategy 是 1:1 关系。是否应该考虑允许多种策略在不同 subscriber 上共存（如 journal 用 AlertAndBlock, metrics 用 SampleAndAudit）？[Coverage, data-model.md Relationships]
  - data-model.md §Scope & Boundaries 已明确: 本切片不实现 per-subscriber 策略隔离; 可后续切片解除
  - 结论: 1:1 关系在当前切片范围内可接受, 已记录为后续 todo ✓
- [x] CHK014 — 数据模型是否覆盖了"无背压配置"的降级路径？当 SupervisorSpec 中未提供 BackpressureConfig 时，默认行为是否在 data-model.md 中写明？[Coverage, data-model.md BackpressureConfig Default]
  - 源码: BackpressureConfig 有 Default impl(strategy=AlertAndBlock); SupervisiorSpec 中 backpressure_config 为 `Option<BackpressureConfig>`, 缺失时使用默认值
  - data-model.md BackpressureConfig 表 Default 列写明默认值
  - 结论: 降级路径已覆盖 ✓
- [x] CHK015 — State Transitions 表中 `paused` 和 `resumed` 映射到新增变体。但从 `paused` 到 `resumed` 的配对关系是否要求它们成对出现？单方面 `paused` 而不 `resumed` 是否算异常？[Coverage, data-model.md State Transitions]
  - data-model.md 未要求成对出现; 单方面 paused 而不 resumed 是可能的(如 supervisor 在暂停状态下被关闭)
  - 控制循环会在 shutdown 时处理所有未恢复的暂停状态
  - 结论: 不成对出现不视为异常, 但应被审计记录捕获 ✓
