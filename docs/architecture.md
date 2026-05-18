# 系统架构 (System Architecture)

> 最后更新: 2026-05-18 | 对应版本: 0.1.2

## 一、架构总览

`rust-tokio-supervisor` 是一个基于 Tokio(异步运行时) 的生产级任务监督库。它的架构围绕**三目录拆分**（核心库 + 中继 + 用户界面）和**三层架构**（控制面 + 状态面 + 事件面）展开。

### 三目录进程架构

```text
┌─────────────────────────────────────────────────────────────────┐
│                   生产部署拓扑 (三目录)                          │
│                                                                 │
│  ┌─────────────────────┐   ┌─────────────────────┐             │
│  │  Core Library       │   │  Relay               │             │
│  │  (目标进程)          │   │  (中继进程)           │             │
│  │                     │   │                      │             │
│  │  Unix Domain Socket │◄─►│  Unix Domain Socket  │             │
│  │  /run/.../*.sock    │   │  /run/.../relay.sock │             │
│  │                     │   │                      │             │
│  │  发出 hello/state   │   │  协议翻译:           │             │
│  │  events/logs 订阅   │   │  JSON ↔ WebSocket    │             │
│  │  响应 command.*     │   │  mTLS 会话门控       │             │
│  └─────────────────────┘   └──────────┬───────────┘             │
│                                        │                        │
│                                        │ wss://                 │
│                                        ▼                        │
│                             ┌──────────────────────┐            │
│                             │  User Interface       │            │
│                             │  (浏览器看板)          │            │
│                             │                      │            │
│                             │  Vue + shadcn-vue    │            │
│                             │  + Tailwind          │            │
│                             │  只连接 relay wss://  │            │
│                             └──────────────────────┘            │
└─────────────────────────────────────────────────────────────────┘
```

**核心原则**: 目标进程不把 IPC(进程间通信) 暴露到网络,只监听本机 Unix 域套接字。Windows 等非 Unix 平台通过 `#[cfg(unix)]` 在编译期排除 dashboard(看板) 模块,核心监督能力保持可用。

### 单进程三层架构

```text
┌──────────────────────────────────────────────────────────┐
│                    Supervisor Runtime                     │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Control Plane│  │  State Plane │  │  Event Plane  │   │
│  │  (控制面)     │  │  (状态面)    │  │  (事件面)     │   │
│  │               │  │              │  │               │   │
│  │  Supervisor   │  │  Supervisor │  │  Supervisor   │   │
│  │  Handle       │  │  State      │  │  Event        │   │
│  │               │  │              │  │               │   │
│  │  Control Loop │  │  ChildState  │  │  Event Stream │   │
│  │               │  │              │  │               │   │
│  │  Commands     │  │  Registry   │  │  Event        │   │
│  │  (add/remove/ │  │              │  │  Journal      │   │
│  │   pause/...)  │  │              │  │               │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                 │                 │            │
│         ▼                 ▼                 ▼            │
│  ┌──────────────────────────────────────────────────┐    │
│  │           Observability Pipeline                  │    │
│  │  (structured log, tracing, metrics, audit)        │    │
│  └──────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

## 二、核心数据流

### 2.1 启动流程

```text
YAML 配置
    │
    ▼
rust-config-tree v0.1.9 加载
    │
    ▼
ConfigState (不可变配置状态)
    │
    ├──→ SupervisorSpec (监督器规格)
    │        │
    │        ├──→ Supervisor::start(spec)
    │        │        │
    │        │        ├──→ 构建 SupervisorTree (监督树)
    │        │        ├──→ 启动 Control Loop (控制循环)
    │        │        ├──→ 初始化 Registry (注册表)
    │        │        └──→ 返回 SupervisorHandle (句柄)
    │        │
    │        └──→ (可选) Dashboard IPC Runtime (看板 IPC)
    │
    ├──→ 默认策略集 (restart/backoff/health/readiness/shutdown)
    └──→ 可观测性配置 (journal/metrics/audit)
```

### 2.2 子任务失败处理流水线

```text
Child 退出
    │
    ▼
Stage 1: classify exit (分类退出)
    ├── Success / NonZeroExit / Crash / Timeout / ExternalCancel / ManualStop
    │
    ▼
Stage 2: record failure window (记录失败窗口)
    ├── 滑动窗口计数器
    │
    ▼
Stage 3: evaluate budget (评估预算)
    ├── RestartBudgetTracker → BudgetVerdict::Granted | Exhausted
    │   预算不足 → 拒绝 (不经过熔断与退避)
    │
    ▼
Stage 4: decide action (决定动作)
    ├── MeltdownTracker (熔断检查) ─── 熔断 → GroupFuseTriggered
    ├── BackoffPolicy (退避计算) ───── 延迟值 + jitter
    ├── GroupStrategy (分组隔离) ───── 确认是否跨组传播
    ├── EscalationPolicy (升级策略) ── critical/optional 分叉
    │
    ▼
Stage 5: emit typed event (发射类型化事件)
    ├── SupervisorEvent 带 correlation_id
    ├── structured log / tracing / metrics / audit
    │
    ▼
Stage 6: execute action (执行动作)
    ├── RestartAfter(delay) / DoNotRestart / Quarantine
    ├── EscalateToParent / ShutdownTree
```

### 2.3 关闭流水线

```text
shutdown_tree 命令
    │
    ▼
Phase 1: request stop (请求停止)
    ├── 传播 CancellationToken 到所有 Child
    │
    ▼
Phase 2: graceful drain (优雅排空)
    ├── 按声明顺序的逆序等待 Child 自行退出
    ├── 每个 Child 有独立 graceful_timeout
    │
    ▼
Phase 3: abort stragglers (强制中止滞留任务)
    ├── 超时后 AbortHandle 中止 AsyncWorker
    ├── BlockingWorker 不保证立即中止
    │
    ▼
Phase 4: reconcile (状态对账)
    ├── 统一更新 Registry / CurrentState / Metrics / EventJournal
    ├── 返回 ShutdownResult (含每 Child 结果 + 对账报告)
```

## 三、模块架构

### 3.1 模块依赖图

```text
                         ┌──────────┐
                         │    id    │ (ChildId, SupervisorId, SupervisorPath)
                         └────┬─────┘
                              │
                    ┌─────────▼─────────┐
                    │      error        │ (SupervisorError, TaskFailureKind)
                    └─────────┬─────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                     │
   ┌─────▼──────┐     ┌──────▼──────┐     ┌───────▼────────┐
   │  config     │     │   spec      │     │    task        │
   │ (ConfigState)│    │ (ChildSpec, │     │ (TaskFactory,  │
   │ (Supervisor │     │  Supervisor │     │  TaskContext,  │
   │  Config)    │     │  Spec)      │     │  TaskResult)   │
   └─────────────┘     └──────┬──────┘     └───────┬────────┘
                              │                     │
                              └──────────┬──────────┘
                                         │
                              ┌──────────▼──────────┐
                              │       tree          │
                              │ (SupervisorTree,    │
                              │  restart/shutdown   │
                              │  order)             │
                              └──────────┬──────────┘
                                         │
              ┌──────────────────────────┼──────────────────────────┐
              │                          │                          │
    ┌─────────▼──────────┐   ┌──────────▼──────────┐   ┌───────────▼──────────┐
    │  child_runner       │   │     policy          │   │     control          │
    │ (ChildRunner,       │   │ (PolicyEngine,       │   │ (SupervisorHandle,   │
    │  TaskExit,          │   │  MeltdownTracker,    │   │  ControlCommand,     │
    │  ChildRunReport)    │   │  BackoffPolicy,      │   │  ChildControlResult) │
    └─────────┬──────────┘   │  RestartBudget,      │   └───────────┬──────────┘
              │              │  GroupStrategy)       │               │
              │              └──────────┬──────────┘               │
              │                         │                          │
              └──────────┬──────────────┼──────────────┬───────────┘
                         │              │              │
              ┌──────────▼──────────┐   │   ┌──────────▼──────────┐
              │     runtime          │◄──┘   │   shutdown          │
              │ (Supervisor,         │       │ (ShutdownCoordinator│
              │  ControlLoop,        │       │  ShutdownResult,    │
              │  SupervisionPipeline,│       │  ShutdownPhase)     │
              │  ChildSlot,          │       └─────────────────────┘
              │  RuntimeWatchdog)    │
              └──────┬───────────────┘
                     │
    ┌────────────────┼────────────────┐
    │                │                │
    ▼                ▼                ▼
┌──────────┐  ┌──────────┐  ┌──────────────────┐
│ registry │  │  state   │  │     observe       │
│ (Registry│  │ (Supervi │  │ (Observability    │
│  Store,  │  │  sorState│  │  Pipeline,        │
│  Child   │  │  ChildSt │  │  MetricsFacade,   │
│  Runtime)│  │  ate)    │  │  FairnessProbe)   │
└──────────┘  └──────────┘  └──────────────────┘
                     │                │
                     ▼                ▼
              ┌──────────┐  ┌──────────────────┐
              │  event   │  │     journal       │
              │ (Supervi │  │ (EventJournal)    │
              │  sorEvent│  └──────────────────┘
              │  Correla │
              │  tionId) │
              └──────────┘
```

### 3.2 模块职责总表

| 模块                    | 职责                 | 关键类型                                                                                                |
| ----------------------- | -------------------- | ------------------------------------------------------------------------------------------------------- |
| `id`                    | 标识符与路径         | `ChildId`, `SupervisorId`, `SupervisorPath`, `Generation`                                               |
| `error`                 | 错误类型与失败分类   | `SupervisorError`, `TaskFailureKind`, `TaskFailure`                                                     |
| `config`                | 配置加载与校验       | `SupervisorConfig`, `ConfigState`, `loader`                                                             |
| `spec`                  | 声明式规格定义       | `ChildSpec`, `SupervisorSpec`, `SupervisionStrategy`                                                    |
| `task`                  | 任务工厂与上下文     | `TaskFactory`, `TaskContext`, `TaskResult`, `Service`                                                   |
| `tree`                  | 监督树构建与排序     | `SupervisorTree`, `SupervisorTreeNode`, `startup_order`, `shutdown_order`                               |
| `child_runner`          | 子任务执行与退出处理 | `ChildRunner`, `TaskExit`, `ChildRunReport`                                                             |
| `policy`                | 策略决策引擎         | `PolicyEngine`, `RestartDecision`, `MeltdownTracker`, `BackoffPolicy`, `RestartBudget`, `GroupStrategy` |
| `control`               | 控制命令与句柄       | `SupervisorHandle`, `ControlCommand`, `ChildControlResult`                                              |
| `runtime`               | 运行时管理与控制循环 | `Supervisor`, `ControlLoop`, `SupervisionPipeline`, `ChildSlot`, `RuntimeWatchdog`                      |
| `shutdown`              | 关闭协调与报告       | `ShutdownCoordinator`, `ShutdownResult`, `ShutdownPhase`                                                |
| `registry`              | 运行时注册表         | `RegistryStore`, `ChildRuntime`                                                                         |
| `state`                 | 当前状态模型         | `SupervisorState`, `ChildState`, `ChildLifecycleState`                                                  |
| `event`                 | 事件模型与载荷       | `SupervisorEvent`, `CorrelationId`, `EventSequence`                                                     |
| `journal`               | 事件日志缓冲区       | `EventJournal` (环形缓冲区)                                                                             |
| `observe`               | 可观测性管线         | `ObservabilityPipeline`, `MetricsFacade`, `FairnessProbe`                                               |
| `health`                | 健康检查             | `HealthPolicy`, `Heartbeat`                                                                             |
| `readiness`             | 就绪检查             | `ReadinessPolicy`, `ReadySignal`                                                                        |
| `summary`               | 运行摘要             | `RunSummary`, `RunSummaryBuilder`                                                                       |
| `test_support`          | 测试支持             | 测试记录器与辅助函数                                                                                    |
| `ipc` (Unix only)       | IPC 安全控制         | `IpcSecurityPipeline`, 9 项控制点 C1-C9                                                                 |
| `dashboard` (Unix only) | 看板 IPC 服务        | `DashboardState`, IPC 协议、注册心跳                                                                    |
| `platform`              | 平台工具             | 平台相关工具函数                                                                                        |
| `types`                 | 通用类型             | 共享的类型别名                                                                                          |

### 3.3 模块边界规则

- **禁止 `pub use`** (公开重导出): 用户必须通过绝对模块路径导入,如 `rust_supervisor::runtime::supervisor::Supervisor`
- **禁止 `super::` 相对路径**: 所有内部导入使用 `crate::` 绝对路径
- **禁止内联单元测试**: 测试文件放在 `src/<module>/tests/*_test.rs` 或 `src/tests/*_test.rs`
- **禁止兼容导出**: 无旧接口别名、迁移层或废弃门面

## 四、关键架构决策

### 4.1 为什么使用 Tokio 原语而非 actor 框架

项目直接使用 `JoinSet`, `CancellationToken`, `mpsc`, `broadcast` 等 Tokio 原语构建监督器核心。

- `JoinSet` 提供结构化并发: drop 时中止所有任务, `abort_all` 后可通过 `join_next` 排空
- `CancellationToken` 提供父子关闭传播: 父令牌可取消子令牌, 子令牌不可反向取消父令牌
- `mpsc` 通道承载控制命令, `broadcast` 通道分发事件

不引入 actor 框架的原因: actor 框架的监督树与本项目需要监督树语义不同, 且会增加不必要的框架假设。

### 4.2 三层平面分离

| 平面                  | 职责               | 数据源             | 消费者                          |
| --------------------- | ------------------ | ------------------ | ------------------------------- |
| Control Plane(控制面) | 接收命令、调度动作 | `SupervisorHandle` | 控制循环                        |
| State Plane(状态面)   | 暴露当前状态       | 运行时注册表       | `current_state` 查询、dashboard |
| Event Plane(事件面)   | 记录生命周期历史   | 控制循环各阶段     | subscriber、audit、replay、测试 |

状态面回答"现在是什么",事件面回答"发生过什么",两者不混用。

### 4.3 策略管线顺序

策略评估按 `budget → meltdown → backoff` 固定顺序执行:

- 预算不足时直接拒绝,不经过熔断与退避
- 熔断触发后不计算退避延迟
- 公平性探针在控制循环主路径上持续运行

### 4.4 配置模型

- 所有可调配置通过 `rust-config-tree` v0.1.9 从 YAML 加载
- `SupervisorConfig` 同时支持 `confique::Config`, `schemars::JsonSchema`, `serde::Serialize/Deserialize`
- `ConfigState` 加载后不可变,派生 `SupervisorSpec`、默认策略、关闭预算和可观测性配置
- 模块内部不得保存可调配置默认值

### 4.5 平台编译隔离

```rust
// 非 Unix 平台编译期排除 dashboard 和 ipc 模块
#[cfg(unix)]
pub mod dashboard;
#[cfg(unix)]
pub mod ipc;
```

核心监督能力在所有 Rust 支持平台上可编译。Unix 平台额外提供 dashboard IPC(进程间通信) 能力。

## 五、配置结构

```yaml
supervisor:
  strategy: OneForAll # 监督策略

policy:
  child_restart_limit: 10 # 子任务窗口内最大重启次数
  child_restart_window_ms: 60000 # 子任务重启统计窗口
  supervisor_failure_limit: 30 # 监督器级故障上限
  supervisor_failure_window_ms: 60000 # 监督器级故障窗口
  initial_backoff_ms: 100 # 初始退避延迟
  max_backoff_ms: 5000 # 最大退避延迟
  jitter_ratio: 0.10 # 抖动比率
  heartbeat_interval_ms: 1000 # 心跳间隔
  stale_after_ms: 3000 # 心跳过期阈值

shutdown:
  graceful_timeout_ms: 5000 # 优雅关闭超时
  abort_wait_ms: 1000 # 强制中止等待

observability:
  event_journal_capacity: 256 # 事件日志容量
  metrics_enabled: true
  audit_enabled: true

ipc: # 可选,仅 Unix
  enabled: true
  target_id: payments-worker-a
  path: /run/rust-supervisor/payments-worker-a.sock
  permissions: "0600"
  bind_mode: create_new
  registration:
    enabled: true
    relay_registration_path: /run/rust-supervisor/dashboard-relay-registration.sock
    display_name: "payments worker a"
    lease_seconds: 30
    registration_heartbeat_interval_seconds: 15
```

## 六、可观测性架构

```text
生命周期事实
    │
    ├──→ SupervisorEvent (类型化事件)
    │       ├──→ EventStream (broadcast 通道, 供 subscriber 消费)
    │       ├──→ EventJournal (环形缓冲区, 固定容量)
    │       └──→ TestRecorder (测试用)
    │
    ├──→ Structured Log (tracing event)
    │
    ├──→ Tracing Span (每个 child attempt 一个 span)
    │       ├── supervisor_restart_total (counter)
    │       ├── supervisor_child_state (gauge)
    │       ├── supervisor_child_uptime_seconds (histogram)
    │       ├── supervisor_backoff_seconds (histogram)
    │       ├── supervisor_healthcheck_latency_seconds (histogram)
    │       ├── supervisor_meltdown_total (counter)
    │       ├── supervisor_shutdown_duration_seconds (histogram)
    │       ├── supervisor_event_lag_total (counter)
    │       └── supervisor_config_version (gauge)
    │
    ├──→ Audit Event (控制命令审计)
    │       ├── command_id
    │       ├── requested_by
    │       ├── reason
    │       ├── target_path
    │       ├── accepted_at
    │       └── result
    │
    └──→ RunSummary (运行摘要, 故障升级或关闭时生成)
            ├── started_at / finished_at
            ├── shutdown cause
            ├── restart count
            ├── failure list
            ├── recent events
            ├── final current state
            └── final decision
```

## 七、IPC 安全控制点

看板 IPC 配置了 9 项安全控制点 (C1-C9):

| 编号 | 控制点                     | 说明                            |
| ---- | -------------------------- | ------------------------------- |
| C1   | socket owner 校验          | 套接字文件所有者与进程 UID 一致 |
| C2   | peer credentials 校验      | 对端进程身份验证                |
| C3   | command authorization      | 命令级别授权模型                |
| C4   | replay protection          | 重放攻击防护 (ReplayWindow)     |
| C5   | request size limit         | 请求体大小上限                  |
| C6   | rate limit                 | 速率限制 (TokenBucket)          |
| C7   | audit persistence          | 审计持久化 (磁盘落盘)           |
| C8   | command idempotency key    | 命令幂等键                      |
| C9   | external command allowlist | 外部命令白名单                  |

## 八、聚合仓库架构

完整产品由三个独立仓库组成:

| 仓库                  | 目录                      | 进程角色         | 技术栈                      |
| --------------------- | ------------------------- | ---------------- | --------------------------- |
| rust-supervisor       | `~/rust-supervisor`       | 目标进程(核心库) | Rust + Tokio                |
| rust-supervisor-relay | `~/rust-supervisor-relay` | 中继进程         | Rust + Tokio + TLS          |
| rust-supervisor-ui    | `~/rust-supervisor-ui`    | 浏览器看板       | Vue + shadcn-vue + Tailwind |

## 九、相关文档

- [产品路线图](product-roadmap.md)
- [质量门禁 - 英文](en/quality-gates.md)
- [质量门禁 - 中文](zh/quality-gates.md)
- [并行治理 - 英文](en/parallel-governance.md)
- [并行治理 - 中文](zh/parallel-governance.md)
- [公共 API 契约](../specs/001-create-supervisor-core/contracts/public-api.md)
- [词汇表](../specs/001-create-supervisor-core/glossary.md)
- [数据模型](../specs/001-create-supervisor-core/data-model.md)
