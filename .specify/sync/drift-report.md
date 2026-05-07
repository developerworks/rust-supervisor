# Spec Drift Report(规格漂移报告)

Generated(生成时间): 2026-05-08T00:34:25+08:00
Project(项目): rust-tokio-supervisor

## Summary(摘要)

| Category(类别) | Count(数量) |
|---|---:|
| Specs Analyzed(已分析规格) | 4 |
| Requirements Checked(已检查需求) | 208 |
| Aligned(已对齐) | 183 (88.0%) |
| Drifted(已漂移) | 2 (1.0%) |
| Not Implemented(未实现) | 23 (11.1%) |
| Unspecced Code(无规格代码) | 1 |

## Scope(范围)

本次分析读取 `specs/*/spec.md`, 当前仓库 `src/`, `tests/`, `examples/` 和 `manual/`. 因为 `003-supervisor-dashboard` 明确把 relay(中继) 放在 `/Users/0x00/Documents/rust-supervisor-relay`, 把 dashboard client(看板客户端) 放在 `/Users/0x00/Documents/rust-supervisor-ui`, 本报告也只读核对了这两个相邻目录. 本报告没有修改 implementation code(实现代码).

## Detailed Findings(详细发现)

### Spec(规格): 001-create-supervisor-core - 创建监督器核心

#### Aligned(已对齐)

- FR-001 到 FR-062: 核心监督器声明, 任务工厂, 任务上下文, 监督树, 重启策略, 退出分类, 熔断, 健康检查, 关闭, 控制命令, 状态平面, 事件平面, 可观测性, 配置, 文档和发布边界, 已由 `src/spec/`, `src/task/`, `src/tree/`, `src/policy/`, `src/health/`, `src/shutdown/`, `src/control/`, `src/state/`, `src/event/`, `src/observe/`, `src/runtime/`, `src/summary/`, `examples/`, `manual/` 和对应 `src/tests/*_test.rs` 覆盖.
- FR-064 到 FR-077: 测试命名, YAML(数据序列化格式) 主配置, glossary(词汇表), 硬编码常量检查, module dependency map(模块依赖图), module dependency rule(模块依赖规则), parallel workstream(并行工作流), unattended implementation(无人值守实现), task completion ledger(任务完成台账), blocker elimination(卡点消除), lead agent supervision(主代理监督), correction loop(纠偏循环) 和 top-level directory module(顶层目录模块) 规则, 已由 `src/tests/*_test.rs`, `specs/001-create-supervisor-core/*`, `Cargo.toml` 和 `manual/` 覆盖.
- SC-001 到 SC-030 以及 SC-032 到 SC-045: 对应的监督行为, 可观测性, 配置, 文档同步, 发布, 依赖, 并行治理和源码布局检查已经存在于当前仓库的测试, 示例和文档中.

#### Drifted(已漂移)

- FR-063: 规格要求源码不得出现任何 `*Snapshot`, `*View` 后缀或 `snapshot()` 查询方法, 但 003 已经在当前仓库实现 `DashboardSnapshot` 和 `DashboardIpcService::snapshot`.
  - Location(位置): `src/dashboard/model.rs:74`, `src/dashboard/ipc_server.rs:174`
  - Actual(实际行为): dashboard(看板) 模块使用 snapshot(快照) 作为协议模型和 IPC(进程间通信) 查询方法.
  - Severity(严重程度): major(重大)
- SC-031: 规格要求 naming check(命名检查) 证明源码, 示例, 公开契约和文档中不存在 `*Snapshot`, `*View`, `snapshot()` 或 `state_view`, 但当前测试跳过 `src/dashboard/`, 所以它不再证明全局禁止规则.
  - Location(位置): `src/tests/naming_contract_test.rs:17`, `src/dashboard/model.rs:74`, `src/dashboard/ipc_server.rs:174`
  - Actual(实际行为): `cargo test source_code_avoids_forbidden_snapshot_and_view_names` 通过, 但通过原因包含 dashboard(看板) 源码例外.
  - Severity(严重程度): major(重大)

#### Not Implemented(未实现)

- None(无).

### Spec(规格): 002-config-schema-support - 配置结构体模式支持

#### Aligned(已对齐)

- FR-001 到 FR-017: `SupervisorConfig` 和 nested configuration struct(嵌套配置结构体) 集中在 `src/config/configurable.rs`, 支持 `confique::Config`, `JsonSchema`, `Serialize` 和 `Deserialize`, 并和 `ConfigState` 分离. `src/config/state.rs`, `src/config/loader.rs`, `src/config/yaml.rs`, `src/config/tests/*_test.rs`, `examples/config/supervisor.template.yaml` 和 `manual/*/configuration.md` 覆盖 schema(结构模式), template(模板), YAML(数据序列化格式), semantic validation(语义校验), startup rejection(启动拒绝) 和 no compatibility export(无兼容导出).
- SC-001 到 SC-007: schema coverage check(结构模式覆盖检查), nested config(嵌套配置), single root YAML template target(单根 YAML 模板目标), `x-tree-split` 默认次数为 0, 6 类非法配置拒绝, 配置失败不返回 runtime handle(运行时句柄) 和文档同步, 已由 `src/config/tests/configurable_schema_test.rs`, `src/config/tests/configurable_template_test.rs`, `src/config/tests/no_baked_in_tree_split_test.rs`, `src/config/tests/invalid_config_rejected_test.rs`, `src/tests/supervisor_config_test.rs` 和 `manual/` 覆盖.

#### Drifted(已漂移)

- None(无).

#### Not Implemented(未实现)

- None(无).

### Spec(规格): 003-supervisor-dashboard - 监督任务可视化界面

#### Aligned(已对齐)

- FR-001 到 FR-003, FR-007 到 FR-012, FR-015 到 FR-018, FR-022, FR-025: 当前仓库实现 target process IPC(目标进程进程间通信) 配置, 本机 Unix domain socket(Unix 域套接字) 服务端, `hello`, `snapshot`, `events.subscribe`, `logs.tail`, command(命令) 映射, event/log record(事件和日志记录), structured error(结构化错误), protocol alias rejection(协议别名拒绝) 和共享模型. 主要位置是 `src/dashboard/`, `src/config/configurable.rs`, `src/config/state.rs`, `tests/dashboard_*_test.rs`, `examples/config/supervisor.yaml` 和 `manual/dashboard.md`.
- FR-004 到 FR-006, FR-013, FR-014, FR-019, FR-026: relay(中继) 目录 `/Users/0x00/Documents/rust-supervisor-relay` 已实现 `DashboardRelayConfig`, `TargetProcessRegistry`, dynamic registration(动态注册), registration lease(注册租约), mTLS(双向传输层安全协议认证), trusted proxy(可信代理), session gating(会话门控), IPC client(进程间通信客户端), reconnect(重连), command audit(命令审计) 和结构化错误. 主要位置是 `src/config.rs`, `src/registry.rs`, `src/auth.rs`, `src/session.rs`, `src/ipc_client.rs`, `src/relay.rs`, `src/command.rs`, `src/audit.rs` 和 `tests/relay_*_test.rs`.
- FR-020, FR-021, FR-024, FR-027: dashboard client(看板客户端) 目录 `/Users/0x00/Documents/rust-supervisor-ui` 已使用 Vue(网页界面框架), shadcn-vue(组件库), Tailwind(样式框架), Vue Flow(流程图组件), Vitest(前端测试工具) 和 Playwright(浏览器测试工具), 并实现 target list(目标列表), topology canvas(拓扑画布), node detail(节点详情), event/log filter(事件日志过滤), dropped count(丢弃数量), control panel(控制面板), confirmation dialog(确认对话框), `wss://` session client(会话客户端) 和 React(网页界面库) 排除基线.
- SC-001 到 SC-012: 当前仓库 dashboard(看板) 测试通过 `cargo test --test dashboard_config_test --test dashboard_protocol_shape_test --test dashboard_snapshot_test --test dashboard_stream_test --test dashboard_performance_test`, 并且相邻 relay(中继) 和 UI(用户界面) 目录中存在对应的 `relay_*_test.rs`, Playwright(浏览器测试) 和 Vitest(前端测试) 文件.

#### Drifted(已漂移)

- None(无).

#### Not Implemented(未实现)

- None(无).

### Spec(规格): 004-agent-retrieval-rules - 智能体检索规则演化

#### Aligned(已对齐)

- None(无). 当前目录只有 `spec.md` 和 `checklists/requirements.md`, 尚未进入 plan(计划), tasks(任务) 或 implementation(实现) 阶段.

#### Drifted(已漂移)

- None(无).

#### Not Implemented(未实现)

- FR-001 到 FR-016: 当前 `src/` 中没有 `risk pattern(风险模式)`, `evidence plan(证据计划)`, `evidence record(证据记录)`, `causal chain(因果链)`, `behavior rule(行为规则)`, `rule evolution record(规则演化记录)`, `parallel subtask(并行子任务)`, `agent result(智能体结果)` 或 `synthesis report(汇总报告)` 的实现模块.
- SC-001 到 SC-007: 当前仓库没有用于 20 个已知经验风险样本, 证据缺失和冲突样本, 两轮迭代, 3 个并行子任务, rule evolution(规则演化) 记录, final synthesis(最终汇总) 或 stop condition(停止条件) 的测试与验收证据.

## Unspecced Code(无规格代码)

| Feature(功能) | Location(位置) | Lines(行数) | Suggested Spec(建议规格) |
|---|---|---:|---|
| Spec Kit sync extension(规格工具同步扩展) 本地命令和技能资产 | `.specify/extensions/sync/`, `.agents/skills/speckit-sync-analyze/SKILL.md` | 1480 | `005-spec-sync-tooling` |

## Inter-Spec Conflicts(规格间冲突)

- `001-create-supervisor-core` 的 FR-063 和 SC-031 全局禁止 `*Snapshot` 和 `snapshot()`, 但 `003-supervisor-dashboard` 的 FR-008, FR-014, SC-001 以及 data model(数据模型) 明确要求 snapshot(快照) 作为 dashboard(看板) 协议对象. 当前代码选择实现 003, 因此 001 的命名规则已经和后续规格冲突.

## Recommendations(建议)

1. 先解决 `001-create-supervisor-core` 和 `003-supervisor-dashboard` 的 snapshot(快照) 命名冲突. 如果 dashboard snapshot(看板快照) 是正式新边界, 需要用 sync apply(同步应用) 或单独规格修订把 001 的全局禁止规则改成核心状态查询边界禁止, 并保留 dashboard(看板) 协议例外.
2. 对 `004-agent-retrieval-rules` 进入 plan(计划) 和 tasks(任务) 阶段前, 不要把当前状态解读为代码缺陷. 它是新规格尚未实现.
3. 为 `.specify/extensions/sync/` 和 `speckit-sync-*` 技能资产补一个独立工具规格, 或明确把它们归类为本地开发工具, 不进入产品规格同步范围.
