# Feature Specification (功能规格): 平台边界, 说明文档与看板 IPC (进程间通信) 安全强化

**Feature Branch (功能分支)**: `[006-1-platform-docs-ipc-security]`
**Created (创建日期)**: 2026-05-17
**Updated (更新日期)**: 2026-05-19
**Status (状态)**: Accepted (已接受)
**Input (输入)**: 本规格处理一条横切线: IPC 安全与平台边界. 当前代码大量使用 tokio::net::UnixListener(Unix 域套接字监听器) 和 std::os::unix(Unix 平台接口), 但没有 feature gate(功能开关) 或平台条件编译保护. 必须明确只支持 Unix-like(类 Unix 系统), 或者把 dashboard IPC(看板进程间通信) 做成可选 feature(功能开关). 已实现的正确做法包括: 要求绝对路径, stale socket(陈旧套接字) 替换时拒绝 symlink(符号链接), 危险命令需要确认. 工业级还需补齐 9 项控制点: socket owner(套接字所有者) 校验, peer credentials(对端身份) 校验, command authorization(命令授权), replay protection(重放保护), request size limit(请求大小限制), rate limit(速率限制), audit persistence(审计持久化), command idempotency key(命令幂等键), 外部命令 allowlist(白名单). 三目录看板架构 (core library(核心库), relay(中继), user interface(用户界面)) 以及 target process(目标进程) 只开启本地 Unix domain socket IPC(Unix 域套接字进程间通信) 的策略必须在 README 中固定写明.

## Dependency Note (依赖说明)

本切片承接 specs/003-supervisor-dashboard/spec.md 以及相关 IPC 契约中已经写明的路径约束与 symlink(符号链接) 拒绝规则. 本节只在身份校验, 指令授权与配额三条线上追加硬性条件, 不推翻既有边界. 平台条件编译策略与 006-8 (交付包) 的支持矩阵对齐.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### User Story 1 (用户故事一) - 购买者开箱前就看清平台边界 (Priority (优先级): P1)

集成方要在多种操作系统上接入监督库. 在选定工件与动手编译之前, 他们需要拿到一张支持矩阵, 写明当前交付组合在非类 Unix 主机上能否完成核心监督能力的编译与链接, 在不走本地套接字的前提下能否卸掉整条看板链路, 又不把安全漏洞留给买方自己踩.

**Why this priority (为什么是这个优先级)**: 工件组合选错, 往往拖到集成收尾阶段才发现编译过不了关, 先前对控制面暴露范围的判断也就站不住脚了.

**Independent Test (独立测试)**: 准备三张买方岗位卡片 (集成工程师, 运维负责人, 安全复核员). 参与者只靠公开发布的 README, 选型附录与支持矩阵完成一次书面选型复盘. 复盘结论不得与支持矩阵每行字面陈述矛盾.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 买方目标主机不是类 Unix, **When (当)** 其阅读支持声明并勾选不包含看板本地套接字的裁剪组合, **Then (则)** 仍须完成核心监督能力的编译与链接. README 必须在表格或附录中列出被裁剪字段名称以及推荐的替代接入路径.
2. **Given (假设)** 买方目标主机是类 Unix, **When (当)** 其启用走本地套接字的看板链路, **Then (则)** README 须写明目标进程只接受本地的 Unix Domain Socket(Unix 域套接字), 不得把远端 TCP 监听写成默认开箱可用的观测路径.

### User Story 2 (用户故事二) - 架构三目录拆分一眼可读 (Priority (优先级): P2)

运维负责人负责部署拓扑. README 的架构小节必须固定在同一套标题层级里, 点明核心库, relay(中继), user interface(用户界面) 三件套的进程边界, 套接字归属与日志字段前缀, 方便他们把挂载目录与 Unix 账号映射回这三条边界.

**Why this priority (为什么是这个优先级)**: 只有三套件的目录挂载与账号映射能对上账, 才能做到最小权限挂载与分段升级.

**Independent Test (独立测试)**: 任选一名未读过源码的同事. 只凭架构小节在白板上画出三条数据流连线. 评测员核对白板是否错误地把面向互联网的端口画在了错误套件一侧.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** README 或等价入口页中存在架构小节, **When (当)** 读者只浏览章节标题与小节第一段而不打开源码树, **Then (则)** 其仍能口述三件套各自持有的本地 IPC 文件路径类别, 协议翻译职责分工以及界面渲染进程归属.

### User Story 3 (用户故事三) - 看板 IPC 安全控制点可被安全官逐项勾验 (Priority (优先级): P3)

安全复核人员需要一张检查表, 与本规格正文对齐, 每行绑定下列 9 项控制点的预期取值快照: (C1) socket owner(套接字所有者) 校验, (C2) peer credentials(对端身份) 校验, (C3) command authorization(命令授权), (C4) replay protection(重放保护), (C5) request size limit(请求大小限制), (C6) rate limit(速率限制), (C7) audit persistence(审计持久化), (C8) command idempotency key(命令幂等键), (C9) 外部命令 allowlist(白名单). 同时, 对重放窗口长度与单次请求体字节上限给出出厂数值或可推导公式.

**Why this priority (为什么是这个优先级)**: 工业采购验收通常要求纸质或电子化勾选记录, 口头复述不够归档用.

**Independent Test (独立测试)**: 使用冻结版本的检查表, 针对 9 项 IPC 控制点各自构造一组应当放行样本与一组应当拒绝样本. 比对服务端响应载荷内是否带有可追溯的决定标识, 审计流水是否写入匹配条目, 以及重放计数器是否与实验室台账登记的期望值一致.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 连接器提交的 peer credentials(对端内核凭证) 快照与支持矩阵声明不符, **When (当)** 其发起任一写监督状态的控制指令, **Then (则)** 目标进程必须返回拒绝响应, 写入可追溯的决定原因码, 且监督状态不得偏离调用前的快照.
2. **Given (假设)** 连接器重复提交最近一次已经被服务端判定消耗完毕的一次性写令牌, **When (当)** 第二次提交抵达, **Then (则)** 必须拒绝并且监督状态保持第二次调用前的取值.
3. **Given (假设)** 连接器提交的请求正文字节长度大于配置文件写入的请求体大小上限, **When (当)** 传输层读完声明长度与实际字节数, **Then (则)** 必须在业务解码之前终止请求并写入审计流水引用编号.

### Edge Cases (边界情况)

- 同一宿主机挂载多个监督实例且监听路径前缀重叠时, README 与安全加固示例配置必须写明如何通过 socket owner(套接字文件所有者), POSIX ACL 以及监听路径前缀三重字段区分实例, 避免连接器串实例.
- 当外部二进制路径成为控制面扩展点时, 凡未列入 allowlist(白名单) 的可执行绝对路径必须在默认配置文件中被注释为空数组或等价禁用标记. 运行时命中禁用条目必须返回拒绝并且不得在磁盘或网络上留下写到一半就停下来的配置文件痕迹.
- 当托管审计卷的挂载点短时变为只读或磁盘配额用尽时, 必须写明厂商采取的两种策略之一并绑定运维告警阈值清单: fail closed(默认拒绝高风险写动作), 或 defer with bounded queue(延迟落盘且有界队列). 禁止静默丢弃应当写入审计卷的敏感写指令副本.

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 发布方必须为核心监督组合与看板链路组合分别维护一张支持矩阵, 表中至少给出三列可读字段: 主机操作系统族别, 本地套接字文件路径能力是否默认启用, 非类 Unix 组合下允许的裁剪标记位. 买方不得在矩阵标记为不支持的组合上仅靠默认特性开关拿到表面上编译成功实则缺失 IPC 硬化字段的工件.
- **FR-002**: 官方 README 或等价主页必须使用固定小节标题复述 dashboard(看板) 的三目录拆分: core library(核心库), relay(中继), user interface(用户界面). 小节正文必须给出可复制粘贴的目录路径示例片段以便挂载核对. 必须写明 target process(目标进程) 二进制默认只监听本地 IPC 套接字, 禁止把同一监听端口映射文档写成可直接绑定 0.0.0.0/0 的示例行.
- **FR-003**: 看板 IPC 在工业默认配置文件模板中必须具备 9 项可逐项勾选验收的控制点, 编号固定以便其它切片引用: (C1) socket owner(套接字所有者) 与 peer identity(对等身份) 校验一致, (C2) peer credentials(对端身份) 校验, (C3) command authorization(命令授权) 模型, (C4) replay protection(重放保护), (C5) request size limit(请求大小限制), (C6) rate limit(速率限制), (C7) audit persistence(审计持久化), (C8) command idempotency key(指令幂等键) 语义, (C9) 外部命令 allowlist(白名单). 任一控制点在抽检中不达标时, 高风险写指令一律拒绝并记下带流水号的审计条目.

### Key Entities (关键实体) _(涉及数据时填写)_

- **PeerIdentityExpectation(对等身份期望)**: 配置或安装阶段写明的允许接入本地监听器的身份约束集合. 可与进程账号字段以及 peer credentials(对端内核凭证) 快照逐字段比对.
- **IpcRiskAction(高风险 IPC 动作)**: 停机, 重启, 隔离子进程, 删除持久化卷挂载项或其他会使故障半径向外扩散的监督指令类别枚举值集合. 用于授权矩阵的行索引以及审计严重性分级列索引.
- **AuditRecord(审计记录)**: 单次 IPC 写请求的不可篡改条目, 至少携带 UTC 时间戳, 指令枚举值, 发起人身份摘要哈希, 可选 correlation id(关联标识), 裁决布尔值以及拒绝时的结构化错误码枚举值.

## Constitution Alignment (宪章对齐) _(mandatory (必填))_

### Supervision Contract (监督契约)

- **Lifecycle impact (生命周期影响)**: 本切片不改变监督状态机的合法迁移集合, 但任何迁移触发指令必须经过 IPC 路径进入. 调用失败时不得留下半启动实例.
- **Failure behavior (失败行为)**: IPC 路径返回拒绝时必须携带人类可读的 structured error(结构化错误) 载荷以及机器可读错误码. 除非调用方出示先前成功的幂等回复副本, 否则监督状态必须与调用发生前保持一致.
- **Shutdown behavior (关闭行为)**: 关停类指令必须与普通控制指令共用同一套授权凭证链路和审计写入侧. 禁止另走捷径绕开出厂 fail closed(默认拒绝高风险写动作) 默认值.

### Rust Boundary and Observability Requirements (Rust 边界和可观察性需求)

- **Module ownership (模块所有权)**: 平台支持与 IPC 安全默认值只能存在于配置加载模块与运行时入口模块的受测试边界内. 演示二进制只能调用公开构造函数而不得私自改写默认常量.
- **Compatibility exports (兼容导出)**: None (无).
- **Diagnostics (诊断)**: 每一次拒绝路径必须暴露稳定的 tracing(结构化追踪) target 名称前缀, 以及可在值班手册附录检索的错误码章节锚点.
- **Dependency impact (依赖影响)**: 如需引入操作系统凭证读取绑定库, 只能在实现阶段计划书中论证其 syscall(系统调用) 覆盖矩阵. 规格正文禁止锁死具体库名称.

### Chinese Writing (中文写作)

- **Writing language (写作语言)**: 本文档必须使用中文.
- **Term format (术语格式)**: 英文术语必须写成 English(中文说明).
- **Forbidden style (禁止风格)**: 禁止全角标点, 禁止口语省略主语的长串顺口溜句式, 禁止把英文形容词机械堆叠进汉语名词短语而不给出可度量字段名.

## Success Criteria (成功标准) _(mandatory (必填))_

### Measurable Outcomes (可衡量结果)

- **SC-001**: 盲测参与者须在自然光线可读纸张或离线 PDF 条件下 30 分钟内完成一次与支持矩阵相符的工件勾选. 评委比对勾选组合与支持矩阵布尔列不得出现互相矛盾的条目.
- **SC-002**: 架构小节盲测采用十道封闭式是非题, 其中三道针对三件套目录挂载边界, 三道针对 IPC 套接字归属, 正确率不得低于 95%.
- **SC-003**: 9 项 IPC 控制点 C1-C9 各自至少存档一组实验室抓包样本证明放行路径生效, 以及一组伪造样本证明拒绝路径生效. 伪造样本触发后数据库中不得新增未经许可的监督状态迁移记录.
- **SC-004**: 人为将 audit persistence(审计持久化) 后端离线超过连续 24 小时期间, 值班控制台须在滚动日志中看到规格声明的失败策略告警计数递增.

## Assumptions (假设)

- mTLS(双向传输层安全协议认证) 只落在跨主机中继链路切片内描述. 本地 Unix Domain Socket(Unix 域套接字) 监听端口不把双向证书校验列为出厂必需步骤.
- 采购方运维手册允许在类 Unix 内核上启用 SO_PEERCRED(Linux) 或 LOCAL_PEERCRED(macOS) 等级别的对等凭证读取. 若目标内核不具备等价 syscall, 支持矩阵必须把该行标注为硬性不可用而非静默降级.
- Unix-only 条件编译策略由计划阶段的 data-model.md 或 config 模块冻结. 本规格只约束平台边界必须在 README 支持矩阵中明示.
