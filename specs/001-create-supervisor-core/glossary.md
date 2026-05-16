# Glossary(词汇表): 创建监督器核心

**Purpose(目的)**: 统一本功能规格,计划,数据模型,公开契约,quickstart(快速开始) 和任务清单中的专业词汇.
**Scope(范围)**: 本文件覆盖 `specs/001-create-supervisor-core/` 中涉及的核心生命周期治理,配置,可观测性,测试,文档和发布词汇.

## Runtime And Supervision(运行时和监督)

| Term(术语) | Chinese(中文说明) | Definition(定义) | Usage Rule(使用规则) |
|---|---|---|---|
| supervisor | 监督器 | 管理 child(子任务) 生命周期的运行时治理节点. | 表示治理者,不表示业务工作任务. |
| supervisor core | 监督器核心 | 本 crate(包) 提供的生命周期治理核心. | 不包含 business data plane(业务数据面) 逻辑. |
| child | 子任务 | 被 supervisor(监督器) 管理的工作节点或嵌套监督器节点. | 必须通过 `ChildSpec` 声明. |
| worker | 工作任务 | 执行业务工作的 child(子任务). | 不负责治理其它 child(子任务). |
| root supervisor | 根监督器 | 一棵监督树的顶层 supervisor(监督器). | root shutdown(根关闭) 从这里开始. |
| supervisor tree | 监督树 | supervisor(监督器) 和 child(子任务) 组成的层级结构. | 用于表达分组重启,局部关闭和父级升级. |
| `ChildSpec` | 子任务规格 | child(子任务) 的声明式定义. | 必须包含身份,任务类型,策略,依赖,标签和关键程度. |
| `SupervisorSpec` | 监督器规格 | supervisor(监督器) 的声明式配置. | 必须从 `ConfigState`(配置状态) 派生. |
| `SupervisorPath` | 监督器路径 | 用于定位树中节点的稳定路径. | 日志,指标,事件,current state(当前状态) 和控制命令必须共用它. |
| `ChildId` | 子任务标识 | child(子任务) 在父级内的稳定标识. | 同一父级范围内必须唯一. |
| `SupervisorHandle` | 监督器句柄 | 运行时控制入口. | 提供幂等控制命令和状态查询. |
| `TaskFactory` | 任务工厂 | 为每次启动或重启构造新一轮运行实例的工厂. | 不得克隆旧任务实例来表达重启. |
| `TaskContext` | 任务上下文 | 传入单次运行的任务上下文. | 包含身份,路径,取消,心跳,就绪和事件接收点. |
| `TaskResult` | 任务结果 | 单次运行的退出结果. | 必须区分成功,取消和类型化失败. |
| `TaskKind` | 任务类型 | 区分异步工作任务,阻塞工作任务和监督器节点. | 关闭和升级规则依赖它. |
| `AsyncWorker` | 异步工作任务 | 可通过取消令牌和 abort(强制终止) 管理的异步任务. | 可以使用普通 async task(异步任务) 关闭语义. |
| `BlockingWorker` | 阻塞工作任务 | `spawn_blocking`(阻塞任务启动) 或其它不可立即 abort(强制终止) 的任务. | 必须有独立关闭策略和升级策略. |
| `Service trait` | 服务特征 | `TaskFactory` 之上的项目自有人体工学适配层. | 不得替换 `TaskFactory` 内核. |
| `service_fn` | 函数适配器 | 把函数适配到服务特征或任务工厂的帮助入口. | 不得隐藏 `TaskContext` 的生命周期能力. |
| `JoinSet` | 任务集合 | Tokio(异步运行时) 中拥有多个任务的集合. | 用于结构化并发和关闭排空. |
| `CancellationToken` | 取消令牌 | 用于传播关闭请求的取消原语. | 父令牌可以取消子令牌,子令牌不得反向取消父令牌. |

## Policy And State(策略和状态)

| Term(术语) | Chinese(中文说明) | Definition(定义) | Usage Rule(使用规则) |
|---|---|---|---|
| control plane | 控制面 | 处理生命周期命令,状态查询和治理决策的平面. | 不承载高频业务消息. |
| data plane | 数据面 | 业务任务自己的高频数据处理路径. | 不得被 supervisor core(监督器核心) 接管. |
| registry | 注册表 | 保存 child spec(子任务规格) 和 runtime state(运行时状态) 的索引. | 负责当前运行时所有权. |
| state plane | 状态平面 | 暴露当前状态的读取平面. | 回答当前真实状态,不表示事件历史. |
| `SupervisorState` | 监督器状态 | 当前监督树状态的只读模型. | 正式命名,不得使用 `*Snapshot` 或 `*View` 命名. |
| `current_state` | 当前状态 | `SupervisorHandle` 上的当前状态查询命令. | 不得提供 `snapshot()` 查询方法. |
| `ChildState` | 子任务状态(历史叙述轴) | `001` 早期叙述常用的 child 生命周期状态名族, 仍可在事件载荷, 指标文案或迁移说明中出现. | 现行主轴是 `ChildRuntimeRecord` 与 `ManagedChildState`, 权威路径见 **`specs/004-3-child-runtime-state-control`**. 读者把 **`ChildState`** **理解为历史标签或托管展示用词**, **勿**再把 **`ChildState`** **当成与运行时事实并行的第二条权威写入面**. **新规格不得再发明第三套公开子任务状态命名族**. |
| `ChildRuntimeRecord` | 子任务运行状态记录 | 由 **`ChildRuntimeState`** **通过 **`to_record`** **导出的结构化公开读出模型**, 携带 **`generation`(代次)** **`attempt`(尝试)** 等字段, 供 **`ChildControlResult`**, **`current_state`** 与 **`dashboard` / IPC(进程间通信)** 对齐. | 权威真源仍是 **`runtime`** 侧的 **`ChildRuntimeState`**. **`ChildRuntimeRecord`** **只承载可越过模块边界的快照语义**, **规程以 **`004-3`** **`spec.md`** **与 **`contracts`** **为准**. |
| `ChildRuntimeState` | 子任务运行状态记录(运行时) | **`runtime`** **模块持有的可变账本**, **`activate_instance`** **等钩子把活动尝试装订到寄存状态**. | **`pub`** **公开面不得另起平行类型顶替本语义**. **字段与 **`stop_state`** **推进见 **`004-3`**. |
| `ManagedChildState` | 受管子任务状态 | **操作者与 **`dashboard`** **看到的投影态**, 由 **`contracts`** **Operation Mapping(操作映射)** **表依据 **`operation`** **从 **`ChildControlResult`** 或 **`ChildRuntimeRecord`** 推导. | **审计与 **`IPC`** **展示必须能保持与 **`ChildRuntimeRecord`** **字段一一可追溯**, **`004-3`** **`contracts/child-runtime-state-control.md`** **为裁决来源**. |
| `ChildControlResult` | 子任务控制结果 | **诸如 **`PauseChild`**, **`RemoveChild`**, **`QuarantineChild`** **等命令的结构化结果载体**, **替代历史 **`CommandResult::ChildState`** 枚举支路**. | **`pub`** **层不得为了所谓兼容而把 **`ChildState`** **命名体系重新塞进 **`CommandResult`**. **全部字段语义由 **`004-3`** **冻结**. |
| `TaskExit` | 任务退出 | 单次运行的退出分类. | 不得只用字符串表达. |
| `TaskFailureKind` | 任务失败类别 | 策略引擎使用的类型化失败类别. | 包含 recoverable(可恢复),fatal config(致命配置),panic(恐慌) 等类别. |
| `SupervisionStrategy` | 监督策略 | 失败后的重启范围策略. | 核心策略包含 `OneForOne`,`OneForAll` 和 `RestForOne`. |
| `OneForOne` | 一对一 | 只重启失败 child(子任务) 的策略. | 不影响 sibling(同级任务). |
| `OneForAll` | 一对全部 | 任意 child(子任务) 失败后重启整个范围的策略. | 必须先停止整组,再按定义顺序启动. |
| `RestForOne` | 从失败处开始 | 重启失败 child(子任务) 以及之后定义的 child(子任务). | 不影响失败节点之前的 child(子任务). |
| `GroupStrategy` | 分组策略 | 基于 child tag(子任务标签) 限定重启范围的策略覆盖. | 优先级高于 supervisor-wide strategy(监督器全局策略). |
| `ChildStrategyOverride` | 子任务级覆盖 | 针对单个 child(子任务) 的策略,预算和升级覆盖. | 优先级高于 group strategy(分组策略). |
| `RestartLimit` | 重启次数限制 | 策略执行计划使用的最大重启次数和统计窗口. | 不替代 `RestartPolicy`,只约束策略治理. |
| `EscalationPolicy` | 升级策略 | 本地重启治理无法继续时的后续动作. | 包含 `EscalateToParent`,`ShutdownTree` 和 `QuarantineScope`. |
| `EscalateToParent` | 升级到父级 | 把失败交给父 supervisor(监督器) 处理. | 用于本地范围无法继续治理的场景. |
| `ShutdownTree` | 关闭整棵树 | 关闭当前 supervisor tree(监督树). | 既可以是控制命令结果,也可以是升级策略动作. |
| `QuarantineScope` | 隔离范围 | 隔离本次计划选中的 child scope(子任务范围). | 用于阻止同一范围继续自动重启. |
| `DynamicSupervisorPolicy` | 动态监督器策略 | 控制运行时 dynamic child manifest(动态子任务清单文本) 添加的开关和数量上限. | `add_child` 必须先执行该策略. |
| `StrategyExecutionPlan` | 策略执行计划 | child exit(子任务退出) 后合并策略,分组,覆盖,预算和升级规则得到的计划. | runtime control loop(运行时控制循环) 必须消费它. |
| `restart_execution_plan` | 重启执行计划函数 | 根据 `SupervisorTree` 和 `SupervisorSpec` 构造 `StrategyExecutionPlan`. | 策略选择逻辑必须集中在这里. |
| `restart_plan` | 重启计划事件 | runtime lifecycle event(运行时生命周期事件) 中记录策略执行计划的事件名. | 用于观察选中的 strategy(策略),group(分组) 和 scope(范围). |
| `RestartPolicy` | 重启策略 | 决定任务退出后是否重启的策略. | 包含 `Permanent`,`Transient` 和 `Temporary`. |
| `Permanent` | 永久 | 正常退出或异常退出后都重启. | 适合核心协调类 worker(工作任务). |
| `Transient` | 瞬时 | 异常退出,panic(恐慌),timeout(超时) 或 unhealthy(不健康) 后重启. | 适合网络连接类 worker(工作任务). |
| `Temporary` | 临时 | 任务退出后永不自动重启. | 适合一次性任务. |
| `BackoffPolicy` | 退避策略 | 重启延迟的增长和重置规则. | 必须支持测试关闭或确定性控制 jitter(抖动). |
| jitter | 抖动 | 给退避延迟加入的随机扰动. | 测试中必须可关闭或确定化. |
| `reset_after` | 稳定后重置 | 稳定运行一段时间后重置重启计数. | 防止长期运行任务被历史失败拖累. |
| `MeltdownPolicy` | 熔断策略 | 限制短时间内失败或重启次数的策略. | 包含 child-level(子任务级) 和 supervisor-level(监督器级) 阈值. |
| fuse | 熔断器 | 对重启风暴进行截止的保护边界. | 超限后进入隔离或父级升级. |
| quarantine | 隔离 | 阻止 child(子任务) 继续自动重启的终态治理状态. | 需要操作者显式介入. |
| `RestartDecision` | 重启决策 | 策略引擎输出的明确决定. | 包含不重启,延迟重启,隔离,升级和关闭整棵树. |
| `ReadinessPolicy` | 就绪策略 | child(子任务) 何时可以进入 ready(已就绪) 的规则. | 支持 immediate readiness(立即就绪) 和 explicit readiness(显式就绪). |
| heartbeat | 心跳 | child(子任务) 证明自己仍然健康的低频信号. | 不等于 ready(已就绪). |
| `HealthPolicy` | 健康策略 | heartbeat interval(心跳间隔) 和 stale threshold(过期阈值). | 过期后必须按策略处理. |

## Shutdown And Observability(关闭和可观测性)

| Term(术语) | Chinese(中文说明) | Definition(定义) | Usage Rule(使用规则) |
|---|---|---|---|
| shutdown protocol | 关闭协议 | 停止监督树的完整流程. | 对外是 cancel-then-abort(先取消后强制终止),对内是四阶段关闭. |
| graceful timeout | 优雅关闭超时 | 等待任务自行退出的最大时间. | 超时后进入强制终止或升级路径. |
| abort wait | 强制终止等待 | abort(强制终止) 后等待任务排空的时间. | 不适用于不可立即终止的阻塞任务假设. |
| four-stage shutdown | 四阶段关闭 | request stop(请求停止),graceful drain(优雅排空),abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账). | 关闭完成前必须全部执行. |
| request stop | 请求停止 | 发出关闭请求并传播取消令牌的阶段. | 是四阶段关闭第一阶段. |
| graceful drain | 优雅排空 | 等待 child(子任务) 自行退出的阶段. | 是四阶段关闭第二阶段. |
| abort stragglers | 强制终止拖尾任务 | 强制终止超时异步任务的阶段. | 是四阶段关闭第三阶段. |
| reconcile | 状态对账 | 统一 registry(注册表),current state(当前状态),metrics(指标) 和 event journal(事件日志缓冲区) 的阶段. | 是四阶段关闭最后阶段. |
| shutdown without orphaned tasks | 关闭后不留下孤儿任务 | root shutdown(根关闭) 完成后 supervisor(监督器) 不再拥有悬挂任务. | 正式术语,不得写成含糊的 `No-Orphan Shutdown`. |
| lifecycle event | 生命周期事件 | 描述一次状态迁移或治理事实的事件. | 必须包含 `When`,`Where` 和 `What`. |
| `SupervisorEvent` | 监督器事件 | 生命周期事件的项目自有结构. | 必须可序列化并携带关联标识. |
| `When` | 何时 | 事件时间维度. | 包含墙钟时间,单调时间,监督器运行时长,以及运行实例身份在实现中的编码字段. |
| `Where` | 何处 | 事件位置维度. | 包含路径,父子标识,任务名和源位置. |
| `What` | 发生内容 | 事件内容维度. | 包含状态迁移,退出原因,失败类别和策略决定. |
| event stream | 事件流 | 订阅者读取生命周期事件的流. | 回答历史顺序. |
| event bus | 事件总线 | 向多个消费者分发生命周期事件的内部边界. | 消费者滞后不得阻塞治理. |
| event journal | 事件日志缓冲区 | 固定容量的最近生命周期事件记录. | 用于事故诊断和 `RunSummary`. |
| `RunSummary` | 运行摘要 | 故障升级或关闭后的运行诊断摘要. | 必须包含最近事件,失败原因,重启次数,关闭原因和最终状态. |
| observability pipeline | 可观测性管线 | 把生命周期事实同步到日志,追踪,指标,审计和测试记录器的边界. | 不绑定具体 exporter(导出器). |
| structured log | 结构化日志 | 带字段的日志事件. | 必须能和生命周期事件关联. |
| tracing | 结构化追踪 | Rust(编程语言) 生态中的 span/event(追踪范围和事件) 机制. | 每个子任务运行实例必须有 span(追踪范围). |
| metrics | 指标 | 可采集的计数器,仪表和直方图. | label(标签) 必须低基数. |
| audit event | 审计事件 | 记录控制命令请求和结果的事件. | 每个已接受控制命令都必须生成. |
| test recorder | 测试记录器 | 测试可读取的可观测性信号记录器. | 用于断言信号缺失,滞后和关联关系. |

## Configuration, Quality, And Release(配置,质量和发布)

| Term(术语) | Chinese(中文说明) | Definition(定义) | Usage Rule(使用规则) |
|---|---|---|---|
| rust-config-tree | 集中配置树 | 项目的集中配置加载工具. | 必须作为 supervisor(监督器) 配置唯一入口. |
| `rust-config-tree` | 集中配置树软件包 | Cargo(构建工具) 依赖中的集中配置树 crate(包). | 必须使用 v0.1.9. |
| rust-config-tree v0.1.9 | 集中配置树版本 | 本功能要求使用的 rust-config-tree(集中配置树) 版本. | 规格,计划和 `Cargo.toml` 必须一致. |
| YAML | 数据序列化格式 | rust-config-tree(集中配置树) 的主配置文件格式. | 示例路径必须使用 `*.yaml`. |
| `*.yaml` | YAML 文件后缀 | YAML(数据序列化格式) 配置文件路径模式. | supervisor(监督器) 主配置必须使用这个后缀. |
| `ConfigState` | 配置状态 | 配置加载,校验和派生后的不可变状态. | 不得命名为 `ConfigSnapshot`. |
| configuration schema | 配置模式 | 配置字段和校验规则集合. | 不得在代码中提供运行时行为的硬编码默认值. |
| include tree | 包含树 | 配置文件之间的包含关系. | 必须能追踪到 `ConfigState`. |
| runtime tunable constant | 运行时可调常量 | 影响运行时行为的阈值,窗口,超时,退避,抖动,容量,开关,预算和策略值. | 必须来自 rust-config-tree(集中配置树) v0.1.9 的 YAML(数据序列化格式) 配置. |
| hard-coded constant | 硬编码常量 | 直接写在源码中的运行时配置值或隐式回退值. | 禁止用于生产运行时行为. |
| hard-coded constant check | 硬编码常量检查 | 验证源码没有把运行时可调常量写死的质量门禁. | 缺失配置必须失败,不得使用硬编码值补齐. |
| documentation sync | 文档同步 | 代码,契约,示例,手册和文档保持一致的检查. | public API(公开接口) 变化时必须同步. |
| glossary | 词汇表 | 专业术语和统一解释的独立文档. | 本文件是正式词汇来源. |
| code documentation | 代码文档 | module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档) 和 doctest(文档测试). | 源码注释和 rustdoc(代码文档注释) 必须使用英文. |
| module boundary | 模块边界 | 每个模块的职责和公开表面. | `mod.rs` 只能包含 `pub mod <mod_name>;`. |
| top-level directory module | 顶层目录模块 | 核心模块直接位于 `src/<module>/` 的源码布局. | 不得保留 `src/supervision/` 中间层. |
| source layout check | 源码布局检查 | 验证源码目录结构符合顶层目录模块规则的质量门禁. | 必须拒绝 `src/supervision/` 中间层和 `src/<module>.rs` 平铺模块文件. |
| absolute import | 绝对导入 | 使用 `crate::` 或外部 crate name(软件包名) 的导入方式. | 不得使用 `super::` 相对导入. |
| cognitive complexity | 认知复杂度 | 衡量函数控制流理解难度的指标. | 普通函数不超过 15,生命周期调度函数不超过 20. |
| maintainability profile | 可维护性画像 | 模块职责,依赖,状态边界,测试和文档映射. | 用于保持高内聚,低耦合和变更局部. |
| lead agent | 主代理 | 并行开发中负责任务分派,子代理监督,偏差识别和纠偏复核的代理. | 必须监督所有 subagent(子代理) 工作流. |
| subagent | 子代理 | 执行一个或多个 parallel workstream(并行工作流) 的代理. | 不得越过自己的 ownership boundary(所有权边界). |
| agent supervision | 代理监督 | lead agent(主代理) 对 subagent(子代理) 工作进行审查和治理的过程. | 每个子代理工作流必须有监督记录. |
| development drift | 开发偏差 | 子代理输出偏离规格,模块边界,依赖规则,测试规则,文档同步或禁止兼容规则的情况. | 必须在同一 implementation cycle(实现周期) 中纠偏. |
| correction loop | 纠偏循环 | 发现偏差,记录偏差,下达修正,复核结果和关闭偏差的流程. | workstream(工作流) 完成前必须闭环. |
| correction record | 纠偏记录 | 记录开发偏差和纠偏动作的证据. | 必须包含偏差类型,影响范围,纠偏动作,复核结果和最终证据. |
| clean review record | 清洁审查记录 | lead agent(主代理) 确认子代理输出没有偏差的审查证据. | 无偏差的 workstream(工作流) 必须使用它证明完成前审查. |
| lead agent supervision record | 主代理监督记录 | lead agent(主代理) 对 subagent(子代理) 工作流进行分派,审查和复核的证据. | 必须覆盖全部 subagent workstream(子代理工作流). |
| subagent workstream | 子代理工作流 | 由 subagent(子代理) 执行且受 lead agent(主代理) 监督的并行工作流. | 完成前必须经过主代理审查. |
| subagent output | 子代理输出 | subagent(子代理) 在工作流中提交的代码,测试,文档或验收证据. | 必须接受主代理审查. |
| correction action | 纠偏动作 | lead agent(主代理) 要求 subagent(子代理) 修正开发偏差的具体动作. | 必须记录在 correction record(纠偏记录) 中. |
| lead agent supervision check | 主代理监督检查 | 验证主代理已经监督全部子代理工作流的质量门禁. | 未监督或未纠偏时必须失败. |
| SBOM | 软件物料清单 | crate(包) 和依赖组成的机器可读清单. | 发布准备阶段必须生成. |
| CycloneDX JSON | CycloneDX JSON 格式 | SBOM(软件物料清单) 输出格式之一. | 文件路径是 `artifacts/sbom/rust-supervisor.cdx.json`. |
| SPDX JSON | SPDX JSON 格式 | SBOM(软件物料清单) 输出格式之一. | 文件路径是 `artifacts/sbom/rust-supervisor.spdx.json`. |
| crates.io readiness | 发布就绪 | 满足 crates.io(软件包发布平台) 发布约定的检查集合. | 包含 package metadata(软件包元数据),README,LICENSE,CHANGELOG,package list(打包清单) 和 dry-run(试运行). |
| compatibility method | 兼容方法 | 旧接口别名,迁移层,历史行为保留开关,废弃门面,兼容包装函数或第三方 API(接口) 形状复制. | 本项目禁止采用. |
| naming check | 命名检查 | 验证正式代码命名的质量门禁. | 必须拒绝 `*Snapshot`,`*View`,`snapshot()` 查询方法和 `state_view` 模块名. |
| test naming check | 测试命名检查 | 验证测试文件命名后缀的质量门禁. | 所有测试文件必须以 `_test.rs` 结尾. |
| correction loop check | 纠偏循环检查 | 验证开发偏差已经完成纠偏闭环的质量门禁. | 未复核通过的工作流不得标记完成. |

## Backtick Terms(反引号词汇)

本节登记反引号中的类型名,枚举值,方法名,字段名,指标名,路径名,命令名,配置键和测试目标.如果某个词汇已经在前文表格中出现,本节仍可以登记它的分类和使用范围.

### Rust Types And Enum Values(Rust 类型和枚举值)

| Term(术语) | Chinese(中文说明) | Definition(定义) | Usage Rule(使用规则) |
|---|---|---|---|
| `Supervisor` | 监督器运行时入口 | 启动监督树的运行时入口类型. | `Supervisor::start` 使用它. |
| `SupervisorTree` | 监督树类型 | 表达监督器层级结构的类型. | 不得表示普通列表. |
| `SupervisorState` | 监督器状态 | 当前监督树状态的只读模型. | 正式状态类型,不得使用 `*View` 后缀. |
| `SupervisorRuntime` | 监督器运行时 | 一个监督器范围内的运行时所有权集合. | 拥有注册表,控制循环,state store(状态存储) 和关闭协调器. |
| `ChildRuntime` | 子任务运行态 | child(子任务) 启动后的运行记录. | 保存状态,句柄,取消令牌和最近失败. |
| `ChildRuntimeState` | 子任务运行状态记录(运行时) | **`runtime`** **内部可变尝试账本**, **`to_record`** **产出 **`ChildRuntimeRecord`**. | **与 Policy 表同名条目一致**. **生命周期细节见 **`specs/004-3-child-runtime-state-control`**. |
| `ChildRuntimeRecord` | 子任务运行状态记录(公开) | **`control`**, **`runtime`**, **`dashboard`** **之间交换的快照**, **与 **`generation fencing`(代次隔离)** **叙述相容**. | **与 Policy 表同名条目一致**. |
| `ChildControlResult` | 子任务控制结果 | **携带 **`ChildAttemptStatus`** **等族的命令结果汇总结构**. | **与 Policy 表同名条目一致**; **取代历史 **`ChildState`** **命令分支**. |
| `ManagedChildState` | 受管子任务状态 | **面向 **`dashboard`** / **审计的派生态**. | **与 Policy 表同名条目一致**, **映射规则见 **`004-3`** **`contracts`**. |
| `SupervisorId` | 监督器标识 | supervisor(监督器) 的稳定标识. | 不替代 `SupervisorPath`. |
| `RunningInstanceId` | 运行实例标识 | 监督器为同一 `child`(子任务) 的每一次被承认的 `fresh future`(新异步任务) 分配的逻辑编号, 单调变化, 用于把退出报告和观测信号钉到正确的一轮运行上. | 读者向文档与规格统一用本词表达该概念; 不得用 **代次** 或 **尝试** 作为该概念的中文名; 不得用 **epoch**(纪元) 或 Unix 时间戳语义替代. |
| `SupervisorError` | 监督器错误 | supervisor core(监督器核心) 的类型化错误. | 不得用字符串替代. |
| `TaskFailure` | 任务失败 | 任务失败的结构化错误. | 必须带失败类别. |
| `TaskFailureKind` | 任务失败类别 | 策略决策使用的失败分类. | 必须可测试. |
| `PolicyDecision` | 策略决定 | 策略引擎输出或事件携带的决定. | 必须可以追踪到输入原因. |
| `PolicyEngine` | 策略引擎 | 读取退出原因和策略并输出决定的组件. | 不得从字符串推断策略. |
| `EventTime` | 事件时间 | `When`(何时) 的结构化时间数据. | 包含墙钟时间,单调时间,监督器运行时长,以及运行实例身份在实现中的编码字段. |
| `EventLocation` | 事件位置 | `Where`(何处) 的结构化位置数据. | 包含路径,父子标识和源位置. |
| `EventPayload` | 事件内容 | `What`(发生内容) 的结构化数据. | 表达状态迁移或治理事实. |
| `ControlCommand` | 控制命令 | 可审计运行时命令. | 每个已接受命令必须生成审计事件. |
| `HealthPolicy` | 健康策略 | 心跳和过期阈值配置. | 与 readiness(就绪) 分离. |
| `Heartbeat` | 心跳记录 | 任务健康信号记录. | 过期后触发 unhealthy(不健康) 处理. |
| `ShutdownPolicy` | 关闭策略 | 关闭超时和升级配置. | blocking worker(阻塞工作任务) 必须单独配置. |
| `ShutdownPhase` | 关闭阶段 | 四阶段关闭中的当前阶段. | 必须可观察. |
| `CodingStandard` | 编码标准 | 文档,模块入口和导入路径规则集合. | 编码阶段必须执行. |
| `CognitiveComplexityBudget` | 认知复杂度预算 | 函数复杂度阈值和拆分记录. | 超限必须拆分. |
| `MaintainabilityProfile` | 可维护性画像 | 模块职责,依赖,测试和文档映射. | 用于保持变更局部. |
| `ReleasePackage` | 发布包 | crates.io(软件包发布平台) 发布准备模型. | 真实上传不属于自动完成条件. |
| `SBOMArtifact` | 软件物料清单产物 | SBOM(软件物料清单) 输出模型. | 必须包含 CycloneDX JSON 和 SPDX JSON. |
| `DocumentationSet` | 文档集合 | 手册,docs(文档),quickstart(快速开始),契约和词汇表集合. | 必须同步更新. |
| `DocumentationSyncCheck` | 文档同步检查 | 检查文档和代码是否一致的质量门禁. | public API(公开接口) 变化时必须运行. |
| `GlossarySet` | 词汇表集合 | 专业词汇和反引号词汇集合. | 必须覆盖所有反引号词汇. |
| `ExampleSuite` | 示例套件 | examples(示例程序) 的集合. | 必须覆盖学习场景. |

### Source Layout Terms(源码布局词汇)

| Term(术语) | Chinese(中文说明) | Definition(定义) | Usage Rule(使用规则) |
|---|---|---|---|
| `src/lib.rs` | 包入口文件 | crate(包) 的公开模块入口文件. | 只能包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明. |
| `src/<module>/` | 顶层模块目录 | 核心模块在 `src/` 下的目录形式. | 每个核心模块必须使用这种目录形式. |
| `src/<module>/mod.rs` | 模块入口文件 | 顶层模块目录内的模块声明入口. | 只能包含 `pub mod <mod_name>;` 声明. |
| `src/<module>/tests/*_test.rs` | 模块单元测试路径 | 顶层模块目录内的单元测试文件路径. | unit test(单元测试) 必须放在被测模块自己的 tests(测试) 目录. |
| `src/<module>.rs` | 平铺模块文件 | 直接放在 `src/` 下的单文件模块形态. | 核心模块禁止使用这种形态. |
| `src/supervision/` | 监督中间层目录 | 曾经考虑过的监督器中间层目录. | 当前计划禁止保留这个中间层. |
| `source_layout_uses_top_level_directory_modules` | 顶层目录模块源码布局测试 | 验证源码是否使用 `src/<module>/` 布局的测试目标. | quickstart(快速开始) 中作为质量门禁测试. |
| `no_supervision_directory_layer_exists` | 无监督中间层测试 | 验证源码不存在 `src/supervision/` 的测试目标. | quickstart(快速开始) 中作为质量门禁测试. |
| `no_flat_top_level_module_files_exist` | 无平铺顶层模块文件测试 | 验证源码不存在 `src/<module>.rs` 的测试目标. | quickstart(快速开始) 中作为质量门禁测试. |

### States, Events, And Decisions(状态,事件和决定)

| Term(术语) | Chinese(中文说明) | Definition(定义) | Usage Rule(使用规则) |
|---|---|---|---|
| `Declared` | 已声明 | child(子任务) 已声明但未启动. | 初始状态. |
| `Starting` | 正在启动 | child(子任务) 正在启动. | 应发送启动事件. |
| `Running` | 运行中 | child(子任务) 已进入运行. | 不等同于 ready(已就绪). |
| `Ready` | 已就绪 | child(子任务) 已可对外提供业务能力. | explicit readiness(显式就绪) 必须显式报告. |
| `Restarting` | 正在重启 | child(子任务) 正在按策略重启. | 必须带策略决定. |
| `Paused` | 已暂停 | child(子任务) 运行治理暂停. | 重复暂停必须幂等. |
| `Quarantined` | 已隔离 | child(子任务) 被隔离. | 不再自动重启. |
| `ShuttingDown` | 正在关闭 | child(子任务) 正在关闭. | 必须参与关闭协议. |
| `Stopped` | 已停止 | child(子任务) 已停止. | 对自动重启来说是终态之一. |
| `Failed` | 已失败 | child(子任务) 已失败. | 必须带失败原因. |
| `Completed` | 已完成 | 任务正常完成. | 策略决定是否重启. |
| `TimedOut` | 已超时 | 任务或阶段超过预算. | 必须产生可观测性信号. |
| `Unhealthy` | 不健康 | 心跳过期或健康检查失败. | 必须按策略处理. |
| `Panicked` | 已恐慌 | 任务发生 panic(恐慌). | 必须分类为 `Panic`. |
| `DoNotRestart` | 不重启 | 策略决定不重启. | 必须记录原因. |
| `RestartAfter` | 延迟后重启 | 策略决定经过退避后重启. | 必须记录退避时长. |
| `RestartAfter(duration)` | 指定延迟后重启 | 带明确 duration(时长) 的重启决定. | 只在数据模型中表达具体形态. |
| `Quarantine` | 隔离决定 | 策略决定进入隔离. | 与状态 `Quarantined` 区分. |
| `EscalateToParent` | 升级到父级 | 策略决定向父 supervisor(监督器) 升级. | 必须携带原因. |
| `ShutdownTree` | 关闭整棵树 | 策略决定关闭监督树. | 必须触发四阶段关闭. |
| `ChildStarting` | 子任务开始启动事件 | child(子任务) 开始启动的事件. | 属于生命周期事件. |
| `ChildRunning` | 子任务运行事件 | child(子任务) 进入运行中的事件. | 不表示已就绪. |
| `ChildReady` | 子任务就绪事件 | child(子任务) 首次就绪的事件. | explicit readiness(显式就绪) 必须产生. |
| `ChildHeartbeat` | 子任务心跳事件 | child(子任务) 发送心跳的事件. | 应保持低频. |
| `ChildFailed` | 子任务失败事件 | child(子任务) 失败的事件. | 必须携带失败类别. |
| `ChildPanicked` | 子任务恐慌事件 | child(子任务) panic(恐慌) 的事件. | 必须触发策略评估. |
| `BackoffScheduled` | 已安排退避事件 | 系统安排延迟重启的事件. | 必须携带退避时长. |
| `ChildRestarting` | 子任务正在重启事件 | child(子任务) 即将重启的事件. | `RunningInstanceId`(运行实例标识) 在进入新一轮运行前必须前进或等价地更新. |
| `ChildRestarted` | 子任务已重启事件 | child(子任务) 完成重启的事件. | 必须能关联被替换的 `RunningInstanceId`(运行实例标识). |
| `ChildQuarantined` | 子任务已隔离事件 | child(子任务) 进入隔离的事件. | 必须说明触发原因. |
| `ChildStopped` | 子任务已停止事件 | child(子任务) 停止的事件. | 关闭或自然结束都可以产生. |
| `ChildUnhealthy` | 子任务不健康事件 | child(子任务) 健康检查失败的事件. | 必须按策略处理. |
| `Meltdown` | 熔断事件 | supervisor(监督器) 或 child(子任务) 超过失败阈值的事件. | 必须可诊断. |
| `ShutdownRequested` | 已请求关闭事件 | shutdown(关闭) 请求已被接受的事件. | 必须携带原因. |
| `ShutdownPhaseChanged` | 关闭阶段变化事件 | 四阶段关闭阶段变化的事件. | 必须记录当前阶段. |
| `ShutdownCompleted` | 关闭完成事件 | 关闭流程完成的事件. | 必须在状态对账后产生. |
| `CommandAccepted` | 命令已接受事件 | 控制命令已被接收. | 属于审计事件. |
| `CommandCompleted` | 命令已完成事件 | 控制命令执行完成. | 必须携带结果. |
| `SubscriberLagged` | 订阅者滞后事件 | 事件消费者落后. | 不得阻塞生命周期治理. |

### Commands, Fields, Metrics, And Paths(命令,字段,指标和路径)

| Term(术语) | Chinese(中文说明) | Definition(定义) | Usage Rule(使用规则) |
|---|---|---|---|
| `add_child` | 添加子任务 | 运行时添加 child(子任务) 的控制命令. | 必须校验并审计. |
| `remove_child` | 移除子任务 | 运行时移除 child(子任务) 的控制命令. | 必须先关闭再删除注册表记录. |
| `restart_child` | 重启子任务 | 对 child(子任务) 发起受策略约束的重启命令. | 必须记录 `RunningInstanceId`(运行实例标识) 与策略决定. |
| `pause_child` | 暂停子任务 | 暂停 child(子任务) 治理的控制命令. | 必须幂等. |
| `resume_child` | 恢复子任务 | 恢复 child(子任务) 治理的控制命令. | 必须幂等. |
| `quarantine_child` | 隔离子任务 | 手动隔离 child(子任务) 的控制命令. | 必须阻止自动重启. |
| `shutdown_tree` | 关闭监督树 | 对监督树执行四阶段关闭的命令. | 重复执行必须返回当前关闭结果. |
| `subscribe_events` | 订阅事件 | 订阅生命周期事件流的命令. | 不返回 current state(当前状态). |
| `id` | 标识字段 | 配置和模型中的稳定标识字段. | 不得为空. |
| `name` | 名称字段 | 便于阅读的名称字段. | 不作为唯一标识. |
| `kind` | 类型字段 | child(子任务) 类型字段. | 指向任务类型. |
| `factory` | 工厂字段 | 指向任务工厂或嵌套监督器规格. | worker(工作任务) 必须提供. |
| `restart_policy` | 重启策略字段 | child(子任务) 重启策略配置键. | 可被默认值填充. |
| `shutdown_policy` | 关闭策略字段 | child(子任务) 关闭策略配置键. | 必须支持阻塞任务边界. |
| `health_policy` | 健康策略字段 | child(子任务) 健康策略配置键. | 不能替代就绪策略. |
| `readiness_policy` | 就绪策略字段 | child(子任务) 就绪策略配置键. | 支持立即和显式就绪. |
| `backoff_policy` | 退避策略字段 | child(子任务) 退避策略配置键. | 测试必须可确定化. |
| `dependencies` | 依赖字段 | child(子任务) 的启动依赖集合. | 必须引用同一棵树内节点. |
| `tags` | 标签字段 | 低基数筛选标签. | 不得放入无界用户输入. |
| `criticality` | 关键程度字段 | child(子任务) 重要性字段. | 影响策略决定. |
| `config_version` | 配置版本字段 | 标识 `ConfigState`(配置状态) 的版本. | 事件,日志和 current state(当前状态) 必须携带. |
| `command_id` | 命令标识字段 | 控制命令的唯一标识. | 审计事件必须包含. |
| `requested_by` | 请求者字段 | 控制命令请求者. | 审计事件必须包含. |
| `reason` | 原因字段 | 控制命令或状态变化的原因. | 不得为空. |
| `target_path` | 目标路径字段 | 控制命令目标 `SupervisorPath`. | 审计事件必须包含. |
| `accepted_at` | 接受时间字段 | 控制命令被接受的时间. | 审计事件必须包含. |
| `result` | 结果字段 | 控制命令执行结果. | 命令完成时必须记录. |
| `correlation_id` | 关联标识字段 | 关联同一生命周期事实的标识. | 事件,日志,指标和审计应共享. |
| `sequence` | 序号字段 | 单调事件序号. | 用于排序和关联. |
| `when` | 何时字段 | JSON(数据交换格式) 或结构体中的事件时间字段. | 对应 `When`. |
| `where` | 何处字段 | JSON(数据交换格式) 或结构体中的事件位置字段. | 对应 `Where`. |
| `what` | 发生内容字段 | JSON(数据交换格式) 或结构体中的事件内容字段. | 对应 `What`. |
| `supervisor_restart_total` | 监督器重启总数指标 | child(子任务) 重启次数计数器. | label(标签) 必须低基数. |
| `supervisor_child_state` | 子任务状态指标 | 当前 child(子任务) 状态指标. | 不得包含错误全文. |
| `supervisor_child_uptime_seconds` | 子任务运行时长指标 | child(子任务) 运行时长指标. | 单位为秒. |
| `supervisor_backoff_seconds` | 退避秒数指标 | 重启退避时长指标. | 单位为秒. |
| `supervisor_healthcheck_latency_seconds` | 健康检查延迟指标 | 健康检查延迟指标. | 单位为秒. |
| `supervisor_meltdown_total` | 熔断总数指标 | 熔断次数计数器. | 必须低基数. |
| `supervisor_shutdown_duration_seconds` | 关闭耗时指标 | 关闭流程耗时指标. | 单位为秒. |
| `supervisor_event_lag_total` | 事件滞后总数指标 | 订阅者滞后或丢弃事件计数器. | 不得阻塞生命周期治理. |
| `supervisor_config_version` | 监督器配置版本指标 | 当前配置版本指标. | 来源于 `ConfigState`. |
| `examples/config/supervisor.yaml` | 示例配置文件 | quickstart(快速开始) 的 YAML(数据序列化格式) 配置路径. | 不得改成 TOML(配置格式) 主配置. |
| `supervisor_tree_story` | 监督树故事示例 | 覆盖多子任务声明,树顺序和重启范围的 example(示例程序). | 用于学习复杂树声明. |
| `runtime_control_story` | 运行时控制故事示例 | 覆盖运行中控制命令和事件订阅的 example(示例程序). | 用于学习操作员控制流. |
| `policy_failure_matrix` | 策略失败矩阵示例 | 覆盖任务退出分类,重启策略和熔断跟踪的 example(示例程序). | 用于学习策略决策. |
| `diagnostic_replay` | 诊断回放示例 | 覆盖事件日志,指标样本和运行摘要的 example(示例程序). | 用于学习诊断回放. |
| `src/tests/*_test.rs` | 集成测试路径模式 | integration test(集成测试) 文件位置和后缀. | 所有集成测试必须匹配. |
| `tests/*_test.rs` | 模块测试路径模式 | 模块自身 tests(测试) 目录中的单元测试文件模式. | 所有单元测试必须匹配. |
| `_test.rs` | 测试文件后缀 | 所有测试文件必须使用的后缀. | 不得使用其它后缀. |
