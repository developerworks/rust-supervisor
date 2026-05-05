# 任务模型

## 任务类型

`TaskKind`(任务类型) 区分 `AsyncWorker`(异步工作任务), `BlockingWorker`(阻塞工作任务) 和 `Supervisor`(监督器节点). blocking worker(阻塞工作任务)不能被当作普通 async worker(异步工作任务)立即强制终止.

## 任务工厂

`TaskFactory`(任务工厂)是核心任务构造契约. 每次 attempt(尝试)必须创建 fresh future(新异步任务). `service_fn`(函数适配器)只是人体工学入口, 它仍然适配到 `TaskFactory`(任务工厂), 不替换内核模型.

`TaskResult`(任务结果)区分 `Succeeded`(成功), `Cancelled`(已取消) 和 `Failed`(已失败). `Failed` 携带 `TaskFailure`(任务失败)和 `TaskFailureKind`(任务失败类别).

## 任务上下文

`TaskContext`(任务上下文)包含 child id(子任务标识), supervisor path(监督器路径), generation(代次), attempt(尝试次数), cancellation token(取消令牌), heartbeat(心跳)发送入口和 readiness(就绪)发送入口.

worker(工作任务)应该通过 `TaskContext::heartbeat` 报告健康信号, 通过 `TaskContext::mark_ready` 报告显式就绪, 通过 `TaskContext::is_cancelled` 或 `TaskContext::cancellation_token` 响应关闭.

## 就绪语义

`ReadinessPolicy`(就绪策略)支持 `Immediate`(立即就绪)和 `Explicit`(显式就绪). 显式就绪的 child(子任务)在报告 ready(已就绪)之前, 不应该被 current state(当前状态)或事件显示为 ready(已就绪).
