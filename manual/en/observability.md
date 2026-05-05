# 可观测性

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

## 诊断回放

event journal(事件日志缓冲区)保存固定容量的最近事件. `RunSummary`(运行摘要)从事件日志, current state(当前状态)和策略决定生成诊断摘要, 用于解释 meltdown(熔断), 关闭超时或父级升级.
