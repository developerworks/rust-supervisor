# Research(研究): 生产级重启策略

**Feature(功能)**: `006-4-restart-policy-production`
**Date(日期)**: 2026-05-18

## 研究问题

### 问题 1: RestartBudget(重启预算) 跟踪算法

**Decision(决定)**: 采用滑动窗口计数器 + 令牌桶混合模型.

**Rationale(理由)**:
- 纯滑动窗口适合 burst(突发) 检测, 但无法限制窗口内密集重试
- 纯令牌桶适合限速, 但无法追溯历史故障密度
- 混合模型: 滑动窗口判定当前故障密度是否超阈值, 令牌桶限制有效重启速率
- `RestartBudgetTracker` 内部维护 `window_failures: VecDeque<u128>`(时间戳队列) 和 `token_bucket: u32`(当前令牌计数)
- 每次故障: 入队时间戳, 扣减令牌; 窗口外时间戳出队时归还令牌
- 每次成功重启: 按恢复速率归还令牌(最多至上限)

**Alternatives considered(考虑过的替代方案)**:
- Leaky bucket(漏桶): 不适合, 因为故障是离散事件而非连续流量
- Fixed window counter(固定窗口): 有边界切换问题, 滑窗更平滑
- 纯滑动窗口: 被拒绝, 因为无法限制密集重试的绝对速率

**Implementation(实现)**:
- 新文件 `src/policy/budget.rs`
- `RestartBudgetConfig` 包含: `window: Duration`, `max_burst: u32`, `recovery_rate_per_sec: f64`
- `RestartBudgetTracker` 包含: `config`, `failures: VecDeque<u128>`, `tokens: f64`, `last_update: u128`
- `fn try_consume(&mut self, now_unix_nanos: u128) -> BudgetVerdict` 返回 `Granted` 或 `Exhausted { retry_after_ns: u128 }`

### 问题 2: FairnessProbe(公平性探针)

**Decision(决定)**: 控制循环主路径插入轻量探针, 每 N 次迭代记录一次调度机会.

**Rationale(理由)**:
- 不需要额外的后台线程或定时器, 避免增加并发复杂度
- 探针在 control_loop 每次处理完一个事件后递增 `scheduling_opportunity_counter`
- 每 10 秒窗口内, 记录至少 N 个不同 child_id 获得过调度
- 如果某个 child 连续被跳过超过阈值, 发射 `What::FairnessProbeStarvation` 事件

**Alternatives considered(考虑过的替代方案)**:
- 后台 tokio task 轮询: 被拒绝, 增加不必要的异步任务
- 在策略评估阶段插入: 被拒绝, 策略评估可能被熔断跳过, 探针应独立

**Implementation(实现)**:
- 新文件 `src/observe/fairness.rs`
- `FairnessProbe` 结构体: `scheduling_opportunities: u64`, `per_child_ops: HashMap<ChildId, u64>`, `last_probe_unix_nanos: u128`
- `fn record_opportunity(&mut self, child_id: &ChildId)` 记录一次调度机会
- `fn check(&self, now_unix_nanos: u128) -> Option<FairnessProbeResult>` 返回饥饿检测结果

### 问题 3: GroupStrategy(分组策略) 隔离验证

**Decision(决定)**: 在 MeltdownTracker 中扩展 group(分组) 维度的计数器, 添加跨组依赖边检测.

**Rationale(理由)**:
- 现有 `MeltdownTracker` 已支持 child/group/supervisor 三级计数
- 缺的是 group 之间的隔离断言和跨组依赖边的声明
- 当 group A 触发熔断时, 只有声明了 `depends_on: [group_a]` 的 group B 才受影响
- 未声明依赖的 group 必须继续正常调度

**Alternatives considered(考虑过的替代方案)**:
- 独立 GroupIsolationValidator: 被拒绝, 与 MeltdownTracker 职责重叠
- 零信任模型(所有 group 默认隔离): 被采纳为基础策略, 依赖边声明为显式例外

**Implementation(实现)**:
- 新文件 `src/policy/group.rs`
- `GroupDependencyEdge` 结构体: `from_group: String`, `to_group: String`, `failure_propagation: PropagationPolicy`
- `GroupIsolationPolicy` 结构体: `dependencies: Vec<GroupDependencyEdge>`, 提供 `fn affected_by(&self, group: &str, failed_group: &str) -> bool`
- `PropagationPolicy` 枚举: `None`(不传播), `EscalateOnly`(仅升级父监督器), `Full`(完全传播)

### 问题 4: Critical/Optional(关键/可选) 分叉观测

**Decision(决定)**: 在 `EffectivePolicy` 中增加 `severity` 字段, 在事件发射链路中注入 `SeverityClass` 标签.

**Rationale(理由)**:
- 已有 `WorkRole` 角色分类, 但角色不等同于严重程度
- 同一 `Service` 角色内部可以有 critical 和 optional 实例
- `SeverityClass` 独立于 `WorkRole`, 由配置声明
- 事件发射时自动附加 `severity_class` 标签, metrics 打点时自动附加对应维度

**Alternatives considered(考虑过的替代方案)**:
- 复用 `WorkRole` 做严重程度判断: 被拒绝, 混淆了两个正交维度
- 在事件 payload 中硬编码: 被拒绝, 不利于后期新增分类

**Implementation(实现)**:
- 在 `src/policy/role_defaults.rs` 或 `src/spec/child.rs` 中新增 `SeverityClass` 枚举
- 枚举值: `Critical`(关键), `Optional`(可选), `Standard`(默认)
- `EffectivePolicy` 新增 `severity: SeverityClass` 字段
- `What` 事件枚举中新增 `EscalationBifurcated` 变体, 携带 `severity: SeverityClass`
- 事件发射函数自动从 `EffectivePolicy` 读取 severity 并写入事件

### 问题 5: 统一评估管线 (Budget + Backoff + Meltdown 集成)

**Decision(决定)**: 在现有 `SupervisionPipeline` 六阶段中增强 "evaluate budget" 阶段, 将 budget, backoff, meltdown 串入单次评估.

**Rationale(理由)**:
- 现有六阶段: classify exit → record window → evaluate budget → decide action → emit events → execute action
- "evaluate budget" 阶段当前只做简单的 failure_window 计数
- 需要扩展为: budget.try_consume() → 若拒绝则熔断 → meltdown.track() → 若熔断则升级
- 保持顺序: budget 先于 meltdown(预算不足直接拒绝, 不经过熔断), meltdown 先于 backoff(熔断后不计算退避)

**Alternatives considered(考虑过的替代方案)**:
- 并行评估: 被拒绝, 三个子系统有顺序依赖(budget → meltdown → backoff)
- 在 control_loop 中硬编码顺序: 被拒绝, 应该封装在 pipeline 中

**Implementation(实现)**:
- 修改 `src/runtime/pipeline.rs` 的 `evaluate_budget` 阶段
- 注入 `RestartBudgetTracker` 和 `GroupIsolationPolicy` 引用
- 返回 `BudgetEvaluation` 包含: `verdict: BudgetVerdict`, `fuse: Option<MeltdownOutcome>`, `backoff_delay: Duration`

---

## 风险点

1. **预算令牌回收精度**: 滑动窗口的时间戳出队必须与令牌归还原子化, 避免预算误耗尽或误恢复.
2. **公平性探针性能**: 探针在控制循环主路径上, 必须 O(1) 完成, 避免 HashMap 遍历.
3. **分组依赖边声明格式**: 需要与 006-6(config-dynamic-children) 的配置模型对齐, 避免重复声明格式.
