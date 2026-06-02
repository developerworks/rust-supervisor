# 可观测性

语言: [English](../en/observability.html)

## 事件模型

`SupervisorEvent`(监督器事件)描述一次 lifecycle fact(生命周期事实). 它包含 `When`(何时), `Where`(何处), `What`(发生内容), sequence(序号)和 correlation id(关联标识).

`When`(何时)记录 wall time(墙钟时间), monotonic time(单调时间), uptime(运行时长), generation(代次)和 attempt(尝试次数). `Where`(何处)记录 supervisor path(监督器路径), child id(子任务标识), parent id(父标识)和任务名称. `What`(发生内容)记录状态迁移, 策略决定, 健康状态, 退出原因或控制命令.

## 管线输出

observability pipeline(可观测性管线)把同一事实同步为这些信号:

- `SupervisorEvent`(监督器事件).
- structured log(结构化日志).
- tracing(结构化追踪) span(追踪范围)和 event(追踪事件).
- metrics(指标).
- audit event(审计事件).
- event journal(事件日志缓冲区).
- test recorder(测试记录器).

## 指标标签

metrics label(指标标签)必须保持低基数. 可以使用 supervisor path(监督器路径), child id(子任务标识), state(状态), decision(决定)和 failure category(失败类别). 不应该使用错误全文, 用户输入或无界动态值.

## 真实关闭流水线

`ShutdownTree`(关闭监督树) 执行真实 shutdown pipeline(关闭流水线) 后, observability pipeline(可观测性管线) 必须能看到每个阶段的事实. `ChildShutdownCancelDelivered`(子任务取消已送达) 表示 runtime(运行时) 已经向运行中的 child attempt(子任务尝试) 发送 `CancellationToken`(取消令牌). `ChildShutdownGraceful`(子任务优雅完成) 表示 child task(子任务) 在 graceful drain(优雅排空) 时间预算内返回. `ChildShutdownAborted`(子任务已强制中止) 表示 runtime(运行时) 已经对滞留任务请求 `abort`(强制中止). `ChildShutdownLateReport`(子任务迟到报告) 表示 child task(子任务) 在正常关闭核算窗口之后才返回. `ShutdownCompleted`(关闭完成) 表示 pipeline(流水线) 已经输出最终 reconcile report(对账报告).

metrics(指标) 使用低基数标签记录关闭事实. `supervisor_shutdown_duration_seconds`(监督器关闭耗时秒数) 记录完整 pipeline(流水线) 耗时. `supervisor_shutdown_child_outcomes_total`(监督器子任务关闭结果总数) 按 status(状态) 和 phase(阶段) 计数, 不把 `child_id`(子任务标识) 写入指标标签. `supervisor_shutdown_abort_total`(监督器关闭强制中止总数) 按 bounded reason(有界原因) 计数. `supervisor_shutdown_late_reports_total`(监督器关闭迟到报告总数) 按 phase(阶段) 计数.

audit event(审计事件) 会记录 cancel delivered(取消已送达), graceful outcome(优雅结果), abort outcome(强制中止结果), late report(迟到报告) 和 completed reconcile(完成对账). 核心 runtime(运行时) 不拥有 dashboard IPC socket(看板进程间通信套接字) 时, reconcile report(对账报告) 会把 socket status(套接字状态) 写成 `NotOwned`(非运行时拥有).

## 诊断回放

event journal(事件日志缓冲区)保存固定容量的最近事件. `RunSummary`(运行摘要)从事件日志, current state(当前状态)和策略决定生成诊断摘要, 用于解释 meltdown(熔断), 关闭超时或父级升级.
