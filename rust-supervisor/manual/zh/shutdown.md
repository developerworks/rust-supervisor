# 关闭协议

语言: [English](../en/shutdown.html)

## 正式术语

本项目使用 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务) 描述关闭目标. root shutdown(根关闭)完成后, runtime(运行时)不应该留下 orphan task(孤儿任务).

## 四阶段

关闭协议包含四个阶段:

- request stop(请求停止): 接受关闭原因并传播 cancellation token(取消令牌).
- graceful drain(优雅排空): 等待 child(子任务)自行结束.
- abort stragglers(强制终止拖尾任务): 对超时的异步任务执行强制终止或升级.
- reconcile(状态对账): 统一 registry(注册表), current state(当前状态), metrics(指标)和 event journal(事件日志缓冲区).

## 顺序

启动按声明顺序执行. 关闭按声明顺序的逆序执行. 这个规则由 `startup_order` 和 `shutdown_order` 提供.

## 阻塞任务边界

`BlockingWorker`(阻塞工作任务)表示 `spawn_blocking`(阻塞任务启动) 或其它不能假设立即 abort(强制终止)的任务. 关闭超时后, runtime(运行时)应该记录不可立即终止边界, 并按照升级策略处理.

## 关闭原因

`ShutdownCause`(关闭原因)记录 `requested_by`(请求者)和 `reason`(原因). 它应该进入审计和诊断输出.

## 完成结果

`shutdown_tree`(关闭监督树)返回 `ShutdownResult`(关闭结果). 流水线完成后 `ShutdownResult.report` 含有 `ShutdownPipelineReport`(关闭流水线报告), 包含逐子任务结果, 对账报告和 dashboard socket(看板套接字)状态. 核心 runtime(运行时)不拥有 dashboard IPC socket(看板进程间通信套接字)时, 报告会把 socket status(套接字状态)记录为 `NotOwned`(非运行时拥有).
