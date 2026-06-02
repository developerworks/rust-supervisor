# Worker Contract(后台任务契约)

`Worker`(后台任务) 表示有限后台工作. `Worker`(后台任务) 成功后应该停止, 失败后按 bounded retry(有界重试) 处理.

## Lifecycle(生命周期)

```text
init -> work -> complete
```

## Required Method(必选方法)

`work` 是必选方法. 它必须表达有限后台工作主体.

```rust
async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()>;
```

## Optional Methods(可选方法)

`init` 和 `complete` 是可选方法. 如果使用者没有实现它们, 默认行为是不做事并返回成功.

```rust
async fn init(&mut self, ctx: &WorkerContext) -> WorkerResult<()>;
async fn complete(&mut self, ctx: &WorkerContext) -> WorkerResult<()>;
```

## Context(上下文)

`WorkerContext`(后台任务上下文) 必须提供下列能力.

- `ready` 报告 readiness(就绪).
- `heartbeat` 报告 heartbeat(心跳).
- `is_cancelled` 查询 cancellation(取消) 是否已经请求.
- `wait_cancelled` 等待 cancellation(取消).

## Macro Rules(宏规则)

- `#[worker]` 必须要求 `id` 和 `name`.
- `#[worker]` 必须检查 `work` 方法存在.
- `#[worker]` 必须生成 `TaskRole::Worker`(任务角色: 后台任务).
- `#[worker]` 不允许要求 `primary` 参数.

## Runtime Rules(运行时规则)

- `work` 返回成功时, runtime(运行时) 必须调用 `complete`, 然后记录正常完成.
- `work` 返回错误时, runtime(运行时) 按 worker(后台任务) 失败策略处理.
- 收到 cancellation(取消) 请求时, `WorkerContext`(后台任务上下文) 必须能让用户代码感知取消.
