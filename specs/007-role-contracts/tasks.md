# Tasks(任务): 007 Role Contracts(角色接入契约)

**Input(输入)**: 设计文档来自 `specs/007-role-contracts/`
**Prerequisites(前置文档)**: spec.md, api-draft.md, data-model.md, contracts/*.md
**Tests(测试)**: 行为变化必须先有测试任务, 再有实现任务. 纯文档变更必须说明运行时测试为什么不适用.
**Organization(组织方式)**: 任务先交付 `Service`(服务) 最小闭环, 再复制到其他角色.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]** 表示可以并行执行, 因为任务修改不同文件, 并且不依赖未完成任务.
- **[Story]** 标记任务属于哪个用户故事.
- 任务描述必须写出准确文件路径.
- 任务描述必须使用中文完整句子, 英文术语必须写成 `English(中文说明)`.
- Rust(编程语言) 测试不得写入生产模块文件. 测试应放在外部测试目标或专用测试目录.

---

## Phase 1(阶段 1): Specification(规格)

**Purpose(目的)**: 固定角色契约和 API(应用程序接口) 形态.

- [X] T001 [P] [US1] 在 `specs/007-role-contracts/spec.md` 中审阅并冻结 `Service`(服务) 用户故事, 功能需求和成功标准.
- [X] T002 [P] [US1] 在 `specs/007-role-contracts/api-draft.md` 中审阅并冻结 `#[service]` 和 `ServiceRole`(服务角色特征) 草案.
- [X] T003 [P] [US1] 在 `specs/007-role-contracts/contracts/service-contract.md` 中审阅并冻结 `Service`(服务) 生命周期规则.
- [X] T004 [P] [US2] 在 `specs/007-role-contracts/data-model.md` 中审阅并冻结 `RoleContext`(角色上下文), `RoleResult`(角色结果) 和 `RoleAdapter`(角色适配器) 模型.

---

## Phase 2(阶段 2): Service Runtime Contract(服务运行时契约)

**Purpose(目的)**: 先实现不依赖 macro(宏) 的显式契约.

- [X] T005 [US2] 在 `rust-supervisor/src/role/mod.rs` 中注册 `role` 模块入口.
- [X] T006 [P] [US2] 在 `rust-supervisor/src/role/context/service.rs` 中实现 `ServiceContext`(服务上下文).
- [X] T007 [P] [US2] 在 `rust-supervisor/src/role/traits/service.rs` 中实现 `ServiceRole`(服务角色特征) 和默认生命周期方法.
- [X] T008 [P] [US2] 在 `rust-supervisor/src/role/result/service.rs` 中实现 `ServiceResult`(服务结果) 与 `ServiceError`(服务错误).
- [X] T009 [US2] 在 `rust-supervisor/src/role/adapter/service.rs` 中实现 `ServiceRoleAdapter`(服务角色适配器).
- [X] T010 [US2] 在 `rust-supervisor/src/role/adapter/service.rs` 中把 `ServiceRoleAdapter`(服务角色适配器) 接入现有 `TaskFactory`(任务工厂).

---

## Phase 3(阶段 3): Service Macro Entry(服务宏入口)

**Purpose(目的)**: 实现 `#[service]` 默认入口.

- [X] T011 [US1] 在 `rust-supervisor/Cargo.toml` 中接入 `rust-supervisor-macros` 依赖, 并避免新增 compatibility exports(兼容导出).
- [X] T012 [US1] 在 `rust-supervisor-macros/Cargo.toml` 中添加 `syn`(Rust 语法解析库), `quote`(代码生成库) 和 `proc-macro2`(过程宏辅助库).
- [X] T013 [P] [US1] 在 `rust-supervisor-macros/src/parse/role_args.rs` 中解析 `#[service(id, name)]` 参数.
- [X] T014 [P] [US1] 在 `rust-supervisor-macros/src/parse/lifecycle_impl.rs` 中解析 `impl block`(实现块) 并检查 `run` 方法.
- [X] T015 [US1] 在 `rust-supervisor-macros/src/expand/service.rs` 中生成 `ServiceRole`(服务角色特征) 实现和 adapter(适配器) 桥接代码.
- [X] T016 [US1] 在 `rust-supervisor-macros/src/attribute/service.rs` 中注册 `#[service]` attribute macro(属性宏).

---

## Phase 4(阶段 4): Service Tests and Example(服务测试和示例)

**Purpose(目的)**: 验证 `Service`(服务) 最小闭环.

- [X] T017 [US1] 在 `rust-supervisor/tests/role_contracts/service_macro_pass.rs` 中添加 `#[service]` compile-pass(编译通过) 测试.
- [X] T018 [US1] 在 `rust-supervisor/tests/role_contracts/service_macro_fail.rs` 中添加缺少 `run` 方法的 compile-fail(编译失败) 测试.
- [X] T019 [US1] 在 `rust-supervisor/examples/step_02_supervisor_with_service/main.rs` 中迁移自由函数 `run_service` 为 `#[service]` 角色契约示例.
- [X] T020 [US1] 运行 `cargo check --workspace --all-targets`, 确认主包和宏包能一起编译.
- [X] T021 [US1] 运行 `cargo test --workspace`, 确认现有测试和新增宏测试全部通过.

---

## Phase 5(阶段 5): Other Role Contracts(其他角色契约)

**Purpose(目的)**: 在 `Service`(服务) 闭环稳定后复制结构到其他角色.

- [X] T022 [P] [US2] 在 `rust-supervisor/src/role/context/worker.rs`, `rust-supervisor/src/role/traits/worker.rs` 和 `rust-supervisor/src/role/adapter/worker.rs` 中实现 `Worker`(后台任务) 运行时契约.
- [X] T023 [P] [US2] 在 `rust-supervisor/src/role/context/job.rs`, `rust-supervisor/src/role/traits/job.rs` 和 `rust-supervisor/src/role/adapter/job.rs` 中实现 `Job`(一次性任务) 运行时契约.
- [X] T024 [P] [US2] 在 `rust-supervisor/src/role/context/sidecar.rs`, `rust-supervisor/src/role/traits/sidecar.rs` 和 `rust-supervisor/src/role/adapter/sidecar.rs` 中实现 `Sidecar`(边车) 运行时契约.
- [X] T025 [P] [US2] 在 `rust-supervisor/src/role/context/supervisor.rs`, `rust-supervisor/src/role/traits/supervisor.rs` 和 `rust-supervisor/src/role/adapter/supervisor.rs` 中实现 `Supervisor`(监督器) 运行时契约.
- [X] T026 [P] [US1] 在 `rust-supervisor-macros/src/attribute/worker.rs` 和对应 `parse/`, `expand/` 文件中实现 `#[worker]`.
- [X] T027 [P] [US1] 在 `rust-supervisor-macros/src/attribute/job.rs` 和对应 `parse/`, `expand/` 文件中实现 `#[job]`.
- [X] T028 [P] [US1] 在 `rust-supervisor-macros/src/attribute/sidecar.rs` 和对应 `parse/`, `expand/` 文件中实现 `#[sidecar]`.
- [X] T029 [P] [US1] 在 `rust-supervisor-macros/src/attribute/supervisor_role.rs` 和对应 `parse/`, `expand/` 文件中实现 `#[supervisor_role]`.

---

## Phase 6(阶段 6): Template Entry(模板入口)

**Purpose(目的)**: 提供 abstract default implementation(抽象默认实现) 入口.

- [X] T030 [P] [US3] 在 `rust-supervisor/src/role/templates/service.rs` 中实现 `Service`(服务) 模板入口.
- [X] T031 [P] [US3] 在 `rust-supervisor/src/role/templates/worker.rs` 中实现 `Worker`(后台任务) 模板入口.
- [X] T032 [P] [US3] 在 `rust-supervisor/src/role/templates/job.rs` 中实现 `Job`(一次性任务) 模板入口.
- [X] T033 [P] [US3] 在 `rust-supervisor/src/role/templates/sidecar.rs` 中实现 `Sidecar`(边车) 模板入口.
- [X] T034 [P] [US3] 在 `rust-supervisor/src/role/templates/supervisor.rs` 中实现 `Supervisor`(监督器) 模板入口.

---

## Phase 7(阶段 7): Quality Gates(质量门禁)

**Purpose(目的)**: 完成回归验证和文档同步.

- [X] T035 [P] 在 `rust-supervisor/tests/role_contracts/` 中补齐 5 个角色的 compile-pass(编译通过) 测试.
- [X] T036 [P] 在 `rust-supervisor/tests/role_contracts/` 中补齐 5 个角色的 compile-fail(编译失败) 测试.
- [X] T037 在 `rust-supervisor/src/tests/glossary_coverage_test.rs` 中确认新术语已经纳入 glossary(术语表) 或测试例外.
- [X] T038 运行 `cargo fmt --check --all`, 确认格式化无漂移.
- [X] T039 运行 `cargo check --workspace --all-targets`, 确认编译无回归.
- [X] T040 运行 `cargo test --workspace`, 确认全部测试通过.

## Dependencies(依赖关系)

```text
Phase 1 -> Phase 2 -> Phase 3 -> Phase 4 -> Phase 5 -> Phase 6 -> Phase 7
```

`Service`(服务) 闭环是所有其他角色复制实现的前置条件. Phase 5 中标记为 [P] 的任务必须在 `Service`(服务) 闭环稳定后再并行执行.

## Summary(摘要)

本任务拆分共有 40 个任务. MVP(最小可用产品) 范围是 Phase 1 到 Phase 4, 也就是完成 `Service`(服务) 的运行时契约, `#[service]` 宏入口, 编译测试和示例迁移.
