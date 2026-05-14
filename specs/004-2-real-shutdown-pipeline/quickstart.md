# Quickstart(快速开始): 真实关闭流水线

本 quickstart(快速开始) 用于验证 `004-2-real-shutdown-pipeline` 的实现是否满足规格. 命令默认在仓库根目录执行.

## 1. 运行格式检查

```bash
cargo fmt --check
```

期望结果: 命令成功退出.

## 2. 运行真实关闭流水线测试

```bash
cargo test --test supervisor_real_shutdown_pipeline_test
```

期望结果: 测试证明 `ShutdownTree(关闭监督树)` 会向运行中的 child task(子任务) 发送 `CancellationToken(取消令牌)`, 按 `shutdown_order(关闭顺序)` 等待任务完成, 超时后执行 `abort(强制中止)`, 并返回覆盖全部 child(子任务) 的结果摘要.

## 3. 运行控制和关闭回归测试

```bash
cargo test --test supervisor_control_test --test supervisor_shutdown_test --test observability_smoke_test
```

期望结果: 控制命令语义保持稳定, 关闭阶段和观测事件继续可用.

## 4. 验证 dashboard protocol(仪表盘协议) 没有漂移

```bash
cargo test --test dashboard_protocol_shape_test
```

期望结果: dashboard(仪表盘) 控制命令协议仍然可以识别 `ShutdownTree(关闭监督树)`, 并且请求形状不被真实关闭流水线改动.

## 5. 验证命名契约

```bash
cargo test --test naming_contract_test source_code_uses_approved_state_names
```

期望结果: Rust(编程语言) 源码中状态命名继续使用已批准词表.

## 6. 运行近似全量验收

```bash
cargo test -- --skip checked_artifacts_avoid_forbidden_state_terms
```

期望结果: 除当前已知的 sibling UI(同级用户界面) 命名契约阻塞外, supervisor runtime(监督器运行时) 测试通过. 如果 sibling UI(同级用户界面) 命名问题已经修复, 可以再运行完整命令.

```bash
cargo test
```

## 7. 人工检查结果摘要

实现完成后, 操作者需要在测试或示例中确认 `ShutdownResult(关闭结果)` 满足以下条件.

- `phase(阶段)` 最终是 `Completed(已完成)`.
- `report(报告)` 在完成后是 `Some(有值)`.
- `report.outcomes(报告结果集合)` 覆盖全部声明 child(子任务).
- 每个运行中的 child(子任务) 都有 `cancel_delivered = true`.
- 忽略取消的 child(子任务) 被记录为 `Aborted(已强制中止)` 或 `AbortFailed(强制中止失败)`.
- `reconcile(对账)` 说明 registry(注册表), runtime handles(运行时句柄), journal(日志), metrics(指标) 和 socket(套接字) 的最终状态.
