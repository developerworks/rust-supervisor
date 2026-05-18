# Contract(契约): 代次隔离重启

本契约描述 `RestartChild(重启子任务)` 在 generation fencing(代次隔离) 下的公开行为. 本契约不是网络协议. 它约束 Rust public API(Rust 公开接口), runtime diagnostics(运行时诊断), observability events(可观测事件), metrics(指标), audit(审计) 和 dashboard model(仪表盘模型).

## Public API(公开接口)

### `ControlCommand::RestartChild(重启子任务)`

`RestartChild(重启子任务)` 的请求字段保持不变. 本功能只改变命令处理语义和结果内容.

```rust
ControlCommand::RestartChild { meta: CommandMeta, child_id: ChildId }
```

Contract rules(契约规则):

- 命令必须校验 `CommandMeta(命令元数据)` 中的 `requested_by(请求人)` 和 `reason(原因)`.
- 命令命中未知 child id(子任务标识) 时, 必须沿用既有 unknown child(未知子任务) 结构化错误.
- 命令命中正在关闭的 supervisor tree(监督树) 时, 不得启动新 attempt(尝试), 并且必须返回或记录 `BlockedByShutdown(被关闭阻止)` 结论.
- 命令命中已有 active attempt(活动尝试) 时, 必须先进入 pending restart(待重启), 并向旧 attempt(尝试) 送达取消.
- 命令命中无活动 attempt(尝试) 的已声明 child(子任务) 时, 可以立即启动新 generation(代次), 但仍必须通过公共启动门禁.

### `CommandResult::ChildControl(子任务控制命令结果)`

`RestartChild(重启子任务)` 必须返回 `CommandResult::ChildControl(子任务控制命令结果)`. `ChildControlResult(子任务控制结果)` 需要新增可选 `generation_fence(代次隔离结果)` 字段, 非重启命令把该字段设为 `None(无值)`.

```rust
pub struct ChildControlResult {
    pub child_id: ChildId,
    pub attempt: Option<ChildStartCount>,
    pub generation: Option<Generation>,
    pub operation_before: ChildControlOperation,
    pub operation_after: ChildControlOperation,
    pub status: Option<ChildAttemptStatus>,
    pub cancel_delivered: bool,
    pub stop_state: ChildStopState,
    pub restart_limit: RestartLimitState,
    pub liveness: ChildLivenessState,
    pub idempotent: bool,
    pub failure: Option<ChildControlFailure>,
    pub generation_fence: Option<GenerationFenceOutcome>,
}
```

Contract rules(契约规则):

- `generation_fence(代次隔离结果)` 必须存在于 `RestartChild(重启子任务)` 的成功结果中.
- `RestartChild(重启子任务)` 已经接受但旧 attempt(尝试) 仍未退出时, `generation_fence.decision(代次隔离决定)` 必须为 `QueuedAfterStop(停止后启动)`.
- 重复 `RestartChild(重启子任务)` 命中同一 pending restart(待重启请求) 时, `generation_fence.decision(代次隔离决定)` 必须为 `AlreadyPending(已存在待重启)`, `cancel_delivered(取消已送达)` 必须为 `false(否)`.
- `ChildControlResult.attempt(控制结果尝试)` 和 `ChildControlResult.generation(控制结果代次)` 必须指向本次命令观察到的旧活动尝试. 新 generation(代次) 尚未启动时, 不得把这两个字段提前改成目标代次.
- `operation_after(命令后操作)` 在 pending restart(待重启) 期间保持 `Active(活跃)`, 因为重启仍属于活跃治理路径. pending restart(待重启) 状态由 `generation_fence(代次隔离结果)` 表达.

## Runtime Semantics(运行时语义)

### Accepted Restart(已接受重启)

当 `RestartChild(重启子任务)` 命中正在运行的 child(子任务) 时, runtime(运行时) 必须执行下列顺序:

1. 读取当前 `(child_id, generation, attempt)(子任务标识, 代次和尝试)`.
2. 创建 `PendingRestart(待重启请求)`, 并计算 `target_generation(目标代次)`.
3. 调用当前 `ChildRuntimeState(子任务运行状态记录)` 的 `cancel(取消)`.
4. 把 `GenerationFenceState.phase(代次隔离阶段)` 设为 `WaitingForOldStop(等待旧尝试停止)`.
5. 返回 `CommandResult::ChildControl(子任务控制命令结果)`, 其中 `GenerationFenceOutcome.decision(代次隔离决定)` 为 `QueuedAfterStop(停止后启动)`.
6. 等旧 attempt(尝试) 的退出报告到达后, 清理旧运行句柄, 再启动 `target_generation(目标代次)`.

### Abort Escalation(强制中止升级)

当 pending restart(待重启请求) 的 `stop_deadline_at_unix_nanos(停止截止时间)` 已经过期, 且旧 attempt(尝试) 仍未退出时, runtime(运行时) 必须执行下列规则:

- 如果 `abort_requested(已请求强制中止)` 为 `false(否)`, runtime(运行时) 必须请求旧 attempt(尝试) 的 `abort(强制中止)`, 并把 `GenerationFenceState.phase(代次隔离阶段)` 设为 `AbortingOld(正在中止旧尝试)`.
- runtime(运行时) 不得在请求 abort(强制中止) 的同一刻启动新 attempt(尝试).
- 新 attempt(尝试) 只能在旧 attempt(尝试) 的 completion report(完成报告) 到达后启动.
- 如果 abort(强制中止) 后仍无完成报告, 后续 `CurrentState(当前状态)` 必须显示 pending restart(待重启) 仍在等待旧尝试终止.

### Automatic Restart(自动重启)

自动重启必须经过与手动重启相同的启动门禁:

- `handle_child_exit(处理子任务退出)` 得到策略重启决定后, 必须先确认同一运行状态记录没有 active attempt(活动尝试) 和 pending restart(待重启请求).
- 如果 pending restart(待重启请求) 已存在, 自动重启不得再分配新的 generation(代次).
- 如果 operation(操作) 是 `Paused(已暂停)`, `Quarantined(已隔离)` 或 `Removed(已移除)`, 自动重启继续被阻止.

### DelayedSpawnAttached(延迟附着启动子任务消息) 与正 backoff(退避延迟)

当 **`policy`** 产出的重启 **`spawn`** 计划含 **正 `backoff` 延迟**时, **`runtime`** **必须**通过内部 **`ChildStartMessage::DelayedSpawnAttached`** 变体(或仅重命名、语义等价的邮箱变体) **`enqueue`(入队)** 到 **`runtime control loop`** 邮箱, 让延迟到期后的 **`activate_instance`** 回调 **仍** 在同一条 **`control loop`** 执行轮次上串行发生. **不得**在未 **再次** **`enter control loop`** 的前提下, 让 **`tokio::spawn`(Tokio 派生异步任务)** 单独完成 **`ChildRuntimeState`** 的活动句柄绑定却声称满足 **`generation fencing`(代次隔离)** 与 **`FR-002`** 单活动门禁.

### Stale Report(过期报告)

exit handler(退出处理) 收到报告时必须先比较报告中的 `(generation, attempt)(代次和尝试)`:

- 如果报告匹配当前 active attempt(活动尝试), 该报告按正常退出处理.
- 如果报告匹配 pending restart(待重启请求) 的 old attempt(旧尝试), 该报告释放 fence(隔离边界), 然后允许启动目标 generation(目标代次).
- 如果报告不匹配上述两类, 该报告必须作为 stale report(过期报告) 处理. 它不得覆盖当前状态, 但必须进入事件, audit(审计) 和 metrics(指标).

## Event Contract(事件契约)

`src/event/payload.rs` 必须新增下列类型化事件. 每个事件必须经 `ObservabilityPipeline(可观测流水线)` 发送, 不得只写 broadcast string(广播字符串).

| Event(事件) | Required fields(必需字段) | Purpose(目的) |
|-------------|---------------------------|---------------|
| `ChildRestartFenceEntered(子任务重启隔离已进入)` | `child_id`, `old_generation`, `old_attempt`, `target_generation`, `command_id`, `requested_by`, `reason`, `stop_deadline_at_unix_nanos` | 重启命令接受后, 旧 attempt(尝试) 被纳入 fence(隔离边界). |
| `ChildRestartFenceAbortRequested(子任务重启隔离已请求中止)` | `child_id`, `old_generation`, `old_attempt`, `target_generation`, `command_id`, `deadline_unix_nanos` | 旧 attempt(尝试) 超过优雅等待时间后, runtime(运行时) 请求强制中止. |
| `ChildRestartFenceReleased(子任务重启隔离已释放)` | `child_id`, `old_generation`, `old_attempt`, `target_generation`, `exit_kind` | 旧 attempt(尝试) 完成后, 新 generation(代次) 被允许启动. |
| `ChildRestartConflict(子任务重启冲突)` | `child_id`, `current_generation`, `current_attempt`, `target_generation`, `command_id`, `decision`, `reason` | 重复或不可执行的重启请求被合并或拒绝. |
| `ChildAttemptStaleReport(子任务过期报告)` | `child_id`, `reported_generation`, `reported_attempt`, `current_generation`, `current_attempt`, `exit_kind`, `handled_as` | 旧 generation(代次) 的报告已被识别为过期, 且未覆盖当前状态. |

### Implementation phase note(实现阶段说明)

Event Contract(事件契约) 表中各事件应出现的实现阶段与 `specs/004-4-generation-fencing/tasks.md` 开篇 **Event Timing(事件时序)** 分段一致. 编码时以 `tasks.md` 的任务编号为阶段性权威, 本契约不另行分配任务编号, 避免契约与任务双源漂移. **Metrics Contract(指标契约)** 与下文 **Audit Contract(审计契约)** 要求在流水线收到对应类型化事件或等价分支时即可驱动计数器或仪表更新; **T027** 任务负责对照本节全表做收口校验与缺口补齐, 口径与 **Event Timing** 中 Metrics Contract 默认段落一致.

## Metrics Contract(指标契约)

`src/observe/metrics.rs` 必须从上述事件派生下列指标样本:

- `supervisor_child_restart_fence_total(子任务重启隔离总数)`: 标签包含 `result(结果)`, 取值包括 `entered(已进入)`, `released(已释放)`, `abort_requested(已请求中止)`, `already_pending(已有待重启)`, `rejected(已拒绝)`.
- `supervisor_child_attempt_stale_report_total(子任务过期报告总数)`: 标签包含 `handled_as(处理结果)`. 不得包含高基数的 `child_id(子任务标识)` 标签.
- `supervisor_child_restart_pending_total(子任务待重启数量)`: gauge(仪表) 指标, 表示当前待重启请求数量.

## Audit Contract(审计契约)

`src/observe/pipeline.rs` 必须扩展 child control audit(子任务控制审计), 写入下列字段:

- `command_id(命令标识)`.
- `requested_by(请求人)`.
- `reason(原因)`.
- `child_id(子任务标识)`.
- `old_generation(旧代次)`.
- `old_attempt(旧尝试)`.
- `target_generation(目标代次)`.
- `generation_fence_decision(代次隔离决定)`.
- `abort_requested(已请求强制中止)`.
- `stale_report(过期报告)`.
- `failure(失败原因)`.

## Dashboard Contract(仪表盘契约)

dashboard model(仪表盘模型) 必须显示 generation fencing(代次隔离) 的最小事实:

- `DashboardChildControlResult(仪表盘子任务控制结果)` 必须包含 `generation_fence(代次隔离结果)`.
- `DashboardChildRuntimeRecord(仪表盘子任务运行状态记录)` 必须包含 pending restart(待重启) 摘要, 至少包括 `phase(阶段)`, `old_generation(旧代次)`, `old_attempt(旧尝试)` 和 `target_generation(目标代次)`.
- dashboard protocol shape test(仪表盘协议形状测试) 必须继续证明请求字段没有漂移, 同时覆盖返回结果新增字段.

## Naming Contract(命名契约)

- 文档和代码必须统一使用 `Generation(代次)` 表示重启代次.
- `Epoch(纪元)` 只能用于时间戳起点, 例如 `UNIX_EPOCH(Unix 纪元常量)`.
- 不得把 generation(代次) 命名为 epoch(纪元).
- 新增公开类型必须加入 `naming_contract_test(命名契约测试)` 的批准名称集合.
