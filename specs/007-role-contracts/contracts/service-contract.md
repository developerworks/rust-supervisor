# Service Contract(服务契约)

`Service`(服务) 表示长期在线的受监督单元. `Service`(服务) 的成功退出不一定表示业务完成, runtime(运行时) 可以按角色默认策略恢复它.

## Lifecycle(生命周期)

```text
init -> run -> shutdown
```

## Required Method(必选方法)

`run` 是必选方法. 它必须表达服务主体逻辑.

```rust
async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()>;
```

## Optional Methods(可选方法)

`init` 和 `shutdown` 是可选方法. 如果使用者没有实现它们, 默认行为是不做事并返回成功.

```rust
async fn init(&mut self, ctx: &ServiceContext) -> ServiceResult<()>;
async fn shutdown(&mut self, ctx: &ServiceContext) -> ServiceResult<()>;
```

## Context(上下文)

`ServiceContext`(服务上下文) 必须提供下列能力.

- `ready` 报告 readiness(就绪).
- `heartbeat` 报告 heartbeat(心跳).
- `is_shutdown_requested` 查询 shutdown(关闭) 是否已经请求.
- `wait_shutdown` 等待 shutdown(关闭).

## Macro Rules(宏规则)

- `#[service]` 必须要求 `id` 和 `name`.
- `#[service]` 必须检查 `run` 方法存在.
- `#[service]` 必须生成 `TaskRole::Service`(任务角色: 服务).
- `#[service]` 不允许要求 `primary` 参数.

## Runtime Rules(运行时规则)

- `init` 成功后, runtime(运行时) 进入 `run`.
- `run` 返回错误时, runtime(运行时) 按服务失败策略处理.
- 收到 shutdown(关闭) 请求时, runtime(运行时) 必须让 `run` 看到关闭信号, 然后执行 `shutdown`.
- `shutdown` 返回错误时, runtime(运行时) 必须产生可观察诊断.
- 当 shutdown(关闭) 已经被请求, 并且 `init`, `run` 和 `shutdown` 都成功完成时, adapter(适配器) 必须返回 `TaskResult::Cancelled`(任务取消结果), 而不是把受控关闭记录为普通成功.
