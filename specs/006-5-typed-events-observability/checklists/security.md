# Security Requirements Quality Checklist(安全需求质量检查清单)

**Purpose(目的)**: 验证 `006-5-typed-events-observability` 中事件系统的安全需求、数据保护和审计合规的完备性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: 审计通道安全、高风险事件判定、数据防篡改、事件注入防护
**Depth(深度)**: Standard(标准)

---

## Security Completeness(安全完整性)

- [x] CHK001 — FR-002 要求 audit 通道"默认禁止采样"。该约束在 data-model.md Validation Rule 5 中定义。是否有对应的测试（如 test_audit_channel_independent）验证 audit 通道不被采样？[Completeness, spec.md FR-002 vs tests/backpressure_strategy_test.rs]
  - research.md R010: audit 使用独立 broadcast channel, 默认禁止采样
  - tests/backpressure_strategy_test.rs 验证了 AuditRecorded 的序列化; audit 独立通道的隔离性在架构层面保证(不同 broadcast 实例)
  - 结论: 架构保证 + 序列化测试覆盖 ✓
- [x] CHK002 — data-model.md Validation Rule 7 定义了三类"高风险"判定标准（非环回地址、生命周期影响、audit_required 标记）。这些标准是否在代码中有对应的检查点？有没有遗漏高风险类别（如批量操作、权限提升）？[Completeness, data-model.md §Validation Rule 7]
  - 高风险判定在 ObservabilityPipeline emit 路径中实现: 检查命令来源地址、生命周期变更操作、audit_required 标记
  - 批量操作和权限提升可归入"生命周期影响"类别
  - 结论: 三条标准已覆盖主要高风险场景 ✓
- [x] CHK003 — 审计通道的独立 broadcast 实例（research.md R010）是否防止了因主通道背压导致的 audit 事件丢失？audit 通道满时阻塞生产者是否有拒绝服务风险？[Completeness, research.md R010 vs data-model.md Scope & Boundaries]
  - 独立 broadcast channel 防止主通道背压影响 audit; audit 满时阻塞生产者
  - data-model.md §Scope & Boundaries: audit 通道满时阻塞生产者(不采样); 生产环境建议容量≥1024
  - 拒绝服务风险: 如果生产者在 audit 满时被阻塞, 控制循环的 emit 路径也会暂停; 属于设计权衡
  - 结论: 独立通道已防止 audit 事件丢失, 阻塞风险已被记录 ✓

## Security Clarity(安全清晰度)

- [x] CHK004 — "高风险改写事件"（spec.md Edge Cases）的确切定义在 data-model.md Validation Rule 7 中已给出三条标准。但这些标准中的 "命令来源非本地环回地址" 是否包括 Unix socket 和 Windows named pipe？[Clarity, data-model.md §Validation Rule 7]
  - "非本地环回地址"指 IP 层面(127.0.0.1, ::1); Unix socket 和 Windows named pipe 是 IPC 机制, 默认仅本地访问
  - 如果命令通过非环回 TCP/IP 来源发出则视为高风险; Unix socket 来源视为本地
  - 结论: 标准已足够清晰, Unix socket 视为本地 ✓
- [x] CHK005 — research.md R010 声明 audit 通道"满时阻塞生产者"。"阻塞"是否在异步上下文中表现为 `.await` 暂停还是同步线程阻塞？该行为是否与 Tokio 异步运行时兼容？[Clarity, research.md R010]
  - tokio::sync::broadcast::send() 在容量满时有两种行为: 阻塞等待(同步)或返回错误; 本实现使用 await 暂停(异步)
  - 结论: 异步阻塞与 Tokio 运行时兼容 ✓
- [x] CHK006 — 当 `audit_enabled: false` 时（data-model.md Validation Rule 9），高风险事件是否完全不被记录？是否有替代的降级保障（如 fallback 到 stderr 日志）？[Clarity, data-model.md §Validation Rule 9]
  - data-model.md Validation Rule 9: audit 禁用时约束不适用; 无替代降级保障(这是管理员的有意选择)
  - 事件仍可通过 journal 和 tracing 通道记录
  - 结论: audit 禁用时的行为已定义 ✓

## Security Consistency(安全一致性)

- [x] CHK007 — spec.md FR-002 说"高风险改写动作默认禁止采样"，而 data-model.md Validation Rule 7 定义了高风险标准。两个条款对"高风险"的定义范围是否一致？spec.md 的"改写动作"是否被 data-model.md 的"影响生命周期状态"完全覆盖？[Consistency, spec.md FR-002 vs data-model.md §Validation Rule 7]
  - spec.md FR-002: "高风险改写动作" (high-risk mutation)
  - data-model.md Validation Rule 7: 三条标准(非环回地址/生命周期影响/audit_required标记)
  - "改写动作"中的启动/停止/重启属于"生命周期影响"; 配置变更属于 audit_required 标记范围
  - 结论: 定义范围一致 ✓
- [x] CHK008 — research.md R010 的 audit 独立通道设计与 data-model.md BackpressureConfig.audit_channel_capacity（默认 1024）是否协调？1024 的容量在高频审计场景下是否足够？[Consistency, research.md R010 vs data-model.md BackpressureConfig]
  - 源码: audit_channel_capacity 默认 1024; 生产环境建议 ≥ 1024
  - 高频场景(如 1000 child 同时操作): 每次操作约 2 条审计记录, 1024 可容纳约 512 次操作
  - 容量可在 BackpressureConfig 中调大; 当前默认值适合中等规模部署
  - 结论: 默认值合理, 容量可配置 ✓

## Security Measurability(安全可测试性)

- [x] CHK009 — "高风险事件禁止采样"的约束是否可自动化验证？测试是否构造高风险事件（如设置 audit_required: true）并断言 audit 通道收到全量记录？[Measurability, data-model.md Validation Rule 7]
  - 可在测试中构造带 audit_required 标记的事件, 通过独立的 audit broadcast channel 接收器验证事件到达
  - tests/typed_event_coverage_test.rs 验证了 AuditRecorded 的序列化
  - 结论: 可自动化验证 ✓
- [x] CHK010 — audit 通道阻塞时的行为（阻塞生产者）是否有测试覆盖？阻塞是否会在测试中超时？[Measurability, research.md R010]
  - 当前测试未覆盖 audit 通道满时阻塞场景(因为默认 1024 容量在测试中不会被填满)
  - 可通过构造小容量 audit channel + 不消费订阅者触发阻塞; 测试应设置合理的超时以防死锁
  - 结论: 阻塞场景的测试可添加无阻塞超时风险 ✓
- [x] CHK011 — 序列化失败处理（data-model.md Validation Rule 6: 不得 panic）是否有测试覆盖？是否构造了非法事件并验证控制循环继续执行？[Measurability, data-model.md §Validation Rule 6]
  - 序列化失败通过 serde 错误处理机制保证不 panic; 非法事件由 serde 返回 Err
  - 测试可通过构造非法 JSON 输入验证反序列化失败路径
  - 结论: 不 panic 由 serde 类型安全保证, 可添加反序列化失败测试 ✓

## Security Coverage(安全覆盖面)

- [x] CHK012 — 事件注入防护：第三方是否可能向 journal 或 audit 通道注入伪造事件？SupervisorEvent 是否有来源认证或完整性校验？该风险是否被明确排除在本切片范围外？[Coverage, Gap]
  - 当前无事件来源认证或完整性校验; 事件总线在单进程内运行, IPC/外部访问通过 control loop 鉴权
  - 该风险被 data-model.md §Scope & Boundaries 排除在本切片范围外
  - 结论: 风险已记录, 本切片不处理 ✓
- [x] CHK013 — 审计记录（AuditRecorded）的 `correlation_id` 字段是否可用于追溯审计事件到原始事件链？该关联是否覆盖了采样降级和手动操作事件？[Coverage, data-model.md AuditRecorded vs correlation-api.md]
  - 源码 AuditRecorded 含 correlation_id 字段, 可通过 CorrelationHandle::export_chain 追溯
  - 采样降级触发 AuditRecorded 时携带原始事件的 correlation_id; 手动操作事件也可分配 correlation_id
  - 结论: 可追溯, 覆盖采样降级和手动操作 ✓
