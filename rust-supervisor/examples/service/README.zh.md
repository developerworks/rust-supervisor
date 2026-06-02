# Service(常驻服务) 角色示例

[English README](README.md)

这个示例展示一个被 supervisor(监督器) 管理的 `TaskRole::Service`(任务角色: 常驻服务) 子任务. Service(常驻服务) 角色表示长期在线的任务, 它应该完成初始化, 报告 readiness(就绪状态), 发出 heartbeat(心跳), 并在 supervisor(监督器) 关闭时协作停止.

运行命令:

```bash
cargo run --package rust-tokio-supervisor --example service
```

进程会一直运行, 直到你按下 `Ctrl+C`. 这个信号会被当成 operator stop request(操作员停止请求), 然后示例调用 `shutdown_tree`(关闭监督树), 并输出 graceful shutdown outcome(优雅关闭结果).

## 示例展示内容

- 初始化: `service_task.rs` 构建 `quote-service` 子任务, 并调用 `mark_ready()` 报告 readiness(就绪状态).
- 运行: service(常驻服务) 每秒输出一次当前 UNIX time(UNIX 时间), 然后发出 heartbeat(心跳) 和 running tick(运行周期) 事实.
- 观测: `observation.rs` 周期性输出 `current_state`(当前状态), runtime event text(运行时事件文本) 和 shutdown report(关闭报告).
- 停止: 按下 `Ctrl+C` 后, `main.rs` 调用 `shutdown_tree`(关闭监督树), cancellation(取消信号) 到达 service(常驻服务), service(常驻服务) 返回 `TaskResult::Cancelled`(任务已取消).

## 文件结构

- `main.rs`: 组合 supervisor(监督器), event subscription(事件订阅), state snapshot(状态快照) 和 shutdown command(关闭命令).
- `service_task.rs`: 声明 `TaskRole::Service`(任务角色: 常驻服务) 子任务, 并实现 async service body(异步服务主体).
- `observation.rs`: 输出 service fact(服务事实), runtime event(运行时事件), current state record(当前状态记录) 和 shutdown outcome(关闭结果).

## 预期输出形态

输出应该包含这些阶段:

```text
service initialization: initialized child=quote-service path=/quote-service
state after-initialization: child_count=1 shutdown_completed=false
service example: running until Ctrl+C
service business: child=quote-service tick=... now_unix=...
service while-running: running child=quote-service tick=...
operator signal=ctrl_c
service during-stop: stopping child=quote-service
shutdown outcome: child=quote-service status=Graceful phase=GracefulDrain cancel_delivered=true
state after-shutdown: child_count=1 shutdown_completed=true
```

关键行为是: service(常驻服务) 不会自己退出. 它在运行期保持 active(活跃), 通过 `current_state`(当前状态) 暴露 readiness(就绪状态) 和 heartbeat(心跳) 状态, 只在收到 operator signal(操作员信号) 后进入关闭阶段并收到 cancellation(取消信号), 最后形成 graceful child shutdown outcome(优雅子任务关闭结果).
