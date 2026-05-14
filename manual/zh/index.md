# rust-supervisor 手册

语言: [English](../en/index.html)

## 项目定位

`rust-supervisor` 是 Rust(编程语言) 任务监督核心库. 它面向 Tokio(异步运行时) 服务, 用声明式模型管理 child(子任务) 的启动, 停止, 重启, 隔离, 状态查询, 事件记录, 健康检查和 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务).

本项目没有旧接口负担. 使用者应该通过拥有模块路径读取公开类型, 例如 `rust_supervisor::runtime::supervisor::Supervisor`.

## 阅读路径

- [快速开始](getting-started.md): 从 YAML(数据序列化格式)配置启动最小 supervisor(监督器).
- [配置模型](configuration.md): 理解 `SupervisorConfig`, `ConfigState` 和配置拒绝启动边界.
- [监督树](supervisor-tree.md): 理解 `SupervisorSpec`, `SupervisorTree` 和注册表关系.
- [任务模型](task-model.md): 理解 `ChildSpec`, `TaskFactory`, `TaskContext` 和 readiness(就绪).
- [策略模型](policies.md): 理解重启, 退避, 熔断, 隔离和任务退出分类.
- [运行时控制](runtime-control.md): 理解 `SupervisorHandle` 的控制命令和幂等语义.
- [Dashboard(看板)](dashboard.md): 理解 target process(目标进程), relay(中继) 和 dashboard client(看板客户端) 的三端使用流程.
- [关闭协议](shutdown.md): 理解四阶段关闭和 blocking worker(阻塞工作任务)边界.
- [可观测性](observability.md): 理解事件, 日志, 追踪, 指标, 审计和运行摘要.
- [示例程序](examples.md): 逐个运行 `examples/` 下的学习示例.
- [质量门禁](quality-gates.md): 运行格式化, 编译, 测试, 文档, SBOM(软件物料清单)和发布检查.

## 能力边界

supervisor core(监督器核心) 只管理 lifecycle governance(生命周期治理). 高频业务消息属于 data plane(数据面), 不应该每条都经过 supervisor(监督器). control plane(控制面) 只处理生命周期命令, 当前状态, 事件和治理决策.
