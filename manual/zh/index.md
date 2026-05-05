# 手册入口

## 目标

本手册面向使用 `rust-supervisor` 的维护者. 项目提供 supervisor(监督器) 树, child(子任务) 生命周期治理, restart policy(重启策略), current_state(当前状态), event journal(事件日志缓冲区), RunSummary(运行摘要) 和 four-stage shutdown(四阶段关闭).

## 配置

主配置使用 rust-config-tree(集中配置树) v0.1.9 加载 YAML(数据序列化格式). 固定示例路径是 `examples/config/supervisor.yaml`. 配置加载后得到 `ConfigState`(配置状态), 再派生 `SupervisorSpec`(监督器规格).
关闭术语统一使用 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务).

## 运行

```bash
cargo run --example supervisor_quickstart
```

运行过程应该创建 supervisor(监督器), 启动 child(子任务), 查询 current_state(当前状态), 最后执行 shutdown_tree(关闭整棵树).

## 关闭

root shutdown(根关闭) 必须按 request stop(请求停止), graceful drain(优雅排空), abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 执行. blocking worker(阻塞工作任务) 不能被当作普通 async worker(异步工作任务) 立即 abort(强制终止).

## 观测

每个 lifecycle fact(生命周期事实) 应该对应 SupervisorEvent(监督器事件), structured log(结构化日志), tracing(结构化追踪), metrics(指标), audit event(审计事件), event journal(事件日志缓冲区) 和 test recorder(测试记录器) 记录.
