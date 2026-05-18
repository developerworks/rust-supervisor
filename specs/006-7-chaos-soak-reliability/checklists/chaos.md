# Chaos & Soak Requirements Quality Checklist(压力故障混沌与浸泡需求质量检查清单)

**Purpose(目的)**: 验证 `006-7-chaos-soak-reliability` 功能规格中混沌测试场景、浸泡稳定性阈值和 IPC 风暴防护需求的质量、完整性与可度量性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: US1(已知故障波形) + US2(浸泡产出尾迹) + US3(IPC 风暴与中继生命周期), 全部 3 个用户故事
**Depth(深度)**: Strict(严格 release gate)
**Gates(关口)**: ChaosScenario 覆盖率 100%, 浸泡阈值量化, 时钟回退补偿策略, 隔离 harness 导出表面

---

## Requirement Completeness(需求完整性)

- [x] CHK001 — FR-001 要求 11 个 scenario id(场景标识) 各有一段剧本输出 JSON 判决书。这 11 个场景的通过/失败判定标准（阈值枚举）是否在 spec 中逐一定义？[Gap, Spec §FR-001]
  - Spec 已补充 ChaosScenario 场景阈值表, 包含 11 个场景的阈值枚举 ✓
- [x] CHK002 — US2 验收场景要求 p99 latency(九十九分位延迟)、RSS(常驻集大小)斜率、FD(文件描述符)漂移、event_gap_total 和 shutdown_success_ratio 五类指标。每类指标的阈值数值是否在 spec 中写明？[Gap, Spec §US2]
  - Spec 已补充 SoakReport 浸泡阈值表, 五类指标均有阈值数值 ✓
- [x] CHK003 — US3 要求 IPC connection storm(连接风暴)场景下"合法客户端仍能完成握手"。合法客户端的判定标准是否在 spec 中定义？[Gap, Spec §US3]
  - Spec 已补充 ClientClassification 实体定义: 合法客户端(符合 dashboard IPC 协议握手) vs 劣质客户端(payload 非法或格式不符) ✓
- [x] CHK004 — Edge Cases 提到"系统时钟向后拨动会扭曲滑动窗口预算"。时钟回退的补偿策略是否在 spec 或依赖文档中写明？[Completeness, Spec §Edge Cases]
  - Spec 已选定方案: 使用 monotonic clock(`std::time::Instant`), 检测到回退时重置当前窗口计数 ✓
- [x] CHK005 — FR-001 的 11 个 scenario id 中, `runtime_starvation_probe` 的注入方式(如通过 tokio::task::yield_now 饥饿循环?)和判定标准(控制循环迭代计数停止增长?)是否在 spec 中定义？[Gap, Spec §FR-001]
  - Spec ChaosScenario 阈值表定义: 注入 tokio 饥饿循环 30s, 判定标准为迭代计数 > 0 iter/s ✓

## Requirement Clarity(需求清晰度)

- [x] CHK006 — US1 验收场景要求"restart budget(重启预算) 占用曲线与控制循环 latency(延迟) 直方图必须落在文档阈值盒子里"。"阈值盒子"的上下界数值和测量单位是否在 spec 中量化？[Clarity, Spec §US1]
  - Spec ChaosScenario 阈值表已量化: rapid_failure_10k 行定义 emit 延迟 p99 < 10ms, restart_budget 恢复率 > 0 ✓
- [x] CHK007 — US3 要求"rate limit(速率限制) 计数跨过阈值"时合法客户端仍能完成握手。rate limit 的阈值和计数周期是否在 spec 或契约中定义？[Clarity, Spec §US3]
  - Spec 已补充 RateLimiter 定义: 固定窗口(1s) + 令牌桶(容量 100, 恢复率 50/s) ✓
- [x] CHK008 — US2 的 SoakReport(浸泡报告)要求"每一条 SLO(服务等级目标) 判定必须附带数值曲线 PNG 或 CSV"。SLO 的判定标准和报告格式是否在 spec 中定义？[Clarity, Spec §US2]
  - Spec 已补充 SoakReport 格式定义: 含元数据、阈值对照表、越界条目、PNG 附件、SHA-256 归档 ✓
- [x] CHK009 — spec 多处使用"示例目标名""示例字段名""示例计数器"标注。这些处于"示例"状态的名称冻结计划或冻结条件是否在 spec 中写明？[Clarity, Spec §US1/US2]
  - Spec 已补充冻结命名表(6 项), 含 chaos_suite, slow_consumer_ms, event_gap_total 等 ✓

## Requirement Consistency(需求一致性)

- [x] CHK010 — FR-001 要求"仓库必须维护一套 CI 可直接调用的混沌套件入口", 但 FR-003 要求"混沌套件与浸泡归档路径必须登记进 specs/006-2 的 QualityGateOutcome 外链列"。混沌套件的执行入口是否在两条 FR 中一致？[Consistency, Spec §FR-001 vs FR-003]
  - 两条 FR 不矛盾: FR-001 = 执行入口(`cargo test --test chaos_suite`), FR-003 = 结果归档(006-2 的外链列); Dependency Note 已明确归档格式 ✓
- [x] CHK011 — US2 要求 24h 窗外"每一条 SLO 判定必须附带数值曲线", 但 SC-002 要求"p99 latency 越阈样本数为 0 或绑定豁免工单"。SLO 判定中的"越阈"标准是否与 SC-002 的 blocking 标记标准一致？[Consistency, Spec §US2 vs SC-002]
  - Spec SoakReport 浸泡阈值表已统一: blocking 标记条件 = 连续 5 个采样窗口越阈; 单次越阈可挂豁免工单 ✓
- [x] CHK012 — Dependency Note 声明依赖 006-3 至 006-5 的可对账控制平面语义。006-5 的背压策略(AlertAndBlock/SampleAndAudit)在 US1 验收场景 2 中被引用——006-5 的默认策略是否已在 006-7 的依赖文档中锁定版本？[Consistency, Spec §Dependency Note vs US1]
  - Dependency Note 已补充依赖版本锁定表(3 行: 006-3/4/5), 含验证方式 ✓

## Acceptance Criteria Quality(验收标准可度量性)

- [x] CHK013 — SC-001 要求"波形关键字与仓库 ChaosScenario 表的 scenario id 集合逐项对上号, 覆盖率 100%"。ChaosScenario 表的权威存放位置和格式是否在 spec 中定义？[Measurability, Spec §SC-001]
  - Spec 已补充"覆盖率计算口径"定义: 存在性检查(11 个 id 各有一份剧本); 阈值表内嵌在 spec 中 ✓
- [x] CHK014 — SC-002 要求"24h 窗外 p99 latency 越阈样本数为 0"。p99 latency 的测量条件和测量工具是否在 spec 中指定？[Measurability, Spec §SC-002]
  - SC-002 已补充测量条件: 合成稳态流量 1000 req/s, p99 在 1s 滑动窗口上计算 ✓
- [x] CHK015 — SC-003 要求"合成关停样本 100 次里 shutdown_success_ratio 不低于 99%"。"关停成功"的定义是否在 spec 中明确？[Measurability, Spec §SC-003]
  - SC-003 已补充"关停成功"定义: ShutdownCoordinator 最终阶段为 Completed + 所有 slot 停用 + 审计记录完整 ✓

## Scenario Coverage(场景覆盖)

- [x] CHK016 — US1 覆盖了 11 个故障波形场景。但多个故障同时发生(如 child_panic_storm + ipc_connection_storm 并发)的组合场景是否也在范围内？[Coverage, Spec §US1]
  - Spec Edge Cases 已明确排除: 组合故障推迟至后续切片 ✓
- [x] CHK017 — US2 覆盖了正常浸泡场景。但浸泡中途发生硬件故障或 CI runner 被回收时的行为是否在 spec 中定义？[Coverage, Spec §US2]
  - Spec Edge Cases 已补充: 浸泡中断视为无效, 不产生 SoakReport; 分片累计算法要求≥8h/片 ✓
- [x] CHK018 — US3 覆盖了 IPC 连接风暴和 relay 崩溃。但 dashboard IPC 的注册/心跳超时是否也纳入风暴测试范围？[Coverage, Spec §US3]
  - US3 验收场景 2 引用 006-1 的同步超时契约; 心跳超时风暴不在本切片范围内 ✓

## Edge Case Coverage(边界条件覆盖)

- [x] CHK019 — 系统时钟回拨场景: spec 给了两个选项(补偿策略/release notes 声明)。滑动窗口保护机制在时钟回拨时的破坏性行为是否在文档中写明？[Edge Case, Spec §Edge Cases]
  - Spec 已选定 monotonic clock 方案: 使用 Instant 避免 wall clock 回退影响; 检测到回退时重置窗口计数 ✓
- [x] CHK020 — `command_channel_full` 场景: command channel(mpsc channel) 满时的行为是阻塞发送方还是丢弃命令？该行为是否与控制循环的背压策略一致？[Edge Case, Spec §FR-001]
  - ChaosScenario 阈值表 command_channel_full 行定义: send() 返回 Err(Closed) 而非无限阻塞; 与控制循环背压策略独立 ✓
- [x] CHK021 — `relay_crash_loop` 场景: relay 进程被反复拉起和崩溃之间的状态同步(进程 PID 变化? socket 路径被占用?)是否在 spec 中定义？[Edge Case, Spec §FR-001]
  - US3 验收场景 2 定义: 第 5 次拉起后链路对齐在 10s 内完成; socket_path_contention 场景覆盖路径冲突 ✓
- [x] CHK022 — 异步运行时饥饿探针(`runtime_starvation_probe`)若能够度量, 降级动作是否在 spec 或 research 中定义？[Edge Case, Spec §Edge Cases]
  - Spec Edge Cases 已补充: 5s 内 poll_count 无增长时尝试 yield_now, 仍无增长则输出 RuntimeStarved 诊断事件并暂停非核心事件发射 ✓

## Non-Functional Requirements(非功能需求)

- [x] CHK023 — 混沌场景的超时预算是否定义？每个 scenario 的最大执行时间和 CI 流水线超时是否量化？[NFR, Gap]
  - ChaosScenario 阈值表 expectation exit code = 0; 各场景的主要阈值中隐含执行窗口(如 rapid_failure_10k 为 60s); CI 流水线超时共用 `cargo test` 的超时配置 ✓
- [x] CHK024 — 浸泡报表的存储预算(CI 归档保留期限? CSV/PNG 总大小上限?)是否在 spec 中定义？[NFR, Gap]
  - SoakReport 格式已定义归档 hash 输出; 保留期限由 006-2 的 QualityGateOutcome 策略控制(不在本切片范围) ✓
- [x] CHK025 — 混沌 harness 的进程级隔离要求(Rust 默认二进制发布特性不允许悄悄改写)在 Constitution 中声明, 但 dev-dependency 的编译时间影响和测试 crate 的构建时间预算是否量化？[NFR, Gap]
  - Assumptions 已补充: 构建时间不超过 `cargo test --release` 全量的 120% ✓

## Dependencies & Assumptions(依赖与假设)

- [x] CHK026 — spec 强依赖 006-3 至 006-5 的可对账控制平面语义。这些依赖切片的当前实现版本是否在 spec 中锁定？[Dependency, Spec §Dependency Note]
  - Dependency Note 已补充依赖版本锁定表(含验证方式: CI 编译依赖 + contract 测试) ✓
- [x] CHK027 — 假设"CI 可为单次浸泡分配连续 24h 独占 runner"。如果 CI 不支持 24h 独占 runner, 分片累计算法的等价性证明是否在 Assumptions 中写明？[Assumption, Spec §Assumptions]
  - Assumptions 已补充分片约束: 每片≥8h, 片间隔≤4h, 同 commit ✓
- [x] CHK028 — FR-003 要求结果登记进 006-2 的 QualityGateOutcome。006-2 的 QualityGateOutcome schema 是否已冻结？如果 006-2 仍在迭代, 本切片的输出字段格式是否会因 006-2 变更而阻塞？[Dependency, Spec §FR-003 → specs/006-2]
  - Assumptions 已补充: 本切片适配策略——优先适配新 schema, 不等待 006-2 完成 ✓

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK029 — US1 验收场景 1 要求"监督器进程自身 panic 计数保持 0"。panic 在这里是指 Rust panic 还是进程崩溃？[Ambiguity, Spec §US1]
  - Spec 已补充"panic 语义定义": 指 Rust 语言级 panic(catch_unwind 捕获); 进程级用"crash"表述 ✓
- [x] CHK030 — Key Entities 只列出了 ChaosScenario 和 SoakReport 两个实体。US3 涉及"rate limit""合法/劣质客户端"等概念——这些是否也需要作为 Key Entities 或数据模型定义？[Ambiguity, Spec §Key Entities]
  - Spec Key Entities 已补充 RateLimiter 和 ClientClassification 两个实体 ✓
- [x] CHK031 — SC-001 要求"覆盖率 100%", 但 FR-001 的 11 个 scenario id 中有一些的通过标准比另一些更难定义明确的成败边界。"100%"的分母是存在性还是阈值枚举完整性？[Ambiguity, Spec §SC-001]
  - Spec 已补充"覆盖率计算口径": 存在性检查(11 个 id 各有一份剧本), 非阈值完整性 ✓

## Constitution Compliance(宪章合规)

- [x] CHK032 — Constitution Alignment 要求"混沌 harness 只允许通过测试夹具注入故障"。该约束是否在 11 个 scenario 的剧本设计中一致体现了"夹具注入"而非"代码改写"的原则？[Compliance, Spec §Constitution]
  - Spec 补充了"混沌 harness 隔离策略": 源码在 `tests/chaos/`, 仅 dev-dependency; 11 个场景的故障注入方式已在 ChaosScenario 表中列明 ✓
- [x] CHK033 — Module ownership 要求"混沌与浸泡 harness 源码路径必须与默认库 lib 导出表面隔离"。该隔离策略是否已在 spec 或 plan 中明确？[Compliance, Spec §Module ownership]
  - "混沌 harness 隔离策略"已明确: `tests/chaos/` 目录, 仅 dev-dependency ✓
- [x] CHK034 — Diagnostics 要求"每条场景必须有稳定 scenario id 字符串, 并与 semver 戳并列写入报表头部"。11 个 scenario id 的命名规范和 semver 戳的读取来源是否在 spec 中定义？[Compliance, Spec §Diagnostics]
  - Spec 已补充: scenario id 使用 snake_case; semver 从 `env!("CARGO_PKG_VERSION")` 读取; JSON 判决书格式已包含 scenario_id + semver ✓

## Notes(说明)

- 完成检查项后使用 `[x]` 标记.
- 评论或发现可以直接写在相关检查项下.
- 需要时链接相关资源或文档.
- 检查项必须按顺序编号, 方便引用.
