# Feature Specification (功能规格): 配置声明与动态子任务治理

**Feature Branch (功能分支)**: `[006-6-config-dynamic-children]`
**Created (创建日期)**: 2026-05-17
**Updated (更新日期)**: 2026-05-19
**Status (状态)**: Accepted (已接受)
**Input (输入)**: 本规格对应第四序列里程碑: 配置必须支持声明 children(子任务), dependencies(依赖), health(健康检查), readiness(就绪检查), resource limits(资源限制), command permissions(命令权限), environment(环境变量), secrets reference(密钥引用), restart budgets(重启预算). add_child 不能只保存 manifest(清单), 必须解析, 验证, 注册, 启动, 并持久化审计.

## Dependency Note (依赖说明)

与本切片耦合的 ConfigState(配置状态) 与 SupervisorSpec(监督器规格) 基线在 `specs/002-config-schema-support/spec.md` 对照表中描述. 本节扩大语义覆盖面, 并要求动态追加路径也有完整可查台账义务.

**字段冲突检查**(research R009): 002 切片 SupervisorSpec 基线包含 `SupervisionStrategy`, `RestartLimit`, `ShutdownPolicy`, `HealthPolicy`, `BackoffPolicy`. 本切片新增字段(`resource_limits`, `command_permissions`, `secrets`, `environment` 等, 均在 `ChildDeclaration` 级)与 002 基线无字段名冲突.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### User Story 1 (用户故事一) - 拓扑一次写清而不是只在代码里拼装 (Priority (优先级): P1)

platform engineer(平台工程师) 需要 YAML 加载完成后得到的 ChildDeclaration(子任务声明) 列表与运行时注册表 JSON 逐项比对字段路径一致, 依赖 DAG(有向无环图) 可读.

**Why this priority (为什么是这个优先级)**: 拓扑只靠内存拼装会在重启后失真.

**Independent Test (独立测试)**: 选用仓库 golden(黄金样本) YAML. 比对解析树导出与运行时注册表导出差异计数必须为 0.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 配置写明三层依赖 A<-B<-C, **When (当)** 监督器加载文件, **Then (则)** 启动序列必须遵循拓扑排序输出. 一旦发现环路必须在 audit(审计) 记下字段路径并拒绝进入 running(运行中).

### User Story 2 (用户故事二) - add child 走全流水线 (Priority (优先级): P1)

SRE(站点可靠性工程师) 需要在运行时追加的子节点经历与冷启动同一套校验节拍, 不能被悄悄塞进哈希表后来又遗失.

**Why this priority (为什么是这个优先级)**: 半截注册会把资源配置与安全边界撕开裂缝.

**Independent Test (独立测试)**: 伪造非法密钥引用调用 add child(追加子任务) API. 断言 audit(审计) 出现拒绝条目并且拓扑视图回到调用前值.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 动态载荷含有非法密钥占位符语法, **When (当)** 调用 add child(追加子任务) RPC, **Then (则)** 必须返回 structured error(结构化错误), 并且在 audit(审计) 写入携带操作者摘要的失败记录.
2. **Given (假设)** 动态载荷语法合法, **When (当)** API 返回 accepted, **Then (则)** 在 50ms 内查询拓扑 API 时必须看见 `starting`(启动中)或 `running`(运行中)(p99 < 50ms, 见 plan.md NFR), 且 `resource_limits`(资源限制)字段与载荷字面一致, 审计条目编号可被二次检索.

   > **拓扑查询 API 返回值格式**: `{ "children": [{ "name": "...", "state": "starting|running", "resource_limits": {...}, ... }] }`. 完整结构待后续切片定义.

### User Story 3 (用户故事三) - 变更可对账不怕重启丢中间态 (Priority (优先级): P2)

合规审计员需要 audit(审计) 卷上的每一条动态追加尝试都能对上磁盘里的监督规格快照哈希, 重启机器后仍能复盘.

**Why this priority (为什么是这个优先级)**: 审计链路断层会直接否决采购验收.

**Independent Test (独立测试)**: 重启宿主后枚举 audit(审计) 流水最新 50 条. 比对载荷里的快照哈希是否与 SupervisorSpec(监督器规格) 导出一致.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 在一次 add child(追加子任务) 中途注入断电故障指令到夹具, **When (当)** 进程重启并完成恢复流程, **Then (则)** 拓扑视图要么回到调用前值, 要么留下 compensating(补偿) 段落写明未完成事务编号. 不允许停在半解析空白状态.

### Edge Cases (边界情况)

- secrets reference(密钥引用) 占位符语法合法但实际 vault(保险库) 离线必须能与密钥缺失区分开. 分别在 `validation_failed` 与 `runtime_secret_miss` 两级打点.
- resource limits(资源限制) 宿主内核不支持时必须选定 `ignore`(忽略声明)或 `reject_boot`(拒绝拉起), 二者之一写死在默认 YAML schema 注解. 本切片选定 `ignore`(全局默认, 见 data-model 校验规则 6).
- **supervisor 关闭中**: add_child 在 supervisor 正在执行关停流程时返回 `Err(SupervisorShuttingDown)`, 不执行任何注册或审计写入.
- **child 数达到上限**: 运行时 child 数 ≥ 1000(见 plan.md 规模上限)时, add_child 返回 `Err(ChildLimitExceeded { max: 1000, current: N })`. 静态 YAML 加载时超过上限同样返回加载错误.
- **审计存储失败**: 审计通道(环形缓冲区)写入失败时, add_child 进入 compensating 并返回 `Err(AuditStorageFailure)`. 环形缓冲区容量应保证至少容纳 2 × max_children 条审计记录, 避免正常操作下覆盖未读记录.

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 静态 YAML 必须允许声明监督根节点之下的完整子任务拓扑. 字段最少包含 children(子任务), dependencies(依赖), health(健康检查), readiness(就绪检查), resource limits(资源限制), command permissions(命令权限), environment(环境变量), secrets reference(密钥引用), restart budgets(重启预算). 加载阶段必须拒绝任何违反 schema 的行, 并在响应里带回字段路径片段. 禁止半解析对象渗入运行时.
- **FR-002**: add child(追加子任务) 必须把解析, 校验, 注册, 拉起, audit(审计) 持久化五步连成一串当成同一桩事务来写. 哪一步失手要么整体退回调用前的拓扑视图, 要么写上 compensating(补偿) 段落给人善后指针. 不许只靠一份没校验过的草稿 manifest(清单) 充当证据.

### Key Entities (关键实体) _(涉及数据时填写)_

- **ChildDeclaration(子任务声明)**: YAML 文件或运行时载荷里的单行子任务绑定对象. 包含字段路径可用于审计索引.

## Constitution Alignment (宪章对齐) _(mandatory (必填))_

### Supervision Contract (监督契约)

- **Lifecycle impact (生命周期影响)**: 扩大配置驱动启动覆盖面, 必须与 006-3 关停语义以及并发承认条款联合验收.

  **跨切片联合验证场景**:
  1. **add_child 中途收到关停**: supervisor 正在执行 add_child 五步事务(步骤 3 注册后)时收到 006-3 关停信号. 预期: 当前 add_child 进入 compensating 并完成回滚, 随后 supervisor 进入关停流程.
  2. **child 刚启动就被关停**: add_child 步骤 4 拉起 child 后, 在 child 进入 running 之前 supervisor 收到关停信号. 预期: child 的 CancellationToken 被触发, child 按 006-3 关停协议退出.
  3. **并发 add_child 与重启冲突**: add_child 事务进行中时, 另一个 child 触发重启(006-3 并发承认条款). 预期: add_child 互斥锁阻止并发, 重启排队等待; 或重启优先、add_child 被补偿.

  **联合测试计划**: 上述场景需在 tests/add_child_transaction_test.rs 中增加集成测试用例, 使用 006-3 提供的关停夹具. 具体实现延迟至 Phase 4(US2 实现完成后).

### Rust Boundary and Observability Requirements (Rust 边界和可观察性需求)

- **Module ownership (模块所有权)**: spec(规格) 解析层与 config(配置) 校验层目录分层不得塌缩成单一 god module(全能模块).
- **Compatibility exports (兼容导出)**: None (无).
- **Diagnostics (诊断)**: 任意 YAML 拒绝必须打印 `field_path`(JSON Pointer 格式) 与人读 `hint`(提示段落).
- **Dependency impact (依赖影响)**: 不适用, 除非计划证明必须新增宿主 libc(C 运行库) 级别适配.

### Chinese Writing (中文写作)

- **Writing language (写作语言)**: 本文档必须使用中文.
- **Term format (术语格式)**: 英文术语必须写成 English(中文说明).
- **Forbidden style (禁止风格)**: 禁止全角标点, 禁止把流水线步骤写成口语口令却不编号.

## Success Criteria (成功标准) _(mandatory (必填))_

### Measurable Outcomes (可衡量结果)

- **SC-001**: golden(黄金样本) YAML 在 CI 对照脚本里字段一致率达到 100%.
- **SC-002**: add child(追加子任务) 10_000 次顺序追加(无并发), 每次追加后通过测试夹具模拟移除, 注册表漂移计数为 0, audit(审计) 缺失条目数为 0.
  > **NOTE**: remove_child API 不在本切片范围(见 research R010). 压力脚本中的移除操作使用测试夹具直接清理注册表, 而非真实 API. 完整移除链路推迟至后续切片.

## Assumptions (假设)

- secrets reference(密钥引用) 的真正解密下发由宿主 vault(保险库) 适配层完成. 监督器只负责契约校验边界与 audit(审计) 映射.
