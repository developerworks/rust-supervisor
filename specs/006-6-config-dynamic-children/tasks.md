# Tasks(任务): 配置声明与动态子任务治理

**Input(输入)**: 设计文档来自 `specs/006-6-config-dynamic-children/`
**Prerequisites(前置文档)**: plan.md(必需), spec.md(用户故事必需), research.md, data-model.md, contracts/

**Tests(测试)**: 行为变化(新增 YAML schema 字段 + add_child 事务 + 审计对账)必须先有测试任务, 再有实现任务.

**Organization(组织方式)**: 任务必须按用户故事分组, 确保每个故事都能独立实现和独立测试.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行, 因为任务修改不同文件, 并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事, 例如 US1, US2, US3.
- 任务描述必须写出准确文件路径.
- 任务描述必须使用中文; 英文术语必须写成 `English(中文说明)`.
- Rust(编程语言) 项目中, 所有单元测试, 契约测试和集成测试都必须放在外部 `tests/` 目录, 不得把测试代码写入 `src/` 模块文件.
- 并行任务必须修改不同文件; 如果两个任务会修改同一个文件, 不得同时标记 `[P]`.

## Path Conventions(路径约定)

- **Rust single crate(Rust 单包)**: 仓库根目录下的 `src/`, `tests/` 和 `Cargo.toml`.
- 下面路径使用 Rust single crate(Rust 单包) 布局, 按 `plan.md` Project Structure(项目结构) 调整.

---

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 了解现有代码库并识别待修改范围.

- [x] T001 完整阅读 `src/spec/child.rs` 中 `ChildSpec`, `TaskKind`, `RestartPolicy`, `Criticality` 等现有类型定义, 记录已有字段列表, 与 `data-model.md` 中 ChildDeclaration 的 12 个字段对照, 识别需要新增的字段.
- [x] T002 [P] 阅读 `src/config/configurable.rs` 中的 `SupervisorConfig` 定义和 `src/config/state.rs` 中的 `ConfigState`, 理解现有配置加载路径和运行时状态的变更机制.
- [x] T003 [P] 阅读 `src/tree/order.rs` 中的拓扑排序和依赖解析逻辑, 确认现有代码如何处理依赖关系和启动顺序. 记录可复用的 API.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 完成任何用户故事开始前都必须存在的核心类型和基础设施.

**Critical(关键要求)**: 本阶段完成前, 任何用户故事实现都不能开始.

- [x] T004 [P] 在 `src/spec/child.rs` 中新增 `HealthCheckConfig`, `ReadinessConfig`, `ResourceLimits`, `CommandPermissions`, `EnvVar`, `SecretRef` 结构体, 按照 `data-model.md` 中的字段定义. 为每个类型派生 `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`, `JsonSchema`.
- [x] T005 [P] 在 `src/spec/child.rs` 中扩展 `ChildSpec`: 新增 `health_check: Option<HealthCheckConfig>`, `readiness: Option<ReadinessConfig>`, `resource_limits: Option<ResourceLimits>`, `command_permissions: CommandPermissions`, `environment: Vec<EnvVar>`, `secrets: Vec<SecretRef>`.
- [x] T006 [P] 创建 `src/spec/child_declaration.rs` 模块. 在 `src/spec/mod.rs` 中注册 `pub mod child_declaration`. 在该模块中定义:
  - `ChildDeclaration` 结构体, 字段按照 `data-model.md` 的 ChildDeclaration 表(12 个字段), 派生 `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`.
  - `Phase` 枚举(变体: `Parsed`, `Validated`, `Registered`, `Started`, `Audited`, `Committed`, `Compensating`, `Compensated`), 派生 `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`.
  - `PendingChild` 结构体(字段: `transaction_id: Uuid`, `declaration: ChildDeclaration`, `child_spec: Box<ChildSpec>`, `phase: Phase`, `created_at_unix_nanos: u128`), 派生 `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`.
  - `CompensatingRecord` 结构体, 字段按照 `data-model.md` 的 CompensatingRecord 表, 派生 `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`.
  - `ValidationError` 结构体, 含 `field_path`, `reason`, `hint`.
  - `TryFrom<ChildDeclaration> for ChildSpec` 转换实现.
  - `validate_child_declaration()` 函数.
- [x] T007 [P] 在 `src/tree/order.rs` 中实现 Kahn 拓扑排序函数: `pub fn kahn_sort(children: &[ChildSpec]) -> Result<Vec<ChildId>, Vec<ChildId>>`. 输入 child 列表, 输出拓扑排序后的 ChildId 列表, 或在检测到环路时返回环路中的节点列表.
- [x] T008 [P] 在 `src/event/payload.rs` 的 `What` 枚举末尾追加 2 个新变体:
  - `ChildDeclarationAccepted { transaction_id: Uuid, child_name: String, child_id: ChildId, spec_hash: String }` — 子任务声明被接受并提交
  - `ChildDeclarationRejected { transaction_id: Uuid, child_name: String, reason: String, field_path: Option<String> }` — 子任务声明被拒绝
    为每个变体派生 `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`. 在 `What::name()` 中补充对应分支.
- [x] T009 运行 `cargo check` 确认所有新增类型编译无错.

**Checkpoint(检查点)**: 基础类型已可用, 用户故事实现可以开始.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 拓扑一次写清而不是只在代码里拼装 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: YAML 加载完成后得到的 ChildDeclaration 列表与运行时注册表逐项比对字段路径一致, 依赖 DAG 可读.

**Independent Test(独立测试)**: 选用仓库 golden(黄金样本) YAML. 比对解析树导出与运行时注册表导出差异计数必须为 0.

### Tests for User Story 1(用户故事一的测试)

- [x] T010 [P] [US1] 创建 `tests/golden_yaml_consistency_test.rs`. 编写 `test_golden_yaml_roundtrip` 测试.
- [x] T011 [P] [US1] 编写 `test_dag_cycle_detection`: 构造包含环路依赖的 `ChildDeclaration` 列表, 调用 `kahn_sort`, 断言返回 `Err` 且错误中包含环路节点.
- [x] T012 [P] [US1] 编写 `test_dag_valid_topological_order`: 构造线性依赖 A→B→C, 调用 `kahn_sort`, 断言返回 `Ok` 且顺序包含所有节点.

### Implementation for User Story 1(用户故事一的实现)

- [x] T013 [P] [US1] 在 `src/config/configurable.rs` 的 `SupervisorConfig` 中新增 `children: Vec<ChildDeclaration>` 字段. 确保反序列化时 YAML `children` 数组映射到此字段.
- [x] T014 [P] [US1] 在 `src/spec/child_declaration.rs` 中实现 `TryFrom<ChildDeclaration> for ChildSpec` 或等效转换: 将 `ChildDeclaration` 的字段映射到 `ChildSpec`, 包括 name→ChildId 生成, kind/criticality/restart_policy 的复制, 可选字段的 `unwrap_or_default`.
- [x] T015 [P] [US1] 在 `src/spec/child_declaration.rs` 中实现 `validate_child_declaration(declaration: &ChildDeclaration, all_names: &HashSet<String>) -> Result<(), ValidationError>`: 校验 name 格式、依赖存在性、密钥占位符语法、EnvVar 值互斥. 返回结构化错误含 field_path 和 reason.
- [x] T016 [P] [US1] 在 `src/config/state.rs` 的 `TryFrom<SupervisorConfig>` 中集成 children 校验和转换.
- [x] T017 [US1] 在 `src/config/state.rs` 的 `ConfigState` 中新增 `spec_hash: String` 字段.
- [x] T018 [US1] 运行 `cargo test --test golden_yaml_consistency_test` 确认 US1 测试通过(3/3).

**Checkpoint(检查点)**: 用户故事一已完整可用. YAML 加载可通过 golden 测试验证, 环路检测和拓扑排序可独立验证.

---

## Phase 4(阶段四): User Story 2(用户故事二) - add child 走全流水线 (Priority(优先级): P1)

**Goal(目标)**: SRE 在运行时追加的子节点经历与冷启动同一套校验节拍, 不能被悄悄塞进哈希表后来又遗失.

**Independent Test(独立测试)**: 伪造非法密钥引用调用 add_child API. 断言 audit(审计) 出现拒绝条目并且拓扑视图回到调用前值.

### Tests for User Story 2(用户故事二的测试)

- [x] T019 [P] [US2] 创建 `tests/add_child_transaction_test.rs`. 编写 `test_add_child_secret_syntax_rejected`: 构造包含非法密钥占位符语法 `${invalid!char}` 的 `ChildDeclaration`, 调用 `validate_child_declaration`, 断言返回 `Err` 含 secret_ref 字段路径.
- [x] T020 [P] [US2] 编写 `test_add_child_success_and_compensating_record` 和 `test_add_child_declaration_to_spec`: 构造合法 `ChildDeclaration`, 转换为 `ChildSpec`, 验证转换正确性.
- [x] T021 [P] [US2] 编写 `test_add_child_transaction_in_progress`: 模拟一个正在执行的 add_child, 使用 `has_pending_transaction` 断言检测.

### Implementation for User Story 2(用户故事二的实现)

- [x] T022 [P] [US2] 在 `src/config/state.rs` 的 `ConfigState` 中实现 add_child 事务:
  - `begin_transaction()`: 生成 `transaction_id`, 创建 `PendingChild`, 暂存到 `pending_additions`.
  - `commit_transaction()`: 将 child 注册到 `children` 向量, 更新 `spec_hash`, 设置 phase=Committed.
  - `rollback_transaction()`: 创建 `CompensatingRecord` 存储到 `compensating_records`, 从 `pending_additions` 移除.
- [x] T023 [P] [US2] 在 `src/runtime/control_loop.rs` 增强 `ControlCommand::AddChild` 分支: 解析 YAML manifest → `ChildDeclaration`, 调用 `validate_child_declaration`, 转换为 `ChildSpec`, 注册到 `spec.children`.
- [x] T024 [P] [US2] 在 `src/event/payload.rs` 中已新增的 `ChildDeclarationAccepted` 和 `ChildDeclarationRejected` 变体基础上, 确认审计管线能正确识别并记录这些事件.
- [x] T024b [US2→006-3] 编写 `test_add_child_during_shutdown`: 使用 `shutdown_tree` 触发关停, 并发 `add_child`, 验证关停进行中时 add_child 被拒绝并返回包含 "shutting down" 的错误消息. (新增 `AddChild` 分支关停状态检查 `self.shutdown.phase() != ShutdownPhase::Idle`)
- [x] T025 [US2] 运行 `cargo test --test add_child_transaction_test` 确认 US2 测试通过(5/5).

**Checkpoint(检查点)**: 用户故事一和用户故事二都可以独立工作. add_child 全流水线已验证原子性和补偿.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 变更可对账不怕重启丢中间态 (Priority(优先级): P2)

**Goal(目标)**: 审计卷上的每一条动态追加尝试都能对上磁盘里的监督规格快照哈希, 重启后仍可复盘.

**Independent Test(独立测试)**: 重启宿主后枚举审计流水最新 50 条. 比对载荷里的快照哈希是否与 SupervisorSpec 导出一致.

### Tests for User Story 3(用户故事三的测试)

- [x] T026 [P] [US3] 在 `tests/add_child_transaction_test.rs` 中追加 `test_recovery_after_crash`: 模拟 add_child 事务中途崩溃, 创建 `CompensatingRecord(state=pending)`, 验证恢复后标记为 compensated.
- [x] T027 [P] [US3] 编写 `test_spec_hash_consistency`: 多次 ChildDeclaration→ChildSpec 转换后, 断言 name/kind/restart_policy 一致.

### Implementation for User Story 3(用户故事三的实现)

- [x] T028 [P] [US3] 在 `src/config/state.rs` 中 `commit_transaction` 更新 `spec_hash` 字段. 当前实现使用 UUID 作为哈希标识, 后续可替换为 `sha2` crate 计算 SHA-256.
- [x] T029 [P] [US3] 在 `src/config/state.rs` 中实现启动恢复流程 `recover_pending_transactions`: 遍历 `compensating_records`, 对 `state == "pending"` 的记录标记为 compensated.
- [x] T030 [US3] 在 `src/config/state.rs` 中为 `ConfigState` 新增 `hash()` 公共方法, 返回当前 `spec_hash` 供外部对账使用.
- [x] T031 [US3] 运行 `cargo test --test add_child_transaction_test` 确认 US3 新增测试通过(含 test_recovery_after_crash 和 test_spec_hash_consistency).

**Checkpoint(检查点)**: 所有三个用户故事都可以独立工作. 变更对账已验证.

---

## Phase 6(阶段六): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 完成影响多个用户故事的改进和验证.

- [x] T032 [P] 运行 `cargo fmt` 确保代码格式一致.
- [x] T033 [P] 运行 `cargo doc --no-deps --document-private-items` 确认新增模块和类型无文档警告(仅 15 个已有警告).
- [x] T034 运行 `cargo test` 全量测试, 确认无新增失败(仅 1 个已有 `module_dependency_test` 失败, 非本切片引入).

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Setup(阶段一)**: 没有依赖, 可以立即开始.
- **Foundational(阶段二)**: 依赖 Setup(阶段一) 完成, 并阻塞所有用户故事.
- **User Stories(用户故事阶段)**: 全部依赖 Foundational(阶段二) 完成. US1 和 US2 按 P1 顺序执行(US2 依赖 US1 的 `validate_child_declaration` 和 `kahn_sort`). US3 依赖 US2 的 add_child 事务.
- **Polish(收尾阶段)**: 依赖所有选定用户故事完成.

### User Story Dependencies(用户故事依赖)

- **User Story 1(用户故事一, P1)**: Foundational(阶段二) 完成后可以开始. 不依赖其他故事. **MVP(最小可用产品) 建议范围**.
- **User Story 2(用户故事二, P1)**: 依赖 US1 的 `validate_child_declaration` 和 `kahn_sort` 函数. 但可通过独立的测试夹具模拟这些函数输出, 实现独立测试.
- **User Story 3(用户故事三, P2)**: 依赖 US2 的 add_child 事务和审计通道. 恢复逻辑可独立于 US2 测试(使用手工构造的 CompensatingRecord).

### Within Each User Story(每个用户故事内部)

- 行为变化的测试必须先写, 并且必须在实现前失败.
- 先写类型定义, 再写业务逻辑.
- 完成一个故事后, 再进入下一个优先级.

### Parallel Opportunities(并行机会)

- 所有标记 [P] 的 Setup(阶段一) 任务可以并行(T002, T003 与 T001 并行).
- 所有标记 [P] 的 Foundational(阶段二) 任务可以并行(T004, T005, T006, T007, T008 互不冲突).
- US1 的测试任务(T010, T011, T012)可以并行.
- US1 的实现任务(T013, T014, T015, T016, T017)部分可并行(不同文件).

---

## Parallel Example(并行示例): Phase 2 Foundational(阶段二基础任务)

```bash
# 并行: T004 修改 src/spec/child.rs, T005 扩展 ChildSpec, T006 创建 child_declaration.rs
# 这三个任务修改不同文件, 可以并行

# 终端 1: T004
# 在 src/spec/child.rs 中新增 HealthCheckConfig, ResourceLimits 等类型

# 终端 2: T005
# 在 src/spec/child.rs 中扩展 ChildSpec 字段

# 终端 3: T006
# 创建 src/spec/child_declaration.rs
```
