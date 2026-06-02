# ChildSpecBuilder (子任务规格构建器)

语言: [English](../en/child-spec-builder.html)

## 一句话结论

`ChildSpecBuilder` 是在 Rust 代码里构造 `ChildSpec`(子任务规格) 的链式 API(应用程序接口). 配置和 RPC(远程过程调用) 仍应使用 `ChildDeclaration`(子任务声明). 构建出口是 `build() -> Result<ChildSpec, SupervisorError>`, 内部会调用 `ChildSpec::validate()` 做校验.

与 [`child-spec.md`](child-spec.md) 的关系: 那篇文档说明声明与规格的分工; 本篇专门讲 Builder 的入口, setter(设置器) 与常见用法.

## 模块路径

```rust
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
```

模块定义在 [`src/spec/child_builder.rs`](../../src/spec/child_builder.rs). 按项目模块边界规则, **没有** `pub use` 重导出.

## 什么时候用 Builder

| 场景 | 推荐方式 |
|---|---|
| YAML 配置, `add_child` RPC 载荷 | `ChildDeclaration` + `TryFrom` |
| 测试, examples(示例), 代码里直接拼运行时规格 | `ChildSpecBuilder` |
| 只需 worker 默认 bundle(默认值包), 不想写链式调用 | `ChildSpec::worker(...)?` (内部委托 Builder) |

旧写法 (逐字段赋值) 仍可能出现在历史代码里, 新代码优先用 Builder.

## 入口方法

| 方法 | 用途 | 默认值要点 |
|---|---|---|
| `worker(id, name, kind, factory)` | 异步或阻塞 worker | 与 `ChildSpec::worker` 一致: `Transient` 重启, `Critical` 关键性, `TaskRole::Worker` 等 |
| `service(id, name, kind, factory)` | 常驻 service(服务) | 基于 worker 默认值: `TaskRole::Service`, `Critical` 关键性 |
| `job(id, name, kind, factory)` | 有限生命周期 job(一次性任务) | 基于 worker 默认值: `TaskRole::Job`, `Optional` 关键性 |
| `sidecar(id, name, kind, factory, sidecar_config)` | 跟随 primary child(主子任务) 的 sidecar(边车) | 基于 worker 默认值: `TaskRole::Sidecar`, 写入 `sidecar_config`, 并自动加入 primary child 依赖 |
| `supervisor(id, name)` | 嵌套 supervisor(监督器) | `kind = Supervisor`, `factory = None`, `task_role = Supervisor`, `criticality = Critical` |
| `new(id, name)` | 最小骨架 | 仅填 `id` / `name` 与 baseline(基线) 策略; 调用方需自行补 `kind`, 以及 worker 所需的 `factory` |

## 构建出口

| 方法 | 行为 |
|---|---|
| `build()` | 取出内部 `ChildSpec`, 调用 `validate()`, 成功返回 `Ok(spec)`, 失败返回 `SupervisorError` |

所有入口方法和 setter(设置器) 都返回 `ChildSpecBuilder`, 只表示"还在构造中". 只有 `build()` 会消费 builder(构建器), 并返回最终 `ChildSpec`.

**没有** `build_validated()`. 校验统一在 `build()` 内完成.

`ChildSpec::worker(...)` 同样返回 `Result<ChildSpec, SupervisorError>`, 实现为 `ChildSpecBuilder::worker(...).build()`.

## 基本用法

```rust
use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::task_role_defaults::TaskRole;
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;

fn build_worker() -> Result<ChildSpec, SupervisorError> {
    let factory = Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded }));
    ChildSpecBuilder::worker(
        ChildId::new("invoice-worker"),
        "Invoice Worker",
        TaskKind::AsyncWorker,
        factory,
    )
    .task_role(TaskRole::Worker)
    .tag("invoice")
    .build()
}
```

错误用 `?` 向上传播, 或在测试里 `build().expect("...")`.

## 链式 setter 范围

每个 setter 消费 `self` 并返回 `Self`, 可任意顺序链接 (在语义允许的前提下).

**策略类**: `isolation`, `restart_policy`, `shutdown_policy`, `health_policy`, `readiness_policy`, `backoff_policy`

**拓扑与分类**: `dependencies`, `dependency`, `tags`, `tag`, `criticality`, `task_role`, `without_task_role`, `sidecar_config`, `without_sidecar_config`, `severity`, `without_severity`, `group`, `without_group`

**配置块**: `health_check`, `without_health_check`, `readiness`, `without_readiness`, `resource_limits`, `without_resource_limits`, `command_permissions`, `environment`, `env_var`, `secrets`, `secret`, `cleanup_paths`, `cleanup_path`

**运行时**: `kind`, `factory`, `without_factory` (供 `new()` 或 supervisor 路径使用)

命名约定: 复数字段用 `dependencies(...)`, `tags(...)`; 单条便捷方法用 `dependency(...)`, `tag(...)`. `environment` / `env_var`, `secrets` / `secret`, `cleanup_paths` / `cleanup_path` 同理.

## 常见组合示例

### Service (服务)

常驻服务优先使用 `service(...)`, 不需要手动设置 `TaskRole::Service`:

```rust
ChildSpecBuilder::service(id, "API Service", TaskKind::AsyncWorker, factory)
    .tag("service")
    .build()?;
```

### Job (一次性任务)

有限生命周期任务优先使用 `job(...)`. 如果需要一次运行后停止, 可以继续覆盖 `restart_policy`:

```rust
ChildSpecBuilder::job(id, "Nightly Export", TaskKind::AsyncWorker, factory)
    .restart_policy(RestartPolicy::Temporary)
    .build()?;
```

### Sidecar (边车)

跟随主任务的边车优先使用 `sidecar(...)`. 这个入口会写入 `sidecar_config`, 并把 primary child 自动加入依赖:

```rust
use rust_supervisor::policy::task_role_defaults::SidecarConfig;

ChildSpecBuilder::sidecar(
    id,
    "Metrics Sidecar",
    TaskKind::AsyncWorker,
    factory,
    SidecarConfig::new(primary_id.clone(), false),
)
.build()?;
```

如果仍然用 setter(设置器) 手动设置 `task_role = Sidecar`, 就必须同时设置 `sidecar_config`, 否则 `build()` 校验失败.

### 从 `new()` 拼出 worker

```rust
ChildSpecBuilder::new(ChildId::new("custom"), "custom")
    .kind(TaskKind::AsyncWorker)
    .factory(factory)
    .build()?;
```

## 数据流 (简图)

```text
ChildSpecBuilder::worker / service / job / sidecar / supervisor / new
        |
        v
   链式 setter (policy, role, deps, env, ...)
        |
        v
   build()  -->  ChildSpec::validate()
        |
        +-- Ok(ChildSpec)  -->  Supervisor::start / 注册拓扑
        +-- Err(SupervisorError)
```

## 示例程序

可运行演示:

```bash
cargo run --example child_spec_builder
```

源码: [`examples/child_spec_builder.rs`](../../examples/child_spec_builder.rs). 覆盖 worker, service, job, sidecar, supervisor, `new()` 路径, 以及故意失败的 sidecar 组合.

## 测试与回归

外部测试: [`src/spec/tests/child_builder_test.rs`](../../src/spec/tests/child_builder_test.rs)

| 测试 | 验证点 |
|---|---|
| `worker_builder_matches_child_spec_worker_defaults` | Builder 与 `ChildSpec::worker` 字段一致 |
| `supervisor_builder_produces_valid_supervisor_child` | supervisor 入口无 factory 且可校验 |
| `service_builder_sets_service_role` | service 入口设置 `TaskRole::Service` 与 `Critical` 关键性 |
| `job_builder_sets_job_role_and_optional_criticality` | job 入口设置 `TaskRole::Job` 与 `Optional` 关键性 |
| `sidecar_builder_sets_sidecar_role_binding_and_dependency` | sidecar 入口设置绑定并自动加入 primary child 依赖 |
| `builder_setters_apply_expected_fields` | sidecar, dependency, tag 等 setter |
| `build_rejects_invalid_sidecar_combination` | 缺 `sidecar_config` 时 `build()` 失败 |
| `new_builder_can_build_valid_worker_with_factory` | `new()` 路径补全后可构建 |

运行:

```bash
cargo test --test child_builder_test
```

## 已知边界

- `TryFrom<ChildDeclaration>` 的默认值 bundle 尚未与 Builder 完全共用; 两处默认值仍可能独立演进, 改默认值时需同时对照声明转换路径.
- Builder 不负责 serde(序列化/反序列化); 动态加子任务仍走 `ChildDeclaration`.
- 未批量迁移历史 examples/tests 中的 `ChildSpec::worker`; 两种写法在运行时等价 (只要都处理 `Result`).

## 进一步阅读

- [`child-spec.md`](child-spec.md) — `ChildDeclaration` 与 `ChildSpec` 的关系, 以及简短 Builder 介绍
- [`docs/architecture.md`](../../docs/architecture.md) — 模块边界与禁止重导出规则
