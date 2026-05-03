---

description: "Task list template(任务列表模板) for feature implementation(功能实现)"
---

# Tasks(任务): [FEATURE NAME(功能名称)]

**Input(输入)**: 设计文档来自 `/specs/[###-feature-name]/`
**Prerequisites(前置文档)**: plan.md(必需)、spec.md(用户故事必需)、research.md、data-model.md、contracts/

**Tests(测试)**: 行为变化必须先有测试任务，再有实现任务。纯文档或纯模板变更必须说明运行时测试为什么不适用。

**Organization(组织方式)**: 任务必须按用户故事分组，确保每个故事都能独立实现和独立测试。

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行，因为任务修改不同文件，并且不依赖未完成任务。
- **[Story]**: 标记任务属于哪个用户故事，例如 US1、US2、US3。
- 任务描述必须写出准确文件路径。
- 任务描述必须使用中文；英文术语必须写成 `English(中文说明)`。

## Path Conventions(路径约定)

- **Rust single crate(Rust 单包)**: 仓库根目录下的 `src/`、`tests/` 和 `Cargo.toml`。
- **Web app(网页应用)**: `backend/src/` 和 `frontend/src/`。
- **Mobile(移动端)**: `api/src/`、`ios/src/` 或 `android/src/`。
- 下面路径默认使用 Rust single crate(Rust 单包) 布局；如果 `plan.md` 选择其他结构，必须按计划调整。

<!--
  ============================================================================
  IMPORTANT(重要): 下面任务只是示例。

  /speckit-tasks 命令必须根据这些内容生成真实任务：
  - spec.md 中按 P1、P2、P3 排序的用户故事
  - plan.md 中的功能需求
  - data-model.md 中的实体
  - contracts/ 中的契约

  任务必须按用户故事组织，确保每个故事都可以独立实现、独立测试，并作为 MVP(最小可用产品) 增量交付。

  生成的 tasks.md 文件不得保留这些示例任务。
  ============================================================================
-->

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 初始化项目结构和基础依赖。

- [ ] T001 按实现计划创建项目结构。
- [ ] T002 为已说明理由的 Rust crate(库) 更新 `Cargo.toml`。
- [ ] T003 [P] 确认 `cargo fmt` 和 `cargo test` 验证命令。

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 完成任何用户故事开始前都必须存在的核心基础设施。

**Critical(关键要求)**: 本阶段完成前，任何用户故事实现都不能开始。

基础任务示例，请按项目情况调整：

- [ ] T004 在 `src/` 中定义模块所有权和可见性边界。
- [ ] T005 [P] 定义 supervision lifecycle contract(监督生命周期契约) 类型或文档。
- [ ] T006 [P] 配置 diagnostics(诊断)、结构化错误或日志路径。
- [ ] T007 创建所有故事都会依赖的共享领域类型或运行时类型。
- [ ] T008 配置错误处理、取消边界和关闭边界。
- [ ] T009 在需要时设置环境或配置管理。

**Checkpoint(检查点)**: 基础已经可用，用户故事实现可以开始。

---

## Phase 3(阶段三): User Story 1(用户故事一) - [Title(标题)] (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: [简述这个故事交付什么。]

**Independent Test(独立测试)**: [说明怎样单独验证这个故事。]

### Tests for User Story 1(用户故事一的测试)

> **NOTE(说明): 必须先写这些测试，并确认它们在实现前失败。**

- [ ] T010 [P] [US1] 在 `tests/[name].rs` 中添加 [行为] 的契约测试或回归测试。
- [ ] T011 [P] [US1] 在 `src/[module].rs` 中添加 [模块行为] 的单元测试。

### Implementation for User Story 1(用户故事一的实现)

- [ ] T012 [P] [US1] 在 `src/[module].rs` 中创建或更新 [运行时类型或领域类型]。
- [ ] T013 [P] [US1] 在 `src/[module].rs` 中创建或更新 [支撑类型]。
- [ ] T014 [US1] 在 `src/[module].rs` 中实现 [监督行为或服务行为]。
- [ ] T015 [US1] 如果需要，在 `src/main.rs` 中连接 CLI(命令行) 或运行时入口。
- [ ] T016 [US1] 添加校验和结构化错误处理。
- [ ] T017 [US1] 为生命周期和失败结果添加诊断。

**Checkpoint(检查点)**: 用户故事一已经完整可用，并且可以独立测试。

---

## Phase 4(阶段四): User Story 2(用户故事二) - [Title(标题)] (Priority(优先级): P2)

**Goal(目标)**: [简述这个故事交付什么。]

**Independent Test(独立测试)**: [说明怎样单独验证这个故事。]

### Tests for User Story 2(用户故事二的测试)

- [ ] T018 [P] [US2] 在 `tests/[name].rs` 中添加 [行为] 的契约测试或回归测试。
- [ ] T019 [P] [US2] 在 `src/[module].rs` 中添加 [模块行为] 的单元测试。

### Implementation for User Story 2(用户故事二的实现)

- [ ] T020 [P] [US2] 在 `src/[module].rs` 中创建或更新 [运行时类型或领域类型]。
- [ ] T021 [US2] 在 `src/[module].rs` 中实现 [监督行为或服务行为]。
- [ ] T022 [US2] 在 `src/[module].rs` 中实现 [CLI(命令行) 或运行时入口]。
- [ ] T023 [US2] 在需要时集成 User Story 1(用户故事一) 组件。

**Checkpoint(检查点)**: 用户故事一和用户故事二都可以独立工作。

---

## Phase 5(阶段五): User Story 3(用户故事三) - [Title(标题)] (Priority(优先级): P3)

**Goal(目标)**: [简述这个故事交付什么。]

**Independent Test(独立测试)**: [说明怎样单独验证这个故事。]

### Tests for User Story 3(用户故事三的测试)

- [ ] T024 [P] [US3] 在 `tests/[name].rs` 中添加 [行为] 的契约测试或回归测试。
- [ ] T025 [P] [US3] 在 `src/[module].rs` 中添加 [模块行为] 的单元测试。

### Implementation for User Story 3(用户故事三的实现)

- [ ] T026 [P] [US3] 在 `src/[module].rs` 中创建或更新 [运行时类型或领域类型]。
- [ ] T027 [US3] 在 `src/[module].rs` 中实现 [监督行为或服务行为]。
- [ ] T028 [US3] 在 `src/[module].rs` 中实现 [CLI(命令行) 或运行时入口]。

**Checkpoint(检查点)**: 所有用户故事都可以独立工作。

---

[按需要继续添加用户故事阶段，并保持同一结构。]

---

## Phase N(最终阶段): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 完成影响多个用户故事的改进。

- [ ] TXXX [P] 更新 `docs/` 中的文档。
- [ ] TXXX 清理代码并完成重构。
- [ ] TXXX 优化跨故事性能。
- [ ] TXXX [P] 在 `src/` 或 `tests/` 中增加单元测试或集成测试。
- [ ] TXXX 加固安全行为。
- [ ] TXXX 运行 `quickstart.md` 中的验证。
- [ ] TXXX 运行 `cargo fmt`。
- [ ] TXXX 运行 `cargo test`。

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Setup(阶段一)**: 没有依赖，可以立即开始。
- **Foundational(阶段二)**: 依赖 Setup(阶段一) 完成，并阻塞所有用户故事。
- **User Stories(用户故事阶段)**: 全部依赖 Foundational(阶段二) 完成。之后可以按人员情况并行，也可以按 P1、P2、P3 顺序执行。
- **Polish(收尾阶段)**: 依赖所有选定用户故事完成。

### User Story Dependencies(用户故事依赖)

- **User Story 1(用户故事一，P1)**: Foundational(阶段二) 完成后可以开始，不依赖其他故事。
- **User Story 2(用户故事二，P2)**: Foundational(阶段二) 完成后可以开始，可以集成 US1，但必须仍能独立测试。
- **User Story 3(用户故事三，P3)**: Foundational(阶段二) 完成后可以开始，可以集成 US1 或 US2，但必须仍能独立测试。

### Within Each User Story(每个用户故事内部)

- 行为变化的测试必须先写，并且必须在实现前失败。
- 先写模型，再写服务。
- 先写服务，再写端点。
- 先写核心实现，再写集成。
- 完成一个故事后，再进入下一个优先级。

### Parallel Opportunities(并行机会)

- 所有标记 [P] 的 Setup(阶段一) 任务可以并行。
- 所有标记 [P] 的 Foundational(阶段二) 任务可以在阶段内部并行。
- Foundational(阶段二) 完成后，不同用户故事可以由不同人员并行。
- 同一用户故事中标记 [P] 的测试可以并行。
- 同一用户故事中标记 [P] 的模型任务可以并行。

---

## Parallel Example(并行示例): User Story 1(用户故事一)

```bash
# 同时启动用户故事一的测试任务：
Task(任务): "在 tests/[name].rs 中为 [behavior] 添加契约测试或回归测试"
Task(任务): "在 src/[module].rs 中为 [module behavior] 添加单元测试"

# 同时启动用户故事一的独立模块工作：
Task(任务): "在 src/[module].rs 中创建或更新 [runtime/domain type]"
Task(任务): "在 src/[module].rs 中创建或更新 [supporting type]"
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 Phase 1(阶段一): Setup(初始化)。
2. 完成 Phase 2(阶段二): Foundational(基础)，该阶段会阻塞所有故事。
3. 完成 Phase 3(阶段三): User Story 1(用户故事一)。
4. 停止并验证 User Story 1(用户故事一)。
5. 在可用时进行演示或交付。

### Incremental Delivery(增量交付)

1. 完成 Setup(初始化) 和 Foundational(基础)。
2. 增加 User Story 1(用户故事一)，独立测试后交付 MVP(最小可用产品)。
3. 增加 User Story 2(用户故事二)，独立测试后交付。
4. 增加 User Story 3(用户故事三)，独立测试后交付。
5. 每个故事都必须增加价值，并且不得破坏已经完成的故事。

### Parallel Team Strategy(并行团队策略)

1. 团队先一起完成 Setup(初始化) 和 Foundational(基础)。
2. Foundational(基础) 完成后，开发者可以按故事分工。
3. 每个故事必须独立完成并集成。

---

## Notes(说明)

- [P] 表示任务修改不同文件，并且没有依赖。
- [Story] 标签把任务映射到具体用户故事，方便追踪。
- 每个用户故事都必须能独立完成和独立测试。
- 实现前必须确认测试失败。
- 每个任务或逻辑组完成后可以提交。
- 可以在任何检查点停止并验证该故事。
- 避免模糊任务、同文件冲突，以及破坏故事独立性的跨故事依赖。
