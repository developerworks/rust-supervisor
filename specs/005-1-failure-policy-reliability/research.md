# Research(研究结论): `005-1` 失败流水线与生产退避

本文承接 `plan.md` 中 Technical Context(技术背景), 冻结 Phase 0(研究阶段) 取舍, 不留 NEEDS CLARIFICATION(需要澄清).

## 1. 流水线是否与现有 `PolicyEngine`(策略引擎) 决策并存

- **Decision(决定)**: 规格里的 **`policy pipeline`(策略流水线)** 写明监督运行时在一次进程结束之后必须先走完的阶段顺序; 现有 **`restart_execution_plan`(重启执行计划)** (见 `src/tree/order.rs`) 继续产出 **`restart scope`(重启范围)** 以及字段快照 **`restart limit`(重启次数限制)**, **`escalation policy`(升级策略)**; **`PolicyEngine`(策略引擎)** 仍产出 **`RestartDecision`(重启决策)**, 但其生效路径须在 **`evaluate budget`(评估预算)** 之后并入熔断结论, **闸门档位**, **`BackoffPolicy`(退避策略)** 给出的等待时长, **然后才能进入 **`execute action`(执行动作)\*\*.
- **Rationale(理由)**: 代码已在 `refresh_restart_limit_for_child` 与 `restart_execution_plan` 之间存在碎片化用法; 规格要求 **`restart limit`** 与 **`escalation policy`** 进入单一可对账链条, 避免 **`execute_restart_decision`** 绕开限额语义.
- **Alternatives considered(曾考虑的备选)**: 完全重写策略引擎为单一巨型函数; 被拒绝, 因为破坏模块边界且迁移风险过高.

## 2. **`MeltdownTracker`(熔断跟踪器)** 与三层 **`scope`(作用域)**

- **Decision(决定)**: 现行 `src/policy/meltdown.rs` 仅区分 **`child`(子任务)** 与 **`supervisor`(监督器)**; 规格 **`FR-002`** 要求 **`group`(分组)** 独立计数桶; 计划在运行时持有三套并行计数状态, 键分别绑定 **`ChildId`(子任务标识)**, **`restart_execution_plan`** 里的 **`group`** 字段稳定键 (若无分组则不计作用域), 以及托管树的 **`supervisor`** 实例边界 (与现行 **`SupervisorPath`(监督路径)** 或等价实例边界一致); 合并规则严格采用 **`protection restrictiveness ladder`(保护从严档位序)**.
- **Rationale(理由)**: **`restart_execution_plan`** 已携带 **`group`** 字段; 事件侧需要对账 **`scopes_triggered`(已触发作用域列表)** 与 **`lead_scope`(主导归因作用域)**.
- **Alternatives considered(曾考虑的备选)**: 把分组熔断折算进子任务计数; 被拒绝, 因为无法用 **`lead_scope`** 诚实地归因到 **`group`**.

## 3. **`TypedSupervisionEvent`(类型化监督事件)** 增量写入路径

- **Decision(决定)**: 新增字段与新增 **`payload`(载荷)** 变体写入 `src/event/payload.rs` 及其 **`serde`(序列化)** 契约, 经 `src/observe/` 管道转发; 字符串 **`broadcast::Sender<String>`** 仅供过渡期诊断, **不得单独作为 **`005-1`** 的结构化验收证据**.
- **Rationale(理由)**: 仓库已有结构化 **`Where`(位置)**, **`CorrelationId`(关联标识)**, **`EventSequence`(事件序号)**; 扩充比并行再造一套 **`DTO`(数据传输对象)** 更符合模块所有权.

## 4. **`BackoffPolicy`(退避策略)**, **`full jitter`(全抖动)**, **`decorrelated jitter`(去相关抖动)**, 时钟与随机源

- **Decision(决定)**: 生产路径使用 **`rand`** crate(库) 提供的可控 RNG(随机数发生器); 测试路径注入 **`StdRng`(标准 RNG)** 种子或 **`tokio`** `pause`/`advance` 时钟 (已有 **`tokio`** dev **`test-util`**); **`cold start budget`(冷启动预算)** 与 **`hot loop detection`(热循环检测)** 阈值读配置或 **`SupervisorSpec`(监督器规格)**, 默认值以满足 **`spec.md`** Assumptions(假设) 秒级稳定触发为准.
- **Rationale(理由)**: **`FR-003`** 明确要求 **`seed`(随机种子)** 与 **`inject clock`(注入时钟)** 双路径可重复.
- **Alternatives considered(曾考虑的备选)**: 引入新 **`crate`** 只做 **`jitter`(抖动)**; 被拒绝, 除非 **`cargo`** 体积或审计明确提出需求 (**Small Increment(小增量)** 闸门).

## 5. 与 `specs/005-2-work-role-defaults/spec.md` 的边界

- **Decision(决定)**: **`005-1`** 交付统一 **`evaluate budget`** 语义与事件字段; **`005-2`** 只替换 **`RoleDefaultPolicy`(角色默认策略包)** 写入 **`decide action`** 的输入, 不得分叉第二条失败旁路.
- **Rationale(理由)**: **`005-2`** Dependency Note(依赖说明) 已写明 **`evaluate budget`** 字段用法一致 **`005-1`**.
