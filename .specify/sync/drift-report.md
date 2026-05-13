# Spec Drift Report(规格漂移报告)

Generated(生成时间): 2026-05-12T02:03:03+08:00
Project(项目): rust-tokio-supervisor

## Summary(摘要)

| Category(类别) | Count(数量) |
|---|---:|
| Specs Analyzed(已分析规格) | 3 |
| Requirements Checked(已检查需求) | 185 |
| Aligned(已对齐) | 183 (98.9%) |
| Drifted(已漂移) | 1 (0.5%) |
| Not Implemented(未实现) | 1 (0.5%) |
| Unspecced Code(无规格代码) | 1 |

## Scope(范围)

本次分析读取 `specs/*/spec.md`, 当前仓库 `src/`, `tests/`, `examples/` 和 `manual/`. 因为 `003-supervisor-dashboard` 明确把 relay(中继) 放在 `/Users/0x00/Documents/rust-supervisor-relay`, 把 dashboard client(看板客户端) 放在 `/Users/0x00/Documents/rust-supervisor-ui`, 本报告也只读核对了这两个相邻目录, 并排除了 `node_modules`, `target`, `dist` 和 sync backup(同步备份) 目录. 本报告没有修改 implementation code(实现代码).

## Detailed Findings(详细发现)

### Spec(规格): 001-create-supervisor-core - 创建监督器核心

#### Aligned(已对齐)

- FR-001 到 FR-077: 核心监督器声明, 任务工厂, 任务上下文, 监督树, 重启策略, 退出分类, 熔断, 健康检查, 关闭, 控制命令, 状态平面, 事件平面, 可观测性, 配置, 文档, 发布边界, YAML(数据序列化格式), glossary(词汇表), 硬编码常量检查, module dependency map(模块依赖图), module dependency rule(模块依赖规则), parallel workstream(并行工作流), unattended implementation(无人值守实现), task completion ledger(任务完成台账), blocker elimination(卡点消除), lead agent supervision(主代理监督), correction loop(纠偏循环) 和 top-level directory module(顶层目录模块) 规则已经由 `src/spec/`, `src/task/`, `src/tree/`, `src/policy/`, `src/health/`, `src/shutdown/`, `src/control/`, `src/state/`, `src/dashboard/state.rs`, `src/tests/*_test.rs`, `examples/`, `manual/`, `specs/001-create-supervisor-core/*` 和 `Cargo.toml` 覆盖.
- SC-001 到 SC-045: 监督行为, 可观测性, 配置, 文档同步, 发布, 依赖, 并行治理和源码布局检查已经存在于当前仓库的测试, 示例和文档中.
- FR-063 和 SC-031 当前已经重新对齐. 当前仓库, `/Users/0x00/Documents/rust-supervisor-relay` 和 `/Users/0x00/Documents/rust-supervisor-ui` 的 `src/`, `tests/`, `manual/`, `examples/` 和 003 契约文档中没有命中 `*Snapshot`, `*View`, `snapshot()` 或 `state_view` 代码命名.

#### Drifted(已漂移)

- None(无).

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

- FR-001 到 FR-003, FR-007 到 FR-012, FR-015 到 FR-018, FR-022, FR-025: 当前仓库实现 target process IPC(目标进程进程间通信) 配置, 本机 Unix domain socket(Unix 域套接字) 服务端, `hello`, `state`, `events.subscribe`, `logs.tail`, command(命令) 映射, event/log record(事件和日志记录), structured error(结构化错误), protocol alias rejection(协议别名拒绝) 和共享模型. 主要位置是 `src/dashboard/`, `src/config/configurable.rs`, `src/config/state.rs`, `tests/dashboard_*_test.rs`, `examples/config/supervisor.yaml` 和 `manual/dashboard.md`.
- FR-004 到 FR-006, FR-013, FR-014, FR-019, FR-026: relay(中继) 目录 `/Users/0x00/Documents/rust-supervisor-relay` 已实现 `DashboardRelayConfig`, `TargetProcessRegistry`, dynamic registration(动态注册), registration lease(注册租约), mTLS(双向传输层安全协议认证), trusted proxy(可信代理), session gating(会话门控), IPC client(进程间通信客户端), reconnect(重连), command audit(命令审计) 和结构化错误. 主要位置是 `src/config.rs`, `src/registry.rs`, `src/auth.rs`, `src/session.rs`, `src/ipc_client.rs`, `src/relay.rs`, `src/command.rs`, `src/audit.rs` 和 `tests/relay_*_test.rs`.
- FR-020, FR-021, FR-024, FR-027: dashboard client(看板客户端) 目录 `/Users/0x00/Documents/rust-supervisor-ui` 已使用 Vue(网页界面框架), shadcn-vue(组件库), Tailwind(样式框架), Vue Flow(流程图组件), Vitest(前端测试工具) 和 Playwright(浏览器测试工具), 并实现 target list(目标列表), topology canvas(拓扑画布), node detail(节点详情), event/log filter(事件日志过滤), dropped count(丢弃数量), control panel(控制面板), confirmation dialog(确认对话框), `wss://` session client(会话客户端) 和 React(网页界面库) 排除基线.
- SC-001, SC-002, SC-004 到 SC-011: 当前仓库, relay(中继) 和 UI(用户界面) 目录中存在对应的 `dashboard_*_test.rs`, `relay_*_test.rs`, Playwright(浏览器测试) 和 Vitest(前端测试) 文件. 当前 `package.json` 没有 React(网页界面库) runtime dependency(运行时依赖), `src/` 下也没有 `.tsx` 或 `.jsx` component file(组件文件).

#### Drifted(已漂移)

- SC-012: 规格要求 dashboard client(看板客户端) 交付物中 Vue(网页界面框架), shadcn-vue(组件库) 和 Tailwind(样式框架) 基线必须可验证, React(网页界面库) 运行时依赖和 React(网页界面库) 组件文件数量必须为 0. 当前 `package.json` 实际没有 React(网页界面库) 依赖, `src/` 下也没有 `.tsx` 或 `.jsx` 文件, 但 `tests/dashboard-performance.spec.ts` 只断言 `window.__RUST_SUPERVISOR_UI_BASELINE__`, 没有把 React(网页界面库) runtime dependency(运行时依赖) 和 component file(组件文件) 数量为 0 编成自动验证.
  - Location(位置): `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts:11`, `/Users/0x00/Documents/rust-supervisor-ui/package.json:14`, `specs/003-supervisor-dashboard/tasks.md:143`
  - Severity(严重程度): moderate(中等)

#### Not Implemented(未实现)

- SC-003: 规格要求 `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts` 使用包含 5 个目标进程, 200 个 child task(子任务), failed(失败), quarantined(隔离) 和 restarting(重启中) 节点的固定测试数据集重复执行 20 次定位流程, 其中至少 19 次必须在 30 秒内定位到指定异常 child task(子任务) 及其最近事件. 当前测试只执行 1 次定位流程, mock data(模拟数据) 只有 3 个 target process(目标进程) 和少量 child task(子任务), 并且 `T066` 仍是未完成状态.
  - Location(位置): `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts:3`, `/Users/0x00/Documents/rust-supervisor-ui/src/mock/dashboardData.ts:149`, `specs/003-supervisor-dashboard/tasks.md:143`
  - Severity(严重程度): major(重大)

## Unspecced Code(无规格代码)

| Feature(功能) | Location(位置) | Lines(行数) | Suggested Spec(建议规格) |
|---|---|---:|---|
| Spec Kit sync extension(规格工具同步扩展) 本地命令和技能资产 | `.specify/extensions/sync/`, `.agents/skills/speckit-sync-*` | 2554 | `004-spec-sync-tooling` |

## Inter-Spec Conflicts(规格间冲突)

- None(无). 001 的 naming rule(命名规则) 和 003 的 dashboard state(看板状态) 命名已经重新对齐.

## Recommendations(建议)

1. 先实现 `T066`, 把 `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts` 扩展为固定数据集 20 次定位流程, 并补足 5 个目标进程和 200 个 child task(子任务) 的测试数据.
2. 在同一个 `T066` 中增加 React(网页界面库) 排除验证, 直接读取 `package.json` 的 runtime dependency(运行时依赖), 并扫描 `src/` 下 `.tsx` 和 `.jsx` component file(组件文件) 数量.
3. 为 `.specify/extensions/sync/` 和 `speckit-sync-*` 技能资产补一个独立工具规格, 或明确把它们归类为本地开发工具, 不进入产品规格同步范围.
