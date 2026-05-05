# Tasks(任务): 配置结构体模式支持

**Input(输入)**: 设计文档来自 `specs/002-config-schema-support/`
**Prerequisites(前置文档)**: plan.md(必需),spec.md(用户故事必需),research.md,data-model.md,contracts/,quickstart.md

**Tests(测试)**: 本功能改变公开 configuration API(配置接口),schema generation(结构模式生成),template generation(模板生成) 和 runtime startup rejection(运行时启动拒绝),所以必须先列测试任务,再列实现任务。

**Organization(组织方式)**: 任务按 User Story(用户故事) 分组。Phase 1(阶段一) 只处理依赖和测试注册。Phase 2(阶段二) 只处理所有故事共享的阻塞边界。三个用户故事在 Phase 2(阶段二) 之后可以并行开发,但同一文件只能由一个 workstream(工作流) 负责。

## Format(格式): `[ID] [P?] [Story?] Description(描述)`

- **[P]**: 可以并行执行,因为任务修改不同文件,并且不依赖未完成任务。
- **[Story]**: 标记任务属于哪个用户故事,例如 US1,US2,US3。
- 任务描述写出准确文件路径。
- 任务描述使用中文;英文术语使用 `English(中文说明)`。
- Rust(编程语言) 测试必须放在外部 `tests/` 目录或模块自身的 `tests/` 子目录,测试文件必须以 `_test.rs` 结尾。
- `src/config/mod.rs` 只能包含 `pub mod ...;` 模块声明,不得重新导出。

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 准备依赖和测试入口,让后续测试任务可以编译并运行。

- [ ] T001 在 `Cargo.toml` 和 `Cargo.lock` 中加入直接依赖 `confique = { version = "0.4.0", features = ["yaml"] }` 和 `schemars = { version = "1", features = ["derive"] }`。
- [ ] T002 在 `Cargo.toml` 中注册 `src/config/tests/configurable_schema_test.rs`,`src/config/tests/configurable_confique_test.rs`,`src/config/tests/configurable_template_test.rs`,`src/config/tests/no_baked_in_tree_split_test.rs` 和 `src/config/tests/invalid_config_rejected_test.rs`。

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 建立所有用户故事共享的模块所有权边界。

**Critical(关键要求)**: 本阶段完成前,任何用户故事实现不能开始。

- [ ] T003 在 `src/config/configurable.rs` 中创建 configurable boundary(可配置边界) 模块骨架,并写入英文 module rustdoc(模块代码文档)。
- [ ] T004 在 `src/config/mod.rs` 中声明 `pub mod configurable;`,`pub mod loader;`,`pub mod state;`,`pub mod yaml;`,并删除任何重新导出。
- [ ] T005 在 `src/config/state.rs` 中保留 `ConfigState`(配置状态),`TryFrom<SupervisorConfig>`(转换),validation(校验) 和 `to_supervisor_spec`(派生监督器规格) 的所有权说明。

**Checkpoint(检查点)**: raw configuration input(原始配置输入) 和 validated config state(已校验配置状态) 的模块边界已经清楚,用户故事实现可以开始。

---

## Phase 3(阶段三): User Story 1(用户故事一) - 复用根配置结构体 (Priority(优先级): P1)

**Goal(目标)**: crate user(crate 使用者) 可以复用 `SupervisorConfig`(监督器配置) 完成 YAML(数据序列化格式) 加载,template generation(模板生成),schema generation(结构模式生成) 和 validation(校验),不需要第二套配置模型。

**Independent Test(独立测试)**: 运行 `cargo test configurable_schema_test`,`cargo test configurable_confique_test` 和 `cargo test yaml_config_test`,并确认 schema(结构模式),trait bound(特征约束) 和配置状态转换都来自 `SupervisorConfig`(监督器配置)。

### Tests for User Story 1(用户故事一的测试)

- [ ] T006 [P] [US1] 在 `src/config/tests/configurable_schema_test.rs` 中添加 `schemars::schema_for!(SupervisorConfig)` 覆盖 `supervisor`,`policy`,`shutdown`,`observability` 的测试。
- [ ] T007 [P] [US1] 在 `src/config/tests/configurable_confique_test.rs` 中添加 `SupervisorConfig: confique::Config` 和 nested configuration struct(嵌套配置结构体) trait bound(特征约束) 测试。
- [ ] T008 [P] [US1] 在 `src/config/tests/yaml_config_test.rs` 中添加从 `SupervisorConfig`(监督器配置) 到 `ConfigState`(配置状态) 再到 `SupervisorSpec`(监督器规格) 的回归测试。

### Implementation for User Story 1(用户故事一的实现)

- [ ] T009 [US1] 在 `src/config/configurable.rs` 中集中定义 `SupervisorConfig`,`SupervisorRootConfig`,`PolicyConfig`,`ShutdownConfig`,`ObservabilityConfig`,并统一派生 `Debug`,`Clone`,`PartialEq`,`Serialize`,`Deserialize`,`confique::Config`,`schemars::JsonSchema`。
- [ ] T010 [US1] 在 `src/config/state.rs` 中移除 raw configuration input(原始配置输入) 结构体定义,并改为使用 `crate::config::configurable::SupervisorConfig` 和相关嵌套结构体。
- [ ] T011 [P] [US1] 在 `src/config/loader.rs` 中改用 `crate::config::configurable::SupervisorConfig` 作为 YAML(数据序列化格式) 输入模型。
- [ ] T012 [P] [US1] 在 `src/config/yaml.rs` 中改用 `crate::config::configurable::SupervisorConfig` 作为 YAML(数据序列化格式) 输入模型。
- [ ] T013 [US1] 在 `src/spec/supervisor.rs` 中让 `SupervisionStrategy`(监督策略) 支持 `schemars::JsonSchema`,并保持 `Serialize`(序列化) 和 `Deserialize`(反序列化)。
- [ ] T014 [US1] 在 `src/config/configurable.rs`,`src/config/state.rs`,`src/config/loader.rs`,`src/config/yaml.rs` 中补齐英文 rustdoc(代码文档),并保证导入全部使用绝对路径。

**Checkpoint(检查点)**: User Story 1(用户故事一) 已经可用,公开配置模型可以独立生成 schema(结构模式) 并进入已校验状态。

---

## Phase 4(阶段四): User Story 2(用户故事二) - 自行决定树形拆分策略 (Priority(优先级): P2)

**Goal(目标)**: 官方 schema(结构模式) 和 official YAML template(官方 YAML 模板) 默认不包含 `x-tree-split`(树形拆分扩展),使用者可以在自己的项目中自行决定拆分策略。

**Independent Test(独立测试)**: 运行 `cargo test configurable_template_test no_baked_in_tree_split_test`,并确认默认 template target(模板目标) 数量为 1,官方 schema(结构模式) 和模板中 `x-tree-split`(树形拆分扩展) 出现次数为 0。

### Tests for User Story 2(用户故事二的测试)

- [ ] T015 [P] [US2] 在 `src/config/tests/configurable_template_test.rs` 中添加 `rust_config_tree::template_targets_for_paths::<SupervisorConfig>` 默认只产生一个 root YAML template target(根 YAML 模板目标) 的测试。
- [ ] T016 [P] [US2] 在 `src/config/tests/no_baked_in_tree_split_test.rs` 中添加官方 schema(结构模式),官方 template(模板) 和 `examples/config/supervisor.template.yaml` 不包含 `x-tree-split`(树形拆分扩展) 的测试。

### Implementation for User Story 2(用户故事二的实现)

- [ ] T017 [US2] 在 `src/config/configurable.rs` 中为 `SupervisorConfig`(监督器配置) 提供 `rust-config-tree`(配置树库) template generation(模板生成) 所需的 schema metadata(结构模式元数据),并确保不内置 `x-tree-split`(树形拆分扩展)。
- [ ] T018 [P] [US2] 在 `examples/config/supervisor.yaml` 中提供完整单文件 YAML(数据序列化格式) 示例配置。
- [ ] T019 [P] [US2] 在 `examples/config/supervisor.template.yaml` 中提供完整单文件 YAML(数据序列化格式) 官方模板。
- [ ] T020 [P] [US2] 在 `README.md` 中说明 schema-ready configuration model(可生成结构模式的配置模型),单文件 template(模板) 和使用者自主管理 `x-tree-split`(树形拆分扩展) 的边界。
- [ ] T021 [P] [US2] 在 `README.zh.md` 中说明 schema-ready configuration model(可生成结构模式的配置模型),单文件 template(模板) 和使用者自主管理 `x-tree-split`(树形拆分扩展) 的边界。
- [ ] T022 [P] [US2] 在 `manual/en/configuration.md` 和 `manual/en/SUMMARY.md` 中补齐英文手册的配置模板和 tree split decision(树形拆分决策) 边界。
- [ ] T023 [P] [US2] 在 `manual/zh/configuration.md` 和 `manual/zh/SUMMARY.md` 中补齐中文手册的配置模板和 tree split decision(树形拆分决策) 边界。

**Checkpoint(检查点)**: User Story 2(用户故事二) 已经可用,官方配置模板保持单文件,并且没有强制使用者的拆分布局。

---

## Phase 5(阶段五): User Story 3(用户故事三) - 校验失败拒绝启动 (Priority(优先级): P3)

**Goal(目标)**: 配置错误必须在 runtime startup(运行时启动) 前返回 `SupervisorError::FatalConfig`(致命配置错误),并且不得返回 `SupervisorHandle`(监督器句柄)。

**Independent Test(独立测试)**: 运行 `cargo test invalid_config_rejected_test supervisor_config_test`,并确认缺失必填项,非法 enum value(枚举值),零值 capacity(容量),零值 timeout(超时),越界 `jitter_ratio`(抖动比例) 和反向 backoff(退避) 都在启动前失败。

### Tests for User Story 3(用户故事三的测试)

- [ ] T024 [P] [US3] 在 `src/config/tests/invalid_config_rejected_test.rs` 中添加缺失必填项,非法 enum value(枚举值),零值 capacity(容量),零值 timeout(超时),越界 `jitter_ratio`(抖动比例) 和反向 backoff(退避) 的 `FatalConfig`(致命配置错误) 测试。
- [ ] T025 [P] [US3] 在 `src/tests/supervisor_config_test.rs` 中添加 `Supervisor::start_from_config_state` 和 `Supervisor::start_from_config_file` 不返回 `SupervisorHandle`(监督器句柄) 的启动拒绝测试。

### Implementation for User Story 3(用户故事三的实现)

- [ ] T026 [US3] 在 `src/config/state.rs` 中增强 semantic validation(语义校验),确保正数约束,backoff(退避) 顺序,jitter ratio(抖动比例),shutdown budget(关闭预算) 和 capacity(容量) 的错误包含字段或 section(配置分区)。
- [ ] T027 [US3] 在 `src/runtime/supervisor.rs` 中实现 `Supervisor::start_from_config_state`,并在创建 channel(通道) 前调用 `ConfigState::to_supervisor_spec`。
- [ ] T028 [US3] 在 `src/runtime/supervisor.rs` 中实现 `Supervisor::start_from_config_file`,并复用 `crate::config::loader::load_config_state` 和 `Supervisor::start_from_config_state`。
- [ ] T029 [US3] 在 `src/config/loader.rs` 和 `src/config/yaml.rs` 中统一 syntax validation(语法校验),structural validation(结构校验) 和 semantic validation(语义校验) 的 `FatalConfig`(致命配置错误) 消息格式。

**Checkpoint(检查点)**: User Story 3(用户故事三) 已经可用,非法配置无法进入运行时启动阶段。

---

## Phase 6(阶段六): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 同步文档,验证 crate(包) 发布约束和最终质量门。

- [ ] T030 [P] 在 `src/tests/supervisor_docs_sync_test.rs` 中增加 README(说明文档),manual(手册),examples(示例程序) 和 `SupervisorConfig`(监督器配置) 字段一致性检查。
- [ ] T031 [P] 在 `src/tests/release_readiness_test.rs` 中增加 `confique`(配置库),`schemars`(结构模式库),README(说明文档) 和 examples(示例程序) 的发布清单检查。
- [ ] T032 [P] 在 `specs/002-config-schema-support/contracts/public-api.md` 中根据最终实现同步公开 API(接口) 契约。
- [ ] T033 [P] 在 `specs/002-config-schema-support/quickstart.md` 中根据最终实现同步验证命令和示例代码。
- [ ] T034 运行 `cargo fmt --all --check`,并把结果记录到 `artifacts/validation/002-config-schema-support.md`。
- [ ] T035 运行 `cargo test`,并把结果记录到 `artifacts/validation/002-config-schema-support.md`。
- [ ] T036 运行 `cargo clippy --all-targets --all-features -- -D warnings`,并把结果记录到 `artifacts/validation/002-config-schema-support.md`。
- [ ] T037 运行 `cargo package --allow-dirty`,并把 crate.io(crate 发布站点) 发布约束结果记录到 `artifacts/validation/002-config-schema-support.md`。

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Setup(阶段一)**: 没有依赖,必须先完成。
- **Foundational(阶段二)**: 依赖 Setup(阶段一),并阻塞所有用户故事。
- **User Stories(用户故事阶段)**: 全部依赖 Foundational(阶段二)。US1(用户故事一),US2(用户故事二) 和 US3(用户故事三) 可以由不同 workstream(工作流) 并行推进,但必须遵守文件所有权。
- **Polish(收尾阶段)**: 依赖所有选定用户故事完成。

### User Story Dependencies(用户故事依赖)

- **US1(用户故事一,P1)**: 依赖 Foundational(阶段二),不依赖 US2(用户故事二) 或 US3(用户故事三)。
- **US2(用户故事二,P2)**: 依赖 Foundational(阶段二)。T017 需要 `SupervisorConfig`(监督器配置) 的最终结构,所以该任务依赖 T009。文档和示例任务可以在 T009 后并行。
- **US3(用户故事三,P3)**: 依赖 Foundational(阶段二)。T027 和 T028 依赖 T026,因为启动入口必须复用已校验配置状态。

### File Ownership(文件所有权)

- **Configuration model workstream(配置模型工作流)**: 负责 `src/config/configurable.rs`,`src/config/state.rs`,`src/spec/supervisor.rs`。
- **Loading and startup workstream(加载和启动工作流)**: 负责 `src/config/loader.rs`,`src/config/yaml.rs`,`src/runtime/supervisor.rs`。
- **Template and docs workstream(模板和文档工作流)**: 负责 `examples/config/`,`README.md`,`README.zh.md`,`manual/en/`,`manual/zh/`。
- **Validation workstream(验证工作流)**: 负责 `src/config/tests/`,`src/tests/`,`artifacts/validation/002-config-schema-support.md`。

### Within Each User Story(每个用户故事内部)

- 行为变化的测试必须先写,并且必须在实现前失败。
- 先完成配置模型,再完成 loader(加载器) 和 startup(启动入口)。
- 先完成单故事验证,再进入跨故事收尾。
- 同一文件不能被多个 subagent(子代理) 同时修改。

## Parallel Opportunities(并行机会)

- T006,T007,T008 可以并行,因为它们修改不同测试文件。
- T011,T012 可以并行,因为它们修改不同 loader/parser(加载器和解析器) 文件。
- T015,T016 可以并行,因为它们修改不同测试文件。
- T018,T019,T020,T021,T022,T023 可以并行,因为它们修改不同示例和文档文件。
- T024,T025 可以并行,因为它们修改不同测试文件。
- T030,T031,T032,T033 可以并行,因为它们修改不同验证或文档文件。

## Parallel Example(并行示例)

```bash
# US1(用户故事一) 测试并行:
Task: "T006 在 src/config/tests/configurable_schema_test.rs 中添加 schema generation(结构模式生成) 测试"
Task: "T007 在 src/config/tests/configurable_confique_test.rs 中添加 confique::Config(配置派生) trait bound(特征约束) 测试"
Task: "T008 在 src/config/tests/yaml_config_test.rs 中添加配置状态转换测试"

# US2(用户故事二) 文档和模板并行:
Task: "T018 在 examples/config/supervisor.yaml 中提供完整单文件配置"
Task: "T019 在 examples/config/supervisor.template.yaml 中提供完整官方模板"
Task: "T020 在 README.md 中说明 x-tree-split(树形拆分扩展) 边界"
Task: "T023 在 manual/zh/configuration.md 和 manual/zh/SUMMARY.md 中补齐中文手册"

# US3(用户故事三) 测试并行:
Task: "T024 在 src/config/tests/invalid_config_rejected_test.rs 中添加非法配置测试"
Task: "T025 在 src/tests/supervisor_config_test.rs 中添加启动拒绝测试"
```

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 T001 到 T005,建立依赖和模块边界。
2. 完成 US1(用户故事一),让 `SupervisorConfig`(监督器配置) 成为唯一公开配置模型。
3. 运行 US1(用户故事一) 独立测试。
4. 交付可复用 root configuration struct(根配置结构体) 作为 MVP(最小可用产品)。

### Incremental Delivery(增量交付)

1. 完成 US1(用户故事一),交付 schema-ready configuration model(可生成结构模式的配置模型)。
2. 完成 US2(用户故事二),交付官方单文件 template(模板) 和 `x-tree-split`(树形拆分扩展) 边界。
3. 完成 US3(用户故事三),交付非法配置启动拒绝。
4. 完成 Polish(收尾阶段),同步文档和发布检查。

### Parallel Team Strategy(并行团队策略)

1. 主代理先完成 T001 到 T005,并锁定文件所有权。
2. 子代理按 File Ownership(文件所有权) 分配 workstream(工作流),每个 workstream(工作流) 只能修改自己的路径。
3. 主代理在每个 checkpoint(检查点) 后运行目标测试,发现模块边界或术语漂移时立即纠偏。
4. 所有 workstream(工作流) 合并后统一运行 T034 到 T037。
