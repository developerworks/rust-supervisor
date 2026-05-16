# Sync Drift Report(同步漂移报告)

Generated(生成时间): 2026-05-17T04:07:27+08:00
Scope(范围): `005-1-failure-policy-reliability`
Skill(技能): `speckit-sync-analyze`

一句话结论: `005-1` 当前没有达到规格对齐状态, 主要原因是真实 `runtime control loop(运行时控制循环)` 没有接入 `SupervisionPipeline(监督管线)`, 并且多组验收测试仍然使用占位代码或本地模拟.

## Summary(摘要)

| Category(类别) | Count(数量) |
|----------------|-------------|
| Specs Analyzed(已分析规格) | 1 |
| Requirements Checked(已检查需求) | 7 |
| Aligned(对齐) | 0 |
| Drifted(漂移) | 7 |
| Not Implemented(未实现) | 0 |
| Unspecced Code(未入规格代码) | 0 |

## Detailed Findings(详细发现)

### FR-001, Critical(严重), `policy pipeline(策略流水线)` 没有成为真实运行路径

Expected(期望): `spec.md:95` 和 `contracts/pipeline-and-events.md:5` 到 `contracts/pipeline-and-events.md:25` 要求每一条运行结束情形都进入同一条 6 阶段流水线, `success(成功)`, `nonzero_exit(非零退出)`, `panic(崩溃)`, `timeout(超时)`, `external_cancel(外部取消)`, `manual_stop(人工停止)` 都必须可分类, 每个阶段都必须留下可对账记录点, 且禁止绕过流水线直接自动重启.

Actual(实际): `src/runtime/pipeline.rs:204` 到 `src/runtime/pipeline.rs:234` 有 6 阶段函数调用顺序, 但是 `src/runtime/control_loop.rs:433` 到 `src/runtime/control_loop.rs:510` 的真实退出处理路径仍然直接刷新重启限制, 计算 `restart_decision(重启决定)`, 然后执行重启决定. `src/runtime/control_loop.rs:1343` 到 `src/runtime/control_loop.rs:1352` 也直接读取 `restart_execution_plan(重启执行计划)` 并调用 `spawn_child_start(启动子任务)`. 未看到真实控制循环调用 `SupervisionPipeline::execute_pipeline`.

Evidence(证据):

- `src/runtime/pipeline.rs:248` 到 `src/runtime/pipeline.rs:256` 只把 `Succeeded(成功)`, `Cancelled(已取消)`, `Timeout(超时)` 和其他失败映射到分类结果, 没有真实覆盖 `panic(崩溃)` 与 `manual_stop(人工停止)` 的入口.
- `src/runtime/pipeline.rs:277` 到 `src/runtime/pipeline.rs:286` 只有需要重启的失败才记录失败窗口, `success(成功)` 路径没有按阶段留下可对账状态.
- `src/runtime/pipeline.rs:389` 到 `src/runtime/pipeline.rs:435` 只发出一个事件, 没有为每个阶段发出 `PipelineStageDiagnostic(流水线阶段诊断)`.
- `src/runtime/pipeline.rs:447` 到 `src/runtime/pipeline.rs:465` 的 `execute action(执行动作)` 仍然是占位逻辑, 注释明确写着实际重启, 排队, 拒绝, 退避和并发闸门尚未实现.

Impact(影响): `SC-001` 不能成立, 审查者不能从事件或诊断导出中核对真实运行样本的 6 阶段顺序. 当前实现还可能继续绕过规格规定的 `decide action(决定动作)` 和 `execute action(执行动作)` 边界.

Recommended Resolution(建议处理): 把 `handle_child_exit(处理子任务退出)` 的自动策略路径改为构造 `PipelineContext(流水线上下文)` 并调用 `SupervisionPipeline::execute_pipeline`, 再让第 6 阶段负责执行重启, 排队或拒绝. 同时补齐全部最小 `exit kind(退出类别)` 映射和每阶段诊断事件.

### FR-002, Critical(严重), `MeltdownTracker(熔断跟踪器)` 没有按真实作用域隔离

Expected(期望): `spec.md:96` 要求 `child(子任务)`, `group(分组)`, `supervisor(监督器)` 3 层状态互不混算. `contracts/pipeline-and-events.md:40` 到 `contracts/pipeline-and-events.md:52` 要求 `evaluate budget(评估预算)` 读取 3 套作用域计数, 输出 `scopes_triggered(已触发作用域列表)`, `lead_scope(主导归因作用域)` 和 `effective_protective_action(生效保护处置)`.

Actual(实际): `src/policy/meltdown.rs:98` 到 `src/policy/meltdown.rs:108` 只有 3 个聚合 `VecDeque(双端队列)`, 没有按 `ChildId(子任务标识)`, `group_id(分组标识)` 或监督器实例键控. `src/policy/meltdown.rs:140` 到 `src/policy/meltdown.rs:145` 每次 `record_child_restart(记录子任务重启)` 都把同一条失败同时写入 3 个计数队列.

Evidence(证据):

- `tests/supervisor_meltdown_group_isolation.rs:24` 到 `tests/supervisor_meltdown_group_isolation.rs:28` 通过给不同分组手动创建不同 `MeltdownTracker` 实例来模拟隔离, 这没有验证生产代码在一个监督器实例内的按组隔离.
- `src/policy/meltdown.rs:353` 到 `src/policy/meltdown.rs:371` 先按 `MeltdownOutcome(熔断结果)` 的固定严重度取最大值, 再简单按 child, group, supervisor 顺序选择第一个触发作用域. 这没有保证 `lead_scope(主导归因作用域)` 只从与 `effective meltdown verdict(有效熔断判定)` 同样严格的局部判定里选择.
- `src/runtime/pipeline.rs:426` 到 `src/runtime/pipeline.rs:430` 只填充 `effective_protective_action`, `cold_start_reason(冷启动原因)`, `hot_loop_reason(热循环原因)` 和 `throttle_gate_owner(节流闸门归属)`, 没有填充 `scopes_triggered` 与 `lead_scope`.

Impact(影响): `SC-002` 不能成立, 单个分组或子任务的密集失败可能占用其他层配额, 事件也不能可靠说明哪一层触发了最终保护.

Recommended Resolution(建议处理): 将 `MeltdownTracker` 状态改为按 `ChildId`, `group_id` 和监督器实例分别保存. 将每层 `local verdict(局部判定)` 映射到 `ProtectionAction(保护动作)` 档位, 只在同档并列时按 child, group, supervisor 选择 `lead_scope`. 在真实流水线事件中填充 `scopes_triggered` 和 `lead_scope`.

### FR-003, Critical(严重), 生产级退避和并发闸门没有接入运行路径

Expected(期望): `spec.md:97` 和 `contracts/pipeline-and-events.md:54` 到 `contracts/pipeline-and-events.md:58` 要求线上重启等待必须支持 `full jitter(全抖动)`, `decorrelated jitter(去相关抖动)`, 最大并发重启限制, `cold start budget(冷启动预算)`, `hot loop detection(热循环检测)`, 并且测试模式可以固定 `RNG seed(随机种子)` 与 `clock(时钟)`.

Actual(实际): `src/policy/backoff.rs:11` 到 `src/policy/backoff.rs:29` 定义了抖动模式, `src/policy/backoff.rs:320` 到 `src/policy/backoff.rs:551` 定义了冷启动预算与热循环检测类型, `src/runtime/concurrent_gate.rs:16` 到 `src/runtime/concurrent_gate.rs:370` 定义了并发闸门. 但是 `src/runtime/control_loop.rs:1254` 到 `src/runtime/control_loop.rs:1265` 的真实重启决策只使用从子任务配置映射来的 `BackoffPolicy(退避策略)`. `src/runtime/control_loop.rs:2652` 到 `src/runtime/control_loop.rs:2659` 调用 `BackoffPolicy::new`, 而 `src/policy/backoff.rs:79` 到 `src/policy/backoff.rs:85` 使默认 `jitter_mode(抖动模式)` 保持 `Disabled(关闭)`.

Evidence(证据):

- 未看到 `control_loop(控制循环)` 调用 `ColdStartBudget`, `HotLoopDetector` 或 `CombinedThrottleGate(组合节流闸门)`.
- `src/runtime/pipeline.rs:460` 到 `src/runtime/pipeline.rs:464` 仍然把退避延迟和并发闸门写成 TODO(待办).
- `src/event/payload.rs:133` 到 `src/event/payload.rs:139` 将全局闸门格式化为 `"supervisor_instance"`, 但是 `contracts/pipeline-and-events.md:85` 到 `contracts/pipeline-and-events.md:92` 要求输出 `"supervisor_global"`.

Impact(影响): `SC-003` 与 `SC-004` 不能证明成立, 生产运行时不会按规格执行并发限制, 冷启动收紧或热循环保护, 也不会按契约输出一致的闸门归属.

Recommended Resolution(建议处理): 将抖动模式, 并发闸门, 冷启动预算和热循环检测接入 `evaluate budget(评估预算)`, `decide action(决定动作)` 和 `execute action(执行动作)`. 将 `throttle_gate_owner(节流闸门归属)` 的序列化值改为契约要求的 `"supervisor_global"` 与 `"group:{group_id}"`.

### SC-001, High(高), 6 阶段顺序测试仍是占位或观测管线测试

Expected(期望): `spec.md:148` 要求固定验收场景中 100% 模拟失败样本都能从事件或诊断导出核对 6 阶段顺序.

Actual(实际): `tests/supervisor_pipeline_order.rs:80` 到 `tests/supervisor_pipeline_order.rs:118` 只把一个 `ChildFailed(子任务失败)` 事件发进 `ObservabilityPipeline(可观察性管线)`, 然后检查事件已记录. 注释在 `tests/supervisor_pipeline_order.rs:113` 到 `tests/supervisor_pipeline_order.rs:117` 仍然列出后续需要验证的真实 6 阶段执行与诊断字段.

Recommended Resolution(建议处理): 改为驱动真实 `RuntimeControlState(运行时控制状态)` 或 `SupervisionPipeline(监督管线)`, 并断言每个样本都按序产生 6 个阶段诊断.

### SC-002, High(高), 分组隔离测试没有覆盖同一监督器实例内的隔离

Expected(期望): `spec.md:149` 要求一个分组触发保护后, 其他分组在同等时间窗内至少 90% 用例仍可独立完成受控重启尝试.

Actual(实际): `tests/supervisor_meltdown_group_isolation.rs:24` 到 `tests/supervisor_meltdown_group_isolation.rs:28` 和 `tests/supervisor_meltdown_group_isolation.rs:68` 到 `tests/supervisor_meltdown_group_isolation.rs:70` 使用多个独立 `MeltdownTracker` 实例模拟多个分组. 这没有验证同一个监督器实例内部的 `group(分组)` 级桶, 也没有验证真实重启尝试仍可执行.

Recommended Resolution(建议处理): 构造单个监督器树, 配置至少 2 个分组, 在同一个 `MeltdownTracker` 或流水线上注入分组失败, 再通过运行时事件验证其他分组仍可完成受控重启.

### SC-003, High(高), 并发闸门验收没有使用生产闸门

Expected(期望): `spec.md:150` 要求并发失败压力样本中的瞬时并行自动重启峰值不超过声明上限, 超出部分必须进入显式推迟或队列诊断.

Actual(实际): `tests/supervisor_concurrent_restart_throttle.rs:10` 到 `tests/supervisor_concurrent_restart_throttle.rs:47` 定义了本地 `ConcurrentRestartGate(并发重启闸门)` 模拟结构, 没有使用 `src/runtime/concurrent_gate.rs` 的生产闸门, 也没有驱动真实运行时重启路径. `tests/supervisor_concurrent_restart_throttle.rs:120` 到 `tests/supervisor_concurrent_restart_throttle.rs:129` 还验证了 `"supervisor_instance"` 字符串, 这与契约要求的 `"supervisor_global"` 不一致.

Recommended Resolution(建议处理): 删除测试本地闸门模拟, 改为使用 `CombinedThrottleGate(组合节流闸门)` 或完整运行时路径, 并断言队列诊断和 `throttle_gate_owner(节流闸门归属)` 符合契约.

### SC-004, High(高), 抖动分散度验收没有验证生产算法和 1.3 比值

Expected(期望): `spec.md:151` 与 `contracts/pipeline-and-events.md:60` 到 `contracts/pipeline-and-events.md:71` 要求用 `Coefficient of Variation(CV,变异系数)` 计算 `CV_jitter_strategy / CV_fixed_baseline >= 1.3`.

Actual(实际): `tests/supervisor_backoff_jitter_distribution.rs:40` 到 `tests/supervisor_backoff_jitter_distribution.rs:47` 使用手写样本验证方差大于 0. `tests/supervisor_backoff_jitter_distribution.rs:50` 到 `tests/supervisor_backoff_jitter_distribution.rs:69` 使用本地 LCG(线性同余生成器) 模拟可重复性. `tests/supervisor_backoff_jitter_distribution.rs:89` 到 `tests/supervisor_backoff_jitter_distribution.rs:102` 因固定基准 CV 为 0, 只断言抖动 CV 大于固定 CV, 没有实际验证 1.3 比值, 也没有调用生产 `BackoffPolicy(退避策略)` 的全抖动或去相关抖动路径.

Recommended Resolution(建议处理): 直接使用生产 `BackoffPolicy(退避策略)` 生成至少 10 个 `next_wait(下一次等待)` 样本, 用非 0 固定基准或契约重新定义固定基准计算方式, 然后断言比值满足 `>= 1.3`.

## Task State Mismatches(任务状态不一致)

- `tasks.md:63` 到 `tasks.md:67` 把 US1(用户故事一) 实现任务标为完成, 但是真实运行路径没有进入 `SupervisionPipeline(监督管线)`, 且 `execute action(执行动作)` 仍然是占位逻辑.
- `tasks.md:86` 到 `tasks.md:90` 把 US2(用户故事二) 实现任务标为完成, 但是 `MeltdownTracker(熔断跟踪器)` 没有按 `ChildId(子任务标识)` 与 `group_id(分组标识)` 键控, 事件也没有填充 `scopes_triggered(已触发作用域列表)` 和 `lead_scope(主导归因作用域)`.
- `tasks.md:110` 到 `tasks.md:117` 把 US3(用户故事三) 实现任务标为完成, 但是生产运行时没有接入冷启动预算, 热循环检测和并发闸门.
- `tasks.md:127` 把端到端集成测试标为完成, 但是当前测试更像组件拼接或本地模拟, 没有证明真实运行时的 6 阶段端到端路径.

## Unspecced Code(未入规格代码)

本次按用户要求只分析 `005-1-failure-policy-reliability`. 没有把其他功能规格对应的代码计入未入规格代码.

## Recommended Next Steps(建议下一步)

1. 先修复 FR-001, 因为真实运行路径不进入流水线会让后续 FR-002 与 FR-003 无法可靠落地.
2. 再修复 FR-002, 用真实作用域键控和事件字段支撑分组隔离验收.
3. 最后修复 FR-003 与 SC-003, SC-004, 把退避, 冷启动, 热循环和并发闸门接入 `evaluate budget(评估预算)` 到 `execute action(执行动作)` 的闭环.
4. 修复测试时, 应避免只验证类型存在或本地模拟, 应尽量驱动生产路径并断言事件字段.
