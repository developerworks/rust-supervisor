# Public API Contract(公开接口契约): 监督器核心

本契约描述本功能预期提供的 Rust library(库) 表面.名称必须由本项目拥有,不得成为参考 crate(库) 的 API(接口) 形状复制或 compatibility method(兼容方法).

## Module Boundaries(模块边界)

```text
rust_supervisor
├── spec
├── id
├── config
├── task
├── runtime
├── child_runner
├── tree
├── policy
├── readiness
├── health
├── shutdown
├── control
├── registry
├── event
├── state
├── journal
├── summary
├── observe
├── error
└── test_support
```

源码必须使用 top-level directory module(顶层目录模块) 结构,不得使用 `src/supervision/` 中间层,也不得使用 `src/<module>.rs` 平铺模块文件.每个顶层模块必须使用下面形状:

```text
src/<module>/
├── mod.rs
├── <owned_file>.rs
└── tests/*_test.rs
```

`src/lib.rs` 只包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明.每个 `src/<module>/mod.rs` 只能包含 `pub mod <mod_name>;` 形式的模块声明.这些入口不得包含 `pub use`(公开重导出),类型定义,函数定义,常量定义或其它逻辑.这些入口不得公开第三方 supervisor(监督器) crate(库) 类型.

所有内部导入必须使用 `crate::` absolute path(绝对路径).所有外部依赖导入必须使用 external crate name(外部软件包名) absolute path(绝对路径).源码不得使用 `super::` relative path(相对路径).

## Naming Contract(命名契约)

代码命名必须使用下面名称:

- `ConfigState`(配置状态): rust-config-tree(集中配置树) 加载和校验后的不可变配置状态.
- `SupervisorState`(监督器状态): 当前监督树状态.
- `ChildState`(子任务状态): 当前子任务状态.
- `current_state`(当前状态): `SupervisorHandle`(监督器句柄) 的状态查询命令.
- `state`(状态): 源码模块和测试命名中的状态边界.

源码,示例,契约和文档不得出现任何 `*Snapshot` 或 `*View` 代码命名,也不得提供 `snapshot()` 查询方法,也不得使用 `state_view` 作为模块名,文件名,方法名或字段名.

## Glossary Contract(词汇表契约)

专业词汇和反引号词汇必须登记在 [glossary.md](../glossary.md).反引号内的 Rust(编程语言) 类型名,枚举值,方法名,字段名,指标名,路径名,命令名,配置键和测试目标都算词汇.

**Rules(规则)**:

- public API(公开接口) 增加新类型,枚举值,方法或字段时,必须同步更新词汇表.
- observability signal(可观测性信号) 增加新事件名或指标名时,必须同步更新词汇表.
- configuration schema(配置模式) 增加新配置键时,必须同步更新词汇表.
- 文档不得对同一个英文词汇使用互相冲突的中文说明.

## Test File Naming Contract(测试文件命名契约)

所有测试文件必须以 `_test.rs` 结尾.不同测试类型使用下面路径:

- integration test(集成测试): `src/tests/*_test.rs`.
- unit test(单元测试): `src/<module>/tests/*_test.rs`.
- contract test(契约测试) 和 quality gate test(质量门禁测试): 根据覆盖范围放入 `src/tests/*_test.rs` 或模块自己的 `tests/*_test.rs`.

实现文件中不得写 inline unit test(内联单元测试) 代码.如果 `src/tests/*_test.rs` 需要 Cargo(构建工具) test target(测试目标),必须在 `Cargo.toml` 中显式声明,并保持文件名和目标名一致.

## Coding Documentation Contract(代码文档契约)

编码阶段必须同步完成这些文档:

- module doc(模块文档): 每个源码模块必须用英文说明职责和边界.
- struct doc(结构体文档): 每个 struct(结构体) 必须用英文说明含义和不变量.
- field doc(字段文档): 每个 struct field(结构体字段) 必须用英文说明来源,单位和约束.
- public function doc(公共函数文档): 每个 public function(公共函数) 必须用英文说明参数,返回值,错误和可运行 doctest(文档测试).
- private function doc(私有函数文档): 每个 private function(私有函数) 必须用英文说明局部不变量,参数和返回值.
- source comment(源码注释): 需要解释局部不变量时必须使用英文.

代码文档不得作为后续补丁推迟.行为实现和对应文档必须在同一变更中完成.

## Cognitive Complexity Contract(认知复杂度契约)

实现必须遵守这些 cognitive complexity(认知复杂度) 预算:

- regular function(普通函数): cognitive complexity(认知复杂度) 不得超过 15.
- lifecycle dispatcher(生命周期调度函数): cognitive complexity(认知复杂度) 不得超过 20.
- nesting depth(嵌套深度): `if`,`match`,`loop`,`while`,`for` 和 error branch(错误分支) 组合后不得超过 3 层.
- split rule(拆分规则): 超限逻辑必须拆分为 state machine(状态机),policy function(策略函数),small helper function(小辅助函数) 或独立模块.

复杂度拆分必须保留清晰命名和测试覆盖.不得通过宏,无意义包装或隐藏控制流来绕过检查.

## Maintainability Contract(可维护性契约)

实现必须遵守这些 maintainability(可维护性) 规则:

- cohesion(内聚): 每个模块只承担一个清晰职责.
- coupling(耦合): 跨模块协作必须通过公开契约类型,不得访问其它模块内部状态.
- state ownership(状态所有权): 共享可变状态只能存在于 runtime(运行时),registry(注册表),current state(当前状态) 或明确 state owner(状态所有者) 中.
- change locality(变更局部性): 行为变化必须能定位到少量模块,并同步对应测试,文档和示例.
- testability(可测试性): 每个模块必须有可定位的 unit test(单元测试) 或 integration test(集成测试) 覆盖.
- domain boundary(领域边界): supervisor core(监督器核心) 不得包含 business data plane(业务数据面) 逻辑.

维护性检查失败时,实现必须优先拆分职责,收敛依赖或移动状态所有权,而不是增加注释掩盖结构问题.

## Parallel Governance Contract(并行治理契约)

实现阶段必须使用 `ModuleDependencyMap`(模块依赖图),`ParallelWorkstream`(并行工作流),`WorkstreamSplitRecord`(工作流拆分记录),`ParallelExecutionBlocker`(并行执行卡点),`BlockerEliminationRecord`(卡点消除记录),`UnattendedImplementationRun`(无人值守实现运行),`TaskCompletionLedger`(任务完成台账),`LeadAgentSupervision`(主代理监督),`SubagentWorkstream`(子代理工作流) 和 `CorrectionRecord`(纠偏记录) 管理并行执行.

**Rules(规则)**:

- `ModuleDependencyMap`(模块依赖图) 必须说明模块层级,允许依赖,禁止依赖和 cycle dependency(循环依赖) 检查结果.
- 每个 `ParallelWorkstream`(并行工作流) 必须拥有明确 scope(范围),primary files(主文件),independent tests(独立测试),blocked by(前置依赖) 和 completion evidence(完成证据).
- 影响并行度的任务必须通过 `WorkstreamSplitRecord`(工作流拆分记录) 拆分,并保留明确 integration point(集成点).
- 每个 `ParallelExecutionBlocker`(并行执行卡点) 必须对应一个 `BlockerEliminationRecord`(卡点消除记录).
- implementation phase(实现阶段) 必须无人值守执行到 `TaskCompletionLedger`(任务完成台账) 没有 `Pending`(待处理),`InProgress`(进行中) 或 `Blocked`(已阻塞) 任务.
- lead agent(主代理) 必须审查所有 subagent workstream(子代理工作流),并对规格偏差,模块边界偏差,测试命名偏差,文档同步偏差和兼容方法偏差创建 `CorrectionRecord`(纠偏记录).
- 任何 workstream(工作流) 只有在 clean review record(清洁审查记录) 或闭环 `CorrectionRecord`(纠偏记录) 存在后才能完成.

## Task Definition Contract(任务定义契约)

```rust
pub type BoxTaskFuture =
    Pin<Box<dyn Future<Output = TaskResult> + Send + 'static>>;

pub trait TaskFactory: Send + Sync + 'static {
    fn build(&self, ctx: TaskContext) -> BoxTaskFuture;
}
```

**Rules(规则)**:

- `build` 必须为每次 attempt(尝试) 创建 fresh future(新异步任务).
- 跨重启持久状态必须放在 supervisor runtime(监督器运行时) 外部.
- `TaskResult`(任务结果) 必须区分成功,取消和类型化失败.

## Service Adapter Contract(服务适配层契约)

`Service trait`(服务特征) 和 `service_fn`(函数适配器) 可以作为项目自有的人体工学适配层存在. 它们必须适配到 `TaskFactory`(任务工厂), 不得替换 `TaskFactory`(任务工厂) 内核, 也不得公开第三方 supervisor(监督器) crate(库) API(接口).

**Rules(规则)**:

- 每次 attempt(尝试) 仍然必须创建 fresh future(新异步任务).
- `TaskContext`(任务上下文) 中的取消,心跳,就绪和事件接收点必须继续可用.
- 适配层公开名称必须由本项目拥有.

## Declarative Spec Contract(声明式规格契约)

`ChildSpec`(子任务规格) 必须包含:

- `id`
- `name`
- `kind`
- `factory` 或嵌套 supervisor spec(监督器规格)
- `restart_policy`
- `shutdown_policy`
- `health_policy`
- `readiness_policy`
- `backoff_policy`
- `dependencies`
- `tags`
- `criticality`

`SupervisorSpec`(监督器规格) 必须包含:

- `path`
- `strategy`
- `children`
- `config_version`
- supervisor-level fuse policy(监督器级熔断策略)
- restart(重启),backoff(退避),health(健康),readiness(就绪) 和 shutdown(关闭) 的默认值.

## Configuration Contract(配置契约)

核心必须通过 rust-config-tree(集中配置树) v0.1.9 加载 YAML(数据序列化格式) centralized configuration(集中化配置).`SupervisorConfig`(监督器配置) 必须生成不可变 `ConfigState`(配置状态),并从同一个 config state(配置状态) 派生:

- `SupervisorSpec`(监督器规格)
- restart policy default(重启策略默认值)
- backoff policy default(退避策略默认值)
- health policy default(健康策略默认值)
- readiness policy default(就绪策略默认值)
- shutdown budget(关闭预算)
- observability option(可观测性选项)

配置错误必须产生 `FatalConfig`(致命配置错误),并拒绝启动整棵树.模块内部不得保存可调配置默认值.

主配置文件必须使用 `*.yaml`.契约,示例和 quickstart(快速开始) 不得把 TOML(配置格式),JSON(数据交换格式) 或其它格式作为主配置格式.

## Task Kind Contract(任务类型契约)

`TaskKind`(任务类型) 必须区分:

- `AsyncWorker`(异步工作任务)
- `BlockingWorker`(阻塞工作任务)
- `Supervisor`(监督器)

`BlockingWorker`(阻塞工作任务) 必须拥有独立 shutdown policy(关闭策略) 和 escalation policy(升级策略).关闭超时后, runtime(运行时) 不得假设 blocking worker(阻塞工作任务) 可以被 abort(强制终止) 立即结束.

## Readiness Contract(就绪契约)

`ReadinessPolicy`(就绪策略) 必须支持:

- `Immediate`(立即就绪)
- `Explicit`(显式就绪)

使用 `Explicit`(显式就绪) 的 child(子任务) 必须通过 `TaskContext`(任务上下文) 报告 ready(已就绪).在报告之前, current state(当前状态) 和 event(事件) 不得显示该 child(子任务) 为 ready(已就绪).第一次报告 ready(已就绪) 必须产生 `ChildReady` 事件.

## Policy Contract(策略契约)

核心枚举如下:

- `SupervisionStrategy`(监督策略): `OneForOne`(一对一),`OneForAll`(一对全部),`RestForOne`(从失败处开始)
- `GroupStrategy`(分组策略): 使用 child tag(子任务标签) 限定策略范围,并可携带 group-level restart budget(分组级重启预算) 和 escalation policy(升级策略)
- `ChildStrategyOverride`(子任务级覆盖): 对单个 child(子任务) 覆盖 strategy(策略),restart budget(重启预算) 和 escalation policy(升级策略)
- `RestartBudget`(重启预算): `max_restarts`(最大重启次数) 和 `window`(统计窗口)
- `EscalationPolicy`(升级策略): `EscalateToParent`(升级到父级),`ShutdownTree`(关闭整棵树),`QuarantineScope`(隔离范围)
- `DynamicSupervisorPolicy`(动态监督器策略): 控制 runtime add_child(运行时添加子任务) 的 `enabled`(启用开关) 和 `child_limit`(子任务上限)
- `StrategyExecutionPlan`(策略执行计划): 合并 supervisor strategy(监督器策略),group strategy(分组策略),per-child override(子任务级覆盖),restart budget(重启预算) 和 escalation policy(升级策略) 后的单次运行计划
- `RestartPolicy`(重启策略): `Permanent`(永久),`Transient`(瞬时),`Temporary`(临时)
- `RestartDecision`(重启决策): `DoNotRestart`(不重启),`RestartAfter`(延迟后重启),`Quarantine`(隔离),`EscalateToParent`(升级到父级),`ShutdownTree`(关闭整棵树)
- `TaskFailureKind`(任务失败类别): `Recoverable`(可恢复),`FatalConfig`(致命配置错误),`FatalBug`(致命代码错误),`ExternalDependency`(外部依赖错误),`Timeout`(超时),`Panic`(恐慌),`Cancelled`(已取消)

policy engine(策略引擎) 必须接收 typed exit(类型化退出) 和策略,并返回一个明确 decision(决定).它不得从字符串推断决定.

strategy execution plan(策略执行计划) 的优先级必须固定为 child override(子任务级覆盖) 优先于 group strategy(分组策略),group strategy(分组策略) 优先于 supervisor-wide strategy(监督器全局策略).运行时必须消费 `StrategyExecutionPlan`(策略执行计划),不得在 control loop(控制循环) 中重复实现策略选择.

## Runtime Control Contract(运行时控制契约)

`SupervisorHandle`(监督器句柄) 必须提供异步命令:

- `add_child`
- `remove_child`
- `restart_child`
- `pause_child`
- `resume_child`
- `quarantine_child`
- `shutdown_tree`
- `current_state`
- `subscribe_events`

`add_child`(添加子任务) 必须先通过 `DynamicSupervisorPolicy`(动态监督器策略) 校验.当 dynamic supervision(动态监督) 被禁用,或声明 child(子任务) 加已接受 dynamic manifest(动态清单文本) 达到上限时,命令必须返回错误.

**Idempotency(幂等性)**:

- shutdown(关闭) 后重复 `shutdown_tree` 必须返回当前关闭结果.
- 对已暂停 child(子任务) 重复 `pause_child` 必须返回暂停状态.
- 对运行中 child(子任务) 重复 `resume_child` 必须返回运行状态.
- 对已隔离 child(子任务) 重复 `quarantine_child` 必须返回隔离状态.

## State Contract(当前状态契约)

`current_state` 必须返回最新树状态,并包含:

- root path(根路径)
- generated sequence(生成序号)
- child path(子任务路径)
- child id(子任务标识) 和 name(名称)
- state(状态)
- health(健康状态)
- generation(代次)
- attempt(尝试次数)
- restart count(重启次数)
- last failure(最近失败)
- last policy decision(最近策略决定)

current state(当前状态) 回答"现在真实状态是什么",不得被当作 lifecycle event history(生命周期事件历史).

## Event Contract(事件契约)

每个 `SupervisorEvent`(监督器事件) 必须包含:

- `when`: wall time(墙钟时间),monotonic time(单调时间),supervisor uptime(监督器运行时长),generation(代次) 和 attempt(尝试次数).
- `where`: supervisor path(监督器路径),parent id(父标识),child id(子任务标识),child name(子任务名称),可用时的 task id(任务标识),host(主机),pid(进程标识),thread name(线程名称) 和 registration location(注册位置).
- `what`: event payload(事件内容),state transition(状态迁移),exit reason(退出原因),failure category(失败类别),restart decision(重启决策),backoff(退避),health(健康) 或 triggering command(触发命令).
- `sequence`: monotonic event sequence(单调事件序号).
- `correlation_id`: command(命令) 或 attempt(尝试) 的 correlation id(关联标识).

每次状态迁移必须只发送一条 lifecycle event(生命周期事件).

## Observability Contract(可观测性契约)

每个 lifecycle fact(生命周期事实) 必须可以派生这些 signal(信号):

- `SupervisorEvent`(监督器事件)
- structured log(结构化日志)
- tracing span/event(追踪范围和事件)
- metrics(指标)
- audit event(审计事件)
- event journal(事件日志缓冲区) 记录
- test recorder(测试记录器) 记录

同一个事实的信号必须共享 sequence(序号),correlation id(关联标识) 或 config version(配置版本),使测试和事故排查可以关联它们.核心不得绑定具体 exporter(导出器).

## Audit Contract(审计契约)

每个 control command(控制命令) 必须发送 command audit data(命令审计数据):

- `command_id`
- `requested_by`
- `reason`
- `target_path`
- `accepted_at`
- `result`

audit event(审计事件) 必须可以通过 event stream(事件流) 读取.

## Shutdown Contract(关闭契约)

root shutdown(根关闭) 必须完成这些阶段:

1. request stop(请求停止): 触发 parent cancellation(父取消), 并把取消传播到 child token(子令牌).
2. graceful drain(优雅排空): 等待 child(子任务) 按 graceful timeout(优雅关闭超时) 自行退出.
3. abort stragglers(强制终止拖尾任务): 超时后只强制终止仍未退出且可终止的 async worker(异步工作任务).
4. reconcile(状态对账): 统一更新 registry(注册表),current state(当前状态),metrics(指标) 和 event journal(事件日志缓冲区).

root shutdown(根关闭) 必须按声明顺序的逆序关闭 child(子任务).阶段完成后, supervisor(监督器) 不得继续拥有 child task(子任务).blocking worker(阻塞工作任务) 关闭超时时必须产生说明不可立即终止边界的事件和策略决定.

## Diagnostic Replay Contract(诊断回放契约)

event journal(事件日志缓冲区) 必须是固定容量缓冲区, 并保存最近生命周期事件.发生 meltdown(熔断),关闭超时或父级升级时, 系统必须生成 `RunSummary`(运行摘要), 并包含:

- started at(开始时间) 和 finished at(结束时间)
- shutdown cause(关闭原因)
- restart count(重启次数)
- failure list(失败列表)
- recent events(最近事件)
- final current state(最终当前状态)
- final decision(最终决定)

## Metrics Contract(指标契约)

核心必须通过 metrics facade(指标门面) 发送这些指标名:

- `supervisor_restart_total`
- `supervisor_child_state`
- `supervisor_child_uptime_seconds`
- `supervisor_backoff_seconds`
- `supervisor_healthcheck_latency_seconds`
- `supervisor_meltdown_total`
- `supervisor_shutdown_duration_seconds`
- `supervisor_event_lag_total`
- `supervisor_config_version`

metric label(指标标签) 必须使用低基数值:supervisor path(监督器路径),child id(子任务标识),state(状态),decision(决定) 和 failure category(失败类别).标签不得包含错误全文,用户输入,动态路径碎片或其它无界值.

## Example Contract(示例契约)

`examples/` 必须至少包含这些 example(示例):

- `supervisor_quickstart`
- `config_tree_supervisor`
- `restart_policy_lab`
- `shutdown_tree`
- `observability_probe`

每个 example(示例) 必须能通过 `cargo run --example <name>` 独立运行,或者明确说明必需输入文件.示例只能使用项目自有 API(接口).

## Documentation Contract(文档契约)

项目必须包含中英双语 manual(手册) 和 docs(文档):

- `manual/zh`
- `manual/en`
- `docs/zh`
- `docs/en`

中英文目录必须同构,并覆盖同一组公开概念.当 public API(公开接口),configuration schema(配置模式),example behavior(示例行为) 或 observability signal(可观测性信号) 变化时,manual(手册),docs(文档),quickstart(快速开始),contracts(契约) 和 examples(示例程序) 必须同步.

## Release Contract(发布契约)

crates.io(软件包发布平台) release readiness(发布就绪) 必须验证:

- `Cargo.toml` 包含 name(名称),version(版本),edition(版本代),description(描述),repository(代码仓库),readme(说明文档),license(许可证) 或 license-file(许可证文件),documentation(文档地址),keywords(关键词) 和 categories(分类).
- README(说明文档),LICENSE(许可证) 和 CHANGELOG(变更日志) 存在并与 crate(包) 目的匹配.
- SBOM(软件物料清单) 生成并通过格式和内容校验.
- `cargo package --list` 的 package contents(打包内容) 不包含 target(构建产物),临时文件或无关大文件.
- `cargo publish --dry-run` 必须通过.
- 真实上传 crates.io(软件包发布平台) 不属于本契约的自动执行内容.

## SBOM Contract(软件物料清单契约)

发布准备必须生成这些 SBOM(软件物料清单) 文件:

- `artifacts/sbom/rust-supervisor.cdx.json`: CycloneDX JSON(CycloneDX JSON 格式).
- `artifacts/sbom/rust-supervisor.spdx.json`: SPDX JSON(SPDX JSON 格式).

SBOM(软件物料清单) 必须包含:

- 当前 crate(包) 的 name(名称),version(版本),repository(代码仓库),license(许可证) 和 package URL(软件包地址).
- 所有 direct dependency(直接依赖) 和 transitive dependency(传递依赖).
- 每个 dependency(依赖) 的 version(版本),license(许可证),checksum(校验和),registry source(注册表来源) 和 source reference(来源引用).
- generation tool(生成工具) 名称和版本.
- `Cargo.lock` 依赖图校验摘要.

SBOM(软件物料清单) 不得包含 secret(密钥),token(令牌),本地绝对路径或构建临时目录.依赖版本必须和 `Cargo.lock` 保持一致.

## Test Support Contract(测试支持契约)

`test_support` 必须提供这些帮助能力:

- paused time setup(暂停时间设置)
- heartbeat control(心跳控制)
- readiness control(就绪控制)
- event collection(事件收集)
- event journal assertion(事件日志断言)
- run summary assertion(运行摘要断言)
- current state assertion(当前状态断言)
- shutdown without orphaned tasks assertion(关闭后不留下孤儿任务断言)
- blocking task shutdown assertion(阻塞任务关闭断言)
- deterministic jitter(确定性抖动)
- config state assertion(配置状态断言)
- YAML configuration assertion(YAML 配置断言)
- test file naming assertion(测试文件命名断言)
- glossary coverage assertion(词汇表覆盖断言)
- observability recorder assertion(可观测性记录器断言)
- documentation sync assertion(文档同步断言)
- coding standard assertion(编码标准断言)
- cognitive complexity assertion(认知复杂度断言)
- maintainability assertion(可维护性断言)
- module dependency map assertion(模块依赖图断言)
- parallel workstream assertion(并行工作流断言)
- blocker elimination assertion(卡点消除断言)
- task completion ledger assertion(任务完成台账断言)
- lead agent supervision assertion(主代理监督断言)
- correction record assertion(纠偏记录断言)
- SBOM assertion(软件物料清单断言)
- release readiness assertion(发布就绪断言)
