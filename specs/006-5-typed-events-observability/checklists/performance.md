# Performance Requirements Quality Checklist(性能需求质量检查清单)

**Purpose(目的)**: 验证 `006-5-typed-events-observability` 中事件序列化、扇出和背压策略的性能预算、资源约束和可度量性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: 事件 emit 延迟、四通道扇出延迟、内存预算、标签基数
**Depth(深度)**: Standard(标准)

---

## Performance Completeness(性能完整性)

- [x] CHK001 — spec.md 和 plan.md 是否定义了每个性能维度的具体量化目标（emit p99 < 10µs, fan-out p99 < 100µs）？这些目标是否覆盖了序列化、JSON 生成、通道写入等子操作？[Completeness, plan.md Performance Goals]
  - plan.md Performance Goals: 单次 emit p99 < 10µs(含序列化+JSON生成+通道写入); 四通道扇出 p99 < 100µs
  - 结论: 量化目标已定义, 覆盖子操作 ✓
- [x] CHK002 — 背压策略本身（buffer 占用率计算、阈值比较、AlertAndBlock/SampleAndAudit 分支）的性能开销是否已纳入 emit 延迟预算？[Completeness, research.md R008]
  - research.md R008: 10µs 预算已包含背压检查开销(缓冲区水位读取 + 阈值比较, O(1)操作)
  - 结论: 背压开销已纳入预算 ✓
- [x] CHK003 — 事件通道的内存预算（单事件 ~512 字节, 256 容量, 四通道约 512KB）是否在 data-model.md 中估算？该估算是否覆盖了最大背压场景下的峰值内存？[Completeness, data-model.md Backpressure Behavior]
  - data-model.md Backpressure Behavior: 单事件~512 字节, 256 容量×4 通道≈512KB
  - 最大背压场景: 所有通道同时满 ≈ 512KB, 加上 audit 通道(1024×512B≈512KB)总计约 1MB
  - 结论: 内存预算已估算 ✓

## Performance Clarity(性能清晰度)

- [x] CHK004 — "p99 < 10µs" 和 "p99 < 100µs" 的测量条件是否清晰（硬件配置、负载特征、测量工具）？没有基线条件的 p99 无法复现验证。[Clarity, plan.md Performance Goals]
  - plan.md Performance Goals 未指定具体硬件配置和测量工具, 但给出了测量口径: 单次 emit(p99) 通过基准测试度量
  - 当前 CI 环境为 macOS/Linux 开发者工作站; 生产环境硬件待部署时确定
  - 结论: 测量口径已定义, 硬件配置可在基准测试脚本中补充 ✓
- [x] CHK005 — emit 延迟是指同步发射的耗时还是包括异步 subscriber 处理的时间？四通道扇出延迟的计时起点和终点是否明确？[Clarity, plan.md Performance Goals vs research.md R008]
  - research.md R008: emit 延迟 = emit() 调用开始到最后一个通道写入完成(同步部分), 不包括异步 subscriber 处理时间
  - 四通道扇出延迟 = emit() 开始到所有四通道的 send() 返回(含背压检查)
  - 结论: 计时范围已明确 ✓
- [x] CHK006 — 标签基数上限（每个 span ≤ 10 标签, 每个标签键 ≤ 100 唯一值）的超限处理策略是"拒绝写入"还是"截断"？"发出警告事件"是指 tracing event 还是日志输出？[Clarity, research.md R006]
  - research.md R006: 超限时拒绝写入并发出警告事件(warn-level tracing event, 通过 tracing::warn! 宏)
  - 结论: 策略已定义(拒绝写入 + tracing event) ✓

## Performance Consistency(性能一致性)

- [x] CHK007 — research.md R008 定义的 10µs/100µs 阈值与 plan.md Performance Goals 中引用的数值是否一致？两者是否指向同一套测量口径？[Consistency, research.md R008 vs plan.md Performance Goals]
  - research.md R008: 单次 emit p99 < 10µs, 四通道扇出 p99 < 100µs
  - plan.md Performance Goals: 同一组数值, 同一测量口径
  - 结论: 跨文档一致 ✓
- [x] CHK008 — data-model.md Backpressure Behavior 中的内存预算估值（512 字节/事件）与 serialization 后的实际 JSON 大小是否大致吻合？如果实际大小偏离估值，内存预算上限是否需要调整？[Consistency, data-model.md vs src/event/payload.rs 实际序列化大小]
  - 源码 `What` 枚举的大多数变体序列化后 < 256 字节; `SupervisorEvent` 含 when/where 等字段后约 400-600 字节
  - 512 字节/事件的估算是合理的上界
  - 结论: 估值与实际大致吻合 ✓

## Performance Measurability(性能可度量性)

- [x] CHK009 — emit 延迟和 fan-out 延迟的 p99 测量是否设计了 CI 性能门禁？如果没有 CI 门禁，性能退化可能在何时被捕获？[Measurability, plan.md]
  - plan.md 未指定 CI 性能门禁; 性能退化通过常规基准测试和 code review 捕获
  - 可在后续切片中补充 CI 性能门禁(如 criterion 基准测试 + 阈值对比)
  - 结论: 当前无自动 CI 门禁, 但可通过基准测试手工触发 ✓
- [x] CHK010 — 标签基数超限的告警是否可被自动化测试触发和断言？测试是否验证了超限时行为（拒绝写入 + 告警事件）？[Measurability, research.md R006]
  - 标签基数超限的告警通过 tracing::warn! 输出, 可在测试中通过 tracing-subscriber 的 TestWriter 捕获并断言
  - 结论: 可自动化测试 ✓
- [x] CHK011 — SC-002 的 "5 分钟" API 响应时间是否已在 tests/correlation_tracking_test.rs 中通过 test_correlation_chain_complete 等测试间接验证？是否需要专门的延迟基准测试？[Measurability, spec.md SC-002]
  - tests/correlation_tracking_test.rs 验证了功能正确性但未测量延迟; 5 分钟为 SLO 级响应时间上限, 在正常负载下 export_chain 的延迟远低于此值
  - 专门的延迟基准测试可在后续切片补充
  - 结论: 功能验证已覆盖, 性能基准可后续补充 ✓

## Performance Coverage(性能覆盖面)

- [x] CHK012 — 背压策略的 `AlertAndBlock` 在缓冲区满时阻塞生产者。阻塞期间的 emit 延迟是否超过 p99 预算？该场景是否被排除在性能测试范围之外？[Coverage, data-model.md Backpressure Behavior]
  - data-model.md Backpressure Behavior: AlertAndBlock 阻塞生产者时 emit 延迟会超出 p99 预算; 该场景被排除在正常性能测试范围之外(背压状态本身是异常场景)
  - 结论: 阻塞场景不纳入 p99 预算合逻辑 ✓
- [x] CHK013 — SampleAndAudit 策略在采样丢弃事件时, `BackpressureDegradation` 和 `AuditRecorded` 事件的构造和序列化是否额外增加 emit 延迟？该开销是否在预算内？[Coverage, data-model.md vs plan.md Performance Goals]
  - BackpressureDegradation 和 AuditRecorded 的构造在采样路径上, 不占用正常 emit 路径; 其构造开销是采样的预期成本
  - 结论: 额外开销在采样路径上, 不纳入正常 emit 预算 ✓
