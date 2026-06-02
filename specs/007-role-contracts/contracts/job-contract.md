# Job Contract(一次性任务契约)

`Job`(一次性任务) 表示只应完成一次的工作. `Job`(一次性任务) 成功后必须停止, 并且不得被 permanent restart(永久重启) 语义静默拉起.

## Lifecycle(生命周期)

```text
init -> run -> complete
```

## Required Method(必选方法)

`run` 是必选方法. 它必须表达一次性任务主体.

```rust
async fn run(&mut self, ctx: &JobContext) -> JobResult<()>;
```

## Optional Methods(可选方法)

`init` 和 `complete` 是可选方法. 如果使用者没有实现它们, 默认行为是不做事并返回成功.

```rust
async fn init(&mut self, ctx: &JobContext) -> JobResult<()>;
async fn complete(&mut self, ctx: &JobContext) -> JobResult<()>;
```

## Context(上下文)

`JobContext`(一次性任务上下文) 必须提供下列能力.

- `ready` 报告 readiness(就绪).
- `heartbeat` 报告 heartbeat(心跳).
- `is_cancelled` 查询 cancellation(取消) 是否已经请求.
- `wait_cancelled` 等待 cancellation(取消).

## Macro Rules(宏规则)

- `#[job]` 必须要求 `id` 和 `name`.
- `#[job]` 必须检查 `run` 方法存在.
- `#[job]` 必须生成 `TaskRole::Job`(任务角色: 一次性任务).
- `#[job]` 不允许生成 permanent restart(永久重启) 的默认语义.

## Runtime Rules(运行时规则)

- `run` 返回成功时, runtime(运行时) 必须调用 `complete`, 然后记录 job(一次性任务) 完成.
- `run` 返回错误时, runtime(运行时) 只能按 job retry budget(一次性任务重试预算) 处理.
- 若用户配置与一次性语义冲突, 系统必须产生可观察诊断.
