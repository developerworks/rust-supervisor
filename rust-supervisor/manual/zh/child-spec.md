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

## `ChildSpec` 构造路径总览

仓库中构造 `ChildSpec`(子任务规格) 的路径可以分成 6 类. 这些路径面向不同使用场景, 不应该混成一个入口.

| 路径                    | 典型入口                                                                                      | 适用场景                                                                               | 校验方式                                                                        |
| ----------------------- | --------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Builder(构建器)         | `ChildSpecBuilder::worker`, `service`, `job`, `sidecar`, `supervisor`, `new`                  | Rust(编程语言) 代码里直接拼运行时规格                                                  | `build()` 调用 `ChildSpec::validate()`                                          |
| Worker 便捷函数         | `ChildSpec::worker(...)`                                                                      | 只需要 worker(后台任务) 默认值包                                                       | 内部委托 `ChildSpecBuilder::worker(...).build()`                                |
| 声明转换                | `TryFrom<ChildDeclaration> for ChildSpec`                                                     | YAML(配置文件), RPC(远程过程调用), dynamic add child(动态添加子任务)                   | 转换前走 `validate_child_declaration`, 转换后由 supervisor(监督器) 规格校验兜底 |
| Role Template(角色模板) | `ServiceTemplate::child_spec`, `JobTemplate::child_spec` 等                                   | 已经手写 `ServiceRole`(服务角色特征) 等 trait(特征), 但不想手写 adapter(适配器) 和规格 | 内部调用对应 `ChildSpecBuilder`                                                 |
| Macro(宏) 生成          | `#[service]`, `#[worker]`, `#[job]`, `#[sidecar]`, `#[supervisor_role]` 生成的 `child_spec()` | 默认角色接入路径, 使用者只写生命周期方法                                               | 宏生成代码调用对应 `ChildSpecBuilder`                                           |
| Serde(序列化和反序列化) | `serde_json::from_value::<ChildSpec>(...)`                                                    | 主要用于测试反序列化默认值和非法枚举                                                   | 不经过 builder(构建器), 使用前必须显式校验或进入后续规格校验                    |

几条关键边界:

- `ChildSpecBuilder::build()` 是 Rust(编程语言) 代码构造路径的主要出口.
- 配置和 RPC(远程过程调用) 不应该直接接收 `ChildSpec`, 应该先接收 `ChildDeclaration`(子任务声明), 再转换为 `ChildSpec`(子任务规格).
- `Role Template`(角色模板) 和 `Macro`(宏) 都不是新的运行时模型. 它们只是把角色生命周期对象装配成 adapter(适配器), 再调用 `ChildSpecBuilder` 生成规格.
- `Serde`(序列化和反序列化) 可以构造 `ChildSpec`, 因为 `ChildSpec` 派生了 `Deserialize`(反序列化特征). 这条路径不会自动调用 `ChildSpecBuilder::build()`.

相邻但不算构造 `ChildSpec` 的路径:

| 入口                                          | 为什么不算                                                                      |
| --------------------------------------------- | ------------------------------------------------------------------------------- |
| `SupervisorSpec::root(Vec<ChildSpec>)`        | 它接收已经构造好的子任务规格列表, 只构造 supervisor(监督器) 规格                |
| `SupervisorSpecBuilder::root(Vec<ChildSpec>)` | 它包装 supervisor(监督器) 规格构造, 不创建单个子任务规格                        |
| `ConfigState::to_supervisor_spec()`           | 它把 `ConfigState` 中已经保存的 `Vec<ChildSpec>` 组装成 supervisor(监督器) 规格 |
| `bind_child_factory(...)`                     | 它给已有 `ChildSpec` 绑定 factory(任务工厂), 不创建新的 `ChildSpec`             |
| `clone()`                                     | 它复制已有 `ChildSpec`, 不从输入模型生成新规格                                  |

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

| 方法                                | 用途                                                   |
| ----------------------------------- | ------------------------------------------------------ |
| `ChildSpecBuilder::worker(...)`     | 异步或阻塞 worker, 默认值与 `ChildSpec::worker` 一致   |
| `ChildSpecBuilder::service(...)`    | 常驻 service(服务), 自动设置 `TaskRole::Service`       |
| `ChildSpecBuilder::job(...)`        | 有限生命周期 job(一次性任务), 自动设置 `TaskRole::Job` |
| `ChildSpecBuilder::sidecar(...)`    | sidecar(边车), 自动设置绑定和主子任务依赖              |
| `ChildSpecBuilder::supervisor(...)` | 嵌套 supervisor, 无 factory                            |
| `ChildSpecBuilder::new(...)`        | 最小骨架, 需自行补 `kind` 和 `factory`                 |

构建出口:

| 方法      | 行为                                                             |
| --------- | ---------------------------------------------------------------- |
| `build()` | 构造后调用 `ChildSpec::validate()`, 失败时返回 `SupervisorError` |

`ChildSpec::worker(...)` 仍可使用, 内部委托 `ChildSpecBuilder::worker(...).build()`, 同样返回 `Result<ChildSpec, SupervisorError>`.

若你关心的是某条具体字段 (例如 `task_role` 只在哪一侧出现), 可以说字段名, 我可以对照 `TryFrom` 逐项说明映射与默认值.
