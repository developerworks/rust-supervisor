## ChildSpec 和 ChildDeclaration 是什么关系?

语言: [English](../en/child-spec.html)

`ChildDeclaration`(子任务声明) 是配置和 RPC 进来的**对外声明**; `ChildSpec`(子任务规格) 是监督运行时真正拿来注册、启动、重启的**内部规格**. 二者字段大量重叠, 但职责不同, 中间用 `TryFrom` 做转换并补默认值.

## 各自是什么

|                    | `ChildDeclaration`                   | `ChildSpec`                        |
| ------------------ | ------------------------------------ | ---------------------------------- |
| **模块**           | `src/spec/child_declaration.rs`      | `src/spec/child.rs`                |
| **角色**           | YAML、`add_child` 载荷等**输入模型** | 注册表、控制循环里的**运行时模型** |
| **典型来源**       | 配置文件反序列化、动态加子任务请求   | 由声明转换而来, 或代码里直接构造   |
| **能否单独跑起来** | 不能, 没有工厂、没有完整策略对象     | 能, 监督器按它管生命周期           |

`ChildDeclaration` 侧重**可序列化、可校验的声明**: 名字、依赖名、环境变量、密钥占位符、`health_check` / `readiness` 配置块等, 并带 `validate_child_declaration` 等规则 (名字格式、`${SECRET}` 语法等).

`ChildSpec` 在声明字段之外, 还带上运行时必需的东西, 例如:

- 已生成的 `ChildId`(子任务标识) (由 `name` 推导)
- `factory: Option<Arc<dyn TaskFactory>>` (真正干活的任务工厂, **不参与 serde**)
- 已物化的 `HealthPolicy`、`ReadinessPolicy`、`ShutdownPolicy`、`BackoffPolicy`
- `isolation`、`cleanup_paths` 等运行期字段

## 怎么连起来

数据流可以看成:

```text
YAML / add_child RPC
        |
        v
  ChildDeclaration  ---- validate_child_declaration ----+
        |                                                  |
        | TryFrom<ChildDeclaration> for ChildSpec           |
        v                                                  |
     ChildSpec  --------------------------------------------+
        |
        v
  注册拓扑、启动子任务、策略流水线、重启/熔断等
```

转换实现在 `child_declaration.rs` 的 `TryFrom<ChildDeclaration> for ChildSpec`, 会做例如:

- `name` -> `ChildId::new(&decl.name)`
- `dependencies` 里的**名字** -> `Vec<ChildId>`
- `health_check` -> `HealthPolicy` (带默认间隔)
- `readiness` 有配置 -> `ReadinessPolicy::Explicit`, 否则 `Immediate`
- `shutdown_policy` / `backoff_policy` 等在转换时填**默认** (声明里未必逐项写出)

动态加子任务时, `PendingChild` 会**同时保留** `declaration` 和转换后的 `child_spec`, 审计里还会对声明做 SHA-256 (`declaration_hash`), 便于对账和补偿.

## 和共享类型的关系

`RestartPolicy`、`TaskKind`、`HealthCheckConfig` 等**公共枚举/配置结构**定义在 `child.rs`, `ChildDeclaration` **复用**它们, 避免两套平行类型. 但**顶层容器**仍是两个: 声明容器 vs 规格容器.

## 怎么记

- 写配置、接 API、做声明校验 -> 想 **`ChildDeclaration`**
- 看监督器怎么管某个子任务、策略引擎读什么 -> 想 **`ChildSpec`**
- 问「YAML 里写的和运行时用的是不是一回事」-> **同源信息, 不同生命周期阶段**: 声明是输入, 规格是落地后的形态

## 代码内构造

配置和 RPC 仍应使用 `ChildDeclaration`. 在 Rust 代码里直接构造运行时规格时, 推荐使用 `ChildSpecBuilder`:

```rust
use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::task_role_defaults::TaskRole;
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;

let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
let spec = ChildSpecBuilder::worker(
    ChildId::new("worker"),
    "worker",
    TaskKind::AsyncWorker,
    Arc::new(factory),
)
.task_role(TaskRole::Worker)
.tag("invoice")
.build()?;
```

入口方法:

| 方法                                | 用途                                                 |
| ----------------------------------- | ---------------------------------------------------- |
| `ChildSpecBuilder::worker(...)`     | 异步或阻塞 worker, 默认值与 `ChildSpec::worker` 一致 |
| `ChildSpecBuilder::service(...)`    | 常驻 service(服务), 自动设置 `TaskRole::Service`     |
| `ChildSpecBuilder::job(...)`        | 有限生命周期 job(一次性任务), 自动设置 `TaskRole::Job` |
| `ChildSpecBuilder::sidecar(...)`    | sidecar(边车), 自动设置绑定和主子任务依赖            |
| `ChildSpecBuilder::supervisor(...)` | 嵌套 supervisor, 无 factory                          |
| `ChildSpecBuilder::new(...)`        | 最小骨架, 需自行补 `kind` 和 `factory`               |

构建出口:

| 方法 | 行为 |
|---|---|
| `build()` | 构造后调用 `ChildSpec::validate()`, 失败时返回 `SupervisorError` |

`ChildSpec::worker(...)` 仍可使用, 内部委托 `ChildSpecBuilder::worker(...).build()`, 同样返回 `Result<ChildSpec, SupervisorError>`.

若你关心的是某条具体字段 (例如 `task_role` 只在哪一侧出现), 可以说字段名, 我可以对照 `TryFrom` 逐项说明映射与默认值.
