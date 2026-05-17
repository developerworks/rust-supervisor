# Feature Specification (功能规格): 压力故障混沌与浸泡稳定性

**Feature Branch (功能分支)**: `[006-7-chaos-soak-reliability]`
**Created (创建日期)**: 2026-05-17
**Status (状态)**: Draft (草稿)
**Input (输入)**: 本规格对应第五序列里程碑: 建立专门的 chaos test(混沌测试) 和 soak test(长稳测试). 场景包括: 子任务 panic(崩溃), 子任务永久阻塞, 子任务忽略取消, 子任务快速失败一万次, event subscriber(事件订阅者) 极慢, command channel(命令通道) 塞满, dashboard IPC(看板进程间通信) 连接风暴, socket(套接字) 路径被占用, relay(中继) 重启, 系统时钟回拨, Tokio runtime(异步运行时) 饥饿. 浸泡报表必须给出 tail latency(尾部延迟), memory growth(内存增长), FD(文件描述符) growth, event loss(事件丢失), shutdown success rate(关闭成功率) 等数值阈值.

## Dependency Note (依赖说明)

本切片验收依赖 006-3 至 006-5 给出的可对账控制平面语义. 混沌 harness(线束) 代码可以落在独立测试 crate(包), 但必须与本仓库监督语义所在 Git commit hash(提交哈希) 锁定在同一指针.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### User Story 1 (用户故事一) - 已知故障波形可复跑 (Priority (优先级): P1)

可靠性工程师需要仓库登记的每种波形都能通过 cargo(Rust 构建工具) 子命令或 Makefile(构建描述文件) 目标一键复跑, 结束时吐出 JSON 裁定文件.

**Why this priority (为什么是这个优先级)**: 不能复跑的波形等于没有波形.

**Independent Test (独立测试)**: CI 夜间任务调用 chaos_suite(示例目标名). 比对退出码与 ChaosScenario(混沌场景) 期望枚举.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 子任务在 60 秒窗口内触发万次快速崩溃脚本, **When (当)** 场景收尾报告写出, **Then (则)** restart budget(重启预算) 占用曲线与控制循环 latency(延迟) 直方图必须落在文档阈值盒子里. 监督器进程自身 panic(崩溃) 计数保持 0.
2. **Given (假设)** 事件订阅回调被人为限速到 slow_consumer_ms(示例字段名), **When (当)** 高频事件泵持续 10 分钟, **Then (则)** 背压策略分支必须与 006-5 默认规格一致. 核心监督状态仍能前进或进入文档写明的降级停机. journal(事件日志) 缺口计数只能为 0 或者命中允许的显式 discard(丢弃) 计数并在报表写明豁免编号.

### User Story 2 (用户故事二) - 浸泡产出尾迹与资源曲线 (Priority (优先级): P1)

性能工程师需要在不少于 24h(二十四小时) 连续窗外读取 p99 latency(九十九分位延迟) 曲线, 常驻内存 RSS(常驻集大小) 斜率, FD(文件描述符) 计数上沿, 关停 success_ratio(示例字段名) 下限对照表.

**Why this priority (为什么是这个优先级)**: 工业交付通常把 24h(二十四小时) 窗外当作最低背书样本长度.

**Independent Test (独立测试)**: 浸泡脚本结束后 SoakReport(浸泡报告) Markdown 必须在 CI 归档目录留下哈希指针.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 负载在窗外维持合成稳态流量脚本, **When (当)** SoakReport(浸泡报告) 生成动作触发, **Then (则)** 每一条 SLO(服务等级目标) 判定必须附带数值曲线 PNG 或 CSV. 越界条目打上 blocking(阻断) 标记或豁免工单编号.

### User Story 3 (用户故事三) - IPC 风暴与中继生命周期 (Priority (优先级): P2)

安全兼可靠性评审需要劣质连接洪流与中继崩溃不能把监督核心拖进无限阻塞.

**Why this priority (为什么是这个优先级)**: 本地 IPC 往往是值班第一反应入口. 一旦被拖死会失去止血窗口.

**Independent Test (独立测试)**: 同时发起 ipc_stress_clients(示例脚本名). 抓取 accept(接受) 队列长度 metrics(指标).

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 并发劣质握手冲击本地 IPC socket(套接字文件), **When (当)** rate limit(速率限制) 计数跨过阈值, **Then (则)** 合法客户端仍能完成握手, 或在 structured error(结构化错误) 里读到资源枯竭枚举. 服务端线程禁止无限卡在 poll(等待可读).
2. **Given (假设)** relay(中继) 进程被 SIGKILL(强制终止) 后由监督器拉起, **When (当)** 会话链路重新对齐完成或超时触发, **Then (则)** 用户可见 dashboard(看板) 状态必须与监督视图在契约阈值内对齐, 否则降级原因段落对用户可读.

### Edge Cases (边界情况)

- 系统时钟向后拨动会扭曲滑动窗口预算. 必须写明单调时钟回退补偿策略, 或者在 release notes(发行说明) 写明不可恢复风险与手工切换剧本.
- 异步运行时层级饥饿若不能直接度量, 必须注入宿主节拍探针, 证明控制循环迭代计数仍在前进.

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 仓库必须维护一套 CI 可直接调用的混沌套件入口. 最少覆盖下列 scenario id(场景标识) 条目各一段剧本输出 JSON 判决书: child_panic_storm, child_block_forever, child_ignore_cancel, rapid_failure_10k, slow_event_subscriber, command_channel_full, ipc_connection_storm, socket_path_contention, relay_crash_loop, clock_step_backward, runtime_starvation_probe. 收尾禁止只靠肉眼扫日志结案.
- **FR-002**: 默认浸泡配置窗外长度不少于 24h(二十四小时). 同窗内需断言 p99 latency(九十九分位延迟) 是否越过阈值, RSS(常驻集大小) 线性回归斜率是否越过阈值, FD(文件描述符) 计数漂移, event_gap_total(示例计数器) 相对 discard_budget(示例), 以及 shutdown_success_ratio(关停成功率). 任一越界必须在 SoakReport(浸泡报告) 标记 blocking(阻断缺陷) 或挂豁免工单编号.
- **FR-003**: 混沌套件与浸泡归档路径必须登记进 specs/006-2-release-supply-chain-gates/spec.md 定义的 QualityGateOutcome(质量闸口结果) 外链列. 禁止仅在个人笔记本跑一次手写邮件就算签字.

### Key Entities (关键实体) _(涉及数据时填写)_

- **ChaosScenario(混沌场景)**: 绑定入口命令 argv(参数向量), 输入夹具路径, 通过与失败阈值枚举, scenario id(场景标识).
- **SoakReport(浸泡报告)**: 窗外曲线 tarball(归档包), 阈值对照 CSV, 告警列表, 豁免指针数组.

## Constitution Alignment (宪章对齐) _(mandatory (必填))_

### Supervision Contract (监督契约)

- **Lifecycle impact (生命周期影响)**: 混沌 harness(线束) 只允许通过测试夹具注入故障. 默认二进制发布特性不允许悄悄改写.

### Rust Boundary and Observability Requirements (Rust 边界和可观察性需求)

- **Module ownership (模块所有权)**: 混沌与浸泡 harness(线束) 源码路径必须与默认库 lib(库目标) 导出表面隔离, 只能通过 dev-dependency(开发依赖) 引用.
- **Compatibility exports (兼容导出)**: None (无).
- **Diagnostics (诊断)**: 每条场景必须有稳定 scenario id(场景标识) 字符串, 并与 semver(语义化版本) 戳并列写入报表头部.

### Chinese Writing (中文写作)

- **Writing language (写作语言)**: 本文档必须使用中文.
- **Term format (术语格式)**: 英文术语必须写成 English(中文说明).
- **Forbidden style (禁止风格)**: 禁止全角标点, 禁止用形容词描写稳定性却不写出阈值百分号或毫秒.

## Success Criteria (成功标准) _(mandatory (必填))_

### Measurable Outcomes (可衡量结果)

- **SC-001**: 用户在输入清单里登记的波形关键字与仓库 ChaosScenario(混沌场景) 表的 scenario id(场景标识) 集合逐项对上号, 覆盖率 100%.
- **SC-002**: 默认 24h(二十四小时) 窗外 p99 latency(九十九分位延迟) 越阈样本数为 0, 或者每笔越阈都绑定可读豁免工单编号.
- **SC-003**: 合成关停样本 100 次里 shutdown_success_ratio(关停成功率) 不低于 99%, 否则不允许合并发布签字.

## Assumptions (假设)

- CI 可为单次浸泡分配连续 24h(二十四小时) 独占 runner(运行器), 或在规格正文接受等价的分片累计算法, 但总观测窗长相加不得低于 24h(二十四小时).
