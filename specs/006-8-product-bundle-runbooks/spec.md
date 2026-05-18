# Feature Specification (功能规格): 最小生产包, 交付文档与放行矩阵占位

**Feature Branch (功能分支)**: `[006-8-product-bundle-runbooks]`
**Created (创建日期)**: 2026-05-17
**Updated (更新日期)**: 2026-05-19
**Status (状态)**: Accepted (已接受)
**Input (输入)**: 本规格对应第六序列里程碑: 做一个最小可用生产包. 核心 crate(包) + examples(示例) + reference service(参考服务) + dashboard relay(看板中继) + deployment guide(部署指南) + operations runbook(运维手册). 每个版本发布前都要跑: unit test(单元测试), integration test(集成测试), property test(性质测试), fuzz test(模糊测试), loom test(并发模型测试), chaos test(混沌测试), 24 小时 soak test(长稳测试), dependency audit(依赖审计), SBOM(软件物料清单), release dry run(发布预演).

## Dependency Note (依赖说明)

放行矩阵字段字典由 specs/006-2-release-supply-chain-gates/spec.md 给出. 本切片约束买方看得见的内容结构与 ReleaseGateMatrixPointer(放行矩阵指针) 外链写法. 发布台账必须把 006-2 与 006-7 归档哈希并排挂上.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### User Story 1 (用户故事一) - MVP 包可被照抄拉起 (Priority (优先级): P1)

购买者集成工程师需要在不写私服胶水 crate(包) 的前提下, 只靠 README, deployment guide(部署指南), 公开 crate(包) 工件拉起核心监督能力与参考拓扑, 并得到脚本可解析的自检 JSON.

**Why this priority (为什么是这个优先级)**: 买方 POC 窗口通常只有几日. 照着文档抄不过去就意味着丢标.

**Independent Test (独立测试)**: 干净容器镜像从零计时. 统计触发第一份 health_snapshot(示例字段名) 所需的 Shell 步数上限.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 读者只有 tarball(归档包) 与 release tag(版本标签) 指针文件, **When (当)** 按 deployment guide(部署指南) 主章节逐步执行, **Then (则)** 在指南承诺的固定步数上限内必须打印 ready(就绪) 与最小看板链路自检字段.

### User Story 2 (用户故事二) - 值守手册可执行 (Priority (优先级): P1)

值班工程师需要在告警触发后的 15 分钟滑动窗口内按 operations runbook(运维手册) 编号步骤走完定位, 止血, 恢复或升级分叉决策. 全过程不要求微信群口头补丁.

**Why this priority (为什么是这个优先级)**: 手册不可执行会把 MTTR(平均修复时间) 拉到无法接受.

**Independent Test (独立测试)**: 桌面演练随机抽取 P1(优先级一) 条目, 由二号工程师对照计时打分表.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 手册枚举某一 P1(优先级一) 场景标题, **When (当)** 桌面沙盘逐步执行, **Then (则)** 每一步末尾都必须写明期望 metrics(指标) 字段取值或可跳转章节锚点. 不允许出现悬空引用.

### User Story 3 (用户故事三) - 放行矩阵随版本并排发布 (Priority (优先级): P2)

发布评审主席需要在 release(版本发布) 页面右侧看见放行矩阵快照. 混沌与 24h(二十四小时) 浸泡两行要么绿色勾选要么挂上工单编号. 不允许灰色空白占位.

**Why this priority (为什么是这个优先级)**: 空白格子会让法务质疑签字有效性.

**Independent Test (独立测试)**: 下载发布 HTML. DOM 解析空白 td(表格单元格) 计数必须为 0.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 签字会议展开外链 006-2 tarball(归档包) 与 006-7 报表 tarball(归档包), **When (当)** 比对哈希字段列, **Then (则)** 两列哈希必须与页面 DOM 中 data-archive-sha256(示例属性名) 相等. 若缺席必须列出豁免工单编号.

### Edge Cases (边界情况)

- 参考命令所需内核能力必须与 006-1 支持矩阵写的是一回事. 不许在非支持内核章节照搬默认 IPC 示例.
- mTLS(双向传输层安全协议认证) 证书链责任分割线必须在 operations runbook(运维手册) 写清. 自检脚本必须把"缺证书链"与"产品缺陷"区分枚举.

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 对外 MVP tarball(归档包) 至少捆绑核心 crate(包) 源码树, examples(示例) 目录, 一份参考拓扑 docker-compose(编排示例文件名) 或等价脚本, 以及与看板最小演示匹配的 relay(中继) 二进制. 若裁剪 IPC 栈必须在 006-1 支持矩阵写明裁剪指纹. tarball(归档包) 内禁止引用未公开的私服 registry(仓库索引).
- **FR-002**: deployment guide(部署指南) 与 operations runbook(运维手册) 必须同步抬升 semver(语义化版本) 版本号. 部署篇写拓扑挂载, IPC 权限位, 密钥引用占位. 值守篇写巡检节拍, P1(优先级一) 事故剧本, 回滚指针, 消费者自备 mTLS(双向传输层安全协议认证) 证书时的自检判别段落.
- **FR-003**: 每一次正式发布正文必须附带 ReleaseGateMatrixPointer(放行矩阵指针), 勾选或外链单元, 集成, 性质, 模糊, loom(并发模型测试), 混沌, 不少于 24h(二十四小时) 浸泡, 依赖审计, 物料清单, 发布预演. 深度测试证据哈希必须与 006-7 报表一致. 供应链指针必须与 006-2 ReleaseRecord(发布记录) 一致.

### Key Entities (关键实体) _(涉及数据时填写)_

- **DeliveryBundle(交付包清单)**: semver(语义化版本) 对齐的最少构件文件名数组, 附带 sha256(哈希算法示例).
- **RunbookProcedure(值守程序块)**: 编号步骤, 期望 metrics(指标) 取值, 升级分叉枚举.
- **ReleaseGateMatrixPointer(放行矩阵指针)**: 指向 006-2 QualityGateOutcome(质量闸口结果) 导出路径与 006-7 SoakReport(浸泡报告) tarball(归档包) 的一组 URL.

## Constitution Alignment (宪章对齐) _(mandatory (必填))_

### Supervision Contract (监督契约)

- **Lifecycle impact (生命周期影响)**: 文档条目不改变监督语义. 但裁剪 tarball(归档包) 会改变购买者能力边界, 必须与 006-1 支持矩阵同窗修订.

### Rust Boundary and Observability Requirements (Rust 边界和可观察性需求)

- **Module ownership (模块所有权)**: examples(示例) 与参考服务代码目录固定在仓库约定路径. 禁止塞进默认二进制 main.rs.
- **Compatibility exports (兼容导出)**: None (无).
- **Diagnostics (诊断)**: 健康自检脚本 stdout(标准输出) JSON 必须具备稳定顶层键名, 方便 CI 文本检索脚本抓取.

### Chinese Writing (中文写作)

- **Writing language (写作语言)**: 本文档必须使用中文.
- **Term format (术语格式)**: 英文术语必须写成 English(中文说明).
- **Forbidden style (禁止风格)**: 禁止全角标点, 禁止手册段落引用不存在锚点 ID.

## Success Criteria (成功标准) _(mandatory (必填))_

### Measurable Outcomes (可衡量结果)

- **SC-001**: deployment guide(部署指南) 主路径桌面盲测通过率不低于 95%, 样本不少于 10 套互相独立的容器镜像.
- **SC-002**: P1(优先级一) 值守条目桌面演练记分表显示至少 90% 条目在手册写明的时间上限内抵达终态分叉.
- **SC-003**: 对外放行矩阵 DOM 裸露空白 td(表格单元格) 计数恒为 0.

## Assumptions (假设)

- 对内详尽 FINAL REPORT(最终报告) 可以通过外链摘要映射到外发精简页面. 采购经理只看到最少段落.
