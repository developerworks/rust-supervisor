# Data Model(数据模型): 配置结构体模式支持

## Entity(实体): SupervisorConfig(监督器配置)

**Purpose(用途)**: `SupervisorConfig`(监督器配置) 是公开 root configuration struct(根配置结构体),负责表达完整 raw configuration input(原始配置输入),并作为 YAML(数据序列化格式) 加载,template generation(模板生成) 和 schema generation(结构模式生成) 的唯一模型。

**Fields(字段)**:

- `supervisor`: `SupervisorRootConfig`(监督器根配置),表达 root supervisor(根监督器) 声明。
- `policy`: `PolicyConfig`(策略配置),表达重启,退避,熔断和健康检查相关可调值。
- `shutdown`: `ShutdownConfig`(关闭配置),表达关闭预算。
- `observability`: `ObservabilityConfig`(可观测性配置),表达事件容量和观测开关。

**Required traits(必备特征)**: `Debug`,`Clone`,`PartialEq`,`Serialize`,`Deserialize`,`confique::Config`,`schemars::JsonSchema`。

**Relationships(关系)**: `SupervisorConfig`(监督器配置) 通过 `TryFrom<SupervisorConfig> for ConfigState`(配置状态转换) 进入 validated config state(已校验配置状态)。

## Entity(实体): SupervisorRootConfig(监督器根配置)

**Purpose(用途)**: `SupervisorRootConfig`(监督器根配置) 表达 root supervisor(根监督器) 的运行时声明入口。

**Fields(字段)**:

- `strategy`: `SupervisionStrategy`(监督策略),用于声明子任务失败时的默认 restart scope(重启范围)。

**Required traits(必备特征)**: `Debug`,`Clone`,`PartialEq`,`Serialize`,`Deserialize`,`confique::Config`,`schemars::JsonSchema`。

**Validation(校验)**: `strategy`(策略) 必须能通过 YAML(数据序列化格式) 反序列化,非法 enum value(枚举值) 必须在 structural validation(结构校验) 阶段失败。

## Entity(实体): PolicyConfig(策略配置)

**Purpose(用途)**: `PolicyConfig`(策略配置) 表达 runtime policy(运行时策略) 的可调项。

**Fields(字段)**:

- `child_restart_limit`: 子任务 restart budget(重启预算) 限制,必须大于 0。
- `child_restart_window_ms`: 子任务 restart window(重启窗口),必须大于 0。
- `supervisor_failure_limit`: 监督器 failure budget(失败预算),必须大于 0。
- `supervisor_failure_window_ms`: 监督器 failure window(失败窗口),必须大于 0。
- `initial_backoff_ms`: 初始 backoff(退避),必须大于 0。
- `max_backoff_ms`: 最大 backoff(退避),必须大于 0。
- `jitter_ratio`: jitter ratio(抖动比例),必须在 0 到 1 之间,包含边界。
- `heartbeat_interval_ms`: heartbeat interval(心跳间隔),必须大于 0。
- `stale_after_ms`: stale threshold(失效阈值),必须大于 0。

**Required traits(必备特征)**: `Debug`,`Clone`,`PartialEq`,`Serialize`,`Deserialize`,`confique::Config`,`schemars::JsonSchema`。

**Validation(校验)**: `initial_backoff_ms` 必须小于或等于 `max_backoff_ms`。`jitter_ratio` 必须满足 `0.0 <= jitter_ratio <= 1.0`。

## Entity(实体): ShutdownConfig(关闭配置)

**Purpose(用途)**: `ShutdownConfig`(关闭配置) 表达 supervisor shutdown(监督器关闭) 的时间预算。

**Fields(字段)**:

- `graceful_timeout_ms`: graceful shutdown(优雅关闭) 超时,必须大于 0。
- `abort_wait_ms`: abort wait(中止等待) 超时,必须大于 0。

**Required traits(必备特征)**: `Debug`,`Clone`,`PartialEq`,`Serialize`,`Deserialize`,`confique::Config`,`schemars::JsonSchema`。

## Entity(实体): ObservabilityConfig(可观测性配置)

**Purpose(用途)**: `ObservabilityConfig`(可观测性配置) 表达事件容量和观测功能开关。

**Fields(字段)**:

- `event_journal_capacity`: event journal(事件日志) 容量,必须大于 0,并且用于派生 runtime channel capacity(运行时通道容量)。
- `metrics_enabled`: metrics recording(指标记录) 开关。
- `audit_enabled`: command audit(命令审计) 开关。

**Required traits(必备特征)**: `Debug`,`Clone`,`PartialEq`,`Serialize`,`Deserialize`,`confique::Config`,`schemars::JsonSchema`。

## Entity(实体): ConfigurableStructSet(可配置结构体集合)

**Purpose(用途)**: `ConfigurableStructSet`(可配置结构体集合) 是一个文档化的所有权概念,指 `src/config/configurable.rs` 中所有 raw configuration input(原始配置输入) 结构体。它不需要成为运行时结构体。

**Owned structs(拥有结构体)**: `SupervisorConfig`,`SupervisorRootConfig`,`PolicyConfig`,`ShutdownConfig`,`ObservabilityConfig`,以及未来新增的公开配置输入结构体。

**Boundary(边界)**: 该集合只能表达输入和生成能力,不能承担 semantic validation(语义校验) 或 runtime startup(运行时启动)。

## Entity(实体): ConfigState(配置状态)

**Purpose(用途)**: `ConfigState`(配置状态) 是 immutable validated state(不可变已校验状态),用于派生 `SupervisorSpec`(监督器规格)。

**Fields(字段)**: 字段结构与 `SupervisorConfig`(监督器配置) 对齐,但语义上表示已校验配置。

**Transitions(状态转换)**:

1. `SupervisorConfig`(监督器配置) 通过 YAML(数据序列化格式) 反序列化产生。
2. `ConfigState::try_from` 完成 semantic validation(语义校验)。
3. `ConfigState::to_supervisor_spec` 派生 `SupervisorSpec`(监督器规格)。
4. `SupervisorSpec::validate` 通过后才能进入 runtime startup(运行时启动)。

## Entity(实体): ConfigurationSchema(配置结构模式)

**Purpose(用途)**: `ConfigurationSchema`(配置结构模式) 是从 `SupervisorConfig`(监督器配置) 生成的 schema(结构模式),用于编辑器提示,外部校验和使用者项目集成。

**Validation(校验)**:

- schema(结构模式) 必须包含 `supervisor`,`policy`,`shutdown`,`observability`。
- schema(结构模式) 必须包含所有公开可配置字段。
- schema(结构模式) 默认不得包含 `x-tree-split`(树形拆分扩展)。

## Entity(实体): ConfigurationTemplate(配置模板)

**Purpose(用途)**: `ConfigurationTemplate`(配置模板) 是从 `SupervisorConfig`(监督器配置) 生成或同步维护的官方 YAML(数据序列化格式) template(模板)。

**Validation(校验)**:

- 默认必须只有一个 root YAML template target(根 YAML 模板目标)。
- 模板必须覆盖所有 runtime tunable configuration(运行时可调配置)。
- 模板默认不得包含 `x-tree-split`(树形拆分扩展)。

## Entity(实体): TreeSplitDecision(树形拆分决策)

**Purpose(用途)**: `TreeSplitDecision`(树形拆分决策) 表示使用者项目是否启用 `x-tree-split`(树形拆分扩展) 的外部选择。

**Boundary(边界)**: 本 crate(包) 不存储,不默认声明,不在官方 template(模板) 中表达该决策。使用者可以在自己的项目中包装 `SupervisorConfig`(监督器配置),再自行添加 schema extension(结构模式扩展)。

## Entity(实体): ConfigurationValidationResult(配置校验结果)

**Purpose(用途)**: `ConfigurationValidationResult`(配置校验结果) 表达配置加载和校验后的结果。

**Success state(成功状态)**: 返回 `ConfigState`(配置状态),并允许继续派生 `SupervisorSpec`(监督器规格)。

**Failure state(失败状态)**: 返回 `SupervisorError::FatalConfig`(致命配置错误),错误消息必须指出字段或 section(配置分区)。

**Startup rule(启动规则)**: 失败状态不得创建 runtime channel(运行时通道),不得启动 control loop(控制循环),不得返回 `SupervisorHandle`(监督器句柄)。
