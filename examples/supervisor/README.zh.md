# Supervisor(监督器) 角色示例

[English README](README.md)

这个示例展示一个声明为 `WorkRole::Supervisor`(工作角色: 监督器) 的可运行子任务. 它把 nested supervisor unit(嵌套监督器单元) 建模为一个长期运行的受监督任务, 用来展示初始化, 持续运行, 协作停止和观测输出.

运行命令:

```bash
cargo run --package rust-tokio-supervisor --example supervisor
```

进程会一直运行, 直到你按下 `Ctrl+C`.

## 示例展示内容

- 初始化: `supervisor_task.rs` 构建 `nested-supervisor-unit` 子任务, 并报告 readiness(就绪状态).
- 运行: supervisor(监督器) 角色单元每秒输出一次 business tick(业务周期).
- 停止: 按下 `Ctrl+C` 后, runtime(运行时) 调用 `shutdown_tree`(关闭监督树), 角色单元收到 cancellation(取消信号).
- 观测: `observation.rs` 输出 role fact(角色事实), runtime event(运行时事件), current state record(当前状态记录) 和 shutdown outcome(关闭结果).

## 文件结构

- `main.rs`: 组合 supervisor(监督器), event subscription(事件订阅), state snapshot(状态快照) 和 shutdown command(关闭命令).
- `supervisor_task.rs`: 声明 `WorkRole::Supervisor`(工作角色: 监督器) 角色单元, 并实现 business loop(业务循环).
- `observation.rs`: 输出 lifecycle fact(生命周期事实), runtime event(运行时事件), state record(状态记录) 和 shutdown outcome(关闭结果).
