# Implementation Plan(实现计划): 配置结构体模式支持

**Branch(分支)**: `002-config-schema-support` | **Date(日期)**: 2026-05-05 | **Spec(规格)**: [spec.md](./spec.md)
**Input(输入)**: 功能规格来自 `specs/002-config-schema-support/spec.md`

## Summary(摘要)

本功能把 supervisor configuration(监督器配置) 的公开输入模型从 validated state(已校验状态) 中拆开,集中放入 `src/config/configurable.rs`。公开 root configuration struct(根配置结构体) 和所有 nested configuration struct(嵌套配置结构体) 必须同时支持 `confique::Config`(配置派生),`schemars::JsonSchema`(结构模式生成特征),`Serialize`(序列化) 和 `Deserialize`(反序列化)。配置加载仍以 YAML(数据序列化格式) 为官方格式,但 schema(结构模式) 和 template(模板) 必须从同一个 root configuration struct(根配置结构体) 生成。

本 crate(包) 不在公开配置结构体,官方 schema(结构模式) 或官方 template(模板) 中默认写入 `x-tree-split`(树形拆分扩展)。使用者可以在自己的项目中包装或复用 `SupervisorConfig`(监督器配置),并自行决定是否启用 `x-tree-split`(树形拆分扩展)。配置校验失败必须在 runtime startup(运行时启动) 前返回 `SupervisorError::FatalConfig`(致命配置错误),不得创建 runtime channel(运行时通道),不得启动 control loop(控制循环),不得返回 `SupervisorHandle`(监督器句柄)。

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024,`rust-version = "1.88"`
**Primary Dependencies(主要依赖)**: `rust-config-tree = "0.1.9"`,`confique = { version = "0.4.0", features = ["yaml"] }`,`schemars = { version = "1", features = ["derive"] }`,`serde`,`serde_yaml`
**Storage(存储)**: YAML(数据序列化格式) 配置文件和生成的 YAML(数据序列化格式) template(模板),没有数据库或持久化状态
**Testing(测试)**: `cargo fmt --all --check`,`cargo test`,`cargo clippy --all-targets --all-features -- -D warnings`,并增加 `configurable_schema_test`,`configurable_template_test`,`no_baked_in_tree_split_test`,`invalid_config_rejected_test`
**Target Platform(目标平台)**: Rust library crate(Rust 库包),Tokio(异步运行时) 本地进程
**Project Type(项目类型)**: Rust single crate(Rust 单包) supervisor runtime(监督器运行时) 库
**Performance Goals(性能目标)**: schema generation(结构模式生成) 和 template generation(模板生成) 在测试中必须完成,无网络依赖,非法配置必须在任何 runtime spawn(运行时派生) 前失败
**Constraints(约束)**: 禁止 compatibility export(兼容导出),禁止官方默认 `x-tree-split`(树形拆分扩展),`src/config/mod.rs` 只能包含 `pub mod ...;` 模块声明,模块导入使用绝对路径,测试文件必须以 `_test.rs` 结尾,代码注释和 rustdoc(代码文档) 使用英文
**Scale/Scope(规模和范围)**: 本功能覆盖 `src/config/`,`src/spec/`,`src/runtime/`,测试清单,README(说明文档),manual(手册),examples(示例程序) 和本 feature(功能) 的 contracts(契约)

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前通过。Phase 1(设计阶段) 后重新检查。*

- **Module Ownership(模块所有权)**: PASS(通过)。配置输入模型归属 `src/config/configurable.rs`,已校验状态归属 `src/config/state.rs`,加载和解析归属 `src/config/loader.rs` 与 `src/config/yaml.rs`,运行时启动入口归属 `src/runtime/supervisor.rs`。本计划不增加 compatibility export(兼容导出)。
- **Supervision Contract(监督契约)**: PASS(通过)。本功能只改变 runtime startup(运行时启动) 前的配置入口。非法配置返回 `SupervisorError::FatalConfig`(致命配置错误),并且不得进入 channel creation(通道创建),control loop spawn(控制循环派生) 或 handle return(句柄返回) 阶段。已启动 runtime(运行时) 的 shutdown protocol(关闭协议) 不变。
- **Test Gate(测试关口)**: PASS(通过)。任务必须先添加 schema(结构模式),template(模板),`x-tree-split`(树形拆分扩展) 边界和 invalid config rejection(非法配置拒绝) 测试,再修改生产代码。最终必须运行 `cargo test`。
- **Observable Failures(可观察失败)**: PASS(通过)。语法错误,结构错误和语义错误都必须返回 `SupervisorError::FatalConfig`(致命配置错误),错误消息必须包含字段或 section(配置分区)。
- **Small Increment(小增量)**: PASS(通过)。新增 `confique` 和 `schemars` 是公开配置契约需要的最小依赖。功能边界局限于配置输入,结构模式,模板和启动前拒绝。
- **Chinese Writing(中文写作)**: PASS(通过)。本计划和派生文档使用中文正文,英文术语使用 `English(中文说明)` 格式。

## Project Structure(项目结构)

### Documentation(文档，本功能)

```text
specs/002-config-schema-support/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── public-api.md
│   └── config-template.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code(源代码，仓库根目录)

```text
src/
├── config/
│   ├── configurable.rs
│   ├── loader.rs
│   ├── mod.rs
│   ├── state.rs
│   ├── yaml.rs
│   └── tests/
│       ├── configurable_confique_test.rs
│       ├── configurable_schema_test.rs
│       ├── configurable_template_test.rs
│       ├── invalid_config_rejected_test.rs
│       ├── no_baked_in_tree_split_test.rs
│       └── yaml_config_test.rs
├── runtime/
│   └── supervisor.rs
├── spec/
│   └── supervisor.rs
└── tests/
    ├── supervisor_config_test.rs
    └── supervisor_docs_sync_test.rs

examples/
└── config/
    ├── supervisor.yaml
    └── supervisor.template.yaml

manual/
├── en/
└── zh/

README.md
README.zh.md
Cargo.toml
Cargo.lock
```

**Structure Decision(结构决定)**: 采用现有 Rust single crate(Rust 单包) 结构。`src/config/configurable.rs` 集中存放所有 raw configuration input(原始配置输入) 结构体。`src/config/state.rs` 只保留 `ConfigState`(配置状态),`TryFrom<SupervisorConfig>`(转换),validation(校验) 和 `to_supervisor_spec`(派生监督器规格)。`src/config/mod.rs` 只能声明 `pub mod configurable;`,`pub mod loader;`,`pub mod state;`,`pub mod yaml;`,不能重新导出。

## Phase 0(研究阶段) Output(输出)

研究结论记录在 [research.md](./research.md)。所有技术未知项都已经收敛为明确决策,包括直接依赖声明,配置结构体集中边界,不内置 `x-tree-split`(树形拆分扩展),默认单文件 YAML(数据序列化格式) template(模板),以及启动前拒绝非法配置。

## Phase 1(设计阶段) Output(输出)

数据模型记录在 [data-model.md](./data-model.md)。公开 API(接口) 契约记录在 [contracts/public-api.md](./contracts/public-api.md)。官方 template(模板) 契约记录在 [contracts/config-template.md](./contracts/config-template.md)。使用流程和验证命令记录在 [quickstart.md](./quickstart.md)。

## Constitution Check(宪章复查)

- **Module Ownership(模块所有权)**: PASS(通过)。设计文档把 raw input(原始输入),validated state(已校验状态),YAML loader(YAML 加载器),schema/template(结构模式和模板),runtime startup(运行时启动) 都分配到明确文件。
- **Supervision Contract(监督契约)**: PASS(通过)。设计文档明确非法配置必须在 `Supervisor::start_from_config_state` 或 `Supervisor::start_from_config_file` 创建 runtime channel(运行时通道) 前失败。
- **Test Gate(测试关口)**: PASS(通过)。设计文档要求每个行为变化先有测试任务。
- **Observable Failures(可观察失败)**: PASS(通过)。契约要求 `FatalConfig`(致命配置错误) 包含字段或 section(配置分区)。
- **Small Increment(小增量)**: PASS(通过)。新增依赖和模块均由公开配置契约直接驱动。
- **Chinese Writing(中文写作)**: PASS(通过)。所有 002 feature(功能) 文档使用中文正文和术语说明。

## Complexity Tracking(复杂度跟踪)

本功能没有宪章违反项,所以不需要复杂度例外。
