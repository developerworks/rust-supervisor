# Worker(后台任务) 角色示例

[English README](README.md)

这个示例展示一个有界 `WorkRole::Worker`(工作角色: 后台任务) 子任务. Worker(后台任务) 角色表示完成一段有限后台工作后可以停止的任务.

运行命令:

```bash
cargo run --package rust-tokio-supervisor --example worker
```

## 示例展示内容

- 初始化: `worker_task.rs` 构建 `invoice-worker` 子任务, 并报告 readiness(就绪状态).
- 运行: worker(后台任务) 处理 3 个 invoice batch(发票批次), 每个批次输出当前 UNIX time(UNIX 时间).
- 完成: worker(后台任务) 返回 `TaskResult::Succeeded`(任务成功), 对齐 worker(后台任务) 成功后停止的角色语义.
- 清理: worker(后台任务) 自己停止后, `main.rs` 调用 `shutdown_tree`(关闭监督树) 清理 runtime(运行时).

## 文件结构

- `main.rs`: 组合 supervisor(监督器), event subscription(事件订阅), state snapshot(状态快照) 和 cleanup shutdown(清理关闭).
- `worker_task.rs`: 声明 `WorkRole::Worker`(工作角色: 后台任务) 子任务, 并实现有界 worker body(后台任务主体).
- `observation.rs`: 输出 worker fact(后台任务事实), runtime event(运行时事件), current state record(当前状态记录) 和 shutdown outcome(关闭结果).
