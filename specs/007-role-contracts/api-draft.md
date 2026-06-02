# API Draft(应用程序接口草案): 角色接入契约

本文固定使用者可见的 API(应用程序接口) 形态. 本文不规定最终代码实现细节, 但所有实现必须能回到本文的使用者路径.

## 1. 设计目标

角色接入契约必须让使用者先看到业务生命周期, 再由框架完成 runtime(运行时) 装配. 使用者在默认路径中不应该直接构造 `TaskFactory`(任务工厂), `TaskContext`(任务上下文) 或 `ChildSpec`(子任务规格).

## 2. 推荐入口: attribute macro(属性宏)

### 2.1 Service(服务)

```rust
struct QuoteService;

#[service(id = "quote-service", name = "Quote Service")]
impl QuoteService {
    async fn init(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        ctx.ready();
        Ok(())
    }

    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        ctx.wait_shutdown().await;
        Ok(())
    }

    async fn shutdown(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        Ok(())
    }
}
```

### 2.2 Worker(后台任务)

```rust
struct InvoiceWorker;

#[worker(id = "invoice-worker", name = "Invoice Worker")]
impl InvoiceWorker {
    async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        for batch in 1..=3 {
            ctx.heartbeat();
            self.process_batch(batch).await?;
        }

        Ok(())
    }
}
```

### 2.3 Job(一次性任务)

```rust
struct DailyReportJob;

#[job(id = "daily-report-job", name = "Daily Report Job")]
impl DailyReportJob {
    async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
        self.generate_report().await?;
        Ok(())
    }
}
```

### 2.4 Sidecar(边车)

```rust
struct MetricsSidecar;

#[sidecar(id = "metrics-sidecar", name = "Metrics Sidecar", primary = "quote-service")]
impl MetricsSidecar {
    async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        while !ctx.is_shutdown_requested() {
            self.flush_metrics().await?;
            ctx.heartbeat();
        }

        Ok(())
    }
}
```

### 2.5 Supervisor(监督器)

```rust
struct ApiSupervisor;

#[supervisor_role(id = "api-supervisor", name = "API Supervisor")]
impl ApiSupervisor {
    async fn build_tree(&mut self, ctx: &SupervisorContext) -> SupervisorResult<SupervisorSpec> {
        Ok(SupervisorSpec::root(vec![]))
    }
}
```

## 3. 显式入口: trait(特征)

显式入口用于不想使用 macro(宏) 的使用者, 也用于给宏生成代码提供稳定目标.

```rust
trait ServiceRole {
    async fn init(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        Ok(())
    }

    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()>;

    async fn shutdown(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        Ok(())
    }
}
```

其他角色使用同一规则.

| Role(角色) | Trait(特征) | Required method(必选方法) | Optional methods(可选方法) |
| --- | --- | --- | --- |
| `Service` | `ServiceRole` | `run` | `init`, `shutdown` |
| `Worker` | `WorkerRole` | `work` | `init`, `complete` |
| `Job` | `JobRole` | `run` | `init`, `complete` |
| `Sidecar` | `SidecarRole` | `run` | `init`, `shutdown` |
| `Supervisor` | `SupervisorRole` | `build_tree` | `run`, `shutdown` |

## 4. 模板入口: abstract default implementation(抽象默认实现)

模板入口用于减少重复空方法. 模板入口不得引入新生命周期名称, 只能为 trait(特征) 入口提供默认实现.

```rust
struct SimpleService<T> {
    inner: T,
}

impl<T> ServiceTemplate for SimpleService<T>
where
    T: ServiceRun,
{
    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        self.inner.run(ctx).await
    }
}
```

## 5. 统一命名规则

| Role(角色) | Macro(宏) | Context(上下文) | Result(结果) |
| --- | --- | --- | --- |
| `Service` | `#[service]` | `ServiceContext` | `ServiceResult` |
| `Worker` | `#[worker]` | `WorkerContext` | `WorkerResult` |
| `Job` | `#[job]` | `JobContext` | `JobResult` |
| `Sidecar` | `#[sidecar]` | `SidecarContext` | `SidecarResult` |
| `Supervisor` | `#[supervisor_role]` | `SupervisorContext` | `SupervisorResult` |

## 6. role context(角色上下文) 规则

所有 context(上下文) 都必须是窄接口. context(上下文) 可以包装 `TaskContext`(任务上下文), 但是不得把完整 `TaskContext`(任务上下文) 暴露给使用者.

```rust
impl ServiceContext {
    fn ready(&self);
    fn heartbeat(&self);
    fn is_shutdown_requested(&self) -> bool;
    async fn wait_shutdown(&self);
}
```

```rust
impl WorkerContext {
    fn ready(&self);
    fn heartbeat(&self);
    fn is_cancelled(&self) -> bool;
    async fn wait_cancelled(&self);
}
```

## 7. macro(宏) 生成规则

macro(宏) 必须完成下列动作.

1. 解析 `id`, `name`, `primary` 等 role metadata(角色元数据).
2. 检查必选 lifecycle method(生命周期方法) 是否存在.
3. 为目标类型生成对应 role trait(角色特征) 实现.
4. 生成 adapter(适配器) 所需的桥接代码.
5. 生成 `child_spec()` 便捷方法.
6. 设置 `ChildSpec.task_role` 为对应 `TaskRole`(任务角色).
7. 对 `Job`(一次性任务) 禁止 permanent restart(永久重启).
8. 对 `Sidecar`(边车) 要求 `primary` 必填.

## 8. runtime adapter(运行时适配器) 规则

adapter(适配器) 是唯一可以同时理解 role contract(角色契约) 和 `TaskFactory`(任务工厂) 的层. adapter(适配器) 必须把角色生命周期转换成现有 runtime(运行时) 可执行的任务函数.

```text
RoleContract(角色契约)
-> RoleAdapter(角色适配器)
-> TaskFactory(任务工厂)
-> Runtime control loop(运行时控制循环)
```

## 9. 错误与限制

- macro(宏) 的错误必须包含角色名称和缺失项.
- trait(特征) 入口必须保留结构化错误返回值.
- template(模板) 入口不得隐藏必选方法.
- 本功能不新增 compatibility exports(兼容导出).
- 形态 1 的自由函数宏和形态 3 的 derive macro(派生宏) 不进入默认路径.
