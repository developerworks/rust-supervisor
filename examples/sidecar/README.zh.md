# Sidecar(边车) 角色示例

[English README](README.md)

这个示例展示一个挂在 primary service(主服务) 旁边的 `WorkRole::Sidecar`(工作角色: 边车) 子任务. Sidecar(边车) 角色表示辅助进程, 它和主服务一起运行, 并在 supervisor tree(监督树) 关闭时一起停止.

运行命令:

```bash
cargo run --package rust-tokio-supervisor --example sidecar
```

进程会一直运行, 直到你按下 `Ctrl+C`.

## 示例展示内容

- 主服务: `api-service` 声明为 `WorkRole::Service`(工作角色: 常驻服务).
- 边车绑定: `metrics-sidecar` 声明为 `WorkRole::Sidecar`(工作角色: 边车), 并使用 `SidecarConfig`(边车配置).
- 运行: 两个子任务每秒输出一次 business tick(业务周期).
- 停止: 按下 `Ctrl+C` 后, 两个子任务都会收到 cancellation(取消信号), 并通过 `shutdown_tree`(关闭监督树) 停止.

## 文件结构

- `main.rs`: 组合 supervisor(监督器), event subscription(事件订阅), state snapshot(状态快照) 和 shutdown command(关闭命令).
- `sidecar_task.rs`: 声明 primary service(主服务), sidecar child(边车子任务) 和 sidecar business logic(边车业务逻辑).
- `observation.rs`: 输出 child fact(子任务事实), runtime event(运行时事件), current state record(当前状态记录) 和 shutdown outcome(关闭结果).
