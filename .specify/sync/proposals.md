# Drift Resolution Proposals(漂移修复提案)

Generated(生成时间): 2026-05-17T04:59:22+08:00
Based on(基于): `.specify/sync/drift-report.json`, generated `2026-05-17T04:07:27+08:00`
Scope(范围): `005-1-failure-policy-reliability`

一句话结论: 当前应优先按 `Spec -> Code(按规格改代码)` 修复 6 项漂移, 并把 `SC-004` 的 `CV(变异系数)` 基准定义交给人工确认, 因为固定延迟基准的 `CV` 为 0 时比值公式不可直接使用.

## Summary(摘要)

| Resolution Type(修复类型) | Count(数量) |
|---------------------------|-------------|
| Backfill(按代码补规格) | 0 |
| Align(按规格改代码) | 6 |
| Human Decision(人工决策) | 1 |
| New Specs(新规格) | 0 |
| Remove from Spec(从规格移除) | 0 |

## Interactive Cues(交互提示)

对每个 `Proposal(提案)`, 你可以用一个字母批复: `A` 表示 approve(批准), `R` 表示 reject(否决), `M` 表示 modify(修改, 后面写明改法), `S` 表示 skip(跳过), `Q` 表示 quit(停止).

当前游标: `DONE(完成)`.

---

### Proposal P1: `005-1-failure-policy-reliability` / `FR-001`

**Resolution Status(决议状态)**: `APPROVED(已批准)`, 2026-05-17T05:02:37+08:00

**Direction(方向)**: `ALIGN(按规格改代码)`

**Current State(当前状态)**:
- Spec says(规格要求): 每一条受监督单元的运行结束情形都必须进入固定 6 阶段 `policy pipeline(策略流水线)`, 并且禁止跳过流水线直接自动重启.
- Code does(代码行为): `src/runtime/control_loop.rs` 在真实退出路径中直接刷新重启限制, 计算 `restart_decision(重启决定)`, 然后执行重启. `src/runtime/pipeline.rs` 有 6 阶段骨架, 但是没有被真实控制循环调用, 第 6 阶段仍是占位逻辑.

**Proposed Resolution(建议修复)**:
- 让 `RuntimeControlState(运行时控制状态)` 在子任务退出路径中构造 `PipelineContext(流水线上下文)` 并调用 `SupervisionPipeline::execute_pipeline`.
- 把 `TaskExit(任务退出)` 映射补齐到最小集合: `success(成功)`, `nonzero_exit(非零退出)`, `panic(崩溃)`, `timeout(超时)`, `external_cancel(外部取消)`, `manual_stop(人工停止)`.
- 把自动重启, 排队重启, 拒绝重启和监督停止放到 `execute action(执行动作)` 阶段执行.
- 为每个阶段输出可对账的 `PipelineStageDiagnostic(流水线阶段诊断)` 或等价事件字段.
- 把 `tests/supervisor_pipeline_order.rs` 和 `tests/supervisor_restart_limit_usage.rs` 从占位验证改成真实运行路径验证.

**Rationale(理由)**: 规格和契约都把 6 阶段顺序写成硬约束. 如果把当前绕过流水线的行为回填进规格, 会直接取消 `005-1` 的核心目标.

**Confidence(置信度)**: `HIGH(高)`

**Action(操作)**:
- [x] Approve
- [ ] Reject
- [ ] Modify
- [ ] Skip

---

### Proposal P2: `005-1-failure-policy-reliability` / `FR-002`

**Resolution Status(决议状态)**: `APPROVED(已批准)`, 2026-05-17T05:03:12+08:00

**Direction(方向)**: `ALIGN(按规格改代码)`

**Current State(当前状态)**:
- Spec says(规格要求): `MeltdownTracker(熔断跟踪器)` 必须按 `child(子任务)`, `group(分组)`, `supervisor(监督器)` 3 层互不混算地保存状态, 事件必须输出 `scopes_triggered(已触发作用域列表)` 和 `lead_scope(主导归因作用域)`.
- Code does(代码行为): `MeltdownTracker` 当前只有 3 个聚合队列, 每次子任务失败会同时写入 3 个队列. 事件发出时没有填充 `scopes_triggered` 与 `lead_scope`.

**Proposed Resolution(建议修复)**:
- 将 `MeltdownTracker` 的状态改成按 `ChildId(子任务标识)`, `group_id(分组标识)` 和监督器实例分别键控.
- 让每层产出 `local verdict(局部判定)`, 并映射到 `ProtectionAction(保护动作)` 的从严档位.
- 合并时先取最严格档位, 只有多个作用域同样严格时才按 `child`, `group`, `supervisor` 做 `tie-break(平局判定)`.
- 在真实 `emit typed event(发出类型化事件)` 阶段填充 `scopes_triggered`, `lead_scope` 和 `effective_protective_action(生效保护处置)`.

**Rationale(理由)**: 分组隔离是 `FR-002` 和 `SC-002` 的验收基础. 当前聚合队列会让局部失败占用其他层配额, 这和规格目标相反.

**Confidence(置信度)**: `HIGH(高)`

**Action(操作)**:
- [x] Approve
- [ ] Reject
- [ ] Modify
- [ ] Skip

---

### Proposal P3: `005-1-failure-policy-reliability` / `FR-003`

**Resolution Status(决议状态)**: `APPROVED(已批准)`, 2026-05-17T05:05:46+08:00

**Direction(方向)**: `ALIGN(按规格改代码)`

**Current State(当前状态)**:
- Spec says(规格要求): 线上自动重启必须支持 `full jitter(全抖动)`, `decorrelated jitter(去相关抖动)`, 最大并发重启限制, `cold start budget(冷启动预算)`, `hot loop detection(热循环检测)`, 并支持可重复测试模式.
- Code does(代码行为): 相关类型和算法已经存在, 但是真实 `control_loop(控制循环)` 只使用默认 `BackoffPolicy::new`, 默认 `jitter_mode(抖动模式)` 为 `Disabled(关闭)`. 冷启动预算, 热循环检测和并发闸门没有接入真实重启路径.

**Proposed Resolution(建议修复)**:
- 在 `evaluate budget(评估预算)` 阶段读取和更新冷启动预算, 热循环检测和并发闸门状态.
- 在 `decide action(决定动作)` 阶段把这些状态合并进 `ProtectionAction(保护动作)`.
- 在 `execute action(执行动作)` 阶段执行排队, 延迟, 拒绝或监督停止.
- 让配置或测试夹具可以选择 `full jitter(全抖动)` 与 `decorrelated jitter(去相关抖动)`, 并固定 `RNG seed(随机种子)` 与 `clock(时钟)`.
- 将 `throttle_gate_owner(节流闸门归属)` 的全局字符串改为契约要求的 `"supervisor_global"`.

**Rationale(理由)**: 当前代码只是准备了算法和类型, 没有改变生产运行行为. 规格明确要求线上负载下的重启调度受这些规则约束.

**Confidence(置信度)**: `MEDIUM(中)`

**Action(操作)**:
- [x] Approve
- [ ] Reject
- [ ] Modify
- [ ] Skip

---

### Proposal P4: `005-1-failure-policy-reliability` / `SC-001`

**Resolution Status(决议状态)**: `APPROVED(已批准)`, 2026-05-17T05:06:16+08:00

**Direction(方向)**: `ALIGN(按规格改代码)`

**Current State(当前状态)**:
- Spec says(规格要求): 固定验收场景中 100% 的失败样本都能从事件或诊断导出核对 6 阶段顺序.
- Code does(代码行为): `tests/supervisor_pipeline_order.rs` 只把事件发进 `ObservabilityPipeline(可观察性管线)` 并检查记录存在, 没有验证真实 6 阶段执行.

**Proposed Resolution(建议修复)**:
- 在 `tests/supervisor_pipeline_order.rs` 中驱动真实 `SupervisionPipeline(监督管线)` 或真实 `RuntimeControlState(运行时控制状态)`.
- 使用固定样本覆盖成功, 非零退出, 崩溃, 超时, 外部取消和人工停止.
- 对每个样本断言阶段顺序和阶段诊断完整性.

**Rationale(理由)**: 这是 `FR-001` 的验收标准. 当前测试只证明观测管线能收事件, 不能证明策略流水线存在.

**Confidence(置信度)**: `HIGH(高)`

**Action(操作)**:
- [x] Approve
- [ ] Reject
- [ ] Modify
- [ ] Skip

---

### Proposal P5: `005-1-failure-policy-reliability` / `SC-002`

**Resolution Status(决议状态)**: `APPROVED(已批准)`, 2026-05-17T05:07:11+08:00

**Direction(方向)**: `ALIGN(按规格改代码)`

**Current State(当前状态)**:
- Spec says(规格要求): 一个分组触发保护后, 其他分组在同等时间窗内至少 90% 用例仍可独立完成一次受控重启尝试.
- Code does(代码行为): 测试通过多个独立 `MeltdownTracker` 实例模拟多个分组, 没有验证同一个监督器实例内部的分组桶隔离.

**Proposed Resolution(建议修复)**:
- 构造一个包含至少 2 个 `group(分组)` 的监督器树.
- 在同一个监督器实例内向一个分组注入密集失败.
- 验证另一个分组的计数没有被占用, 并且仍能走一次受控重启尝试.
- 从事件字段断言 `scopes_triggered(已触发作用域列表)` 与 `lead_scope(主导归因作用域)`.

**Rationale(理由)**: 使用多个 tracker(跟踪器) 实例无法证明生产隔离. 验收必须覆盖同一实例内的键控状态.

**Confidence(置信度)**: `MEDIUM(中)`

**Action(操作)**:
- [x] Approve
- [ ] Reject
- [ ] Modify
- [ ] Skip

---

### Proposal P6: `005-1-failure-policy-reliability` / `SC-003`

**Resolution Status(决议状态)**: `APPROVED(已批准)`, 2026-05-17T05:08:26+08:00

**Direction(方向)**: `ALIGN(按规格改代码)`

**Current State(当前状态)**:
- Spec says(规格要求): 并发失败压力样本中的瞬时并行自动重启峰值不得超过声明上限, 超出部分必须体现为显式推迟或队列诊断.
- Code does(代码行为): `tests/supervisor_concurrent_restart_throttle.rs` 使用本地模拟闸门, 没有使用生产 `CombinedThrottleGate(组合节流闸门)`, 并且验证了与契约冲突的 `"supervisor_instance"` 字符串.

**Proposed Resolution(建议修复)**:
- 删除测试本地 `ConcurrentRestartGate(并发重启闸门)` 模拟结构.
- 使用生产 `CombinedThrottleGate(组合节流闸门)` 或完整运行时路径构造压力样本.
- 保留至少 10 个同时触发的失败样本.
- 断言超限样本进入 `restart_queued(排队重启)` 或更严格档位.
- 断言全局闸门事件字段为 `"supervisor_global"`, 分组闸门字段为 `"group:{group_id}"`.

**Rationale(理由)**: 生产闸门和事件契约必须一起验收, 否则测试只能证明测试本地模拟成立.

**Confidence(置信度)**: `HIGH(高)`

**Action(操作)**:
- [x] Approve
- [ ] Reject
- [ ] Modify
- [ ] Skip

---

### Proposal P7: `005-1-failure-policy-reliability` / `SC-004`

**Resolution Status(决议状态)**: `APPROVED(已批准)`, option `7a`, 2026-05-17T05:09:30+08:00

**Direction(方向)**: `HUMAN_DECISION(需要人工决策)`

**Current State(当前状态)**:
- Spec says(规格要求): `CV_jitter_strategy / CV_fixed_baseline >= 1.3`.
- Code does(代码行为): 测试把固定基准设为常量延迟, 因此 `CV_fixed_baseline` 为 0. 当前测试只断言抖动序列的 `CV` 大于 0, 没有验证 1.3 比值.

**Decision Needed(需要决策)**:
- Option 7a, `ALIGN(按规格改代码)`: 将 `CV_fixed_baseline` 明确定义为无抖动的指数退避样本序列, 而不是常量延迟. 测试用生产 `BackoffPolicy(退避策略)` 生成两组 `next_wait(下一次等待)` 序列并验证比值.
- Option 7b, `BACKFILL(按代码补规格)`: 修改契约中的度量公式. 当固定延迟基准的 `CV` 为 0 时, 改用 `CV_jitter_strategy >= 0.3` 或等价的非除法阈值.
- Option 7c, `MODIFY(修改)`: 保留当前公式, 但由你指定一个非 0 的固定基准序列定义.

**Rationale(理由)**: 如果基准是常量延迟, `CV_fixed_baseline` 等于 0, 比值公式会出现除以 0 的问题. 这里既可能是测试理解错了, 也可能是契约公式需要补充边界定义.

**Confidence(置信度)**: `MEDIUM(中)`

**Action(操作)**:
- [x] Approve 7a
- [ ] Approve 7b
- [ ] Approve 7c with modification
- [ ] Reject
- [ ] Skip

---

## Interactive Session(交互会话)

Proposal queue(提案队列): `DONE(完成)`

Approved decisions(已批准决议): `P1`, `P2`, `P3`, `P4`, `P5`, `P6`, `P7 option 7a`

Next step(下一步): 运行 `speckit.sync.apply(同步应用)` 时, 应按已批准的 `ALIGN(按规格改代码)` 决议修复实现和测试. 对 `SC-004`, 使用 option `7a`: 将 `CV_fixed_baseline(固定基准变异系数)` 定义为无抖动指数退避样本序列, 并用生产 `BackoffPolicy(退避策略)` 验证 `1.3` 比值.

Prompt(提示): 队列已经完成, 当前没有待批复提案.
