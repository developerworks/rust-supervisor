# Feature Specification (功能规格): 压力故障混沌与浸泡稳定性

**Feature Branch (功能分支)**: `[006-7-chaos-soak-reliability]`
**Created (创建日期)**: 2026-05-17
**Status (状态)**: Approved (已批准) — Checklist 已完成, 规格同步更新以反映实际语义
**Input (输入)**: 本规格对应第五序列里程碑: 建立专门的 chaos test(混沌测试) 和 soak test(长稳测试). 场景包括: 子任务 panic(崩溃), 子任务永久阻塞, 子任务忽略取消, 子任务快速失败一万次, event subscriber(事件订阅者) 极慢, command channel(命令通道) 塞满, dashboard IPC(看板进程间通信) 连接风暴, socket(套接字) 路径被占用, relay(中继) 重启, 系统时钟回拨, Tokio runtime(异步运行时) 饥饿. 浸泡报表必须给出 tail latency(尾部延迟), memory growth(内存增长), FD(文件描述符) growth, event loss(事件丢失), shutdown success rate(关闭成功率) 等数值阈值.

> **NOTE**: 本规格 draft 阶段的 Checklist(CHK001–CHK034)已全部审查并标记为已批准(见 `checklists/chaos.md`). 以下为根据 Checklist 缺口分析后补充的冻结定义.

## Dependency Note (依赖说明)

本切片验收依赖 006-3 至 006-5 给出的可对账控制平面语义. 混沌 harness(线束) 代码可以落在独立测试 crate(包), 但必须与本仓库监督语义所在 Git commit hash(提交哈希) 锁定在同一指针.

**依赖版本锁定**: 本切片验收时, 以下依赖切片的功能语义必须在同一 `git` commit 树中:

| 依赖切片                                 | 依赖内容                                      | 验证方式                            |
| ---------------------------------------- | --------------------------------------------- | ----------------------------------- |
| `specs/006-3-lifecycle-shutdown-realism` | 关停语义(shutdown_tree_fanout, ShutdownPhase) | CI 编译依赖 + contract 测试         |
| `specs/006-4-restart-policy-production`  | 重启策略与并发承认条款                        | CI 编译依赖                         |
| `specs/006-5-typed-events-observability` | 背压策略(AlertAndBlock / SampleAndAudit)      | data-model.md §BackpressureBehavior |

依赖切片中引用的语义在当前仓库 `006-6-config-dynamic-children` 分支中已全部实现并合并.

**006-2 集成**: 混沌套件与浸泡归档路径必须登记进 `specs/006-2-release-supply-chain-gates/spec.md` 定义的 `QualityGateOutcome` 外链列. 归档格式见下文 ChaosScenario 表和 SoakReport 定义.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### 通用阈值与命名冻结

以下定义适用于本切片全部三个用户故事, 以解决 Checklist 中标识的缺口.

**"panic" 语义定义**: 如无特别说明, spec 中"panic(崩溃)"指 Rust 语言级 panic(通过 `std::panic::catch_unwind` 捕获). 进程级崩溃(segment fault / SIGKILL)使用"crash(进程崩溃)"表述. 控制循环已配置 `std::panic::set_hook` 记录结构化错误并继续执行, 不会因子任务 panic 而终止.

**混沌 harness 隔离策略**: 混沌与浸泡场景的源码路径为独立测试 crate `tests/chaos/`, 仅通过 `[dev-dependencies]` 引用. 不修改 `src/` 下的默认库代码.

**scenario id 命名规范**: 所有 scenario id 使用 snake_case(蛇形命名法). semver 戳从 `Cargo.toml` 的 `version` 字段通过 `env!("CARGO_PKG_VERSION")` 读取.

**覆盖率计算口径**: SC-001 的"覆盖率 100%"指 FR-001 列出的 11 个 scenario id 每个都有一份可执行的剧本(存在性检查), 不要求每个剧本都定义了完整阈值枚举. 阈值完整性通过各 scenario 的 JSON 判决书 schema 保证.

### User Story 1 (用户故事一) - 已知故障波形可复跑 (Priority (优先级): P1)

可靠性工程师需要仓库登记的每种波形都能通过 cargo(Rust 构建工具) 子命令或 Makefile(构建描述文件) 目标一键复跑, 结束时吐出 JSON 裁定文件.

**Why this priority (为什么是这个优先级)**: 不能复跑的波形等于没有波形.

**Independent Test (独立测试)**: CI 夜间任务调用 `cargo test --test chaos_suite`. 比对退出码与 `ChaosScenario`(混沌场景)期望枚举.

**ChaosScenario 场景阈值表**: 下表定义每个 scenario id 的通过条件. 阈值单位为 p99 值(毫秒)或比率(0–1). 所有数值在 macOS 开发者工作站(Apple Silicon, 16GB)上测量.

| scenario id                | 故障注入方式                                                   | 主要阈值                                           | 次要阈值                                 | 期望退出码 |
| -------------------------- | -------------------------------------------------------------- | -------------------------------------------------- | ---------------------------------------- | ---------- |
| `child_panic_storm`        | 通过夹具在 60s 内反复 spawn 并在 1ms 后 panic                  | 监督器 self_panic_count = 0                        | 控制循环 emit 延迟 p99 < 100µs           | 0          |
| `child_block_forever`      | spawn 一个永不返回的 blocking worker                           | 关停阶段在 graceful_timeout + abort_wait 内完成    | 无泄漏 slot                              | 0          |
| `child_ignore_cancel`      | spawn 后忽略 CancellationToken                                 | abort 后 slot 在 abort_wait 内停用                 | 无 dangling handle                       | 0          |
| `rapid_failure_10k`        | 60s 内触发 10_000 次快速失败(fail -> restart -> fail)          | restart_budget 未耗尽(恢复率 > 0)                  | emit 延迟 p99 < 10ms(非背压路径)         | 0          |
| `slow_event_subscriber`    | subscriber 回调人为限速到 100ms/event(通过 `slow_consumer_ms`) | 背压策略分支与 006-5 默认(AlertAndBlock)一致       | journal 缺口计数 = 0 或 ≤ discard_budget | 0          |
| `command_channel_full`     | 快速填充 command channel(mpsc, capacity=256)至满               | send() 返回 `Err(Closed)` 而非无限阻塞             | 控制循环不 panic                         | 0          |
| `ipc_connection_storm`     | 同时发起 1000 个劣质 TCP 握手(随机 payload)                    | 合法客户端握手成功率 100%                          | 服务端 accept 队列 p50 < 1ms             | 0          |
| `socket_path_contention`   | 在占用的 socket 路径上启动 dashboard IPC                       | 返回结构化错误含 `field_path="ipc.path"` 和 `hint` | 不 panic                                 | 0          |
| `relay_crash_loop`         | relay 进程被 SIGKILL 后由监督器拉起 5 次                       | 第 5 次拉起后链路对齐在 10s 内完成                 | dashboard 状态与监督视图一致             | 0          |
| `clock_step_backward`      | 通过夹具将系统时钟向后拨动 10s                                 | 滑动窗口预算不被扭曲(使用 monotonic clock)         | 熔断器状态未意外重置                     | 0          |
| `runtime_starvation_probe` | 注入 tokio::task::yield_now 饥饿循环 30s                       | 控制循环迭代计数在 30s 内持续前进(>0 iter/s)       | emit 延迟 p99 < 100ms                    | 0          |

**冻结命名**: spec 中的以下"示例"字段名已冻结并替换为正式名称. 后续修改必须递增 schema 版本号.

| 原"示例"名                           | 冻结名                   | 类型              | 说明                                      |
| ------------------------------------ | ------------------------ | ----------------- | ----------------------------------------- |
| `chaos_suite(示例目标名)`            | `chaos_suite`            | cargo test target | CI nightly 调用入口                       |
| `slow_consumer_ms(示例字段名)`       | `slow_consumer_ms`       | u64               | subscriber 回调限速(毫秒), 默认 100       |
| `event_gap_total(示例计数器)`        | `event_gap_total`        | u64               | journal 缺口累计计数                      |
| `discard_budget(示例)`               | `discard_budget`         | u64               | 允许的显式丢弃上限, 默认 0                |
| `shutdown_success_ratio(示例字段名)` | `shutdown_success_ratio` | f64 (0–1)         | 关停成功率, SC-003 阈值 ≥ 0.99            |
| `ipc_stress_clients(示例脚本名)`     | `ipc_stress_clients`     | 并发客户端数      | IPC 风暴测试的默认劣质客户端数, 默认 1000 |

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 子任务在 60 秒窗口内触发万次快速崩溃脚本, **When (当)** 场景收尾报告写出, **Then (则)** restart budget(重启预算) 占用曲线与控制循环 latency(延迟) 直方图必须落在 ChaosScenario 阈值表的 `rapid_failure_10k` 行阈值盒子里. 监督器进程自身 panic(崩溃) 计数保持 0.
2. **Given (假设)** 事件订阅回调被人为限速到 `slow_consumer_ms`(默认 100ms/event), **When (当)** 高频事件泵持续 10 分钟, **Then (则)** 背压策略分支必须与 006-5 默认规格一致. 核心监督状态仍能前进或进入文档写明的降级停机. journal(事件日志)缺口计数(`event_gap_total`)只能为 0 或者命中允许的显式 `discard_budget` 计数并在报表写明豁免编号.

### User Story 2 (用户故事二) - 浸泡产出尾迹与资源曲线 (Priority (优先级): P1)

性能工程师需要在不少于 24h(二十四小时) 连续窗外读取 p99 latency(九十九分位延迟) 曲线, 常驻内存 RSS(常驻集大小) 斜率, FD(文件描述符) 计数上沿, 关停 success_ratio(示例字段名) 下限对照表.

**Why this priority (为什么是这个优先级)**: 工业交付通常把 24h(二十四小时) 窗外当作最低背书样本长度.

**Independent Test (独立测试)**: 浸泡脚本结束后 SoakReport(浸泡报告) Markdown 必须在 CI 归档目录留下哈希指针.

**SoakReport 浸泡阈值**: 以下阈值适用于 24h 浸泡窗外测量. 所有数值在目标生产环境等价硬件上测量, 开发者工作站数值仅供参考.

| 指标                     | 阈值(p99 / 上限)         | 测量方式                       | blocking(阻断)标记条件           |
| ------------------------ | ------------------------ | ------------------------------ | -------------------------------- |
| `p99_latency_ms`         | < 50ms                   | 控制循环 emit 延迟, 每秒采样   | 连续 5 个采样窗口越阈            |
| `rss_growth_mb_per_hour` | < 5 MB/h                 | RSS 线性回归斜率, 每小时采样   | 斜率 p99 > 5 MB/h                |
| `fd_count_drift`         | < 10                     | `/dev/fd` 计数每小时差值       | 任一小时增长 > 10                |
| `event_gap_total`        | ≤ discard_budget(默认 0) | journal 条目数与 emit 计数之差 | event_gap_total > discard_budget |
| `shutdown_success_ratio` | ≥ 0.99                   | 合成关停 100 次的成功率        | < 0.99                           |

**SoakReport 格式**: Markdown 格式, 包含:

- 元数据: 测试时间窗(起始/结束 UTC), supervisor commit hash, 硬件配置
- 阈值对照表: 每项指标附 p99/均值/最大值的 CSV 行
- 越界条目: 每项越界附带 blocking 标记或豁免工单编号链接
- 附件: p99_latency 曲线 PNG, RSS 曲线 PNG, FD 计数曲线 PNG
- 归档: 文件 hash(SHA-256) 输出到 CI 归档目录

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 负载在窗外维持合成稳态流量脚本, **When (当)** SoakReport(浸泡报告) 生成动作触发, **Then (则)** 每一条 SLO(服务等级目标) 判定必须附带数值曲线 PNG 或 CSV(见 SoakReport 格式). 越界条目打上 `blocking`(阻断)标记或豁免工单编号.

### User Story 3 (用户故事三) - IPC 风暴与中继生命周期 (Priority (优先级): P2)

安全兼可靠性评审需要劣质连接洪流与中继崩溃不能把监督核心拖进无限阻塞.

**Why this priority (为什么是这个优先级)**: 本地 IPC 往往是值班第一反应入口. 一旦被拖死会失去止血窗口.

**Independent Test (独立测试)**: 同时发起 `ipc_stress_clients`(默认 1000 并发)劣质握手. 抓取 accept(接受) 队列长度 metrics(指标).

**速率限制定义**: IPC 服务端实现基于连接的速率限制, 使用固定窗口(1s) + 令牌桶(容量 100, 恢复率 50/s). 劣质客户端定义为: 握手 payload 不符合 dashboard IPC 协议的合法格式(如缺失 `target_id` 字段或非法 JSON). 合法客户端定义为: 符合 `contracts/typed-event-schema.md` 和 `specs/006-1-platform-docs-ipc-security/` IPC 协议的握手.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 并发劣质握手冲击本地 IPC socket(套接字文件), **When (当)** rate limit(速率限制) 令牌桶耗尽计数跨过阈值, **Then (则)** 合法客户端仍能完成握手, 或在 structured error(结构化错误) 里读到 `ResourceExhausted { resource: "ipc_accept", limit: 100 }` 枚举. 服务端线程禁止无限卡在 `poll`(等待可读)——在劣质客户端断开后 1s 内恢复 accept.
2. **Given (假设)** relay(中继) 进程被 SIGKILL(强制终止) 后由监督器拉起, **When (当)** 会话链路重新对齐在 10s 超时内完成或超时触发, **Then (则)** 用户可见 dashboard(看板) 状态必须与监督视图在契约阈值内对齐(specs/006-1-platform-docs-ipc-security/contracts/ 定义的同步超时), 否则降级原因段落对用户可读.

### Edge Cases (边界情况)

- 系统时钟向后拨动会扭曲滑动窗口预算. 本切片选择使用 `std::time::Instant`(monotonic clock)作为滑动窗口和熔断器的基础计时器, 避免受 wall clock 回退影响. 恢复流程中重启预算追踪器检测到 `Instant` 回退时(前置后差值 < 0), 重置当前窗口计数. 该策略适用于所有使用滑动窗口的组件(failure_window, meltdown, restart_budget). 如果部署环境下 `Instant` 不可靠(如某些容器化场景), 必须写在 release notes 的风险段落中.
- 异步运行时层级饥饿若不能直接度量, 必须通过 tokio 的 `runtime::Metrics` 注入宿主节拍探针(每 100ms 采集一次 `num_alive_tasks` 和 `instruments.poll_count`), 证明控制循环迭代计数仍在前进. 若 `poll_count` 在 5s 内无增长, 控制循环尝试一次 `tokio::task::yield_now()` 并重新测量; 若仍无增长, 输出 `RuntimeStarved` 诊断事件并进入降级模式(暂停非核心事件发射).

**组合故障场景**: 本切片不要求测试多故障同时注入(如 child_panic_storm + ipc_connection_storm 并发). 组合故障的测试推迟至后续切片(如 006-9 或独立可靠性迭代). 本切片范围限定为单故障波形逐一验证.

**浸泡中断恢复**: 如果在浸泡过程中 CI runner 被回收或硬件故障, 该次浸泡视为无效, 不产生 SoakReport. 分片累计算法(见 Assumptions)要求每个分片不少于 8h, 分片间隔不超过 4h, 所有分片在同一个 git commit 上运行.

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 仓库必须维护一套 CI 可直接调用的混沌套件入口. 最少覆盖下列 scenario id(场景标识) 条目各一段剧本输出 JSON 判决书: child_panic_storm, child_block_forever, child_ignore_cancel, rapid_failure_10k, slow_event_subscriber, command_channel_full, ipc_connection_storm, socket_path_contention, relay_crash_loop, clock_step_backward, runtime_starvation_probe. 收尾禁止只靠肉眼扫日志结案.

  JSON 判决书格式:

  ```json
  {
    "scenario_id": "child_panic_storm",
    "semver": "0.1.2",
    "passed": true,
    "thresholds": {
      "self_panic_count": { "value": 0, "limit": 0, "passed": true },
      "emit_latency_p99_us": { "value": 42, "limit": 100, "passed": true }
    },
    "started_at_unix_nanos": 1716000000000000000,
    "duration_ns": 60000000000,
    "error": null
  }
  ```

- **FR-002**: 默认浸泡配置窗外长度不少于 24h(二十四小时). 同窗内需断言 p99 latency(九十九分位延迟) 是否越过阈值, RSS(常驻集大小) 线性回归斜率是否越过阈值, FD(文件描述符) 计数漂移, event_gap_total(示例计数器) 相对 discard_budget(示例), 以及 shutdown_success_ratio(关停成功率). 任一越界必须在 SoakReport(浸泡报告) 标记 blocking(阻断缺陷) 或挂豁免工单编号.
- **FR-003**: 混沌套件与浸泡归档路径必须登记进 specs/006-2-release-supply-chain-gates/spec.md 定义的 QualityGateOutcome(质量闸口结果) 外链列. 禁止仅在个人笔记本跑一次手写邮件就算签字.

### Key Entities (关键实体) _(涉及数据时填写)_

- **ChaosScenario(混沌场景)**: 绑定入口命令 argv(参数向量), 输入夹具路径, 通过与失败阈值枚举, scenario id(场景标识). scenario id 采用 snake_case 命名, 完整集合见 FR-001.
- **SoakReport(浸泡报告)**: 窗外曲线 tarball(归档包), 阈值对照 CSV(见 SoakReport 浸泡阈值表), 告警列表, 豁免指针数组.
- **RateLimiter(速率限制器)**: IPC 服务端连接速率控制, 固定窗口(1s) + 令牌桶(容量 100, 恢复率 50/s). 用于 US3 的 IPC 风暴防护.
- **ClientClassification(客户端分类)**: 合法客户端(符合 dashboard IPC 协议握手) vs 劣质客户端(payload 非法或格式不符). 用于 US3 的合法/劣质区分.

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

- **SC-001**: 用户在输入清单里登记的波形关键字与仓库 ChaosScenario(混沌场景) 表的 scenario id(场景标识) 集合逐项对上号, 覆盖率 100%(存在性检查, 见"覆盖率计算口径"定义).
- **SC-002**: 默认 24h(二十四小时) 窗外 p99 latency(九十九分位延迟) 越阈样本数为 0, 或者每笔越阈都绑定可读豁免工单编号. 测量条件: 合成稳态流量 1000 req/s, p99 在 1s 滑动窗口上计算.
- **SC-003**: 合成关停样本 100 次里 `shutdown_success_ratio`(关停成功率)不低于 0.99, 否则不允许合并发布签字. "关停成功"定义为: `ShutdownCoordinator` 最终阶段为 `Completed`, 且所有 slot 在 abort_wait 内完成停用, 且审计记录完整.

## Assumptions (假设)

- CI 可为单次浸泡分配连续 24h(二十四小时) 独占 runner(运行器), 或在规格正文接受等价的分片累计算法, 但总观测窗长相加不得低于 24h(二十四小时). 分片算法约束: 每个分片不少于 8h, 分片间隔不超过 4h, 所有分片在同一个 git commit 上运行.
- 006-2 的 `QualityGateOutcome` schema 在本切片实现阶段前应已冻结. 如果 006-2 schema 在实现过程中变更, 本切片优先适配新 schema, 不需要等待 006-2 完成.
- 混沌 harness 的构建时间不超过 `cargo test --release` 全量构建时间的 120%. 如果超出, 需增加 CI 缓存策略或拆分测试编译目标.
