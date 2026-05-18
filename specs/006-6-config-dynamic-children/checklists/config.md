# Config & Dynamic Children Requirements Quality Checklist(配置与动态子任务需求质量检查清单)

**Purpose(目的)**: 验证 `006-6-config-dynamic-children` 功能规格中 YAML 配置声明、动态 add_child 流水线、审计对账和拓扑 DAG 需求的质量、完整性与可度量性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: US1(拓扑声明) + US2(动态 add_child 全流水线) + US3(变更对账与恢复), 全部 3 个用户故事
**Depth(深度)**: Strict(严格 release gate)
**Audience(受众)**: QA(质量验收)
**Gates(关口)**: YAML schema 完备性, 动态事务原子性, 审计对账一致性

---

## Requirement Completeness(需求完整性)

- [x] CHK001 — FR-001 要求 YAML 必须声明 children, dependencies, health, readiness, resource limits, command permissions, environment, secrets reference, restart budgets 共 9 类字段。该字段清单是否在 spec 中逐一定义了数据类型、必填/可选约束和默认值？[Gap, Spec §FR-001]
  - data-model.md ChildDeclaration 表逐一定义了每类的数据类型、必填/可选、默认值 ✓
- [x] CHK002 — FR-002 要求 add_child 五步（解析→校验→注册→拉起→审计持久化）为一桩事务。该事务的 ACID 属性（原子性、一致性、隔离性、持久性）分别在哪些条款中定义？缺少定义的属性是否意味着未要求？[Gap, Spec §FR-002]
  - 原子性: research R003 临时登记 + commit/rollback ✓
  - 一致性: data-model 状态迁移 Parsed→Committed/Compensated ✓
  - 隔离性: research R005 tokio::sync::Mutex ✓
  - 持久性: ❌ 审计写入内存环形缓冲区(data-model 和 research 均无磁盘级持久化保证), 重启后通过快照哈希重建实现逻辑恢复, 但未定义 WAL 或 fsync 级别的持久性. 考虑在 US3 中补充"至少 CompensatingRecord 必须在事务提交前持久化"的要求, 或明确接受非持久性作为设计权衡.
- [x] CHK003 — US3 要求"重启后仍能复盘"。audit 卷上的快照哈希与 SupervisorSpec 导出一致——该一致性检查的触发条件（每次启动？定时？按需？）是否在规格中写明？[Gap, Spec §US3]
  - data-model 恢复流程图只覆盖 compensating records 枚举; 全量一致性检查仅在重启时通过 CompensatingRecord 比对触发, 未定义定时或按需检查
  - 当前设计隐含假设: 审计条目中每个 CompensatingRecord 都带有 declaration_hash, 恢复时只需比对 pending 记录; 已 committed 的记录不再检查. 这意味着 committed 状态的审计条目损坏不会被发现. 考虑增加启动时全量审计校验的可选开关.
- [x] CHK004 — Edge Cases 定义了两类 secrets reference 失败（validation_failed vs runtime_secret_miss）。是否还有其他秘密相关失败模式（如密钥过期、权限不足）需要区分？[Completeness, Spec §Edge Cases]
  - 两级粒度合理：密钥过期/权限不足可归入 runtime_secret_miss 子分类 ✓
- [x] CHK005 — Key Entities 只列出了 ChildDeclaration 一个实体。US2 涉及"事务""补偿段落""拓扑视图"，US3 涉及"快照哈希""审计流水"——这些概念是否也需要作为 Key Entities 定义？[Completeness, Spec §Key Entities]
  - data-model.md 定义了 PendingChild、CompensatingRecord、Phase，但 spec 的 Key Entities 节未引用
  - 建议: spec.md Key Entities 节至少补充 PendingChild、Phase 和 CompensatingRecord 三个实体的一行摘要, 以保持 spec 的可读性而不必跳转 data-model.md.
- [x] CHK006 — FR-002 要求"哪一步失手要么整体退回调用前的拓扑视图, 要么写上 compensating 段落"。compensating 段落的数据结构（字段、位置、生命周期）是否在规格中定义？[Gap, Spec §FR-002]
  - data-model.md CompensatingRecord (7字段) ✓, research R008 ✓, contracts add-child-api TransactionCompensated ✓

## Requirement Clarity(需求清晰度)

- [x] CHK007 — US1 验收场景要求"启动序列必须遵循拓扑排序输出"。拓扑排序的算法（Kahn? DFS?）和依赖边方向（A<-B 表示 B 依赖 A？）是否明确？[Clarity, Spec §US1]
  - 算法: research R002 Kahn ✓; 依赖边方向: `A<-B` 有歧义 — 规格中 `A<-B` 未定义箭头含义. 按 data-model 规则 2"dependencies 中的每个子任务名必须在同层 children 中存在", 合理推断 dependencies = [A] 表示"依赖于 A 先启动", 因此拓扑序中 A 在 B 之前. 建议在 spec 或 contracts 中明确文档化: `dependencies: ["A"]` 表示"B 依赖 A, A 必须在 B 之前启动".
- [x] CHK008 — US2 验收场景要求"立刻查询拓扑 API 时必须看见 starting 或 running"。APIs 的返回值格式（JSON 结构、状态枚举值）和查询超时边界是否定义？[Clarity, Spec §US2]
  - ❌ 返回值格式、超时边界均未定义
  - add-child-api.md 定义了 AddChildResponse 的 Rust 结构体, 但未定义拓扑查询 API 的返回值格式. 且 "立刻查询" 未量化——从 add_child 返回 Accepted 到拓扑 API 可见 starting/running 的预期最大延迟未明确. 建议在 spec 或 contracts 中补充: (1) 拓扑查询 API 的响应结构; (2) "立刻" 的量化定义(如 p99 < 50ms).
- [x] CHK009 — SC-002 要求"10_000 次追加随后移除的压力脚本, 注册表漂移计数为 0, audit 缺失条目数为 0"。漂移计数的具体定义（预期条目数 vs 实际条目数？哈希校验？）和测量工具是否在规格中指定？[Clarity, Spec §SC-002]
  - ❌ 漂移计数定义和测量工具未指定
  - research R010 明确指出 remove_child 不在本切片范围, 移除通过测试夹具直接清理. 这意味着 SC-002 的压力脚本无法在本切片内完整验证——缺少 remove_child API 则"追加随后移除"只能是"追加→夹具清理→比对", 而非真实用户操作流. 建议降低 SC-002 在本切片中的 gate 权重, 或在 spec 中明确标注"remove_child 依赖后续切片".
- [x] CHK010 — Edge Cases 中 resource limits 宿主内核不支持时"必须选定 ignore 或 reject_boot, 二者之一写死在默认 YAML schema 注解"。该选择是全局配置还是 per-child 声明？当前 spec 未指定默认值。[Clarity, Spec §Edge Cases]
  - data-model 校验规则 6 选定全局 ignore，但 spec 未明确
  - 建议: 在 spec.md Edge Cases 节直接引用 data-model.md 规则 6, 或复制该决策到 spec 中, 避免读者需要跨文件理解默认行为.
- [x] CHK011 — US3 的"注入断电故障指令到夹具"——断电故障的具体注入方式（kill -9？网络隔离？文件系统只读？）和判定标准（进程退出码？恢复后拓扑比对？）是否在测试计划中定义？[Clarity, Spec §US3]
  - ❌ 注入方式和判定标准未定义
  - data-model 的恢复流程图和 tasks.md T026 使用"通过夹具将 PendingChild.phase 设为 Audited" 模拟断电中间态. 这本质上是状态模拟, 而非真实的进程级断电. 对于 US3 验证目标(重启后补偿机制), 状态模拟已足够; 但 spec 文字中的"注入断电故障指令"与实际测试手法不一致. 建议 spec 明确说明断电故障通过状态模拟实现, 避免对真实断电注入的期望.

## Requirement Consistency(需求一致性)

- [x] CHK012 — FR-001 要求"加载阶段必须拒绝任何违反 schema 的行", 但 US2 允许动态载荷语法合法时 API 返回 accepted。静态加载 vs 动态加载的拒绝标准是否一致？同一 YAML 字段在两种路径下的校验规则是否相同？[Consistency, Spec §FR-001 vs US2]
  - data-model 校验规则 1-7 统一适用于所有路径; 失败处理不同但合理（启动失败 vs 返回错误）✓
- [x] CHK013 — US3 要求"重启后仍能复盘"并且"审计卷上的每一条动态追加尝试都能对上磁盘里的监督规格快照哈希"。但 006-5 的 correlation id 关联标识是运行时概念——审计条目中的快照哈希是否与 correlation id 共存？如果共存，两者的关系是什么？[Consistency, Spec §US3 → specs/006-5]
  - CompensatingRecord 有 transaction_id 但无 correlation_id，两个概念关系未定义 ❌
  - 当前 AuditRecorded 事件已经包含 correlation_id 字段; 但 add_child 事务中的 CompensatingRecord 是独立结构体, 未引用 CorrelationId. 建议在 CompensatingRecord 中补充可选的 correlation_id 字段, 或者在审计条目中通过 transaction_id 间接关联到 006-5 的事件链. 如果选用间接关联, 需要在 spec 中说明关联方式.
- [x] CHK014 — Dependency Note 声明依赖 specs/002-config-schema-support 的对照表。002 切片的 SupervisorSpec 基线是否已包含本切片需要的 resource limits, command permissions, secrets reference 等字段的 schema? 如果缺少，是本切片扩展还是 002 先补充？[Consistency, Spec §Dependency Note → specs/002]
  - research R009: 本切片字段为全新字段，与 002 基线无冲突 ✓
- [x] CHK015 — Edge Cases 的 secrets reference 两级失败打点（validation_failed, runtime_secret_miss）与 FR-002 的"整体退回"要求是否一致？validation 阶段失败是否也算"失手"并触发事务回滚？[Consistency, Spec §Edge Cases vs FR-002]
  - validation_failed 在校验阶段(步骤 2)触发回退 ✓; runtime_secret_miss(vault调用)在五步中位置未定义 ❌
  - 五步流水线(解析→校验→注册→拉起→审计持久化)中, vault 调用属于"拉起"步骤(步骤 4)的子步骤. runtime_secret_miss 应在该步骤触发 compensating. 但 add-child-api.md 的事务边界描述将"启动 child"列为步骤 4, 未提及 vault 调用. 建议在事务边界描述中明确 vault 解密在步骤 4 中, runtime_secret_miss 是该步骤的一种失败路径.

## Acceptance Criteria Quality(验收标准可度量性)

- [x] CHK016 — SC-001 的 "100%" 分母是 golden YAML 的总字段数还是总行数？字段级一致性的比对路径（JSON Pointer? XPath?）是否定义？[Measurability, Spec §SC-001]
  - ❌ 分母未定义; 比对路径 contracts 使用 JSON Pointer ✓
  - 基于 T010 的测试设计, 比对策略是: 解析树(ChildDeclaration)序列化为 JSON → 运行时注册表(ChildSpec)序列化为 JSON → 逐字段比对. 但 ChildDeclaration 有 12 个字段, ChildSpec 有更多字段(含 factory、tags 等 ChildDeclaration 没有的字段). 两个类型的 JSON 结构不同, 直接 JSON 逐字段比对不可行. 建议 T010 明确比对映射: 声明一个"字段级映射表"说明 ChildDeclaration 的每个字段对应 ChildSpec 的哪个字段, 只比对映射表中的字段.
- [x] CHK017 — SC-002 的 "10_000 次" 和 "漂移计数为 0" 是否有明确的测量窗口（并发追加？顺序追加？移除间隔？）？[Measurability, Spec §SC-002]
  - ❌ 测量窗口未定义; 移除通过夹具(非真实 API) research R010
  - 基于 research R010 的决策, SC-002 在本切片内应修正为: "10_000 次顺序追加(无并发), 每次追加后通过夹具模拟移除, 注册表漂移计数为 0, audit 条目数与追加次数一致". 建议在 spec.md SC-002 中补充此修正说明.
- [x] CHK018 — US1 的 Independent Test "golden YAML 比对解析树导出与运行时注册表导出差异计数必须为 0"——解析树导出和注册表导出的格式是否相同（JSON? YAML? 内部结构体 dump?）？格式不同时如何逐字段比对？[Measurability, Spec §US1]
  - ❌ 导出格式(ChildDeclaration vs ChildSpec)不同，逐字段映射路径未定义
  - 与 CHK016 相同问题. ChildDeclaration 含 name, kind, criticality, restart_policy, dependencies, health_check, readiness, resource_limits, command_permissions, environment, secrets, restart_budget 共 12 个字段. ChildSpec 含 id(由 name 生成), name, kind, restart_policy, shutdown_policy, health_policy, readiness_policy, backoff_policy, dependencies, tags, criticality 等字段. 建议创建明确的字段映射表(如在 contracts/ 或 tasks.md 中), 使得 T010 的测试实现有据可依.
- [x] CHK019 — US2 的 Independent Test "伪造非法密钥引用调用 add_child API"——伪造输入的枚举清单（非法语法、不存在的密钥名、权限不足）是否在测试计划中列出？[Measurability, Spec §US2]
  - ❌ 测试用例枚举未在测试计划中列出
  - tasks.md T019 只描述了一个用例(非法密钥占位符语法). 根据 Edge Cases, 还应该覆盖: (a) `${}` 空占位符, (b) `${invalid!char}` 非法字符, (c) EnvVar 同时设置 value 和 secret_ref. 建议在 tasks.md T019 中扩展测试用例枚举.

## Scenario Coverage(场景覆盖)

- [x] CHK020 — US1 只覆盖了"依赖 DAG 合法"场景。当 DAG 包含环路时规格要求"拒绝进入 running"。但环路检测的算法和错误消息格式是否定义？[Coverage, Spec §US1]
  - research R002: Kahn ✓; contracts add-child-api: DependencyCycle { nodes } ✓
- [x] CHK021 — US2 覆盖了"载荷非法"和"载荷合法"两条路径。但 add_child 在 supervisor 正在关闭时、在 child 数达到上限时、在磁盘空间不足时的行为是否也在范围内？[Coverage, Spec §US2]
  - ❌ 三种边界均未定义; plan.md 有 1000 child 上限但未定义超限行为
  - 建议在 spec 或 contracts 中明确: (1) supervisor 关闭中时 add_child 返回 `Err(SupervisorShuttingDown)`; (2) child 数 ≥ 1000 时返回 `Err(ChildLimitExceeded { max: 1000, current: N })`; (3) 磁盘空间不足(审计写入失败)时返回 `Err(AuditStorageFailure)`. 也建议考虑是否将这些错误变体加入 AddChildError 枚举.
- [x] CHK022 — US3 覆盖了"断电故障"一种恢复场景。其他中间态故障（如 audit 写入成功但注册失败、注册成功但拉起失败）是否也需要 compensating 逻辑？[Coverage, Spec §US3]
  - data-model 状态迁移图覆盖所有步骤的补偿 ✓, 但 spec US3 文字只写了断电
  - 建议在 spec.md US3 中补充一句说明: "compensating 逻辑适用于 add_child 五步中任一步失败, 断电故障只是其中一种触发方式. 其他失败路径(如校验失败、注册失败、拉起失败)同样触发补偿流程." 以消除读者可能认为补偿逻辑仅针对断电的误解.
- [x] CHK023 — remove_child（与 add_child 对应的移除操作）是否在本切片范围内？如果不在，SR-002 的"追加随后移除"压力脚本如何实现？[Coverage, Spec §SC-002]
  - research R010: remove_child 不在本切片范围; 移除使用测试夹具直接清理 ✓

## Edge Case Coverage(边界条件覆盖)

- [x] CHK024 — 当 YAML 文件包含循环 anchors 或别名（YAML 的 &/\* 语法）时，解析器是否会陷入无限递归？规格是否定义了对 YAML 超集/子集的选择？[Edge Case, Spec §FR-001]
  - ❌ serde_yaml 默认展开 anchors, 循环 anchor 错误能否捕获未定义
  - serde_yaml 0.9 对循环 anchor 的行为是: 反序列化时如果遇到递归结构会导致栈溢出 panic. 这不在规格的控制范围内. 建议在 spec 中明确: "YAML 输入使用 serde_yaml 的默认行为解析, 不支持循环 anchor; 循环 anchor 导致解析 panic, 视为加载失败." 或者考虑在加载前增加 serde_yaml::Value 层的深度限制检查.
- [x] CHK025 — add_child 同时发起多个并发请求时，事务隔离性如何保证？两个请求同时修改同一 child 的配置是否允许？[Edge Case, Spec §FR-002]
  - research R005: tokio::sync::Mutex<ConfigState> 互斥; data-model 规则7: 事务不可重入 ✓
- [x] CHK026 — audit 卷写满时的行为：新的 add_child 审计条目是阻塞等待、覆盖最旧条目、还是返回磁盘满错误？[Edge Case, Spec §US3]
  - ❌ audit 使用环形缓冲区(plan.md), 写满时行为未定义
  - 审计通道重用 `src/event/payload.rs` 的事件系统和 `src/journal/ring.rs` 的环形缓冲区. 环形缓冲区默认行为是覆盖最旧条目. 但 add_child 事务要求审计条目不丢失(FR-002 的持久化语义). 覆盖最旧条目意味着先前的审计记录可能丢失, 无法满足 SC-002 的"审计缺失条目数为 0". 建议明确: (a) add_child 的 CompensatingRecord 使用独立(或更大容量)的审计通道, 或 (b) 环形缓冲区写满时阻塞直到空间可用, 或 (c) 在设计层面接受容量上限并规定"审计通道容量必须大于同时存活的 child 总数的两倍".
- [x] CHK027 — 快照哈希的哈希算法（SHA-256? BLAKE3?）和计算范围（整个 SupervisorSpec? 仅 children 子集?）是否在规格中定义？不同版本之间哈希算法升级是否允许？[Edge Case, Spec §US3]
  - research R004: SHA-256, SupervisorSpec JSON 序列化 ✓; 哈希升级未讨论

## Non-Functional Requirements(非功能需求)

- [x] CHK028 — YAML 加载的性能预算是否定义？包含 1000 个 child 的配置文件解析延迟 p99 上限是否量化？[NFR, Gap]
  - plan.md: 1000 child p99 < 50ms ✓
- [x] CHK029 — add_child API 的 p99 延迟上限是否定义？该延迟是否包含解析→校验→注册→拉起→审计持久化全部五步？[NFR, Gap]
  - plan.md: 单次全流水线 p99 < 10ms(含审计持久化) ✓
- [x] CHK030 — 拓扑 DAG 的规模上限（最大 child 数、最大依赖深度）是否在规格中定义？超过上限时的行为是拒绝、告警还是性能降级？[NFR, Gap]
  - plan.md: max 1000 child, 10 层深度 ✓; 超限行为未定义 ❌
  - 与 CHK021 关联. 建议在 spec 中明确超过上限时 `add_child` 返回 `Err(ChildLimitExceeded)`, 静态加载时 YAML 加载失败并给出结构化错误.

## Dependencies & Assumptions(依赖与假设)

- [x] CHK031 — 规格强依赖 `specs/002-config-schema-support/spec.md` 的对照表。002 的对照表当前定义了 SupervisorSpec 的哪些字段？本切片新增的 9 类字段是否与 002 的字段无冲突？[Dependency, Spec §Dependency Note]
  - research R009 确认无冲突; 但 spec Dependency Note 缺少正式引用 ❌
  - 建议在 spec.md Dependency Note 中补充: "本切片新增字段列表: resource_limits, command_permissions, secrets, environment (ChildDeclaration 级). 经 research R009 确认, 与 002 切片 SupervisorSpec 基线(SupervisionStrategy, RestartLimit, ShutdownPolicy, HealthPolicy, BackoffPolicy)无字段名冲突."
- [x] CHK032 — 假设"secrets reference 的真正解密下发由宿主 vault 适配层完成"。监督器是否提供默认的 no-op vault 实现，使得未配置 vault 时 add_child 不 panic？[Assumption, Spec §Assumptions]
  - 设计上仅做语法校验(data-model 规则4), 无 vault 时不会 panic; 但 spec 未明确说明
  - 建议在 spec.md Assumptions 节补充: "监督器不提供 vault 实现. 未配置 vault 时, secrets 仅做语法校验并在运行时将 `${SECRET_NAME}` 占位符原样传递给 child 环境变量. add_child 不会因 vault 缺失而 panic."
- [x] CHK033 — 假设 secrets 占位符语法"合法但 vault 离线"与"密钥缺失"两级区分。该区分是否需要在审计条目中以不同枚举值呈现？审计的 schema 是否定义？[Assumption, Spec §Edge Cases]
  - ❌ CompensatingRecord.error 是自由字符串而非枚举; 审计条目 schema 未定义
  - data-model 中 CompensatingRecord.error 是 `Option<String>`, 即自由文本. 如果需要在审计中区分两级失败, 建议将 error 改为枚举类型或至少定义一组标准化的错误字符串常量. 同时, 审计条目(CompensatingRecord)的 schema 仅在 data-model 中定义, spec 未引用. 建议在 contracts/ 中补充审计条目 schema 契约.

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK034 — 规格中使用 `runtime_secret_miss(枚举示例)` 标注了"示例"。该枚举名是否已在计划阶段冻结？如果不是，发布门禁是否会因字段名停留在"示例"状态而阻塞？[Ambiguity, Spec §Edge Cases]
  - ❌ 字段名仍在"示例"状态; plan.md 未提及冻结状态; release gate 风险
  - 建议: (1) 将 `runtime_secret_miss` 从"示例"状态提升为冻结名称; (2) 在 spec.md 中删除"(枚举示例)"标注; (3) 检查是否有任何契约文件引用了此枚举名(如 contracts/add-child-api.md 的 AddChildError 枚举), 确保命名一致.
- [x] CHK035 — FR-002 要求"整体退回调用前的拓扑视图"——"调用前的拓扑视图"是指内存中的运行时状态还是磁盘上的持久化快照？如果两者都被修改，回退目标是谁？[Ambiguity, Spec §FR-002]
  - data-model: pending_additions rollback → 回退内存状态; YAML 文件不被修改; spec 文字有歧义
  - 建议在 spec.md FR-002 中明确: "整体退回调用前的内存拓扑视图. YAML 文件不被修改. 回退不涉及磁盘 I/O. 已写入审计的失败记录(CompensatingRecord)保留在审计通道中供复盘."
- [x] CHK036 — US3 的"compensating(补偿) 段落写明未完成事务编号"——补偿段落的存储位置（内存? audit 卷? 独立 WAL?）和生命周期（何时清理?）是否定义？[Ambiguity, Spec §US3]
  - research R008: 审计通道 ✓; 清理时机未定义 ❌
  - 基于 research R003, 审计写入环形缓冲区, 无独立 WAL. 清理时机: 当 `state == "compensated"` 的 CompensatingRecord 被后续审计条目覆盖(环形缓冲区)即"自动清理". 但 SC-002 要求审计缺失条目数为 0, 这意味着环形缓冲区容量必须大于 10_000 加上并发条目数. 建议在 spec 中明确: "compensating 记录在审计通道中保留, 生命周期与审计条目相同(环形缓冲区覆盖策略). 容量配置应保证 10_000 次追加的审计记录不被覆盖."

## Constitution Compliance(宪章合规)

- [x] CHK037 — Constitution Alignment 要求"扩大配置驱动启动覆盖面, 必须与 006-3 关停语义以及并发承认条款联合验收"。该联合验收是否在测试计划中体现？是否有跨切片集成测试？[Compliance, Spec §Constitution Alignment]
  - ❌ spec 和 plan 均未提及 006-3 联合测试计划
  - 建议在 plan.md 或 spec.md 中补充: (1) 列出需要与 006-3 联合验证的场景(如: add_child 过程中 supervisor 收到关停信号, child 拉起后立即被关停); (2) 创建跨切片的集成测试任务, 或在现有测试中增加 006-3 联合场景.
- [x] CHK038 — Module ownership 要求"spec 解析层与 config 校验层目录分层不得塌缩成单一 god module"。当前模块结构（`src/spec/` 和 `src/config/`？）是否已在项目结构中体现分层？[Compliance, Spec §Module ownership]
  - plan.md: src/config/ + src/spec/ + src/tree/ 三层分离 ✓; Constitution Check 确认 ✓
- [x] CHK039 — Diagnostics 要求"任意 YAML 拒绝必须打印 field_path 与人读 hint"。field_path 的格式（JSON Pointer? YAML path?）是否在规格或契约中统一？[Compliance, Spec §Diagnostics]
  - contracts/child-declaration-schema.md: JSON Pointer ✓; contracts/add-child-api.md: JSON Pointer ✓
- [x] CHK040 — 写作规范禁止"将流水线步骤写成口语口令却不编号"。US2 的五步流水线（解析→校验→注册→拉起→审计持久化）每次被引用时是否使用一致的编号方式？[Compliance, Spec §Chinese Writing]
  - FR-002: 五步列出 ✓; data-model Phase 枚举: 一致 ✓; contracts 事务边界 1-5 ✓

## Notes(说明)

- 完成检查项后使用 `[x]` 标记.
- 评论或发现可以直接写在相关检查项下.
- 需要时链接相关资源或文档.
- 检查项必须按顺序编号, 方便引用.
