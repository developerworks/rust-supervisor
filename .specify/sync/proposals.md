# Drift Resolution Proposals(偏差处置提案)

Generated(生成时间): 2026-05-15T01:14:53+08:00
Based on(基于): drift-report from 2026-05-15T01:14:53+08:00
Mode(模式): INTERACTIVE(交互式) — **会话已完成**, 全部 7 条提案已审查.

Interactive cursor(交互游标): **DONE(已完成)** — 最后处置: P007 选择 Option C(选项 C), 后续可在 `speckit.sync.apply` 或实现任务中落实已批准的 ALIGN 项与漂移 superseded 标记策略.

## Summary(摘要)

| Resolution Type(处置类型) | Count(数量) |
|---|---:|
| BACKFILL(回填, Code to Spec(代码到规格)) | 0 |
| ALIGN(对齐, Spec to Code(规格到代码)) | 6 |
| HUMAN_DECISION(人工决策) | 1 |
| NEW_SPEC(新规格) | 0 |
| REMOVE_FROM_SPEC(从规格移除) | 0 |

## Proposals(提案)

### Proposal P001: 004-3-child-slot-control/FR-001, FR-003, SC-001..SC-004

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): runtime(运行时) 必须维护完整 `child slot(子任务槽位)`, 并且当前状态和命令结果必须暴露活动尝试, 停止结果, 心跳, 就绪状态和重启预算.
- Code does(代码行为): `ActiveChildAttempt(活动子任务尝试)` 只服务 `shutdown pipeline(关闭流水线)`, `current_state(当前状态)` 只返回 `child_count` 和 `shutdown_completed`.
- Evidence(证据): `src/runtime/shutdown_pipeline.rs:17`, `src/control/command.rs:223`, `src/runtime/control_loop.rs:156`.

**Proposed Resolution(拟议处置)**:

保持规格不变, 在 runtime(运行时) 侧实现通用 `ChildSlot(子任务槽位)`:

- 新增 runtime(运行时) 私有槽位模型, 字段覆盖 spec(声明), `generation(代际)`, `attempt(尝试)`, status(状态), `cancellation_token(取消令牌)`, `abort_handle(中止句柄)`, completion receiver(完成接收端), `last_heartbeat(最后心跳)`, `ready_state(就绪状态)` 和 `restart_budget(重启预算)`.
- 把 `active_attempts` 的职责收敛到 `ChildSlot(子任务槽位)`, 让 `shutdown pipeline(关闭流水线)` 从槽位读取活动尝试, 而不是拥有另一套事实.
- 扩展 `ControlState(控制状态)` 或新增公开状态结构, 让 `current_state(当前状态)` 返回每个 child(子任务) 的槽位摘要.
- 为暂停, 恢复, 隔离, 移除和关闭结果增加槽位最终状态字段.

**Rationale(理由)**: 当前代码已经具备取消令牌和活动尝试句柄, 但是这些状态没有形成统一槽位模型. `004-3` 是后续 `004-4` 的基础, 所以应该让代码追上规格, 不应该降低规格.

**Confidence(置信度)**: HIGH(高)

**Review Status(审查状态)**: APPROVED(已批准) — 交互式审查于对话中确认

**Action(操作)**:
- [X] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
- [ ] Skip(跳过)

---

### Proposal P002: 004-3-child-slot-control/FR-002

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): `pause_child`, `remove_child` 和 `quarantine_child` 必须作用于真实活动任务.
- Code does(代码行为): 这些命令当前主要调用 `set_child_state`, 只写 `ManagedChildState(受管子任务状态)`.
- Evidence(证据): `src/runtime/control_loop.rs:134`, `src/runtime/control_loop.rs:142`, `src/runtime/control_loop.rs:145`.

**Proposed Resolution(拟议处置)**:

保持规格不变, 把停止类命令接入真实槽位执行:

- `pause_child` 应发送 `cancellation_token(取消令牌)`, 等待活动尝试结束或返回超时原因, 然后把槽位状态置为 paused(已暂停).
- `quarantine_child` 应发送取消, 停止当前活动尝试, 禁止自动重启, 并返回最终槽位状态.
- `remove_child` 应停止当前活动尝试, 从控制面状态移除或标记 removed(已移除), 并且后续自动重启不得重新创建该槽位.
- 重复停止同一槽位时必须返回幂等结果, 不得重复启动新的停止流程.

**Rationale(理由)**: 当前行为会让 UI(用户界面) 和操作者看到状态已经改变, 但是真实任务可能仍在运行. 这正是 `004-3` 要修正的运行时语义缺口.

**Confidence(置信度)**: HIGH(高)

**Review Status(审查状态)**: APPROVED(已批准) — 交互式审查于对话中确认

**Action(操作)**:
- [X] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
- [ ] Skip(跳过)

---

### Proposal P003: 004-4-generation-fencing/FR-001..FR-003, SC-001..SC-004

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): `restart_child(重启子任务)` 必须先停止当前活动尝试, 再启动新 `generation(代际)`, 并且旧代际迟到报告不得覆盖新状态.
- Code does(代码行为): `spawn_child_attempt` 会移除旧活动尝试并 `abort(中止)`, 然后立即启动新活动尝试. `record_child_exit` 不校验 `generation(代际)`.
- Evidence(证据): `src/runtime/control_loop.rs:813`, `src/runtime/control_loop.rs:846`, `src/runtime/control_loop.rs:628`, `src/runtime/control_loop.rs:182`.

**Proposed Resolution(拟议处置)**:

保持规格不变, 在 `004-3` 的 `ChildSlot(子任务槽位)` 基础上实现 `generation fencing(代际隔离)`:

- `restart_child` 和自动重启都进入同一个 `restart pipeline(重启流水线)`.
- 重启流程必须先向旧活动尝试发送取消, 在预算内等待完成, 超时后再 `abort(中止)`, 最后启动新 `generation(代际)`.
- `record_child_exit` 必须携带并校验 `generation(代际)` 和 `attempt(尝试)`.
- 旧代际迟到报告必须被记录为 `stale report(过期报告)`, 不能覆盖当前槽位.
- `current_state(当前状态)` 必须暴露当前 `generation(代际)` 和冲突或排队结论.

**Rationale(理由)**: 只在映射表里替换句柄不能证明旧任务已经停止. 代际隔离必须在真实任务生命周期, 退出报告, 当前状态和测试中同时成立.

**Confidence(置信度)**: HIGH(高)

**Review Status(审查状态)**: APPROVED(已批准) — 交互式审查于对话中确认

**Action(操作)**:
- [X] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
- [ ] Skip(跳过)

---

### Proposal P004: 004-1-runtime-lifecycle-guard/FR-002, SC-002

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): 控制循环异常退出时必须主动发出 `typed event(类型化事件)`, `metrics(指标)`, `audit log(审计日志)` 和结构化健康状态.
- Code does(代码行为): `watchdog(看门狗)` 更新健康状态并发送字符串 `broadcast(广播)`, 但是没有把 `RuntimeControlLoopFailed(运行时控制循环失败)` 写入 `observability pipeline(观测流水线)`.
- Evidence(证据): `src/runtime/watchdog.rs:49`, `src/event/payload.rs:344`, `src/observe/metrics.rs:242`, `src/observe/pipeline.rs:292`.

**Proposed Resolution(拟议处置)**:

保持规格不变, 把 runtime watchdog(运行时看门狗) 接入现有 `ObservabilityPipeline(观测流水线)`:

- 让 `SupervisorHandle(监督器控制句柄)` 或 runtime bootstrap(运行时启动路径) 持有观测发送入口.
- 控制循环异常退出时构造 `SupervisorEvent::RuntimeControlLoopFailed(监督器事件:运行时控制循环失败)`.
- 通过 `ObservabilityPipeline::emit` 写入 `journal(事件日志)`, `metrics(指标)`, `audit log(审计日志)` 和 test recorder(测试记录器).
- 保留现有字符串 `broadcast(广播)` 只作为 dashboard(仪表盘) 兼容事件源, 不作为唯一观测事实.

**Rationale(理由)**: 事件模型, 指标映射和审计映射已经存在. 当前缺口是发送方没有接入真实 pipeline(流水线), 所以应改代码.

**Confidence(置信度)**: HIGH(高)

**Review Status(审查状态)**: APPROVED(已批准) — 交互式审查于对话中确认

**Action(操作)**:
- [X] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
- [ ] Skip(跳过)

---

### Proposal P005: 004-2-real-shutdown-pipeline/FR-003

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): 关闭完成时必须清理运行时拥有资源, 记录非运行时拥有资源对账状态, 并返回覆盖每个 child(子任务) 的关闭摘要.
- Code does(代码行为): 关闭报告返回了 `journal(事件日志)` 和 `metrics(指标)` 状态, 但是这些状态来自 `core_runtime_completed()` 默认值, 不是来自真实 sink(接收端) 写入结果.
- Evidence(证据): `src/shutdown/report.rs:163`, `src/runtime/control_loop.rs:230`.

**Proposed Resolution(拟议处置)**:

保持规格不变, 让关闭流水线实际发出 typed shutdown event(类型化关闭事件):

- 在 `execute_shutdown` 阶段发出 shutdown phase changed(关闭阶段变化), child graceful(子任务优雅结束), child aborted(子任务被强制中止), late report(迟到报告) 和 shutdown completed(关闭完成) 的 `SupervisorEvent(监督器事件)`.
- `ShutdownReconcileReport(关闭对账报告)` 的 `journal_status` 和 `metrics_status` 应来自 emit(发送) 结果或 pipeline(流水线) 可观测状态, 不应无条件写成 recorded(已记录).
- 对 dashboard IPC socket(仪表盘进程间通信套接字) 保持 `NotOwned(非运行时拥有)`, 不伪造清理动作.

**Rationale(理由)**: `004-2` 的核心行为已经实现, 这条偏差不是关闭算法缺失, 而是观测和对账证明缺失. 应通过真实事件落点闭合.

**Confidence(置信度)**: HIGH(高)

**Review Status(审查状态)**: APPROVED(已批准) — 交互式审查于对话中确认

**Action(操作)**:
- [X] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
- [ ] Skip(跳过)

---

### Proposal P006: 003-supervisor-dashboard/SC-003, SC-012

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): dashboard(仪表盘) 需要固定数据集 20 轮定位验收, 并且需要自动验证 `React(前端框架)` 运行时依赖和组件文件数量为 0.
- Code does(代码行为): 当前任务层 `T066` 仍未完成.
- Evidence(证据): `specs/003-supervisor-dashboard/tasks.md:143`.

**Proposed Resolution(拟议处置)**:

保持规格不变, 继续沿用已批准的旧提案:

- 在相邻 `rust-supervisor-ui` 仓库补齐 5 个 target process(目标进程), 200 个 child task(子任务), 20 次定位, 至少 19 次成功, 总耗时小于 30 秒的 Playwright(浏览器测试工具) 验收.
- 增加 `package.json` 无 `React(前端框架)` runtime dependency(运行时依赖) 和 `src/` 下无 `.tsx` / `.jsx` 文件的自动验证.
- 该提案只影响 UI(用户界面) 仓库, 不阻塞当前 runtime(运行时) 规格实现顺序.

**Rationale(理由)**: 这两项在旧提案中已经审批为 ALIGN(对齐), 当前仍未在任务层闭合, 所以保留为实现项.

**Confidence(置信度)**: MEDIUM(中)

**Review Status(审查状态)**: APPROVED(已批准) — 交互式审查于对话中确认

**Action(操作)**:
- [X] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
- [ ] Skip(跳过)

---

### Proposal P007: 001-create-supervisor-core broad runtime semantics(宽口径运行时语义)

**Direction(方向)**: HUMAN_DECISION(人工决策)

**Current State(当前状态)**:
- Spec says(规格说明): 001 早期核心规格覆盖事件 sink(接收端), 多层监督树, 失败分类, 阻塞任务语义, 关闭观测和运行时观测.
- Code does(代码行为): 当前项目已经把这些能力拆到 `004-1`, `004-2`, `004-3` 和 `004-4` 逐步实现, 001 中仍有宽口径条款和当前阶段实现不完全一致.
- Evidence(证据): `src/task/context.rs:14`, `src/tree/builder.rs:1`, `src/error/types.rs:56`, `src/child_runner/runner.rs:80`, `src/runtime/watchdog.rs:49`.

**Options(选项)**:

- Option A(选项 A): 保留 001 原文, 把这些偏差全部作为后续代码实现项.
- Option B(选项 B): 更新 001, 明确 004 系列规格是运行时语义的分阶段细化, 001 只保留核心能力边界.
- Option C(选项 C): 不改 001, 但在 drift(偏差) 规则中把已由 004 系列覆盖的条款标记为 superseded(已被后续规格细化).

**Recommendation(建议)**:

选择 Option C(选项 C). 这样不会裁剪 001 的核心原则, 也不会把已拆分到 004 系列的同一问题重复计入实现缺口.

**Confidence(置信度)**: MEDIUM(中)

**Review Status(审查状态)**: APPROVED(已批准) — 交互式审查选择 **Option C(选项 C)**

**Action(操作)**:
- [X] Approve Option C(批准选项 C)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
- [ ] Skip(跳过)
