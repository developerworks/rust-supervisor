# Quickstart(快速开始): 子任务运行状态控制

本 quickstart(快速开始) 用于验证 `004-3-child-runtime-state-control` 的实现是否满足规格. 命令默认在仓库根目录执行.

## 1. 运行格式检查

```bash
cargo fmt --check
```

期望结果: 命令成功退出, 没有格式漂移.

## 2. 运行子任务运行状态控制测试

```bash
cargo test --test supervisor_child_runtime_state_control_test
```

期望结果: 该测试证明 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 会向当前活动尝试发送 `CancellationToken(取消令牌)`, `CurrentState(当前状态)` 中的 `child_runtime_records(子任务运行状态记录集合)` 包含心跳, 就绪状态, 重启次数限制和 `operation(操作)`, 并且代表性测试场景中连续 20 次 `CurrentState(当前状态)` 调用结果构造每次都低于 1 毫秒. `Cargo.toml` 必须已经注册 `supervisor_child_runtime_state_control_test(子任务运行状态控制测试)` 目标, 否则本命令不算可执行验收. 对已经处于目标操作且仍存在于 `child_runtime_states(子任务运行状态记录集合)` 中的运行状态记录重复执行停止类命令时返回幂等结果且不重复发送取消. `RemoveChild(移除子任务)` 首次命中无活动 attempt(尝试) 的占位运行状态记录时会物理删除该运行状态记录, 该首次删除不是幂等返回. `restart_limit(重启次数限制)` 耗尽时控制结果指出 `remaining = 0(剩余为零)` 与 `exhausted = true(已耗尽)`, 自动重启推进到新 `attempt(尝试)` 后控制命令不跨 attempt(尝试) 误送取消.

## 3. 运行控制和关闭回归测试

```bash
cargo test --test supervisor_control_test \
    --test supervisor_real_shutdown_pipeline_test \
    --test supervisor_runtime_lifecycle_test \
    --test supervisor_shutdown_test \
    --test observability_smoke_test
```

期望结果: 控制命令的 audit(审计) 与既有路径没有回归. `004-1-runtime-lifecycle-guard` 与 `004-2-real-shutdown-pipeline` 引入的语义继续可用.

## 4. 验证 dashboard request(仪表盘请求) 没有漂移, 返回结果字段已按契约升级

```bash
cargo test --test dashboard_protocol_shape_test
```

期望结果: 既有 `tests/dashboard_protocol_shape_test.rs` 自动发现目标继续通过. dashboard(仪表盘) 控制命令协议仍然可以识别 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 与 `CurrentState(当前状态)`, 请求字段没有被本功能改动. 返回结果字段必须按契约升级: 停止类命令返回 `ChildControl(子任务控制)` 调用结果, `CurrentState(当前状态)` 返回 `child_runtime_records(子任务运行状态记录集合)`.

## 5. 验证命名契约

```bash
cargo test --test naming_contract_test source_code_uses_approved_state_names
```

期望结果: Rust(编程语言) 源码中 `ChildAttemptStatus(子任务尝试状态)`, `ChildControlOperation(子任务控制操作)`, `ChildStopState(子任务停止状态)`, `ChildControlFailurePhase(子任务控制失败阶段)`, `ChildControlFailure(子任务控制失败原因)`, `RestartLimitState(重启次数限制状态)`, `ChildLivenessState(子任务存活状态)`, `ChildRuntimeState(子任务运行状态记录)`, `ChildRuntimeRecord(子任务运行状态记录)`, `ChildControlResult(子任务控制结果)` 和 `ReadinessState(就绪状态)` 与已批准词表一致. 既有 `ConfigState(配置状态)`, `SupervisorState(监督器状态)` 和 `current_state(当前状态)` 断言必须继续执行. 旧 `CommandResult::ChildState(子任务状态命令结果)` 变体已经被删除, 命名契约不得继续要求该旧变体存在.

## 6. 运行近似全量验收

```bash
cargo test -- --skip checked_artifacts_avoid_forbidden_state_terms
```

期望结果: 除当前已知的 sibling UI(同级用户界面) 命名契约阻塞外, supervisor runtime(监督器运行时) 测试全部通过. 近似全量验收后必须继续运行完整 `cargo test` 命令. 如果完整验收失败点来自 sibling UI(同级用户界面) 命名契约, 则必须记录阻塞测试名称和失败断言, 并与 sibling UI(同级用户界面) 命名契约修复一同协调.

```bash
cargo test
```

## 7. 人工检查结果摘要

实现完成后, 操作者需要在测试或示例中确认下列事实:

- `CurrentState(当前状态)` 返回的 `child_runtime_records(子任务运行状态记录集合)` 中每个 `ChildRuntimeRecord(子任务运行状态记录)` 都包含 `child_id(子任务标识)`, `generation(代次)`, `attempt(尝试)`, `status(状态)`, `operation(操作)`, `liveness(存活状态)`, `restart_limit(重启次数限制)`, `stop_state(停止状态)` 和 `failure(失败原因)`. 无活动 attempt(尝试) 的运行状态记录必须把 `generation / attempt / status` 返回为 `None(无值)`.
- `CurrentState(当前状态)` 调用结果构造必须是非阻塞状态记录读取, 在 `supervisor_child_runtime_state_control_test` 的代表性场景中连续 20 次读取每次都低于 1 毫秒; 若失败, 测试输出最慢耗时和运行状态记录数量.
- 长运行任务在收到 `PauseChild(暂停子任务)` 后, `liveness(存活状态)` 仍可读取最后心跳, `stop_state(停止状态)` 推进到 `CancelDelivered(已送达取消)`, exit handler(退出处理) 收到 `Exited(已退出)` 后推进到 `Completed(已停止)`.
- 对已经处于目标操作且仍存在于 `child_runtime_states(子任务运行状态记录集合)` 中的运行状态记录重复执行停止类命令 10 次, `ChildControlResult.idempotent(幂等)` 全部为 `true(是)`, 且本次 `cancel_delivered(取消已送达)` 全部为 `false(否)`. 该验收必须覆盖已经向活动 attempt(尝试) 送达取消的 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)` 和 `QuarantineChild(隔离子任务)` 重复命令, 还必须覆盖无活动 attempt(尝试) 且不会触发物理删除的 `PauseChild(暂停子任务)` 与 `QuarantineChild(隔离子任务)` 重复命令. 对无活动 attempt(尝试) 的占位运行状态记录首次执行 `RemoveChild(移除子任务)` 时, 必须返回 `NoActiveAttempt(无活动尝试)`, `idempotent = false(幂等为否)`, 并在结果构造后物理删除运行状态记录.
- 忽略取消的任务在停止截止时间经过后, 后续一次 `CurrentState(当前状态)` 或重复停止命令必须先触发 `reconcile_stop_deadlines(调和停止截止时间)`. 停止截止时间必须由取消送达时刻加当前 supervisor runtime(监督器运行时) 已生效的 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 得到. 触发后, `ChildControlResult.failure(控制结果失败原因)` 或 `ChildRuntimeRecord.failure(运行状态记录失败原因)` 必须为 `Some(有值)`, `phase(阶段)` 取 `ChildControlFailurePhase::WaitCompletion(等待完成)`, `reason(原因)` 非空. 本功能采用 lazy-only(惰性触发) 语义, 不要求停止失败事件在没有后续消息时自动按时钟发布.
- `restart_limit(重启次数限制)` 的 `window / limit(窗口与上限)` 来自既有 `RestartLimit(重启次数限制)` 配置来源, `used / remaining / exhausted(已使用, 剩余和已耗尽)` 由 runtime(运行时) 侧重启次数限制跟踪器在 child exit(子任务退出) 处理期间刷新, 不从无状态的 `PolicyEngine(策略引擎)` 或 `RestartPolicy(重启策略)` 读取运行时历史字段. `remaining(剩余)` 必须使用 `limit.saturating_sub(used)`(上限对已使用次数做饱和相减), `RestartLimitState.updated_at_unix_nanos(更新时间)` 必须通过 `RuntimeTimeBase(运行时时间基准)` 和 `previous + 1(前值加一)` 保护规则在连续至少 2 次刷新之间单调递增.
- `ChildControlOperation(子任务控制操作)` 与 `ManagedChildState(受管子任务状态)` 在 audit(审计) 中保持一一对应, 但运行状态字段是唯一事实来源.
