# Public API Contract(公开 API 契约): 配置结构体模式支持

## Scope(范围)

本契约定义 crate user(crate 使用者) 可以依赖的 configuration API(配置接口)。本功能不提供 compatibility export(兼容导出),不提供旧配置别名,不提供迁移层。所有路径都使用真实 module path(模块路径)。

## Configurable Input API(可配置输入接口)

### `rust_supervisor::config::configurable::SupervisorConfig`

`SupervisorConfig`(监督器配置) 是公开 root configuration struct(根配置结构体)。它必须同时支持以下 trait(特征):

- `Debug`
- `Clone`
- `PartialEq`
- `serde::Serialize`
- `serde::Deserialize`
- `confique::Config`
- `schemars::JsonSchema`

该结构体必须包含完整配置分区:

- `supervisor`: `SupervisorRootConfig`(监督器根配置)
- `policy`: `PolicyConfig`(策略配置)
- `shutdown`: `ShutdownConfig`(关闭配置)
- `observability`: `ObservabilityConfig`(可观测性配置)

### Nested configuration structs(嵌套配置结构体)

以下结构体必须位于 `rust_supervisor::config::configurable` module(模块),并支持与 `SupervisorConfig`(监督器配置) 相同的公开派生能力:

- `SupervisorRootConfig`(监督器根配置)
- `PolicyConfig`(策略配置)
- `ShutdownConfig`(关闭配置)
- `ObservabilityConfig`(可观测性配置)

`src/config/mod.rs` 只能声明模块:

```rust
pub mod configurable;
pub mod loader;
pub mod state;
pub mod yaml;
```

该文件不得重新导出任何配置类型。

## Validated State API(已校验状态接口)

### `rust_supervisor::config::state::ConfigState`

`ConfigState`(配置状态) 表示已经通过 semantic validation(语义校验) 的不可变配置状态。它不承担 raw configuration input(原始配置输入) 的集中管理职责。

必须实现:

```rust
impl TryFrom<rust_supervisor::config::configurable::SupervisorConfig> for rust_supervisor::config::state::ConfigState
```

转换失败时必须返回 `SupervisorError::FatalConfig`(致命配置错误),并说明失败字段或 section(配置分区)。

### `ConfigState::to_supervisor_spec`

`to_supervisor_spec`(派生监督器规格) 必须从已校验配置派生 `SupervisorSpec`(监督器规格),并调用 `SupervisorSpec::validate()`。当派生后的监督器规格无效时,必须返回 `SupervisorError::FatalConfig`(致命配置错误)。

## Loader API(加载接口)

### `rust_supervisor::config::loader::load_config_from_yaml_file`

`load_config_from_yaml_file`(加载配置状态) 必须接收 YAML(数据序列化格式) 文件路径,读取完整配置,反序列化为 `SupervisorConfig`(监督器配置),再转换为 `ConfigState`(配置状态)。

错误规则:

- 非 YAML(数据序列化格式) 路径必须返回 `SupervisorError::FatalConfig`(致命配置错误)。
- 文件读取失败必须返回 `SupervisorError::FatalConfig`(致命配置错误)。
- YAML(数据序列化格式) 语法或结构错误必须返回 `SupervisorError::FatalConfig`(致命配置错误)。
- 语义校验失败必须返回 `SupervisorError::FatalConfig`(致命配置错误)。

### `rust_supervisor::config::yaml::parse_config_state`

`parse_config_state`(解析配置状态) 必须接收内存中的 YAML(数据序列化格式) 文本,并使用与 `load_config_from_yaml_file`(加载配置状态) 相同的输入模型和校验路径。

## Runtime Startup API(运行时启动接口)

### `rust_supervisor::runtime::supervisor::Supervisor::start_from_config_state`

该入口接收 `ConfigState`(配置状态),在创建 runtime channel(运行时通道) 前调用 `to_supervisor_spec`(派生监督器规格)。任何配置错误必须返回 `SupervisorError::FatalConfig`(致命配置错误),不得创建 channel(通道),不得 spawn(派生) control loop(控制循环),不得返回 `SupervisorHandle`(监督器句柄)。

### `rust_supervisor::runtime::supervisor::Supervisor::start_from_config_file`

该入口接收 YAML(数据序列化格式) 文件路径,先调用 `load_config_from_yaml_file`(加载配置状态),再调用 `start_from_config_state`(从配置状态启动)。错误边界与 `load_config_from_yaml_file`(加载配置状态) 和 `start_from_config_state`(从配置状态启动) 保持一致。

## Enum Contract(枚举契约)

被 `SupervisorConfig`(监督器配置) 引用的 public enum(公开枚举),至少包括 `SupervisionStrategy`(监督策略),必须支持:

- `serde::Serialize`
- `serde::Deserialize`
- `schemars::JsonSchema`

如 `confique::Config`(配置派生) 对嵌套字段有额外 trait bound(特征约束),实现必须满足这些约束。

## Schema Contract(结构模式契约)

`schemars::schema_for!(SupervisorConfig)` 必须成功。生成的 schema(结构模式) 必须覆盖所有公开可配置字段,并且默认不得包含 `x-tree-split`(树形拆分扩展)。

## Template Contract(模板契约)

官方 YAML(数据序列化格式) template(模板) 必须从 `SupervisorConfig`(监督器配置) 的配置元数据生成或保持同步。默认情况下,template generation(模板生成) 必须只有一个 root YAML template target(根 YAML 模板目标),并且不得包含 `x-tree-split`(树形拆分扩展)。

## Startup Rejection Contract(启动拒绝契约)

以下非法配置必须在 runtime startup(运行时启动) 前失败:

- 缺失必填项。
- 非法 enum value(枚举值)。
- 零值 capacity(容量)。
- 零值 timeout(超时)。
- 越界 `jitter_ratio`(抖动比例)。
- 反向 backoff(退避),即 `initial_backoff_ms > max_backoff_ms`。

失败结果必须是 `SupervisorError::FatalConfig`(致命配置错误),并且返回 `SupervisorHandle`(监督器句柄) 的次数必须为 0。
