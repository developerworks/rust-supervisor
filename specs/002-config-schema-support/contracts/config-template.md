# Config Template Contract(配置模板契约): 配置结构体模式支持

## Official Template Boundary(官方模板边界)

官方 YAML(数据序列化格式) template(模板) 是本 crate(包) 提供给 crate user(crate 使用者) 的学习和初始配置入口。模板必须覆盖所有 runtime tunable configuration(运行时可调配置),但不得替使用者决定 `x-tree-split`(树形拆分扩展) 布局。

## Required Sections(必需分区)

官方模板必须包含以下 top-level section(顶层分区):

- `supervisor`
- `policy`
- `shutdown`
- `observability`

## Required Fields(必需字段)

### `supervisor`

- `strategy`: `SupervisionStrategy`(监督策略),例如 `OneForOne`,`OneForAll`,`RestForOne`。

### `policy`

- `child_restart_limit`: 子任务 restart limit(重启次数限制) 限制。
- `child_restart_window_ms`: 子任务 restart window(重启窗口)。
- `supervisor_failure_limit`: 监督器 failure limit(失败次数限制)。
- `supervisor_failure_window_ms`: 监督器 failure window(失败窗口)。
- `initial_backoff_ms`: 初始 backoff(退避)。
- `max_backoff_ms`: 最大 backoff(退避)。
- `jitter_ratio`: jitter ratio(抖动比例)。
- `heartbeat_interval_ms`: heartbeat interval(心跳间隔)。
- `stale_after_ms`: stale threshold(失效阈值)。

### `shutdown`

- `graceful_timeout_ms`: graceful shutdown(优雅关闭) 超时。
- `abort_wait_ms`: abort wait(中止等待) 超时。

### `observability`

- `event_journal_capacity`: event journal(事件日志) 容量。
- `metrics_enabled`: metrics recording(指标记录) 开关。
- `audit_enabled`: command audit(命令审计) 开关。

## Default Target Rule(默认目标规则)

没有使用者 tree split decision(树形拆分决策) 时,官方 template generation(模板生成) 必须只生成一个 root YAML template target(根 YAML 模板目标)。测试必须检查 target(目标文件) 数量等于 1。

## No Built-in Tree Split Rule(禁止内置树形拆分规则)

官方 template(模板),官方 schema(结构模式),README(说明文档),manual(手册) 和 examples(示例程序) 不得把 `x-tree-split`(树形拆分扩展) 写成默认策略。测试必须检查官方 template(模板) 和官方 schema(结构模式) 中 `x-tree-split`(树形拆分扩展) 出现次数为 0。

## User Extension Rule(使用者扩展规则)

使用者可以在自己的项目中包装 `SupervisorConfig`(监督器配置),并在自己的 schema generation(结构模式生成) 流程中声明 `x-tree-split`(树形拆分扩展)。该行为属于使用者项目,本 crate(包) 只保证公开配置结构体具有 schema-ready(可生成结构模式) 能力。

## Sync Rule(同步规则)

任何公开配置字段新增,删除或改名时,必须同步更新:

- `src/config/configurable.rs`
- `examples/config/supervisor.yaml`
- `examples/config/supervisor.template.yaml`
- `README.md`
- `README.zh.md`
- `manual/en/`
- `manual/zh/`
- `specs/002-config-schema-support/contracts/`

如果任一文档或模板与 `SupervisorConfig`(监督器配置) 不一致,documentation sync check(文档同步检查) 必须失败。
