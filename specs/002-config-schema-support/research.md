# Research(研究结论): 配置结构体模式支持

## Decision(决策): 直接声明 `confique` 依赖

**Decision(决策)**: 在 `Cargo.toml` 中显式加入 `confique = { version = "0.4.0", features = ["yaml"] }`。

**Rationale(理由)**: `confique::Config`(配置派生) 是公开 root configuration struct(根配置结构体) 的用户可见契约。公开类型不能依赖传递依赖来提供 derive macro(派生宏) 或 trait(特征),否则使用者无法可靠生成 template(模板) 或配置元数据。

**Alternatives considered(已考虑替代方案)**: 继续只使用 `serde`(序列化框架)。该方案能完成 YAML(数据序列化格式) 反序列化,但不能满足 template generation(模板生成) 和 `confique::Config`(配置派生) 契约。通过 `rust-config-tree`(配置树库) 的传递依赖间接使用 `confique`(配置库) 也被拒绝,因为公开 API(接口) 不能依赖不稳定的传递依赖边界。

## Decision(决策): 直接声明 `schemars` 依赖

**Decision(决策)**: 在 `Cargo.toml` 中显式加入 `schemars = { version = "1", features = ["derive"] }`。

**Rationale(理由)**: `schemars::JsonSchema`(结构模式生成特征) 是公开配置结构体的必备能力。schema generation(结构模式生成) 必须从同一个 root configuration struct(根配置结构体) 完成,避免维护第二套手写 schema(结构模式) 模型。

**Alternatives considered(已考虑替代方案)**: 手写 JSON schema(JSON 结构模式) 文件。该方案会在字段变化时产生漂移风险,也不能证明所有 nested configuration struct(嵌套配置结构体) 都具有 `JsonSchema`(结构模式生成特征) 能力。

## Decision(决策): `src/config/configurable.rs` 集中存放原始配置输入结构体

**Decision(决策)**: 新增 `src/config/configurable.rs`,集中存放 `SupervisorConfig`,`SupervisorRootConfig`,`PolicyConfig`,`ShutdownConfig`,`ObservabilityConfig` 和后续配置输入结构体。

**Rationale(理由)**: 当前 `src/config/state.rs` 同时包含 raw configuration input(原始配置输入) 和 validated config state(已校验配置状态)。本功能需要让使用者复用公开输入模型,同时让 `ConfigState`(配置状态) 继续表达已校验状态。拆分后,配置输入模型,校验状态和运行时规格派生各自有明确所有权。

**Alternatives considered(已考虑替代方案)**: 把 `src/config/` 目录改名为 `src/configurable/`。该方案被拒绝,因为用户已经明确纠正过边界: `configurable.rs` 是 `src/config/` 下的集中输入模型文件,不是替换整个 `config` module(配置模块)。

## Decision(决策): 不在本 crate(包) 内置 `x-tree-split`

**Decision(决策)**: 本 crate(包) 的公开配置结构体,官方 schema(结构模式),官方 YAML(数据序列化格式) template(模板) 和示例不添加默认 `x-tree-split`(树形拆分扩展)。

**Rationale(理由)**: tree split decision(树形拆分决策) 属于使用者项目的文件组织策略。基础 crate(包) 只提供 schema-ready configuration model(可生成结构模式的配置模型),不能把自己的拆分布局强加给所有使用者。

**Alternatives considered(已考虑替代方案)**: 在 `SupervisorConfig`(监督器配置) 或官方 schema(结构模式) 中直接写入 `x-tree-split`(树形拆分扩展)。该方案与 FR-007(功能需求七) 冲突,并且会让默认 template generation(模板生成) 产生多个目标文件。

## Decision(决策): 官方 template(模板) 默认只生成单个 YAML(数据序列化格式) target(目标文件)

**Decision(决策)**: 使用 `rust_config_tree::template_targets_for_paths::<SupervisorConfig>` 或等价 API(接口) 生成官方 template(模板) 时,默认只允许一个 root YAML template target(根 YAML 模板目标)。

**Rationale(理由)**: 单文件 YAML(数据序列化格式) 是本 crate(包) 的官方配置入口。多个 target(目标文件) 只应该在使用者自己的项目声明 `x-tree-split`(树形拆分扩展) 后出现。

**Alternatives considered(已考虑替代方案)**: 同时发布单文件模板和拆分模板。该方案会让官方边界不清晰,也会让使用者误以为本 crate(包) 已经决定拆分布局。

## Decision(决策): `ConfigState::try_from` 承担语义校验

**Decision(决策)**: `serde_yaml`(YAML 反序列化) 负责 syntax validation(语法校验) 和 structural validation(结构校验),`ConfigState::try_from(SupervisorConfig)` 负责 semantic validation(语义校验),`ConfigState::to_supervisor_spec` 必须继续调用 `SupervisorSpec::validate()`。

**Rationale(理由)**: 配置错误必须在 runtime startup(运行时启动) 前失败。该边界可以保证非法配置不会创建 runtime channel(运行时通道),不会启动 control loop(控制循环),也不会返回 `SupervisorHandle`(监督器句柄)。

**Alternatives considered(已考虑替代方案)**: 在 runtime control loop(运行时控制循环) 中延迟校验。该方案被拒绝,因为它允许非法配置进入运行时生命周期,违反 startup rejection(启动拒绝) 需求。

## Decision(决策): 添加从配置状态启动的显式入口

**Decision(决策)**: 在 `src/runtime/supervisor.rs` 中规划 `Supervisor::start_from_config_state` 和 `Supervisor::start_from_config_file`。前者接收 `ConfigState`(配置状态),后者接收 YAML(数据序列化格式) 文件路径。两个入口都必须在创建 channel(通道) 前完成 `to_supervisor_spec`(派生监督器规格) 和 `SupervisorSpec::validate()`。

**Rationale(理由)**: 使用者需要一个清晰的配置启动路径来验证 startup rejection(启动拒绝)。只暴露 `Supervisor::start(SupervisorSpec)` 会迫使使用者自己组合加载,校验和启动顺序,容易产生遗漏。

**Alternatives considered(已考虑替代方案)**: 只在文档中要求使用者先调用 `load_config_state` 再调用 `Supervisor::start`。该方案能工作,但不能用一个 API(接口) 明确保证非法配置不进入 channel creation(通道创建) 阶段。
