# Job(一次性任务) 角色示例

[English README](README.md)

这个示例展示一个一次性 `WorkRole::Job`(工作角色: 一次性任务) 子任务. Job(一次性任务) 角色适合有限工作, 工作完成后应该停止.

运行命令:

```bash
cargo run --package rust-tokio-supervisor --example job
```

## 示例展示内容

- 初始化: `job_task.rs` 构建 `daily-report-job` 子任务, 并报告 readiness(就绪状态).
- 运行: job(一次性任务) 执行一次 report generation(报告生成) 业务函数, 并输出当前 UNIX time(UNIX 时间).
- 完成: job(一次性任务) 返回 `TaskResult::Succeeded`(任务成功), 对齐 job(一次性任务) 一次性运行的角色语义.
- 清理: job(一次性任务) 自己停止后, `main.rs` 调用 `shutdown_tree`(关闭监督树) 清理 runtime(运行时).

## 文件结构

- `main.rs`: 组合 supervisor(监督器), event subscription(事件订阅), state snapshot(状态快照) 和 cleanup shutdown(清理关闭).
- `job_task.rs`: 声明 `WorkRole::Job`(工作角色: 一次性任务) 子任务, 并实现 one-shot job body(一次性任务主体).
- `observation.rs`: 输出 job fact(一次性任务事实), runtime event(运行时事件), current state record(当前状态记录) 和 shutdown outcome(关闭结果).
