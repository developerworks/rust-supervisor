# Feature Specification (功能规格): 工业级发布门禁与供应链证明

**Feature Branch (功能分支)**: `[006-2-release-supply-chain-gates]`
**Created (创建日期)**: 2026-05-17
**Status (状态)**: Draft (草稿)
**Input (输入)**: 本规格处理第二条横切线: 发布流程与供应链安全. 仓库页面显示还没有发布 release(版本发布). 工业级发布要补齐 signed tag(签名标签), changelog(变更日志), semver(语义化版本), MSRV(最低 Rust 版本) 验证, dependency audit(依赖审计), cargo-deny(依赖策略检查), cargo-semver-checks(接口兼容检查), cargo-mutants(变异测试), code coverage(覆盖率), fuzzing(模糊测试), loom(并发模型测试), miri(未定义行为检查), supply chain attestation(供应链证明). 当前 README 已列出部分门禁 (cargo fmt, cargo check, cargo test, cargo doc, SBOM, cargo publish --dry-run), 但缺少 signed tag 策略和完整门禁台账.

## Dependency Note (依赖说明)

本切片与 specs/006-8-product-bundle-runbooks/spec.md 分工如下: 本切片只约束发布工程能力与记录模板, 006-8 约束对外交付目录与值守文档载体. 闸口是否通过只写在发布流水与归档路径上, 006-8 正文不重复罗列工具命令行.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### User Story 1 (用户故事一) - 版本可追溯 (Priority (优先级): P1)

发布责任人需要每一次对外候选版本在供应链上留下唯一记录: signed tag(签名标签) 指针, changelog(变更日志) 条目, semver(语义化版本) 等级与破坏性声明, 以及和该 tag 绑定的 MSRV(最低 Rust 版本) 声明页. 采购方带着离线压缩包仍能按表核对这四项是否指向同一个 commit 哈希.

**Why this priority (为什么是这个优先级)**: 缺少上述锚点, 事故复盘就圈不定补丁范围, 也无法向客户证明某次安全修复确实包含在交付里.

**Independent Test (独立测试)**: 从发布台账里任取相邻两个对外版本号. 只用发布记录表里的四类指针做一次差异口述. 不要求检出完整源码树, 即能说出用户可见风险摘要.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 一次候选发布已经完成构件打包, **When (当)** 审计者只拿到 signed tag(签名标签) 验签输出与 changelog(变更日志) PDF 摘录页, **Then (则)** 须在 15 分钟内口头判定破坏性改动是否与本轮 semver(语义化版本) 等级匹配.
2. **Given (假设)** 购买者主机上的 rustc(Rust 编译器) 版本低于发布台账附件里的 MSRV(最低 Rust 版本) 行, **When (当)** 其按部署指南第二节的自检脚本执行, **Then (则)** 须在固定 5 步以内退出非零状态并打印指向文档章节号的提示, 而不是拖到中途随机爆出无关编译错误.

### User Story 2 (用户故事二) - 供应链与合规闸口有据可查 (Priority (优先级): P2)

供应链安全专员需要发布流水里固定出现 dependency audit(依赖审计) 摘要行, 许可证 policy(策略) 判定结果, 已知 CVE(公开漏洞编号) 封锁列表命中情况, SBOM(软件物料清单) 文件指针, 以及可被外部脚本复算的 supply chain attestation(供应链证明) 摘要哈希链路.

**Why this priority (为什么是这个优先级)**: 大客户验收通常把上述指针复印进合同附件, 缺失一条就会在法务关卡被打回.

**Independent Test (独立测试)**: 用买方指定的 SBOM(软件物料清单) 消费工具对归档文件跑一次校验. 输出哈希必须与 ReleaseRecord(发布记录) 表里登记的 attestation(证明) 摘要一致, 或在台账脚注写明允许的算法差额.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 任一坐标命中封锁 policy(策略), **When (当)** 发布闸门例行运行结束, **Then (则)** 流水线必须停在 blocked(阻断) 状态, 并在台账附录附上人类可读的坐标清单以及豁免工单占位 URL 或编号.
2. **Given (假设)** SBOM(软件物料清单) 已成功落盘, **When (当)** 外部复验脚本按 operations runbook(运维手册) 第五节重放同样命令行, **Then (则)** 产物摘要哈希必须与 supply chain attestation(供应链证明) JSON 中的声明字段一致, 或在脚注写明允许的规范化差异项.

### User Story 3 (用户故事三) - 深度质量矩阵写进放行记录 (Priority (优先级): P3)

质量工程师需要发布放行表为接口兼容性结论, 变异测试结果快照, 覆盖率阈值判定, fuzzing(模糊测试) 用时与种子集合指针, loom(并发模型测试) 日志指针, miri(未定义行为检查) 日志指针各自留出同名槽位. 本轮若未执行某项深度检查, 槽位里只能填写带编号的豁免工单引用, 不得留白.

**Why this priority (为什么是这个优先级)**: 仅靠单元测试无法在并发与内存模型边角给出采购方可信的背书.

**Independent Test (独立测试)**: 打开最近一次模拟发布的 QualityGateOutcome(质量闸口结果) 导出 CSV. 断言深度矩阵相关列的空单元格数量为 0, 或者每一空单元格同行都存在 ExemptionTicket(豁免工单) 编号列填充.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 本轮迭代改动了对外 pub(公开) 类型表面, **When (当)** 接口兼容性门禁 (cargo-semver-checks) 给出失败结论, **Then (则)** 发布台账必须把本轮标记为暂停, 除非同步附上计划内的 semver(语义化版本) MAJOR(主版本) 抬升说明以及迁移草稿链接.
2. **Given (假设)** loom(并发模型测试) 或 miri(未定义行为检查) 任意一类本轮未跑, **When (当)** 审批人导出放行 PDF, **Then (则)** 对应行必须出现豁免编号与风险评估摘要段落, 不允许只有手写勾选而没有工单号.

### Edge Cases (边界情况)

- 当 PATCH(补丁级别) 版本仅更正文档却仍改写高风险示例命令行时, changelog(变更日志) 必须单列小节解释为何仍标记为 PATCH(补丁级别), 以免买方误认为不存在行为层面的误导风险.
- 当 SBOM(软件物料清单) 生成工具更换导致 schema 版本抬升时, 台账必须附带迁移说明段落或给出双轨并存截止日期, 以免买方存量校验脚本批量失效.
- 当 supply chain attestation(供应链证明) 宿主服务短时故障时, 发布策略必须在"推迟发布"与"签名降级路径"两条里书面选定其一并写明触发阈值. 禁止无证明条目情况下直接把构件标成 released(已发布).

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 发布流水线必须产出并长期保留可被第三方校验的 signed tag(签名标签) 指针, semver(语义化版本) 等级声明, changelog(变更日志) 条目索引, 以及与构件 tarball 同捆的 MSRV(最低 Rust 版本) 自检脚本输出快照. 任一对外版本不得在缺少可读 changelog(变更日志) 小节的情况下标记为 released(已发布).
- **FR-002**: 发布门禁固定包含 dependency audit(依赖审计), 许可证 policy(策略) 判定, 已知漏洞封锁列表比对, SBOM(软件物料清单) 生成步骤, 以及可被外部工具重放的 supply chain attestation(供应链证明) 摘要步骤. 策略失败时必须阻断放行入口, 或只允许附带 ExemptionTicket(豁免工单) 编号的人工绕行节点.
- **FR-003**: 发布放行表必须为下列深度检查各自留出独立记录槽位并附带通过阈值字段: 公开接口兼容性判定摘要 (cargo-semver-checks), 变异测试命令与退出码 (cargo-mutants), 覆盖率阈值对比, fuzzing(模糊测试) 会话标识与用时, loom(并发模型测试) 日志归档指针, miri(未定义行为检查) 日志归档指针. 任一槽位本轮若为空则 QualityGateOutcome(质量闸口结果) 导出视图必须把该行标记为 incomplete(不完整).

### Key Entities (关键实体) _(涉及数据时填写)_

- **ReleaseRecord(发布记录)**: 一次对外版本的不可变指针元组, 至少绑定 tag 哈希, changelog(变更日志) PDF URL, SBOM(软件物料清单) 路径, attestation(证明) JSON 路径以及各闸口摘要哈希列.
- **ExemptionTicket(豁免工单)**: 人工批准的绕行凭证, 必须携带工单编号, 生效截止日期与风险评估段落正文引用位.
- **QualityGateOutcome(质量闸口结果)**: 单行闸口结论枚举, 只允许 passed(通过), failed(失败), waived(豁免), skipped(跳过), missing(缺失) 五类取值或其文档定义的细分别名映射.

## Constitution Alignment (宪章对齐) _(mandatory (必填))_

### Supervision Contract (监督契约)

- **Lifecycle impact (生命周期影响)**: 不适用. 本切片约束交付工程台账与闸门脚本集合, 不改变监督运行时状态机.

### Rust Boundary and Observability Requirements (Rust 边界和可观察性需求)

- **Module ownership (模块所有权)**: CI 描述文件与发布脚本只能存放在仓库约定的 tools/ 或 .github/workflows/ 路径树下. 变更必须经过与普通源码同等力度的评审标签.
- **Compatibility exports (兼容导出)**: None (无).
- **Diagnostics (诊断)**: 任一闸口脚本返回失败时必须打印稳定的 gate_id(闸门代号) 字符串, 以及附录里可供值班人员在电话里复述处置动作的段落锚点.
- **Dependency impact (依赖影响)**: 允许只为门禁引入开发期或 CI 期工具库, 不得在默认 cargo install 使用者路径里静默拉高运行时必需依赖坐标计数.

### Chinese Writing (中文写作)

- **Writing language (写作语言)**: 本文档必须使用中文.
- **Term format (术语格式)**: 英文术语必须写成 English(中文说明).
- **Forbidden style (禁止风格)**: 禁止全角标点, 禁止把英文形容词机械粘贴进汉语名词短语却不写出可对账字段名.

## Success Criteria (成功标准) _(mandatory (必填))_

### Measurable Outcomes (可衡量结果)

- **SC-001**: 连续 3 次模拟发布抽查中, 发布记录表格均能在一页 A4 视图内找齐 FR-001 要求的四类指针, 且校验脚本无需手工改写路径占位符.
- **SC-002**: SBOM(软件物料清单) 外部复验哈希与 ReleaseRecord(发布记录) 登记值在样板数据集上完全一致率达到 100%.
- **SC-003**: 深度质量矩阵相关列在无豁免样本集中空格率为 0%. 在有豁免样本集中 100% 的行携带可读工单编号列填充.
- **SC-004**: MSRV(最低 Rust 版本) 违规样本在自检脚本里 100% 在固定五步以内被拒绝并打印指向文档章节号的升级提示.

## Assumptions (假设)

- 组织已经具备代码签名与时间戳服务, 或者至少在起步阶段接受 Git 附带 Signed-off-by(署名行) 加上 tag 签名作为最低可行的 signed tag(签名标签) 证据形态.
- 深度测试工具链可以只在 CI 夜间队列执行, 只要 QualityGateOutcome(质量闸口结果) 能抓到归档指针即可满足台账义务.
