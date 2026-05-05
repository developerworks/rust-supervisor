# Specification Quality Checklist(规格质量检查清单): 创建监督器核心

**Purpose(目的)**: 在进入计划前验证规格完整性和质量.
**Created(创建日期)**: 2026-05-04
**Feature(功能)**: [spec.md](../spec.md)

## Content Quality(内容质量)

- [x] 规格没有泄漏不必要的实现细节.
- [x] 规格聚焦用户价值和业务需要.
- [x] 规格面向非技术利益相关者也能理解.
- [x] 所有 mandatory(必填) 章节已经完成.
- [x] 规格使用中文写作,并且英文术语使用 `English(中文说明)`.

## Requirement Completeness(需求完整性)

- [x] 文档中没有需要澄清标记.
- [x] 需求可以测试,并且表达明确.
- [x] success criteria(成功标准) 可以衡量.
- [x] success criteria(成功标准) 保持 technology-agnostic(技术无关).
- [x] 所有 acceptance scenario(验收场景) 已经定义.
- [x] edge case(边界情况) 已经识别.
- [x] scope(范围) 已经清晰限定.
- [x] dependency(依赖) 和 assumption(假设) 已经识别.
- [x] rust-config-tree(集中配置树) v0.1.9 和 YAML(数据序列化格式) 主配置格式已经明确.
- [x] hard-coded constant(硬编码常量) 禁止规则已经明确,所有 runtime tunable constant(运行时可调常量) 必须来自 rust-config-tree(集中配置树) 配置并保持可配置.
- [x] 状态相关代码命名已经明确,不得使用 `*View` 后缀,正式命名使用 `SupervisorState`(监督器状态),`ChildState`(子任务状态),`current_state`(当前状态) 和 `state`(状态).
- [x] 测试文件 `_test.rs` 后缀,`src/tests/*_test.rs` 集成测试位置和模块 `tests/*_test.rs` 单元测试位置已经明确.
- [x] Source Code(源代码) 结构已经反向同步,核心模块必须直接位于 `src/<module>/`,不得使用 `src/supervision/` 中间层,不得使用 `src/<module>.rs` 平铺模块文件.
- [x] glossary(词汇表) 文件已经作为专业词汇和反引号词汇的正式来源.
- [x] parallel development(并行开发),unattended implementation(无人值守实现),blocker elimination(卡点消除) 和 lead agent supervision(主代理监督) 已经明确,主代理必须监督子代理输出并及时纠偏.

## Feature Readiness(功能就绪)

- [x] 所有 functional requirement(功能需求) 都有清晰验收标准.
- [x] user scenario(用户场景) 覆盖主要流程.
- [x] 功能满足 success criteria(成功标准) 中定义的可衡量结果.
- [x] 规格没有把实现细节当作用户需求.

## Notes(说明)

- 初始规格通过后,本检查清单记录验证结果.
- 宪章对齐部分引用 policy(策略),runtime(运行时),event(事件) 和 handle(句柄) 边界,因为项目宪章要求监督契约和 Rust(编程语言) 边界可见.
- 澄清流程加入了用户提供的 runtime governance(运行时治理) 约束.`ChildSpec`(子任务规格),`SupervisorTree`(监督树),`CancellationToken`(取消令牌),`watch`(观察通道) 和 `tracing`(结构化追踪) 是本开发者向 supervisor core(监督器核心) 的功能领域词汇,不是无意泄漏的实现细节.
- 2026-05-05 复核后,规格继续把 readiness(就绪),blocking task(阻塞任务),event journal(事件日志缓冲区),`RunSummary`(运行摘要) 和 four-stage shutdown(四阶段关闭) 保留为必须能力.规格还明确禁止 hard-coded constant(硬编码常量),并要求重启阈值,熔断窗口,退避时长,抖动比例,心跳间隔,关闭超时,事件日志容量,指标开关和审计开关等 runtime tunable constant(运行时可调常量) 都来自 rust-config-tree(集中配置树) v0.1.9 的 YAML(数据序列化格式) 配置.状态相关代码命名不得使用 `*View` 后缀,并统一使用 `SupervisorState`(监督器状态),`ChildState`(子任务状态),`current_state`(当前状态) 和 `state`(状态).
- 2026-05-05 复核后,规格补充 lead agent(主代理) 对 subagent(子代理) 的监督要求.每个 subagent workstream(子代理工作流) 必须有审查记录,development drift(开发偏差) 必须有 correction record(纠偏记录),并且 workstream(工作流) 只能在纠偏复核通过后标记完成.
- 2026-05-05 复核后,根据 plan.md(计划文档) 的 Source Code(源代码) 结构反向更新 spec.md(规格文档).核心模块必须采用 top-level directory module(顶层目录模块) 结构,直接位于 `src/<module>/`,不得保留 `src/supervision/` 中间层,也不得使用 `src/<module>.rs` 平铺模块文件.
