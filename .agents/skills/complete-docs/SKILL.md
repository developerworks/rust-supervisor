---
name: complete-docs
description: Supplement and refine engineering documentation for Rust projects and adjacent project docs. Use when Codex needs to add or improve module rustdoc, type/field/function documentation, Intro Links, internal navigation, doctests, README sections, architecture docs, plan documents, or migration-related documentation while keeping wording aligned with current code structure, module ownership, and verified behavior.
---

# Complete Docs

补充和完善工程文档，优先保证“文档真实反映当前代码与结构”，再追求表达完整。

## 工作流

1. 先判定 owner（归属模块）和文档边界。
- 先读目标实现、模块根、导出面、调用方，再决定文档应该写在哪里。
- 不把本模块语义写到无关模块、全局说明或过期迁移备注里。

2. 先补事实，再补表达。
- 先确认职责、输入输出、状态流、可见性、错误语义、时间戳或顺序语义。
- 不根据命名臆测实现，不把“应该如此”写成“现在如此”。

3. 先补导航，再补细节。
- 模块文档优先写职责概览和单一导航区块。
- `# Intro Links` 与 `# 内部导航` 二选一，不要同时出现；有明确阅读顺序时优先用 `# 内部导航`，否则用 `# Intro Links`。
- 类型和函数文档优先写契约、边界、语义，而不是重复签名。

4. 按文档类型选择参考。
- 补 Rust API 文档时，先读 `references/rustdoc-patterns.md`。
- 补 README、架构文档、计划文档时，先读 `references/project-docs.md`。

## 写作规则

- 默认中文为主。英文术语第一次出现时写成“英文名（中文说明）”。
- 优先使用可导航 rustdoc 链接，例如 [`crate::analysis::ExecutionCore`]、[`Type::method`]。
- 不用裸 `src/...`、`tests/...` 路径充当导航说明；需要文件级说明时，使用真实模块入口、重导出入口或可点击 Markdown 链接。
- 模块文档的导航说明只保留一个区块；不要把相同入口同时写进 `# Intro Links` 和 `# 内部导航`。
- 不写废话注释，不解释签名已经表达清楚的内容。
- 函数 / 方法文档中的参数必须有说明；不能只写函数整体用途而省略 `# Arguments` 或逐参数语义。
- 参数说明至少覆盖：参数业务含义、单位 / 精度 / 取值约束、可空或哨兵值语义；涉及顺序、时间、所有权或副作用时也要写明。
- 公共 API 优先补 rustdoc；结构迁移后同步更新 README、架构文档、示例路径和 doctest 路径。
- 能写 doctest 的地方尽量写；不适合写 doctest 时，明确原因并避免伪示例。

## 文档对象优先级

1. 模块：职责、边界、数据流、单一导航区块。
2. 类型：角色、约束、不变式、与相邻类型的关系。
3. 字段：业务语义、单位、生命周期或可空语义。
4. 函数 / 方法：用途、参数说明、输入输出、状态变化、副作用、幂等性、顺序 / 时间语义、错误语义。
5. 仓库文档：入口、导航、运行方式、验收命令、结构迁移后的新路径。

## 验证

- 至少选择一条能证明文档与代码现态一致的验证命令。
- Rust API 文档优先考虑 `cargo test --doc`、`cargo check`。
- 结构或导航变更要补扫 `README.md`、`docs/architecture/*.md`、`PLAN_*.md` 是否仍指向旧路径。
- 验收结论只写真实结果；验证失败时，不把计划或文档标成“完成”。

## 输出要求

- 先给最小必要文档补丁，不顺手扩主题。
- 如果文档依赖未确认语义，先在回复中明确假设或缺口。
- 修改后给出：改了什么、为什么这样写、跑了什么验证、还有什么边界未覆盖。

## References

- `references/rustdoc-patterns.md`：模块 / 类型 / 字段 / 函数 / doctest 写法与模板。
- `references/project-docs.md`：README / 架构 / 计划文档的收口规则与验收清单。
