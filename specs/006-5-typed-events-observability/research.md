# Research(研究): 类型化事件与端到端可追溯闭环

**Branch(分支)**: `006-5-typed-events-observability` | **Date(日期)**: 2026-05-18
**Status(状态)**: Final(定稿)

## Research Items(研究项)

### R001: CorrelationId 生成策略与生命周期

- **Decision(决策)**: 使用 UUID v4 作为 CorrelationId(关联标识) 的生成算法. 已在 `src/event/time.rs` 实现.
- **Rationale(理由)**: UUID v4 无需中心协调节点, 128 位随机空间碰撞概率在单 supervisor 生命周期内可忽略; 已作为 `uuid` crate 依赖引入. ULID(可排序唯一标识) 和带前缀的序列 ID 被否决: ULID 增加时间依赖, 前缀序列需要协调生成器.
- **Alternatives Considered(替代方案)**: (1) ULID — 增加时间戳依赖, 碰撞边界高于 UUID v4; (2) SupervisorInstance + monotonic counter — 需要跨重启持久化计数器, 复杂度高; (3) Hash(child_id + generation) — 无法保证全局唯一, 碰撞不可忽略.

### R002: CorrelationId 跨事件出口传播机制

- **Decision(决策)**: 三种传播方式同时使用: (a) 显式嵌入每个 SupervisorEvent 的 `correlation_id` 字段; (b) 写入 tracing span 的 `correlation.id` 标签; (c) 以 metrics label `correlation_id` 附加到事件计数指标.
- **Rationale(理由)**: 只嵌入事件 payload 会导致 tracing 和 metrics 无法关联到同一 ID, 违反 FR-003 的"链路不断"要求. 三通道同时携带 ID 的开销在可接受范围内(UUID v4 字符串 36 字节).
- **Alternatives Considered(替代方案)**: (1) 仅嵌入 event payload — tracing/metrics 断裂; (2) 仅通过 tracing span context 隐式传递 — 事件 journal 无法关联.

### R003: 背压策略选择方式(编译期 vs 运行时)

- **Decision(决策)**: 运行时配置开关. 在 `SupervisorSpec` 或 `EventConfig` 中新增 `backpressure_strategy` 枚举字段, 可选值: `AlertAndBlock`(告警并阻塞), `SampleAndAudit`(采样并记录).
- **Rationale(理由)**: 编译期 feature flag 无法在部署后切换, 生产环境遇到慢订阅者时只能重新编译. 运行时配置允许运维人员在不重启 supervisor 的前提下(若配置支持热加载)或仅需滚动重启时切换策略.
- **Alternatives Considered(替代方案)**: (1) 编译期 feature flag — 部署后不可切换; (2) 自适应算法自动切换 — 引入不可预测行为, 运维难以复盘.

### R004: 背压触发阈值量化

- **Decision(决策)**: 双阈值触发: (a) 缓冲区占用率 > 80%(软阈值, 触发告警); (b) 缓冲区占用率 > 95%(硬阈值, 触发降级停机). 配置项: `backpressure_warn_threshold_pct: u8`(默认 80), `backpressure_critical_threshold_pct: u8`(默认 95), `backpressure_window_secs: u64`(默认 30, 滑动窗口时间).
- **Rationale(理由)**: 单阈值无法区分"警告"与"必须行动". 80%/95% 的分级与通用运维实践一致.
- **Alternatives Considered(替代方案)**: (1) 单阈值(90%) — 无法分级告警; (2) 基于 subscriber 延迟(>100ms) — 延迟受系统负载影响大, 不如缓冲区占用率稳定.

### R005: 事件 Schema 版本化策略

- **Decision(决策)**: 使用单调递增的 `schema_id: u64` 作为版本号, 在 `SupervisorEvent` 中嵌入. 每次新增/废弃/重命名字段时递增. 向后兼容期: 当前版本和上一版本同时被 journal 回放支持.
- **Rationale(理由)**: SemVer(语义化版本) 过于重量级; 日期戳版本与发布节奏耦合. 单调递增整数版本足够用于区分事件格式, 配合 `serde` 的 `#[serde(deny_unknown_fields)]` 可在反序列化时检测不兼容版本.
- **Alternatives Considered(替代方案)**: (1) SemVer — 每个事件携带 major.minor.patch 开销大; (2) 日期戳 — 同一天多次发布冲突; (3) 无版本号 — 无法向后兼容.

### R006: Tracing 与 Metrics 标签基数硬上限

- **Decision(决策)**: 每个 tracing span 不超过 10 个标签; 每个标签键不超过 100 个唯一值. 超限时: 拒绝写入超限标签并在日志中发出警告事件(不 panic).
- **Rationale(理由)**: OpenTelemetry 实践建议每个 span 标签数 ≤ 10 以控制序列化开销; 100 个唯一值/标签键是考虑到 supervisor 可能管理数百个 child, `child_id` 标签将随 child 数量线性增长.
- **Alternatives Considered(替代方案)**: (1) 无上限 — 可能导致 tracing backend OOM; (2) panic 超限 — 过于激进, 生产环境不应因标签基数崩溃.

### R007: What 枚举演化规则

- **Decision(决策)**: 追加式演化: 新增变体只追加, 不重命名已有变体及其字段. 已有 `#[non_exhaustive]` 属性保持不变, 允许下游 `match` 编译通过.
- **Rationale(理由)**: 已有 `What` 枚举被多处 `match` 消费. 重命名现有变体或字段将导致编译错误, 且与 005-1 契约对齐断裂.
- **Alternatives Considered(替代方案)**: (1) 重命名对齐新命名规范 — 需要同步修改所有消费点; (2) 废弃旧变体 + 新增替代变体 — 更安全, 但会膨胀枚举.

### R008: 性能预算

- **Decision(决策)**: 单次 `SupervisorEvent` 事件构造 + 序列化 + 发射延迟 p99 < 10µs(微秒). 完整扇出到 journal/tracing/metrics/audit 四通道 p99 < 100µs. 通过微基准测试 `#[bench]` 验证.
- **Rationale(理由)**: 控制循环主路径延迟基线在 006-4 中为 p99 < 1ms. 事件发射增加的开销不应超过主路径的 10%. 10µs emit + 100µs fan-out 符合该比例.
- **Alternatives Considered(替代方案)**: (1) 异步发射 — 增加通道延迟和背压复杂度; (2) 同步发射 + 采样降级 — 本切片采用此方案.

### R009: 005-1 契约对齐

- **Decision(决策)**: 本切片新增的 `What` 变体集合必须覆盖 005-1 `pipeline-and-events.md` 中定义的 6 个阶段, 并在该契约中补充新增变体. 如果 005-1 缺少本切片需要的变体(如 `BudgetDenied`, `GenerationFenced`), 本切片扩展契约.
- **Rationale(理由)**: 005-1 契约定义了策略管线的稳定事件变体集合. 本切片不能绕过该契约新增变体而不同步更新.
- **Alternatives Considered(替代方案)**: (1) 仅在本切片规格中定义新增变体 — 导致契约分裂; (2) 等待 005-1 先更新 — 会阻塞本切片.

### R010: Audit 通道独立性

- **Decision(决策)**: audit(审计) 事件使用独立的 `tokio::sync::broadcast` 通道, 容量与主事件通道解耦. audit channel 容量默认 1024, 可配置. 当 audit channel 满时阻塞生产者(不采样 audit).
- **Rationale(理由)**: FR-002 要求"高风险改写动作默认禁止采样". 如果 audit 与主事件共享通道, 背压采样时可能误采样 audit 事件. 独立通道消除该风险.
- **Alternatives Considered(替代方案)**: (1) 共享通道 + 优先级标记 — 增加 fan-out 复杂度; (2) 共享通道 + audit 永不采样 — 但背压阻塞共享通道时 audit 同样被阻塞.
