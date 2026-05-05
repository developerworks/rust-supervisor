# Feature Specification(功能规格): 配置结构体模式支持

**Feature Branch(功能分支)**: `002-config-schema-support`
**Created(创建日期)**: 2026-05-05
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述:"根配置结构体必须同时支持 `confique::Config`(配置派生) 和 `JsonSchema`(结构模式生成特征).配置输入结构体集中存放在 config module(配置模块) 的 configurable boundary(可配置边界).本 crate(包) 不默认启用 `x-tree-split`(树形拆分扩展),使用者可以在自己的项目中自行决定是否启用.配置校验失败必须拒绝启动 runtime(运行时)."

## User Scenarios & Testing(用户场景和测试) *(mandatory(必填))*

### User Story 1(用户故事一) - 复用根配置结构体 (Priority(优先级): P1)

crate user(crate 使用者) 需要直接复用公开 root configuration struct(根配置结构体),用同一个配置模型完成 YAML(数据序列化格式) 加载,template generation(模板生成),schema generation(结构模式生成) 和后续 validation(校验).使用者不应该为配置模板或 schema(结构模式) 重新维护另一套手写模型.

**Why this priority(为什么是这个优先级)**: 配置模型是使用者接入 supervisor(监督器) 的第一入口.如果加载模型,模板模型和 schema(结构模式) 模型不一致,使用者会拿到错误提示,过期模板或无法启动的配置.

**Independent Test(独立测试)**: 使用公开 root configuration struct(根配置结构体) 生成 schema(结构模式) 和 YAML(数据序列化格式) template(模板),再用模板中的完整配置加载出 validated config state(已校验配置状态),并证明没有第二套配置模型参与.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** crate user(crate 使用者) 引入公开 root configuration struct(根配置结构体),**When(当)** 他生成 schema(结构模式),**Then(则)** schema(结构模式) 必须覆盖所有公开可配置 section(配置分区) 和 field(字段).
2. **Given(假设)** crate user(crate 使用者) 生成 YAML(数据序列化格式) template(模板),**When(当)** 他按模板填写配置,**Then(则)** 配置必须能进入 validated config state(已校验配置状态),并能继续派生 supervisor specification(监督器规格).
3. **Given(假设)** 公开配置模型发生字段变化,**When(当)** 维护者重新生成 schema(结构模式) 和 template(模板),**Then(则)** 新字段必须只从同一个 root configuration struct(根配置结构体) 出现,不得要求维护第二套模型.

---

### User Story 2(用户故事二) - 自行决定树形拆分策略 (Priority(优先级): P2)

crate user(crate 使用者) 需要在自己的项目中决定是否使用 `x-tree-split`(树形拆分扩展) 拆分配置文件.本 crate(包) 只提供 schema-ready configuration model(可生成结构模式的配置模型),不在官方结构体,官方 schema(结构模式) 或官方 template(模板) 中强制拆分布局.

**Why this priority(为什么是这个优先级)**: 配置拆分方式属于使用者项目的组织策略.基础 crate(包) 如果默认写死拆分标记,就会把自己的文件布局强加给所有使用者.

**Independent Test(独立测试)**: 生成本 crate(包) 官方 schema(结构模式) 和官方 YAML(数据序列化格式) template(模板),验证其中没有默认 `x-tree-split`(树形拆分扩展).再证明使用者可以在自己的项目中包装配置模型并做自己的 tree split decision(树形拆分决策).

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 使用者没有声明 tree split decision(树形拆分决策),**When(当)** 他生成官方 YAML(数据序列化格式) template(模板),**Then(则)** 系统必须只生成一个 root template target(根模板目标).
2. **Given(假设)** 使用者希望拆分配置,**When(当)** 他在自己的项目中包装公开配置模型并声明 `x-tree-split`(树形拆分扩展),**Then(则)** 该决定属于使用者项目,本 crate(包) 不阻止也不强制.
3. **Given(假设)** 维护者检查官方 schema(结构模式),**When(当)** schema(结构模式) 包含本 crate(包) 自己写死的 `x-tree-split`(树形拆分扩展),**Then(则)** 该 schema(结构模式) 不满足本功能规格.

---

### User Story 3(用户故事三) - 校验失败拒绝启动 (Priority(优先级): P3)

operator(操作者) 和 crate user(crate 使用者) 需要在配置错误时得到明确失败,而不是让 runtime(运行时) 以部分配置,隐式默认值或错误状态启动.配置必须先完成 syntax validation(语法校验),structural validation(结构校验) 和 semantic validation(语义校验),然后才能进入 runtime startup(运行时启动).

**Why this priority(为什么是这个优先级)**: supervisor(监督器) 管理生命周期,重启和关闭策略.非法配置如果仍然启动,会导致错误重启,错误关闭或不可解释的状态.

**Independent Test(独立测试)**: 分别提交缺失必填项,非法 enum value(枚举值),零值容量,零值超时,越界 jitter ratio(抖动比例) 和反向 backoff(退避) 配置,验证每个配置都返回 fatal config error(致命配置错误),并且没有返回 runtime handle(运行时句柄).

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** YAML(数据序列化格式) 配置缺失必填项,**When(当)** 使用者加载配置,**Then(则)** 系统必须返回 fatal config error(致命配置错误).
2. **Given(假设)** 配置字段值不满足取值范围,**When(当)** 使用者从配置启动 runtime(运行时),**Then(则)** 系统必须拒绝启动,并说明失败字段.
3. **Given(假设)** 配置无法派生有效 supervisor specification(监督器规格),**When(当)** 启动入口处理该配置,**Then(则)** 系统不得创建 runtime channel(运行时通道),不得启动 control loop(控制循环),不得返回 runtime handle(运行时句柄).

### Edge Cases(边界情况)

- root configuration struct(根配置结构体) 可以生成 schema(结构模式),但某个 nested configuration struct(嵌套配置结构体) 不能生成 schema(结构模式) 时,configuration schema check(配置结构模式检查) 必须失败.
- template generation(模板生成) 输出多个目标文件,但使用者没有提供 tree split decision(树形拆分决策) 时,default template check(默认模板检查) 必须失败.
- 官方 schema(结构模式),官方 template(模板) 或官方文档中默认启用 `x-tree-split`(树形拆分扩展) 时,configuration schema boundary check(配置结构模式边界检查) 必须失败.
- 配置加载失败后仍然返回 runtime handle(运行时句柄) 时,startup rejection check(启动拒绝检查) 必须失败.
- 可配置输入结构体和 validated config state(已校验配置状态) 混在同一个职责中,导致使用者无法区分 raw configuration input(原始配置输入) 和 validated configuration state(已校验配置状态) 时,configurable boundary check(可配置边界检查) 必须失败.
- schema(结构模式),template(模板),manual(手册) 或 README(说明文档) 与公开配置模型不一致时,documentation sync check(文档同步检查) 必须失败.

## Requirements(需求) *(mandatory(必填))*

### Functional Requirements(功能需求)

- **FR-001**: 系统必须提供一个公开 root configuration struct(根配置结构体),用于表达完整 supervisor configuration(监督器配置) 输入.
- **FR-002**: root configuration struct(根配置结构体) 和所有 nested configuration struct(嵌套配置结构体) 必须支持 `confique::Config`(配置派生).
- **FR-003**: root configuration struct(根配置结构体) 和所有 nested configuration struct(嵌套配置结构体) 必须支持 `JsonSchema`(结构模式生成特征).
- **FR-004**: root configuration struct(根配置结构体) 和所有 nested configuration struct(嵌套配置结构体) 必须支持 `Serialize`(序列化) 和 `Deserialize`(反序列化),以保证 YAML(数据序列化格式) 输入和文档示例使用同一个模型.
- **FR-005**: 系统必须集中管理 configurable struct set(可配置结构体集合),并清楚区分 raw configuration input(原始配置输入) 和 validated config state(已校验配置状态).
- **FR-006**: 系统必须让 root configuration struct(根配置结构体) 引用到的 public enum(公开枚举) 支持配置反序列化和 schema generation(结构模式生成).
- **FR-007**: 系统不得在官方 root configuration struct(根配置结构体),官方 schema(结构模式) 或官方 template(模板) 中默认写入 `x-tree-split`(树形拆分扩展).
- **FR-008**: 系统必须允许 crate user(crate 使用者) 在自己的项目中包装或复用公开配置模型,并自行声明 tree split decision(树形拆分决策).
- **FR-009**: 系统必须在没有使用者 tree split decision(树形拆分决策) 时生成单个 root YAML template target(根 YAML 模板目标).
- **FR-010**: 系统必须让官方 template(模板) 覆盖所有公开 runtime tunable configuration(运行时可调配置),包括 strategy(策略),budget(预算),timeout(超时),capacity(容量),observability switch(可观测性开关),health interval(健康间隔),backoff(退避) 和 shutdown budget(关闭预算).
- **FR-011**: 系统必须在配置进入 runtime startup(运行时启动) 前完成 syntax validation(语法校验),structural validation(结构校验) 和 semantic validation(语义校验).
- **FR-012**: 配置校验失败时,系统必须返回 fatal config error(致命配置错误),并说明失败字段或失败 section(配置分区).
- **FR-013**: 配置校验失败时,系统不得创建 runtime channel(运行时通道),不得启动 control loop(控制循环),不得返回 runtime handle(运行时句柄).
- **FR-014**: 系统必须提供 schema coverage check(结构模式覆盖检查),验证 schema(结构模式) 覆盖所有公开可配置字段.
- **FR-015**: 系统必须提供 default template check(默认模板检查),验证官方 template(模板) 默认只有一个 root YAML template target(根 YAML 模板目标),并且不包含默认 `x-tree-split`(树形拆分扩展).
- **FR-016**: 系统必须同步 README(说明文档),manual(手册),quickstart(快速开始),examples(示例程序) 和 contracts(契约),说明 schema-ready configuration model(可生成结构模式的配置模型),使用者 tree split decision(树形拆分决策) 和 startup rejection(启动拒绝) 边界.
- **FR-017**: 系统不得提供 compatibility export(兼容导出),旧配置别名,迁移层或历史配置字段来表达本功能.

### Key Entities(关键实体) *(include if feature involves data(涉及数据时填写))*

- **SupervisorConfig(监督器配置)**: crate user(crate 使用者) 复用的 root configuration struct(根配置结构体),用于表达完整配置输入,template generation(模板生成) 和 schema generation(结构模式生成).
- **ConfigurableStructSet(可配置结构体集合)**: 所有 nested configuration struct(嵌套配置结构体) 的集合,用于保持配置输入模型集中.
- **ConfigState(配置状态)**: 已通过 validation(校验) 的不可变配置状态,用于派生 supervisor specification(监督器规格),不承担原始输入模型集中职责.
- **ConfigurationSchema(配置结构模式)**: 从 root configuration struct(根配置结构体) 生成的 schema(结构模式),用于编辑器提示,外部校验和使用者项目集成.
- **ConfigurationTemplate(配置模板)**: 使用者学习和填写配置的 YAML(数据序列化格式) 模板,默认情况下必须是单个 root template target(根模板目标).
- **TreeSplitDecision(树形拆分决策)**: 使用者项目是否启用 `x-tree-split`(树形拆分扩展) 的选择,不由本 crate(包) 默认强制.
- **ConfigurationValidationResult(配置校验结果)**: 配置加载,结构校验和语义校验后的结果,成功时进入 `ConfigState`(配置状态),失败时返回 fatal config error(致命配置错误).

## Constitution Alignment(宪章对齐) *(mandatory(必填))*

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本功能影响 runtime startup(运行时启动) 前的配置入口.非法配置必须在启动前失败,不得进入生命周期控制循环.
- **Failure behavior(失败行为)**: 配置错误必须以 fatal config error(致命配置错误) 返回,并说明失败字段或配置分区.
- **Shutdown behavior(关闭行为)**: 本功能不改变已启动 runtime(运行时) 的 shutdown protocol(关闭协议),但必须保证非法配置不会启动需要关闭的 runtime(运行时).

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: 配置输入模型属于 config module(配置模块) 的 configurable boundary(可配置边界).validated config state(已校验配置状态) 和 startup rejection(启动拒绝) 边界必须保持清晰.
- **Compatibility exports(兼容导出)**: None(无).
- **Diagnostics(诊断)**: 配置加载,结构模式生成,模板生成和启动拒绝必须返回可定位的错误信息,并能指出失败字段或配置分区.
- **Dependency impact(依赖影响)**: 本功能需要公开配置结构体支持 `confique::Config`(配置派生) 和 `JsonSchema`(结构模式生成特征),因为使用者需要从同一个配置模型生成 template(模板) 和 schema(结构模式).

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本规格使用中文写作.
- **Term format(术语格式)**: 英文术语以 `English(中文说明)` 形式出现.
- **Forbidden style(禁止风格)**: 本规格不使用非中文正文,片段式语言,生僻词或方言.

## Success Criteria(成功标准) *(mandatory(必填))*

### Measurable Outcomes(可衡量结果)

- **SC-001**: 100% 公开可配置字段都能从 root configuration struct(根配置结构体) 生成的 schema(结构模式) 中找到.
- **SC-002**: 100% nested configuration struct(嵌套配置结构体) 都支持 `confique::Config`(配置派生) 和 `JsonSchema`(结构模式生成特征).
- **SC-003**: 默认 template generation(模板生成) 在没有使用者 tree split decision(树形拆分决策) 时只产生 1 个 root YAML template target(根 YAML 模板目标).
- **SC-004**: 官方 schema(结构模式) 和官方 template(模板) 中 `x-tree-split`(树形拆分扩展) 默认出现次数必须为 0.
- **SC-005**: 缺失必填项,非法 enum value(枚举值),零值容量,零值超时,越界 jitter ratio(抖动比例) 和反向 backoff(退避) 这 6 类非法配置必须全部在启动前失败.
- **SC-006**: 配置校验失败场景中返回 runtime handle(运行时句柄) 的次数必须为 0.
- **SC-007**: README(说明文档),manual(手册),quickstart(快速开始),examples(示例程序) 和 contracts(契约) 必须全部说明 schema-ready configuration model(可生成结构模式的配置模型),tree split decision(树形拆分决策) 和 startup rejection(启动拒绝) 边界.

## Assumptions(假设)

- 本功能基于已有 supervisor core(监督器核心) 配置能力继续演进,不重写 runtime supervision(运行时监督) 行为.
- `confique::Config`(配置派生) 和 `JsonSchema`(结构模式生成特征) 都属于公开配置模型的必备能力.
- `x-tree-split`(树形拆分扩展) 是使用者项目的布局选择,不是本 crate(包) 的默认策略.
- 配置主格式仍然是 YAML(数据序列化格式).
- 配置校验失败必须在 runtime startup(运行时启动) 前结束,不能依赖启动后的控制命令补救.
- 本功能不提供旧配置字段,旧模块路径或兼容层.
