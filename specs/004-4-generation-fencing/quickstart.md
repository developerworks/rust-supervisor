# Quickstart(快速开始): 代次隔离重启

本 quickstart(快速开始) 用于验证 `004-4-generation-fencing` 的实现是否满足规格. 命令默认在仓库根目录执行.

## 1. 运行格式检查

```bash
cargo fmt --check
```

期望结果: 命令成功退出, 没有格式漂移.

## 2. 运行代次隔离测试

```bash
cargo test --test supervisor_generation_fencing_test
```

期望结果: 测试证明 `RestartChild(重启子任务)` 会先取消旧 active attempt(活动尝试), 再等待旧尝试退出或中止完成, 最后启动新的 generation(代次). 同一个 child id(子任务标识) 在任何检查点最多只有一个 active attempt(活动尝试).

## 3. 运行受影响的运行时回归测试

```bash
cargo test --test supervisor_child_runtime_state_control_test \
    --test supervisor_control_test \
    --test supervisor_real_shutdown_pipeline_test \
    --test supervisor_runtime_lifecycle_test \
    --test supervisor_shutdown_test \
    --test observability_smoke_test
```

期望结果: `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 和 shutdown pipeline(关闭流水线) 语义没有回归. 重启路径新增的强制中止升级不得改变停止类命令在 `004-3-child-runtime-state-control` 中的非中止语义.

## 4. 验证 dashboard(仪表盘) 返回结果

```bash
cargo test --test dashboard_protocol_shape_test
```

期望结果: dashboard request(仪表盘请求) 字段没有漂移. `RestartChild(重启子任务)` 返回的 `ChildControl(子任务控制)` 结果包含 `generation_fence(代次隔离结果)`. `CurrentState(当前状态)` 返回的运行状态记录包含 pending restart(待重启) 摘要.

## 5. 验证命名契约

```bash
cargo test --test naming_contract_test source_code_uses_approved_state_names
```

期望结果: 新增公开类型 `GenerationFenceState(代次隔离状态)`, `PendingRestart(待重启请求)`, `GenerationFenceOutcome(代次隔离结果)` 和 `StaleAttemptReport(过期尝试报告)` 都进入批准名称集合. 文档和源码不得把 generation(代次) 误写为 epoch(纪元).

## 6. 运行完整验收

```bash
cargo test
```

期望结果: 全部测试和 doctest(文档测试) 通过.

## 7. 人工检查结果摘要

实现完成后, 操作者需要确认下列事实:

- `RestartChild(重启子任务)` 命中正在运行的 child(子任务) 时, 第一次结果显示 `GenerationFenceDecision::QueuedAfterStop(停止后启动)`.
- 重复 `RestartChild(重启子任务)` 命中同一 pending restart(待重启请求) 时, 结果显示 `GenerationFenceDecision::AlreadyPending(已存在待重启)`, 且不会再次取消或启动新尝试.
- 旧 attempt(尝试) 在 graceful timeout(优雅等待时间) 内退出时, 新 generation(代次) 只在旧退出报告到达后启动.
- 旧 attempt(尝试) 忽略取消时, runtime(运行时) 在截止时间后请求 abort(强制中止), 但仍等旧完成报告到达后才启动新 generation(代次).
- 自动重启和手动重启同时发生时, 只有一个路径能获得启动许可.
- 旧 generation(代次) 的迟到报告会产生 `ChildAttemptStaleReport(子任务过期报告)` 事件, 不会覆盖当前运行状态.
