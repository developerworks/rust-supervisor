---
name: rust-coding
description: Use when working on this Rust project for coding, refactoring, module ownership cleanup, API and architecture decisions, rust-analyzer assisted inspection, Rust service or CLI implementation, documentation updates, recurring-pattern capture, and focused validation. This is a project-local combo skill that coordinates the rust, rust-analyzer-workflows, rust-design-patterns, and write-project-commit skills when relevant.
---

# Rust Coding

本技能是项目内 Rust 编码组合入口，用来把常用 Rust 开发流程收成一个稳定别名。
它统一协调以下技能：

- `rust`
- `rust-analyzer-workflows`
- `rust-design-patterns`
- `write-project-commit`

## 组合技能分派

### 1. 项目重构先进入 rust-analyzer-workflows

当用户意图包含“重构、改名、移动、拆分、归位、owner 清理、导出面清理、模块边界调整、跨文件引用迁移、结构化替换”时，必须先使用 `rust-analyzer-workflows`，而不是先进入通用 `rust` 编码流程。

这类任务的默认分派是：

1. 先用 `rust-analyzer-workflows` 做语义发现、符号定位、引用范围、可用重构动作和轻量诊断；
2. 再用项目规则判断 owner（归属模块）、文件边界、导出面和可见性；
3. 最后才进行文件编辑、模块门面调整和必要的 `cargo check` / 测试验证。

适用场景：

- 类型、枚举、函数、模块或文件改名；
- 移动类型或函数；
- 拆分实现文件；
- 调整 `mod.rs` / `pub use` / `#[path = "..."]`；
- 检查 unresolved import（未解析导入）；
- 查定义或引用；
- 结构化搜索或替换；
- 批量迁移调用方。

默认优先级：

1. 确认 `/root/.cargo/bin/rust-analyzer --version` 或当前 shell 中独立 `rust-analyzer` 可用；
2. `rust-analyzer symbols` / LSP `workspaceSymbol` / `documentSymbol`；
3. LSP `definition` / `references` / `rename` / `codeAction`；
4. `rust-analyzer parse --no-dump`；
5. `rust-analyzer diagnostics`；
6. 必要时再考虑 `cargo check`。

如果独立 `rust-analyzer` 不可用：

- 优先尝试用 `rustup component add rust-analyzer` 为当前工具链安装；
- 不使用 VS Code 扩展自带的 rust-analyzer server 作为兜底；
- 只有安装不可行或失败后，才降级为 `rg` + 局部文件检查；
- 降级时要明确说明原因，并在后续验证中补足 `cargo check` 或定向测试。

### 2. Rust 编码与实现

涉及以下任务时，先使用 `rust`：

- Rust 代码实现；
- owner（归属模块）判断；
- 可见性调整；
- trait（特征）边界；
- 错误处理；
- async（异步）与并发代码；
- 性能敏感路径。

默认规则：

- 不顺手扩大重构范围；
- 不修改业务语义，除非用户明确要求；
- 新代码遵循本项目模块归位规则；
- `mod.rs` 只做模块说明、声明和导出，不承载业务实现；
- 单文件膨胀时优先拆到 owner 目录下的 `types.rs`、`enums.rs`、`runtime.rs`、`mapping.rs`、`constants.rs`、`serde_functions.rs` 等文件。

### 3. Rust 设计模式与 API 边界

涉及以下任务时，使用 `rust-design-patterns`：

- 设计或重构状态机、provider、store、service、builder、adapter、strategy、repository 等模式；
- 比较 trait object（特征对象）、generic（泛型）、enum dispatch（枚举分发）、closure（闭包）之间的取舍；
- 判断是否应该引入某种模式，或明确拒绝过度抽象；
- 规划公共 API、插件边界、对象安全、所有权感知的抽象方案。

默认规则：

- 先判断设计压力，再决定是否需要模式；
- 优先用 enum、trait、泛型、组合、新类型这些 Rust 原生手段；
- 如果经典模式在 Rust 里有更轻量的等价写法，优先采用 Rust 惯用法；
- 如果某个模式只会增加层次和认知负担，要明确建议不要使用。

### 4. 文档补齐

涉及以下任务时，使用 `complete-docs`：

- 模块文档；
- 结构体文档；
- 字段文档；
- 函数文档；
- 参数说明；
- doctest；
- README / PLAN 文档。

默认规则：

- 中文为主；
- 英文术语第一次出现时提供中文旁注；
- 模块文档必须有 Intro Links 和内部导航；
- 函数参数必须说明业务含义、单位、取值约束、空值或哨兵值语义；
- 能写 doctest 的地方尽量写；
- 不适合 doctest 时，不写伪示例。

### 5. 提交准备

只有用户明确要求提交时，才使用 `write-project-commit`。

默认规则：

- 只提交当前主题相关文件；
- 不把其它并行改动带入提交；
- 提交信息使用项目约定格式：

```text
主题
- 修改项
```

## 工作边界

- 用户要求“拆分”，只做结构拆分，不顺手修业务逻辑。
- 用户要求“修复”，先定位错误，再做最小修复。
- 用户要求“全面修复”，可以扩大到同一 owner 模块内的相关问题。
- 用户要求“补文档”，不顺手做结构重构，除非文档无法准确表达当前结构。
- 用户要求“验证”，优先选择最小足够的验证命令。

## 验证策略

默认不主动跑验证命令，除非用户明确要求。

如果用户要求验证，按风险从低到高选择：

1. `rust-analyzer parse --no-dump`
2. `rust-analyzer diagnostics`
3. `cargo check`
4. `cargo test --doc`
5. 定向单元测试
6. `cargo clippy`

## 输出要求

- 先说结论；
- 再说改了哪些文件；
- 再说是否验证；
- 没验证就明确写“未运行验证”；
- 如果遇到工具不可用或权限问题，直接说明原因和降级路径。
