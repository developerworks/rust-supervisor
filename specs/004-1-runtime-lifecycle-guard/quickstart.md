# Quickstart(快速开始): 运行时生命周期守卫

## 前置条件

当前仓库位于 `/Users/0x00/Documents/rust-supervisor-tools/rust-supervisor`. 本功能只修改核心库, 不需要启动 relay(中继) 或 dashboard client(看板客户端).

## 验证步骤

1. 运行运行时生命周期测试.

```bash
cargo test --test supervisor_runtime_lifecycle_test
```

期望结果: 启动后的 `SupervisorHandle(监督器控制句柄)` 返回 alive(存活) 健康状态, 控制循环异常退出后返回 not alive(非存活) 健康状态, 重复 `join` 在 1 秒内返回同一个最终结果.

2. 运行现有控制命令回归测试.

```bash
cargo test --test supervisor_control_test
```

期望结果: 现有 add child(添加子任务), pause child(暂停子任务), resume child(恢复子任务), shutdown tree(关闭监督树) 等控制命令仍然可用, 并且结束后的普通控制命令返回结构化错误.

3. 运行可观察性回归测试.

```bash
cargo test --test observability_smoke_test
```

期望结果: typed event(类型化事件), metrics(指标), audit log(审计日志) 和测试记录器可以看到控制面启动, 关闭, 失败和 join(等待结束) 结果.

4. 运行全量测试.

```bash
cargo test
```

期望结果: 所有测试通过. 如果已有无关格式漂移存在, 不得把无关文件格式化改动混入本功能提交.

## 手工检查

- 检查 `src/runtime/mod.rs` 只增加模块声明, 不添加 `pub use`.
- 检查 `src/lib.rs` 没有 compatibility exports(兼容导出).
- 检查 `manual/zh/runtime-control.md` 已说明 `is_alive`, `health`, `join` 和 `shutdown` 的语义.
- 检查 `RuntimeHealthReport(运行时健康报告)` 在控制循环已经结束后仍然可以读取最终失败原因.
