# Manual(手册) 入口

## 目标

本手册与 `manual/zh/index.md` 同构. 它面向需要接入 `rust-supervisor` 的维护者, 覆盖 supervisor(监督器) 树, child(子任务), restart policy(重启策略), current_state(当前状态), event journal(事件日志缓冲区), RunSummary(运行摘要) 和 four-stage shutdown(四阶段关闭).

## 配置

主配置必须通过 rust-config-tree(集中配置树) v0.1.9 加载 YAML(数据序列化格式). `ConfigState`(配置状态) 是所有运行时可调值的唯一入口, `SupervisorSpec`(监督器规格) 从同一个配置状态派生.
Shutdown terminology uses Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务).

## 运行

```bash
cargo run --example supervisor_quickstart
```

该示例展示 load config(加载配置), build spec(构建规格), start supervisor(启动监督器), query current_state(查询当前状态) 和 shutdown tree(关闭整棵树).

## 关闭

shutdown(关闭) 过程包含 request stop(请求停止), graceful drain(优雅排空), abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账). 关闭顺序必须是声明顺序的逆序.

## 观测

observability(可观测性) 管线必须把同一个 lifecycle fact(生命周期事实) 投递到 SupervisorEvent(监督器事件), structured log(结构化日志), tracing(结构化追踪), metrics(指标), audit event(审计事件) 和 event journal(事件日志缓冲区).
