# Data Model(数据模型): 创建监督器核心

## Entity Overview(实体概览)

模型分为 centralized configuration(集中化配置),declarative configuration(声明式配置),runtime state(运行时状态),policy decision(策略决定),control-plane command(控制平面命令),current state(当前状态),lifecycle event(生命周期事件),observability plane(可观测性平面),documentation artifact(文档产物),glossary artifact(词汇表产物),coding standard(编码标准),cognitive complexity limit(认知复杂度预算),maintainability profile(可维护性画像),parallel execution governance(并行执行治理),SBOM artifact(软件物料清单产物) 和 release package(发布包).

## Configuration Entities(配置实体)

### SupervisorConfig(监督器配置)

- `tree`: rust-config-tree(集中配置树) 读取到的 supervisor tree(监督树) 配置.
- `defaults`: restart(重启),backoff(退避),health(健康),readiness(就绪),shutdown(关闭) 和 meltdown(熔断) 的 configured value(配置值).
- `observability`: structured log(结构化日志),tracing(结构化追踪),metrics(指标),audit(审计),event journal(事件日志缓冲区) 和 test recorder(测试记录器) 配置.
- `shutdown`: root shutdown(根关闭),blocking task(阻塞任务) 和 escalation(升级) 的预算.
- `examples`: 示例使用的输入配置路径和说明.

**Validation rules(校验规则)**:

- 所有可调配置必须通过 rust-config-tree(集中配置树) 进入 `SupervisorConfig`(监督器配置).
- 主配置文件必须使用 YAML(数据序列化格式),示例配置路径必须使用 `examples/config/supervisor.yaml`.
- 模块内部不得保存分散的可调默认值.
- 配置加载失败时不得启动部分 supervisor tree(监督树).

### ConfigState(配置状态)

- `version`: 单调配置版本.
- `loaded_at`: 配置加载时间.
- `checksum`: 规范化配置内容的校验和.
- `source_tree`: rust-config-tree(集中配置树) 的来源 include tree(包含树).
- `supervisor_spec`: 派生后的 `SupervisorSpec`(监督器规格).
- `defaults`: 派生后的集中默认值.
- `observability`: 派生后的可观测性设置.

**Validation rules(校验规则)**:

- `ConfigState`(配置状态) 加载后不可变.
- `SupervisorSpec`(监督器规格),默认策略,关闭预算和可观测性选项必须来自同一个 `ConfigState`(配置状态).
- lifecycle event(生命周期事件),current state(当前状态),run summary(运行摘要) 和 structured log(结构化日志) 必须携带可关联的 `config_version`(配置版本).

## Declarative Entities(声明式实体)

### SupervisorSpec(监督器规格)

- `id`: 稳定 supervisor(监督器) 标识.
- `path`: supervisor(监督器) 范围的稳定 `SupervisorPath`(监督器路径).
- `strategy`: `SupervisionStrategy`(监督策略).
- `children`: 有序 child(子任务) 定义.
- `meltdown_policy`: supervisor-level fuse(监督器级熔断).
- `default_restart_policy`: child(子任务) 默认重启策略.
- `default_backoff_policy`: child(子任务) 默认退避策略.
- `default_health_policy`: 默认 heartbeat/stale(心跳和过期) 策略.
- `default_readiness_policy`: 默认 readiness(就绪) 策略.
- `default_shutdown_policy`: 默认 four-stage shutdown(四阶段关闭) 策略.
- `config_version`: 派生该规格的配置版本.
- `restart_limit`: supervisor-level(监督器级) 默认重启次数限制,可为空.
- `escalation_policy`: supervisor-level(监督器级) 默认升级策略,可为空.
- `group_strategies`: 基于 child tag(子任务标签) 的 group strategy(分组策略) 集合.
- `child_strategy_overrides`: 单个 child(子任务) 的 per-child override(子任务级覆盖) 集合.
- `dynamic_supervisor_policy`: 控制 dynamic child manifest(动态子任务清单文本) 添加的策略.

**Validation rules(校验规则)**:

- `path` 必须从 `/root` 开始.
- 同一个 supervisor(监督器) 内的 child id(子任务标识) 必须唯一.
- child(子任务) 顺序必须稳定,因为 `RestForOne`(从失败处开始) 依赖定义顺序.
- 每个配置过 group strategy(分组策略) 的 group(分组) 必须至少匹配一个 child tag(子任务标签).
- 同一个 child(子任务) 不得同时属于多个配置过 group strategy(分组策略) 的 group(分组).
- child strategy override(子任务级覆盖) 必须引用已声明 child(子任务),并且同一个 child(子任务) 只能出现一次.

### ChildSpec(子任务规格)

- `id`: 父级内唯一的稳定 `ChildId`(子任务标识).
- `name`: 便于阅读的 child(子任务) 名称.
- `kind`: `Worker`(工作任务) 或 `Supervisor`(监督器).
- `factory`: worker(工作任务) 的 task factory(任务工厂),或嵌套 supervisor spec(监督器规格).
- `restart_policy`: `Permanent`(永久),`Transient`(瞬时) 或 `Temporary`(临时).
- `shutdown_policy`: graceful timeout(优雅关闭超时) 和 abort wait(强制终止等待).
- `health_policy`: heartbeat interval(心跳间隔) 和 stale-after threshold(过期阈值).
- `readiness_policy`: immediate readiness(立即就绪) 或 explicit readiness(显式就绪).
- `backoff_policy`: base delay(基础延迟),max delay(最大延迟),jitter(抖动) 和 reset-after(稳定后重置).
- `dependencies`: 必须先启动的路径或标识.
- `tags`: 低基数标签,用于筛选.
- `criticality`: `Critical`(关键) 或 `Degraded`(可降级).

**Validation rules(校验规则)**:

- `id` 必须可以用于路径,并且不能为空.
- `name` 必须不能为空.
- `dependencies` 必须引用同一棵树中可用的 sibling(同级) 或 ancestor(祖先).
- `jitter`(抖动) 在测试中必须可以关闭或确定.
- explicit readiness(显式就绪) 的 child(子任务) 在报告 ready(已就绪) 前不能进入 `Ready`(已就绪) 状态.

### TaskKind(任务类型)

- `AsyncWorker`(异步工作任务): 可以通过取消令牌和 abort(强制终止) 完成收尾.
- `BlockingWorker`(阻塞工作任务): 代表 `spawn_blocking`(阻塞任务启动) 或其它不可立即 abort(强制终止) 的阻塞工作.
- `Supervisor`(监督器): 代表嵌套 supervisor(监督器) 节点.

**Validation rules(校验规则)**:

- `BlockingWorker`(阻塞工作任务) 必须拥有独立 shutdown policy(关闭策略) 和 escalation policy(升级策略).
- 关闭超时后, 系统不得把 `BlockingWorker`(阻塞工作任务) 当作普通 async worker(异步工作任务) 处理.

### TaskFactory(任务工厂)

- 每个 generation/attempt(代次和尝试) 构建一个新的 task future(任务异步值).
- 接收 `TaskContext`(任务上下文).
- 不把跨重启持久任务状态存进 supervisor(监督器).

### Service(服务特征)

- 建立在 `TaskFactory`(任务工厂) 之上的项目自有适配层.
- 允许调用者以 service object(服务对象) 或 `service_fn`(函数适配器) 形式接入 supervisor(监督器).
- 不能替换 `TaskFactory`(任务工厂) 内核.
- 不能成为旧接口别名,迁移层,兼容包装函数或第三方 API(接口) 形状复制.

**Validation rules(校验规则)**:

- `Service`(服务特征) 适配后仍必须为每次 attempt(尝试) 构造 fresh future(新异步任务).
- `service_fn`(函数适配器) 不得隐藏 `TaskContext`(任务上下文) 中的取消,心跳,就绪和事件接收点.

### NamingRule(命名规则)

- `ConfigState`: 配置加载和校验后的不可变配置状态.
- `SupervisorState`: 当前监督树状态.
- `ChildState`: 当前子任务状态.
- `current_state`: 运行时句柄上的当前状态查询命令.
- `state`: 状态模块和测试命名边界.

**Validation rules(校验规则)**:

- 源码,示例,契约和文档不得使用任何 `*Snapshot` 代码命名.
- 源码,示例,契约和文档不得使用任何 `*View` 代码命名.
- 源码,示例,契约和文档不得提供 `snapshot()` 查询方法.
- 源码,示例,契约和文档不得使用 `state_view` 作为模块名,文件名,方法名或字段名.
- 命名检查必须把 `ConfigState`(配置状态),`SupervisorState`(监督器状态),`ChildState`(子任务状态),`current_state`(当前状态) 和 `state`(状态) 视为唯一正式命名.

### TaskContext(任务上下文)

- `child_id`
- `path`
- `generation`
- `attempt`
- `cancel`
- `events`
- `heartbeat`
- `readiness`
- `config_version`

**Validation rules(校验规则)**:

- 新 attempt(尝试) 必须获得新的 `TaskContext`(任务上下文).
- parent cancellation(父取消) 必须取消 child token(子令牌).
- child cancellation(子取消) 不得取消 parent token(父令牌).

## Runtime Entities(运行时实体)

### ChildRuntime(子任务运行态)

- `spec`: 关联的 `ChildSpec`(子任务规格).
- `state`: 当前 `ChildState`(子任务状态).
- `generation`: 重启 generation(代次).
- `attempt`: 当前 generation(代次) 内的 attempt(尝试次数).
- `heartbeat`: 最新 heartbeat(心跳) 记录.
- `join_handle`: 运行中任务的所有权句柄.
- `cancel_token`: child cancellation token(子任务取消令牌).
- `restart_count`: 当前 child window(子任务窗口) 内的重启次数.
- `recent_failures`: 有序失败记录.
- `last_failure`: 最近一次失败,可为空.
- `last_policy_decision`: 最近一次策略决定,可为空.

### Registry(注册表)

- 把 `SupervisorPath`(监督器路径) 映射到 child spec(子任务规格) 和 runtime(运行态).
- 维护定义顺序.
- 支持 add(添加),remove(移除),query(查询),pause(暂停),resume(恢复),quarantine(隔离) 和 shutdown(关闭) 查找.

### SupervisorRuntime(监督器运行时)

- 在一个 supervisor(监督器) 范围内拥有 config state(配置状态),registry(注册表),control loop(控制循环),child runner(子任务运行器),policy engine(策略引擎),event bus(事件总线),current state store(状态存储),observability pipeline(可观测性管线) 和 shutdown coordinator(关闭协调器).

## Policy Entities(策略实体)

### SupervisionStrategy(监督策略)

- `OneForOne`: 只重启失败 child(子任务).
- `OneForAll`: 停止范围内所有 child(子任务),再按定义顺序重启.
- `RestForOne`: 停止失败 child(子任务) 和其后 child(子任务),再按定义顺序重启.

### GroupStrategy(分组策略)

- `group`: child tag(子任务标签),用于选择组内 child(子任务).
- `strategy`: 组内使用的 `SupervisionStrategy`(监督策略).
- `restart_limit`: 可选 group-level(分组级) 重启次数限制.
- `escalation_policy`: 可选 group-level(分组级) 升级策略.

### ChildStrategyOverride(子任务级覆盖)

- `child_id`: 被覆盖的 child(子任务).
- `strategy`: 子任务失败时使用的 `SupervisionStrategy`(监督策略).
- `restart_limit`: 可选 child-level(子任务级) 重启次数限制.
- `escalation_policy`: 可选 child-level(子任务级) 升级策略.

### RestartLimit(重启次数限制)

- `max_restarts`: 窗口内最大重启次数.
- `window`: 统计重启次数的窗口.

### EscalationPolicy(升级策略)

- `EscalateToParent`(升级到父级): 把失败交给父 supervisor(监督器) 治理.
- `ShutdownTree`(关闭整棵树): 停止当前 supervisor tree(监督树).
- `QuarantineScope`(隔离范围): 隔离选中的重启范围.

### DynamicSupervisorPolicy(动态监督器策略)

- `enabled`: 是否允许运行时添加 dynamic child manifest(动态子任务清单文本).
- `child_limit`: 声明 child(子任务) 加动态 manifest(清单文本) 的可选总数上限.

### StrategyExecutionPlan(策略执行计划)

- `failed_child`: 触发计划的 child(子任务).
- `strategy`: 本次执行选中的监督策略.
- `scope`: 本次要重启的 child id(子任务标识) 列表.
- `group`: 约束本次计划的 group(分组),可为空.
- `restart_limit`: 本次计划选中的重启次数限制,可为空.
- `escalation_policy`: 本次计划选中的升级策略,可为空.
- `dynamic_supervisor_enabled`: 本次规格下 dynamic supervisor(动态监督器) 是否启用.

### RestartPolicy(重启策略)

- `Permanent`: 正常退出或异常退出后都重启.
- `Transient`: 异常退出,panic(恐慌),timeout(超时) 或 unhealthy(不健康) 后重启.
- `Temporary`: 永不重启.

### BackoffPolicy(退避策略)

- `initial`: 从 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置读取的初始退避.
- `max`: 从 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置读取的最大退避.
- `jitter`: 从 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置读取的抖动比例.
- `reset_after`: 从 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置读取的稳定后重置窗口.
- `test_jitter`: 测试中关闭或使用确定性来源.

### ReadinessPolicy(就绪策略)

- `Immediate`(立即就绪): child(子任务) 进入 running(运行中) 后可以自动进入 ready(已就绪).
- `Explicit`(显式就绪): child(子任务) 必须通过 `TaskContext`(任务上下文) 报告 ready(已就绪).

**Rules(规则)**:

- explicit readiness(显式就绪) 报告前, current state(当前状态) 和 event(事件) 不得把 child(子任务) 显示为 ready(已就绪).
- child(子任务) 第一次进入 ready(已就绪) 时必须发送 `ChildReady` 事件.

### MeltdownPolicy(熔断策略)

- `child_max_restarts`: 从 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置读取的子任务重启上限.
- `child_window`: 从 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置读取的子任务重启窗口.
- `supervisor_max_failures`: 从 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置读取的监督器失败上限.
- `supervisor_window`: 从 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置读取的监督器失败窗口.
- `reset_after`: 从 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置读取的稳定后计数器清理窗口.

### RestartDecision(重启决策)

- `DoNotRestart`(不重启)
- `RestartAfter(duration)`(延迟后重启)
- `Quarantine`(隔离)
- `EscalateToParent`(升级到父级)
- `ShutdownTree`(关闭整棵树)

## State Machines(状态机)

### ChildState(子任务状态)

```text
Declared
  -> Starting
  -> Running
  -> Ready
  -> Restarting
  -> Paused
  -> Quarantined
  -> ShuttingDown
  -> Stopped
  -> Failed
```

**Rules(规则)**:

- `Quarantined`(已隔离),`Stopped`(已停止) 和 `Failed`(已失败) 对自动重启来说是 terminal(终态).
- 手动控制命令可以把 `Paused`(已暂停) 通过 `resume`(恢复) 移回 `Running`(运行中).
- `Restarting`(正在重启) 必须拥有策略决定和可选 backoff(退避).

### ShutdownState(关闭状态)

```text
Idle
  -> RequestStop
  -> GracefulDrain
  -> AbortStragglers
  -> Reconcile
  -> Completed
```

**Rules(规则)**:

- request stop(请求停止) 阶段必须触发 parent cancellation(父取消) 并传播到 child token(子令牌).
- graceful drain(优雅排空) 阶段必须等待 child(子任务) 自行退出.
- abort stragglers(强制终止拖尾任务) 阶段只能处理超时后仍未退出的 async worker(异步工作任务).
- blocking worker(阻塞工作任务) 在 abort stragglers(强制终止拖尾任务) 阶段必须记录不可立即终止边界, 并按升级策略处理.
- reconcile(状态对账) 阶段必须统一更新 registry(注册表),current state(当前状态),metrics(指标) 和 event journal(事件日志缓冲区).

### TaskExit(任务退出)

- `Completed`(已完成)
- `Failed(TaskFailure)`(已失败)
- `Cancelled`(已取消)
- `TimedOut`(已超时)
- `Unhealthy`(不健康)
- `Panicked`(已恐慌)

### TaskFailureKind(任务失败类别)

- `Recoverable`(可恢复)
- `FatalConfig`(致命配置错误)
- `FatalBug`(致命代码错误)
- `ExternalDependency`(外部依赖错误)
- `Timeout`(超时)
- `Panic`(恐慌)
- `Cancelled`(已取消)

## Control Plane(控制平面)

### ControlCommand(控制命令)

- `command_id`
- `requested_by`
- `reason`
- `target_path`
- `accepted_at`
- `kind`
- `result`

**Command kinds(命令种类)**:

- `add_child`
- `remove_child`
- `restart_child`
- `pause_child`
- `resume_child`
- `quarantine_child`
- `shutdown_tree`
- `current_state`
- `subscribe_events`

**Rules(规则)**:

- 命令必须幂等.
- 每个已接受命令都必须产生 audit event(审计事件).

## State Plane(状态平面)

### SupervisorState(监督器当前状态)

- `root_path`
- `generated_at`
- `sequence`
- `config_version`
- `children`: 以路径为索引的 child current state(子任务当前状态).
- `meltdown_state`
- `shutdown_state`
- `journal_sequence`

### ChildState(子任务当前状态)

- `path`
- `id`
- `name`
- `state`
- `health`
- `generation`
- `attempt`
- `restart_count`
- `last_failure`
- `last_event_sequence`
- `last_policy_decision`
- `readiness`

## Event Plane(事件平面)

### SupervisorEvent(监督器事件)

- `when`: `EventTime`(事件时间).
- `where`: `EventLocation`(事件位置).
- `what`: `EventPayload`(事件内容).
- `policy`: 可选 `PolicyDecision`(策略决定).
- `sequence`: monotonic event sequence(单调事件序号).
- `correlation_id`: command(命令) 或 attempt(尝试) 的 correlation id(关联标识).
- `config_version`: 当前配置版本.

### EventTime(事件时间)

- `unix_nanos`
- `monotonic_nanos`
- `supervisor_uptime_ms`
- `generation`
- `attempt`

### EventLocation(事件位置)

- `supervisor_path`
- `parent_id`
- `child_id`
- `child_name`
- `tokio_task_id`
- `host`
- `pid`
- `thread_name`
- `module_path`
- `source_file`
- `source_line`

### EventJournal(事件日志缓冲区)

- `capacity`: 固定容量.
- `events`: 最近生命周期事件.
- `dropped_count`: 因容量限制丢弃的事件数量.
- `last_sequence`: 最近写入事件序号.

**Rules(规则)**:

- event journal(事件日志缓冲区) 只保留生命周期和诊断所需事件.
- 发生 meltdown(熔断),关闭超时或父级升级时, `RunSummary`(运行摘要) 必须从 event journal(事件日志缓冲区) 读取最近关键事件.

### RunSummary(运行摘要)

- `started_at`
- `finished_at`
- `shutdown_cause`
- `restart_count`
- `failure_count`
- `recent_failures`
- `recent_events`
- `final_state`
- `final_decision`

**Rules(规则)**:

- `RunSummary`(运行摘要) 必须解释最近生命周期事件,失败原因,重启次数,关闭原因和最终状态.
- meltdown(熔断),关闭超时或父级升级发生时必须生成 `RunSummary`(运行摘要).

### EventPayload(事件内容)

- `ChildStarting`
- `ChildRunning`
- `ChildReady`
- `ChildHeartbeat`
- `ChildFailed`
- `ChildPanicked`
- `BackoffScheduled`
- `ChildRestarting`
- `ChildRestarted`
- `ChildQuarantined`
- `ChildStopped`
- `ChildUnhealthy`
- `Meltdown`
- `ShutdownRequested`
- `ShutdownCompleted`
- `CommandAccepted`
- `CommandCompleted`
- `SubscriberLagged`

## Metrics(指标)

必需指标名如下:

- `supervisor_restart_total`
- `supervisor_child_state`
- `supervisor_child_uptime_seconds`
- `supervisor_backoff_seconds`
- `supervisor_healthcheck_latency_seconds`
- `supervisor_meltdown_total`
- `supervisor_shutdown_duration_seconds`
- `supervisor_event_lag_total`
- `supervisor_config_version`

**Label rules(标签规则)**:

- label(标签) 只能使用 supervisor path(监督器路径),child id(子任务标识),state(状态),decision(决定) 和 failure category(失败类别) 等低基数值.
- label(标签) 不得包含错误全文,用户输入,动态路径碎片或其它无界值.

## Observability Plane(可观测性平面)

### ObservabilityPipeline(可观测性管线)

- `event_sink`: lifecycle event(生命周期事件) 接收点.
- `structured_log_sink`: structured log(结构化日志) 接收点.
- `tracing_sink`: tracing span/event(追踪范围和事件) 接收点.
- `metrics_recorder`: metrics(指标) 记录器.
- `audit_sink`: control command(控制命令) 审计接收点.
- `journal`: event journal(事件日志缓冲区).
- `summary_builder`: `RunSummary`(运行摘要) 构建器.
- `test_recorder`: 测试可读取的信号记录器.

**Rules(规则)**:

- 同一个生命周期事实必须能映射到 `SupervisorEvent`(监督器事件),structured log(结构化日志),tracing event(追踪事件),metrics(指标) 和 audit event(审计事件).
- test recorder(测试记录器) 必须可以验证信号缺失,消费者滞后和低基数标签.
- 核心不得绑定具体 exporter(导出器).

## Documentation and Examples(文档和示例)

### ExampleSuite(示例套件)

- `quickstart_example`: `examples/supervisor_quickstart.rs`.
- `config_example`: `examples/config_tree_supervisor.rs`.
- `restart_example`: `examples/restart_policy_lab.rs`.
- `shutdown_example`: `examples/shutdown_tree.rs`.
- `observability_example`: `examples/observability_probe.rs`.

**Rules(规则)**:

- 每个示例必须能通过 `cargo run --example <name>` 独立运行,或者明确说明必需输入文件.
- 示例只展示项目自有 API(接口),不得成为旧接口适配层,迁移层或兼容包装示例.

### DocumentationSet(文档集合)

- `manual_zh`: `manual/zh`.
- `manual_en`: `manual/en`.
- `docs_zh`: `docs/zh`.
- `docs_en`: `docs/en`.
- `glossary`: `specs/001-create-supervisor-core/glossary.md`.
- `quickstart`: `specs/001-create-supervisor-core/quickstart.md`.
- `contracts`: `specs/001-create-supervisor-core/contracts/`.

**Rules(规则)**:

- 中英文目录必须同构.
- 中英文内容必须表达同一语义.
- public API(公开接口),configuration schema(配置模式),example behavior(示例行为),observability signal(可观测性信号) 或 glossary term(词汇表词条) 变化时,文档必须同步更新.

### GlossarySet(词汇表集合)

- `path`: `specs/001-create-supervisor-core/glossary.md`.
- `professional_terms`: 规格文档中出现的专业词汇集合.
- `backtick_terms`: 规格,计划,数据模型,公开契约,quickstart(快速开始) 和任务清单中所有反引号词汇集合.
- `chinese_meanings`: 每个词汇的中文说明.
- `definitions`: 每个词汇的定义.
- `usage_rules`: 每个词汇的使用规则.

**Rules(规则)**:

- 反引号内的 Rust(编程语言) 类型名,枚举值,方法名,字段名,指标名,路径名,命令名,配置键和测试目标都算词汇.
- 每个 backtick term(反引号词汇) 必须作为 `glossary.md`(词汇表) 中的条目或被明确归入词汇表中的同名词条.
- 词汇表必须避免同一个英文词汇对应多个互相冲突的中文说明.
- 新增 public API(公开接口),配置键,指标名,事件名,测试目标或文件路径时,必须同步更新词汇表.

### DocumentationSyncCheck(文档同步检查)

- `public_api_terms`: 文档中公开 API(接口) 名称集合.
- `configuration_terms`: 配置模式名称集合.
- `example_terms`: 示例名称和命令集合.
- `observability_terms`: 可观测性信号名称集合.
- `terminology_terms`: 标准术语集合,包含 `Shutdown Without Orphaned Tasks`(关闭后不留下孤儿任务).
- `backtick_terms`: 反引号词汇集合.
- `glossary_terms`: `glossary.md`(词汇表) 已登记词汇集合.

## Coding and Release Entities(编码和发布实体)

### TestNamingRule(测试命名规则)

- `integration_test_pattern`: `src/tests/*_test.rs`.
- `unit_test_pattern`: `src/<module>/tests/*_test.rs`.
- `forbidden_patterns`: 不以 `_test.rs` 结尾的测试文件路径集合.
- `cargo_targets`: 为 `src/tests/*_test.rs` 声明的 Cargo(构建工具) test target(测试目标) 集合.

**Rules(规则)**:

- 所有 integration test(集成测试),unit test(单元测试),contract test(契约测试) 和 quality gate test(质量门禁测试) 文件必须以 `_test.rs` 结尾.
- integration test(集成测试) 必须放在 `src/tests/*_test.rs`.
- unit test(单元测试) 必须放在被测模块自己的 `tests/*_test.rs` 目录.
- 实现文件中不得写 inline unit test(内联单元测试) 代码.

### CodingStandard(编码标准)

- `module_layout`: `src/<module>/` top-level directory module(顶层目录模块) 布局.
- `module_docs`: 每个 module(模块) 的 module doc(模块文档).
- `struct_docs`: 每个 struct(结构体) 的结构体文档.
- `field_docs`: 每个 struct field(结构体字段) 的字段文档.
- `function_docs`: 每个 public function(公共函数) 和 private function(私有函数) 的函数文档.
- `source_comments`: 需要解释局部不变量的 source comment(源码注释).
- `doctests`: public function(公共函数) 的可运行 doctest(文档测试).
- `module_entries`: `src/lib.rs` 和每个 `src/<module>/mod.rs` 的 `pub mod <mod_name>;` 声明集合.
- `import_paths`: 使用 `crate::` 或 external crate name(外部软件包名) 的 absolute path(绝对路径) 集合.
- `test_naming`: `TestNamingRule`(测试命名规则) 的检查结果.

**Rules(规则)**:

- 源码必须使用 top-level directory module(顶层目录模块) 结构,不得使用 `src/supervision/` 中间层,也不得使用 `src/<module>.rs` 平铺模块文件.
- `src/lib.rs` 只包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明.
- 每个 `src/<module>/mod.rs` 不得出现 `pub use`(公开重导出),类型定义,函数定义,常量定义或逻辑.
- 内部导入不得使用 `super::` relative path(相对路径).
- 编码阶段完成逻辑时必须同步完成对应文档.
- source comment(源码注释) 和 rustdoc(代码文档注释) 必须使用英文.

### CognitiveComplexityBudget(认知复杂度预算)

- `function_name`: 被检查函数名称.
- `module_path`: 函数所属模块路径.
- `function_kind`: `Regular`(普通函数),`LifecycleDispatcher`(生命周期调度函数) 或 `GeneratedBoundary`(生成边界).
- `score`: cognitive complexity(认知复杂度) 分数.
- `max_allowed`: 该函数允许的最大分数.
- `max_nesting`: 控制流最大嵌套层级.
- `split_decision`: 超限时的拆分决定.

**Rules(规则)**:

- `Regular`(普通函数) 的 cognitive complexity(认知复杂度) 不得超过 15.
- `LifecycleDispatcher`(生命周期调度函数) 的 cognitive complexity(认知复杂度) 不得超过 20.
- 控制流嵌套不得超过 3 层.
- 超限逻辑必须拆分为 state machine(状态机),policy function(策略函数),small helper function(小辅助函数) 或独立模块.
- 拆分不得通过隐藏命名,宏或无意义抽象降低表面分数.

### MaintainabilityProfile(可维护性画像)

- `module_path`: 被检查模块路径.
- `responsibility`: 模块单一职责说明.
- `owned_types`: 该模块拥有的类型集合.
- `public_contracts`: 该模块公开给其它模块使用的契约类型.
- `internal_dependencies`: 该模块依赖的项目内部模块.
- `state_boundary`: 该模块是否拥有共享状态,以及共享状态边界.
- `test_files`: 对应 integration test(集成测试) 和 unit test(单元测试) 文件.
- `doc_files`: 对应 manual(手册),docs(文档),quickstart(快速开始) 或 contract(契约) 文件.
- `change_scope`: 变更该模块时必须同步检查的文件集合.

**Rules(规则)**:

- 一个模块只能承担一个清晰职责.
- 跨模块依赖必须通过公开契约类型发生.
- 共享可变状态只能集中在 runtime(运行时),registry(注册表),current state(当前状态) 或明确的 state owner(状态所有者) 内.
- 行为变化必须同步测试,文档和示例.
- supervisor core(监督器核心) 不得包含 business data plane(业务数据面) 逻辑.
- 维护性拆分必须提高 change locality(变更局部性),不能制造仅为降低行数的空壳模块.

## Parallel Execution Governance(并行执行治理)

### ModuleDependencyMap(模块依赖图)

- `layers`: 从 foundation layer(基础层) 到 diagnostics layer(诊断层) 的模块层级.
- `allowed_dependencies`: 每个模块允许依赖的内部模块集合.
- `forbidden_dependencies`: 禁止依赖规则集合.
- `owner_modules`: 每个模块的 owner boundary(所有权边界).
- `cycle_check`: cycle dependency(循环依赖) 检查结果.

**Rules(规则)**:

- `ModuleDependencyMap`(模块依赖图) 必须说明每个模块之间的依赖关系.
- 低层模块不得依赖 runtime(运行时),control(控制),examples(示例程序),manual(手册) 或 docs(文档).
- `event`(事件) 不得依赖 `observe`(可观测性).
- `state`(状态) 不得依赖 runtime orchestration(运行时编排).
- 任何 cycle dependency(循环依赖) 都必须在实现前拆除.

### ParallelWorkstream(并行工作流)

- `id`: 工作流标识,例如 `WS1`.
- `scope`: 工作流负责的功能范围.
- `primary_files`: 工作流拥有写入权的主文件集合.
- `read_only_files`: 工作流只能读取的文件集合.
- `independent_tests`: 工作流独立完成时必须通过的测试集合.
- `blocked_by`: 必须先完成的工作流标识集合.
- `completion_evidence`: 工作流完成证据.

**Rules(规则)**:

- 每个 `ParallelWorkstream`(并行工作流) 必须拥有互不冲突的 primary files(主文件).
- 多个工作流不能同时修改同一主文件,除非 lead agent(主代理) 创建 integration task(集成任务) 并记录文件所有权.
- 工作流必须先完成对应 contract test(契约测试) 或 unit test(单元测试),再进入集成.

### WorkstreamSplitRecord(工作流拆分记录)

- `source_task`: 原始大任务.
- `split_reason`: 拆分原因.
- `new_workstreams`: 拆分后的工作流集合.
- `file_ownership`: 文件所有权分配.
- `test_ownership`: 测试所有权分配.
- `integration_point`: 合并点.

**Rules(规则)**:

- 影响并行度的任务必须拆分到独立模块,独立测试或独立文档产物.
- 拆分后必须保留一个明确 integration point(集成点),避免多个子代理同时修改运行时组合文件.

### ParallelExecutionBlocker(并行执行卡点)

- `id`: 卡点标识.
- `kind`: shared file bottleneck(共享文件瓶颈),unstable contract(不稳定契约),hidden coupling(隐藏耦合),manual gate(人工门禁) 或 long validation chain(长验证链).
- `affected_workstreams`: 受影响的工作流集合.
- `blocked_tasks`: 被阻塞任务集合.
- `severity`: 卡点严重程度.

### BlockerEliminationRecord(卡点消除记录)

- `blocker_id`: 对应 `ParallelExecutionBlocker`(并行执行卡点).
- `action`: 消除动作.
- `owner`: 负责消除的 agent(代理).
- `evidence`: 消除证据.
- `verified_at`: 验证时间.

**Rules(规则)**:

- 每个影响并行执行的卡点必须拥有 `BlockerEliminationRecord`(卡点消除记录).
- 卡点消除证据必须可以追踪到测试,文档或依赖图变化.

### UnattendedImplementationRun(无人值守实现运行)

- `run_id`: 实现运行标识.
- `started_at`: 开始时间.
- `workstreams`: 本次运行包含的工作流集合.
- `task_ledger`: `TaskCompletionLedger`(任务完成台账).
- `supervision_records`: `LeadAgentSupervision`(主代理监督) 记录集合.
- `final_gate_result`: 最终关口结果.

### TaskCompletionLedger(任务完成台账)

- `task_id`: 任务标识.
- `status`: `Pending`(待处理),`InProgress`(进行中),`Done`(已完成) 或 `Blocked`(已阻塞).
- `owner_workstream`: 所属工作流.
- `files_changed`: 修改文件集合.
- `tests_run`: 已运行测试集合.
- `documentation_updated`: 同步文档集合.
- `completion_evidence`: 完成证据.

**Rules(规则)**:

- 实现完成前,`TaskCompletionLedger`(任务完成台账) 不能存在 `Pending`(待处理),`InProgress`(进行中) 或 `Blocked`(已阻塞) 任务.
- 每个完成任务必须记录文件变化,测试结果和文档同步结果.

### LeadAgentSupervision(主代理监督)

- `lead_agent`: 主代理标识.
- `subagent_workstreams`: 被监督的 `SubagentWorkstream`(子代理工作流) 集合.
- `review_checklist`: 审查清单.
- `correction_records`: `CorrectionRecord`(纠偏记录) 集合.
- `final_review_result`: 最终审查结果.

### SubagentWorkstream(子代理工作流)

- `subagent`: 子代理标识.
- `workstream_id`: 工作流标识.
- `owned_files`: 子代理拥有写入权的文件集合.
- `required_tests`: 子代理必须运行的测试集合.
- `handoff_notes`: 交接记录.

### CorrectionRecord(纠偏记录)

- `drift_type`: 偏差类型.
- `affected_files`: 受影响文件集合.
- `expected_requirement`: 期望要求.
- `actual_output`: 实际输出.
- `correction_action`: 纠偏动作.
- `review_result`: 复核结果.
- `final_evidence`: 最终证据.

**Rules(规则)**:

- lead agent(主代理) 必须审查每个 subagent workstream(子代理工作流) 的规格一致性,模块边界,文件边界,测试命名,文档同步和禁止兼容方法.
- 发现偏差时必须创建 `CorrectionRecord`(纠偏记录),并在 workstream(工作流) 完成前闭环.
- 没有 clean review record(清洁审查记录) 或已闭环纠偏记录的 workstream(工作流) 不能标记完成.

### ReleasePackage(发布包)

- `manifest`: `Cargo.toml` package metadata(软件包元数据).
- `readme`: README(说明文档).
- `license`: LICENSE(许可证) 或 license-file(许可证文件).
- `changelog`: CHANGELOG(变更日志).
- `sbom`: `SBOMArtifact`(软件物料清单产物).
- `package_contents`: `cargo package --list` 输出.
- `publish_dry_run`: `cargo publish --dry-run` 结果.

**Rules(规则)**:

- package metadata(软件包元数据) 必须满足 crates.io(软件包发布平台) 发布约定.
- package contents(打包内容) 必须包含源码,示例,README(说明文档),LICENSE(许可证),手册和必要文档,并排除 target(构建产物) 和无关大文件.
- SBOM(软件物料清单) 必须在发布准备阶段生成并通过格式校验.
- 真实上传 crates.io(软件包发布平台) 不属于本功能实现完成条件.

### SBOMArtifact(软件物料清单产物)

- `cyclonedx_path`: `artifacts/sbom/rust-supervisor.cdx.json`.
- `spdx_path`: `artifacts/sbom/rust-supervisor.spdx.json`.
- `generated_at`: 生成时间.
- `tool_name`: 生成工具名称.
- `tool_version`: 生成工具版本.
- `root_package`: 当前 crate(包) 名称,版本,repository(代码仓库) 和 license(许可证).
- `direct_dependencies`: 直接依赖列表.
- `transitive_dependencies`: 传递依赖列表.
- `licenses`: 依赖许可证集合.
- `checksums`: 依赖 checksum(校验和) 集合.
- `source_references`: package URL(软件包地址),source repository(源码仓库) 和 registry source(注册表来源).
- `cargo_lock_hash`: `Cargo.lock` 依赖图的校验摘要.

**Rules(规则)**:

- CycloneDX JSON(CycloneDX JSON 格式) 必须包含 `bomFormat`,`specVersion`,`metadata`,`components` 和 dependency graph(依赖图).
- SPDX JSON(SPDX JSON 格式) 必须包含 `SPDXID`,`creationInfo`,`packages`,`relationships` 和 package license(软件包许可证) 信息.
- SBOM(软件物料清单) 必须和 `Cargo.lock` 中的依赖版本一致.
- SBOM(软件物料清单) 不得包含密钥,token(令牌),本地绝对路径或未清理的构建临时目录.

## Relationships(关系)

- `SupervisorSpec`(监督器规格) 拥有有序 `ChildSpec`(子任务规格) 值.
- `SupervisorConfig`(监督器配置) 通过 rust-config-tree(集中配置树) 生成 `ConfigState`(配置状态).
- `ConfigState`(配置状态) 派生 `SupervisorSpec`(监督器规格),默认策略,关闭预算和 observability(可观测性) 配置.
- `ChildSpec`(子任务规格) 启动后变成 `ChildRuntime`(子任务运行态).
- `TaskFactory`(任务工厂) 使用 `TaskContext`(任务上下文) 构建 attempt(尝试).
- `Service`(服务特征) 和 `service_fn`(函数适配器) 适配到 `TaskFactory`(任务工厂), 但不能替换 `TaskFactory`(任务工厂).
- `ChildRuntime`(子任务运行态) 发送 `SupervisorEvent`(监督器事件),并更新 `SupervisorState`(监督器当前状态).
- `ObservabilityPipeline`(可观测性管线) 消费 `SupervisorEvent`(监督器事件),control command(控制命令),current state(当前状态) 和 `RunSummary`(运行摘要),并派生 structured log(结构化日志),tracing(结构化追踪),metrics(指标),audit(审计) 和 test recorder(测试记录器) 记录.
- `PolicyEngine`(策略引擎) 读取 `TaskExit`(任务退出),`TaskFailureKind`(任务失败类别),策略和计数器,并生成 `RestartDecision`(重启决策).
- `ControlCommand`(控制命令) 产生 audit event(审计事件),并可以改变 registry(注册表) 或 shutdown state(关闭状态).
- `ReadinessPolicy`(就绪策略) 决定 child(子任务) 从 `Running`(运行中) 进入 `Ready`(已就绪) 的条件.
- `EventJournal`(事件日志缓冲区) 保存最近生命周期事件,并为 `RunSummary`(运行摘要) 提供诊断输入.
- `RunSummary`(运行摘要) 在 meltdown(熔断),关闭超时或父级升级时汇总 event journal(事件日志缓冲区),current state(当前状态) 和策略决定.
- `ExampleSuite`(示例套件),`DocumentationSet`(文档集合),`DocumentationSyncCheck`(文档同步检查),`CodingStandard`(编码标准),`CognitiveComplexityBudget`(认知复杂度预算),`MaintainabilityProfile`(可维护性画像),`SBOMArtifact`(软件物料清单产物) 和 `ReleasePackage`(发布包) 共同定义实现完成后的学习,维护和发布边界.
