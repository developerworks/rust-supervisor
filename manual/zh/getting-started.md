# 快速开始

语言: [English](../en/getting-started.html)

> **步骤指引**: 本指南共 5 步(Step 1 of 5 至 Step 5 of 5).

## Step 1 of 5: 前置条件

主配置文件是 `examples/config/supervisor.yaml`. 配置必须通过 rust-config-tree(集中配置树) 加载 YAML(数据序列化格式), 然后形成 `ConfigState`(配置状态).

## Step 2 of 5: 最小运行命令

```bash
cargo run --example supervisor_quickstart
```

该示例执行固定路径:

- `load_config_from_yaml_file` 读取 YAML(数据序列化格式)
- `ConfigState::to_supervisor_spec` 派生 `SupervisorSpec`(监督器规格)
- `Supervisor::start` 启动 runtime(运行时)
- `current_state` 查询当前状态,
- `shutdown_tree` 关闭整棵树.

## Step 3 of 5: 最小代码路径

```rust
use rust_supervisor::config::loader::load_config_from_yaml_file;
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_from_yaml_file("examples/config/supervisor.yaml")?;
    let spec = state.to_supervisor_spec()?;
    let handle = Supervisor::start(spec).await?;
    let current = handle.current_state().await?;
    println!("{current:#?}");
    handle.shutdown_tree("operator", "quickstart complete").await?;
    Ok(())
}
```

## Step 4 of 5: 运行结果

当前示例用于验证接入路径, 不是业务任务模板. 使用者需要把自己的 worker(工作任务) 放入 `ChildSpec`(子任务规格) 和 `TaskFactory`(任务工厂), 不应该在业务代码中分散启动无人管理的后台任务.

## Step 5 of 5: 健康自检

启动后, supervisor(监督器) 会向 stdout(标准输出) 打印健康自检 JSON.
JSON schema(模式) 定义在 [health-selfcheck-schema.md](../specs/006-8-product-bundle-runbooks/contracts/health-selfcheck-schema.md).

预期输出示例:

```json
{
  "status": "ready",
  "supervisor_version": "0.1.2",
  "uptime_secs": 3600,
  "children": { "total": 5, "running": 5, "failed": 0 },
  "dashboard_link": "connected"
}
```

如果 `status` 不是 `"ready"`, 请参考 operations runbook(运维手册) 的故障排查步骤.

---

## 入口方法

`Supervisor`(监督器) 结构体 (`src/runtime/supervisor.rs`) 提供 3 个入口方法:

| 方法                                         | 输入                            | 适用场景                   |
| -------------------------------------------- | ------------------------------- | -------------------------- |
| `Supervisor::start(spec)`                    | `SupervisorSpec` (已构建的规格) | 编程式启动                 |
| `Supervisor::start_from_config_state(state)` | `ConfigState` (已验证的配置)    | 从配置加载器获得配置后启动 |
| `Supervisor::start_from_config_file(path)`   | YAML 文件路径                   | 从 YAML 文件直接启动       |

3 个方法最终都汇聚到私有方法 `start_with_policy()` (`src/runtime/supervisor.rs`),它执行:

1. 调用 `spec.validate()` 验证所有子进程声明
2. 创建 mpsc 命令通道和 broadcast 事件通道
3. 创建 `RuntimeControlPlane`(运行时控制平面) 和 `ObservabilityPipeline`(可观测管道)
4. 构建 `RuntimeControlState`(运行时控制状态)
5. `tokio::spawn(run_control_loop(...))` 启动异步控制循环
6. 启动 `RuntimeWatchdog`(运行时看门狗) 监控控制循环健康
7. 返回 `SupervisorHandle`(监督器句柄),用于后续发送命令(重启、关闭等)和订阅事件

### 用法示例

#### 从 YAML 文件启动 — `start_from_config_state`

完整示例见 [`examples/supervisor_quickstart.rs`](../../examples/supervisor_quickstart.rs),配置文件见 [`examples/config/supervisor.yaml`](../../examples/config/supervisor.yaml):

```rust
use rust_supervisor::config::loader::load_config_from_yaml_file;
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_from_yaml_file("examples/config/supervisor.yaml")?;
    let handle = Supervisor::start_from_config_state(state).await?;
    handle.shutdown_tree("operator", "quickstart complete").await?;
    Ok(())
}
```

`load_config_from_yaml_file` 返回 `ConfigState`,其 `to_supervisor_spec()` 方法内部由 `start_from_config_state` 自动调用.

#### 编程式构建规格启动 — `start`

完整示例见 [`examples/supervisor_tree_story.rs`](../../examples/supervisor_tree_story.rs):

```rust
use std::sync::Arc;
use rust_supervisor::id::types::ChildId;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::factory::{TaskResult, service_fn};

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let factory = service_fn(|ctx| async move {
        ctx.heartbeat();
        ctx.mark_ready();
        println!("child running at path={}", ctx.path);
        TaskResult::Succeeded
    });

    let child = ChildSpec::worker(
        ChildId::new("demo-worker"),
        "Demo Worker",
        TaskKind::AsyncWorker,
        Arc::new(factory),
    );

    let spec = SupervisorSpec::root(vec![child]);
    let handle = Supervisor::start(spec).await?;

    let state = handle.current_state().await?;
    println!("{state:#?}");
    handle.shutdown_tree("operator", "demo complete").await?;
    Ok(())
}
```

`ChildSpec::worker()` 自动设 `work_role = Some(WorkRole::Worker)`,等价于 YAML 中的 `work_role: worker`.

## WorkRole(工作角色) 行为差异

5 个 `WorkRole` 变体通过 `RoleDefaultPolicy::for_role()` 映射到 3 个差异维度,产生完全不同的默认生命周期行为:

| 维度                 | Service                    | Worker                     | Job                           | Sidecar                    | Supervisor                 |
| -------------------- | -------------------------- | -------------------------- | ----------------------------- | -------------------------- | -------------------------- |
| **成功退出时动作**   | `Restart`(重新启动)        | `Stop`(停止)               | `Stop`(停止)                  | `Restart`(重新启动)        | `Restart`(重新启动)        |
| **超时时动作**       | `RestartWithBackoff`(重试) | `RestartWithBackoff`(重试) | `StopAndEscalate`(停止并上报) | `RestartWithBackoff`(重试) | `RestartWithBackoff`(重试) |
| **默认最大重启次数** | 10                         | 3                          | 1                             | 5                          | 3                          |
| **默认严重等级**     | `Critical`                 | `Standard`                 | `Optional`                    | `Standard`                 | `Critical`                 |

具体差异来自 `src/policy/role_defaults.rs` 中的 5 个构造器:

| Role           | Description                                                                                                         |
| -------------- | ------------------------------------------------------------------------------------------------------------------- |
| **Service**    | 常驻服务,成功=重启,最多重试 10 次,严重等级 `Critical` -- 期望永远在线                                               |
| **Worker**     | 后台任务,成功=停止,最多重试 3 次,标准严重等级-- 完成即终止                                                          |
| **Job**        | 一次性任务,成功=停止,超时=直接上报(不再重试),最多重试 1 次,严重等级 Optional -- 跑完就结束                          |
| **Sidecar**    | 辅助进程,同 Service 的常驻行为但重启预算更低(5 次),且必须绑定 `SidecarConfig` 指定主服务(见 `SidecarConfig` 结构体) |
| **Supervisor** | 嵌套监管树,同 Service 的常驻行为,重启预算 3 次,严重等级 Critical                                                    |

`EffectivePolicy::merge()` 在 `work_role` 为 `None` 时会回退到 `WorkRole::Worker` 并记录警告. `semantic_conflicts_for_child()` 则检测角色语义冲突(如 Job 搭配永久重启策略会报错).
