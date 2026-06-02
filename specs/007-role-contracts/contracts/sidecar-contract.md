# Sidecar Contract(边车契约)

`Sidecar`(边车) 表示绑定在 primary service(主服务) 上的辅助任务. `Sidecar`(边车) 不应独立表达完整业务生命周期.

## Lifecycle(生命周期)

```text
init -> run -> shutdown
```

## Required Items(必选项)

`run` 是必选方法. `primary` 是必选宏参数.

```rust
async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()>;
```

## Optional Methods(可选方法)

`init` 和 `shutdown` 是可选方法. 如果使用者没有实现它们, 默认行为是不做事并返回成功.

```rust
async fn init(&mut self, ctx: &SidecarContext) -> SidecarResult<()>;
async fn shutdown(&mut self, ctx: &SidecarContext) -> SidecarResult<()>;
```

## Context(上下文)

`SidecarContext`(边车上下文) 必须提供下列能力.

- `ready` 报告 readiness(就绪).
- `heartbeat` 报告 heartbeat(心跳).
- `primary_id` 读取 primary service(主服务) 标识.
- `is_shutdown_requested` 查询 shutdown(关闭) 是否已经请求.
- `wait_shutdown` 等待 shutdown(关闭).

## Macro Rules(宏规则)

- `#[sidecar]` 必须要求 `id`, `name` 和 `primary`.
- `#[sidecar]` 必须检查 `run` 方法存在.
- `#[sidecar]` 必须生成 `TaskRole::Sidecar`(任务角色: 边车).
- `#[sidecar]` 必须拒绝缺少 `primary` 的写法.

## Runtime Rules(运行时规则)

- primary service(主服务) 停止时, runtime(运行时) 必须请求对应 `Sidecar`(边车) 停止.
- `Sidecar`(边车) 失败时, 默认只重启边车自己.
- 如果边车被声明为 critical(关键), 失败可以升级为主服务不可用诊断.
- `Sidecar`(边车) 不允许绑定另一个 `Sidecar`(边车).
