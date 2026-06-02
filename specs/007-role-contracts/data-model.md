# Data Model(数据模型): 角色接入契约

本文定义 `Role Contract API`(角色契约应用程序接口) 涉及的核心数据结构. 本文中的类型名称是设计目标, 具体模块路径以实现阶段为准.

## 1. RoleMetadata(角色元数据)

`RoleMetadata`(角色元数据) 保存宏参数和显式入口需要的基础声明.

```rust
pub struct RoleMetadata {
    pub id: ChildId,
    pub name: String,
    pub task_role: TaskRole,
    pub primary: Option<ChildId>,
}
```

**字段义务**:

- `id` 必须是稳定的 `ChildId`(子任务标识).
- `name` 必须是面向诊断的人类可读名称.
- `task_role` 必须对应 `TaskRole`(任务角色) 的 5 个已有变体.
- `primary` 只允许 `Sidecar`(边车) 使用, 其他角色必须为 `None`.

## 2. RoleContext(角色上下文)

每个角色必须拥有自己的 context(上下文) 类型. context(上下文) 可以包装 `TaskContext`(任务上下文), 但对外只能暴露角色需要的方法.

```rust
pub struct ServiceContext {
    inner: TaskContext,
}
```

通用能力如下.

| Capability(能力) | Service(服务) | Worker(后台任务) | Job(一次性任务) | Sidecar(边车) | Supervisor(监督器) |
| --- | --- | --- | --- | --- | --- |
| `ready` | yes | yes | yes | yes | yes |
| `heartbeat` | yes | yes | yes | yes | yes |
| `wait_shutdown` | yes | no | no | yes | yes |
| `wait_cancelled` | no | yes | yes | no | no |
| `primary_id` | no | no | no | yes | no |
| `supervisor_id` | no | no | no | no | yes |

## 3. RoleResult(角色结果)

每个角色必须拥有自己的 result(结果) 类型, 并且最终可以映射到 `TaskResult`(任务结果).

```rust
pub type ServiceResult<T = ()> = Result<T, ServiceError>;
pub type WorkerResult<T = ()> = Result<T, WorkerError>;
pub type JobResult<T = ()> = Result<T, JobError>;
pub type SidecarResult<T = ()> = Result<T, SidecarError>;
pub type SupervisorResult<T = ()> = Result<T, SupervisorRoleError>;
```

错误类型必须保留可观察信息. 最小字段如下.

```rust
pub struct RoleError {
    pub role: TaskRole,
    pub child_id: ChildId,
    pub phase: RoleLifecyclePhase,
    pub message: String,
}
```

## 4. RoleLifecyclePhase(角色生命周期阶段)

`RoleLifecyclePhase`(角色生命周期阶段) 用于诊断和错误映射.

```rust
pub enum RoleLifecyclePhase {
    Init,
    Run,
    Work,
    BuildTree,
    Complete,
    Shutdown,
}
```

阶段映射如下.

| Role(角色) | Lifecycle(生命周期) |
| --- | --- |
| `Service` | `Init -> Run -> Shutdown` |
| `Worker` | `Init -> Work -> Complete` |
| `Job` | `Init -> Run -> Complete` |
| `Sidecar` | `Init -> Run -> Shutdown` |
| `Supervisor` | `BuildTree -> Run -> Shutdown` |

## 5. RoleAdapter(角色适配器)

`RoleAdapter`(角色适配器) 负责把 role trait(角色特征) 转换成 `TaskFactory`(任务工厂).

```rust
pub struct ServiceRoleAdapter<T> {
    role: T,
    metadata: RoleMetadata,
}
```

adapter(适配器) 必须完成下列职责.

1. 创建对应 role context(角色上下文).
2. 按角色生命周期调用用户方法.
3. 把 role result(角色结果) 映射为 `TaskResult`(任务结果).
4. 把 role metadata(角色元数据) 写入 `ChildSpec`(子任务规格).
5. 保持 shutdown(关闭) 和 cancellation(取消) 的所有权清晰.

## 6. MacroInput(宏输入)

`MacroInput`(宏输入) 是宏包内部模型, 它不属于主包 runtime(运行时) API(应用程序接口).

```rust
pub struct MacroInput {
    pub metadata: RoleMetadata,
    pub impl_block: ImplBlockModel,
    pub required_methods: Vec<RoleLifecyclePhase>,
}
```

宏包必须分开 parse(解析) 和 expand(展开).

- `parse/` 只负责把 token stream(令牌流) 变成 `MacroInput`(宏输入).
- `expand/` 只负责把 `MacroInput`(宏输入) 变成生成代码.
- `attribute/` 只负责 proc-macro(过程宏) 入口注册和错误转换.
