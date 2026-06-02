# Supervisor Contract(监督器契约)

`Supervisor`(监督器) 角色表示被外层监督器管理的 nested supervisor unit(嵌套监督单元). 它负责构建并运行自己的 child tree(子任务树).

## Boundary(边界)

`TaskRole::Supervisor`(任务角色: 监督器) 表示业务角色. `TaskKind::Supervisor`(任务执行种类: 监督器节点) 表示 runtime(运行时) 执行形态. 本契约必须同时尊重这 2 个概念, 但不得把它们混为一个字段.

## Lifecycle(生命周期)

```text
build_tree -> run -> shutdown
```

## Required Method(必选方法)

`build_tree` 是必选方法. 它必须返回 `SupervisorSpec`(监督器规格).

```rust
async fn build_tree(&mut self, ctx: &SupervisorContext) -> SupervisorResult<SupervisorSpec>;
```

## Optional Methods(可选方法)

`run` 和 `shutdown` 是可选方法. 默认 `run` 等待外部关闭. 默认 `shutdown` 向子树传播关闭.

```rust
async fn run(&mut self, ctx: &SupervisorContext, tree: SupervisorHandle) -> SupervisorResult<()>;
async fn shutdown(&mut self, ctx: &SupervisorContext, tree: SupervisorHandle) -> SupervisorResult<()>;
```

## Context(上下文)

`SupervisorContext`(监督器上下文) 必须提供下列能力.

- `ready` 报告 readiness(就绪).
- `heartbeat` 报告 heartbeat(心跳).
- `supervisor_id` 读取当前监督器标识.
- `is_shutdown_requested` 查询 shutdown(关闭) 是否已经请求.
- `wait_shutdown` 等待 shutdown(关闭).

## Macro Rules(宏规则)

- `#[supervisor_role]` 必须要求 `id` 和 `name`.
- `#[supervisor_role]` 必须检查 `build_tree` 方法存在.
- `#[supervisor_role]` 必须生成 `TaskRole::Supervisor`(任务角色: 监督器).
- `#[supervisor_role]` 必须生成或要求 `TaskKind::Supervisor`(任务执行种类: 监督器节点) 的运行形态.

## Runtime Rules(运行时规则)

- 外层 runtime(运行时) 必须把嵌套监督器当作一个 child(子任务) 管理.
- 内层 runtime(运行时) 必须管理自己的 child tree(子任务树).
- 外层 shutdown(关闭) 必须传播到内层 child tree(子任务树).
- 内层失败必须能映射成外层可观察的失败结果.
