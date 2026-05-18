# Research(研究): 配置声明与动态子任务治理

**Branch(分支)**: `006-6-config-dynamic-children` | **Date(日期)**: 2026-05-19
**Status(状态)**: Final(定稿)

## Research Items(研究项)

### R001: YAML Schema 工具选型

- **Decision(决策)**: 在现有 `confique` 派生宏基础上扩展 `SupervisorConfig`, 配合 `serde_yaml` 反序列化. 不引入 JSON Schema / OpenAPI / CUE 等新工具.
- **Rationale(理由)**: `confique` 0.4.0 已用于项目配置定义, 支持 YAML 格式, 可派生 `#[derive(Config)]` 提供运行时类型. `serde_yaml` 0.9 已引入. 引入 JSON Schema 生成(cf. `schemars`)可在 CI 中验证 YAML 文件结构, 但本切片不要求独立 IDL(方案) 文件.
- **Alternatives Considered(替代方案)**: (1) JSON Schema + `jsonschema` crate — 需要维护两份 schema; (2) CUE — 新 DSL(领域特定语言) 学习成本高; (3) 纯 `serde_yaml` 无 schema — 无法提供字段级错误路径.

### R002: 拓扑 DAG 环路检测算法

- **Decision(决策)**: 使用 Kahn 拓扑排序算法, 在 YAML 加载阶段和 add_child 校验阶段各执行一次. 时间复杂度 O(V+E), 空间复杂度 O(V).
- **Rationale(理由)**: Kahn 算法可在排序过程中同时检测环路. DFS 标记法也能检测但需要额外处理非连通图. Kahn 的线性输出(拓扑序列)可直接用于启动顺序.
- **Alternatives Considered(替代方案)**: (1) DFS 颜色标记 — 实现简单但无法给出拓扑序列; (2) Floyd-Warshall — O(V^3) 不适合 1000 节点场景.

### R003: add_child 事务实现方式

- **Decision(决策)**: 使用临时登记 + commit/rollback 模式. 在 `ConfigState` 中添加 `pending_additions: Vec<PendingChild>` 作为暂存区. 五步依次完成后 commit(`pending → active`), 任一步失败时 rollback(丢弃 pending 并恢复拓扑). 故障注入点(断电模拟)通过夹具在 rollback 路径上提前返回模拟.
- **Rationale(理由)**: 五步事务涉及内存状态修改(注册表)和审计写入(环形缓冲区). 审计写入不可回滚但补偿段落可标记未完成事务. 断电后重启时遍历 audit 中的 `pending` 标记, 识别未提交的 add_child.
- **Alternatives Considered(替代方案)**: (1) WAL(预写日志) — 引入持久化依赖; (2) 两阶段提交 — 过于重量级.

### R004: 快照哈希算法与计算范围

- **Decision(决策)**: 使用 SHA-256 作为哈希算法, 计算范围为 `SupervisorSpec` 的 JSON 序列化(通过 `serde_json::to_string`). 每次 add_child commit 时更新哈希, 快照哈希存储在内存中.
- **Rationale(理由)**: SHA-256 是标准哈希算法, Rust 标准库 `std::collections::hash_map::DefaultHasher` 不保证跨进程一致性. `serde_json` 序列化保证确定性输出(字段按定义顺序).
- **Alternatives Considered(替代方案)**: (1) BLAKE3 — 更快但增加依赖; (2) 仅哈希 children 列表 — 不够完整.

### R005: 并发 add_child 隔离性

- **Decision(决策)**: 使用 `tokio::sync::Mutex<ConfigState>` 对 add_child 做互斥. 不允许多个 add_child 同时执行. 这是因为 ConfigState 的拓扑更新和审计写入不是原子操作.
- **Rationale(理由)**: 单进程内 add_child 是高耗时操作(解析 + 校验 + 注册 + 拉起), 并发执行可能导致中间状态不一致. 互斥锁在当前规模的 supervisor(单进程)下可接受.
- **Alternatives Considered(替代方案)**: (1) 乐观锁 + CAS — 复杂度高; (2) 多线程 ConfigState 分片 — 拓扑 DAG 需要全局视图, 分片困难.

### R006: 9 类字段的 schema 扩展策略

- **Decision(决策)**: 在 `src/spec/child.rs` 的 `ChildSpec` 中新增可选字段, 使用 `Option<T>` 表示未配置. 在 `src/config/configurable.rs` 的 `SupervisorConfig` 中新增 `children: Vec<ChildDeclaration>` 顶层字段. `ChildDeclaration` 包含所有 9 类字段, 在 `try_from` 转换为 `ChildSpec` 时做校验.
- **Rationale(理由)**: 保持 `ChildSpec` 作为运行时的类型化表示, `ChildDeclaration` 作为 YAML 加载的声明式表示. 分离解析和运行时的关注点.
- **Alternatives Considered(替代方案)**: (1) 直接反序列化为 ChildSpec — YAML 字段名与 Rust 字段名需严格一致; (2) confique 直接派生在 ChildDeclaration 上 — `confique` 当前不支持嵌套列表.

### R007: secrets reference 占位符语法

- **Decision(决策)**: 使用 `${SECRET_NAME}` 格式作为密钥占位符语法. 校验阶段检测语法合法性. vault 离线和密钥缺失在运行时区分并在 audit 中以不同枚举值记录.
- **Rationale(理由)**: `${VAR}` 是 shell 和环境变量的通用占位符格式, 用户无需学习新语法. 两级区分(validation_failed vs runtime_secret_miss)满足 spec Edge Cases 要求.
- **Alternatives Considered(替代方案)**: (1) `{{secret.name}}` — 与 Go template 语法冲突; (2) `@secret_name` — 不够直观.

### R008: audit 补偿段落实体

- **Decision(决策)**: `compensating` 段落实体包含以下字段: `transaction_id: Uuid`(唯一事务编号), `operation: String`("add_child"), `state: String`("pending" | "committed" | "compensated"), `child_name: String`, `declaration_hash: String`(ChildDeclaration 的 SHA-256), `error: Option<String>`(失败原因), `created_at_unix_nanos: u128`. 存储在 audit 通道中, 与普通审计条目同列.
- **Rationale(理由)**: 足够信息供重启后判断事务是否完成. `declaration_hash` 允许恢复时重建 ChildDeclaration.
- **Alternatives Considered(替代方案)**: (1) 独立 WAL 文件 — 增量持久化层; (2) 仅标记 + 无 hash — 无法恢复.

### R009: 002 切片基线兼容性

- **Decision(决策)**: 阅读 `specs/002-config-schema-support/spec.md` 的对照表, 确认 002 定义了 `SupervisorSpec` 的核心字段(path, strategy, children, restart_limit). 本切片新增的 resource limits, command permissions, secrets reference, environment 为全新字段, 与 002 基线无冲突.
- **Rationale(理由)**: 002 切片在 006-6 分支中已合并, `src/spec/supervisor.rs` 已有 `SupervisorSpec` 的完整定义. 本切片新增字段直接追加到 `ChildSpec`.
- **Alternatives Considered(替代方案)**: 无.

### R010: remove_child 范围

- **Decision(决策)**: remove_child 操作不在本切片范围内. SC-002 的"10_000 次追加随后移除"压力脚本需要 remove_child API, 但 remove_child 的实现推迟到后续切片(如 006-7 或 006-9). 本切片仅验证 add_child 的原子性和审计完整性, 移除操作使用测试夹具直接清理注册表.
- **Rationale(理由)**: remove_child 涉及停止运行中的 child、清理资源、释放依赖, 复杂度与 add_child 相当, 不应挤占本切片焦点.
- **Alternatives Considered(替代方案)**: (1) 包含最小化 remove_child — 增加切片的耦合范围; (2) 仅测试 add_child + 重启恢复 — 无法验证"追加随后移除"的漂移计数.
