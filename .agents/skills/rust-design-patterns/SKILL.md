---
name: rust-design-patterns
description: Use when Codex needs to choose, implement, refactor, or review Rust design patterns and idioms, including GoF patterns in Rust, Rust Unofficial Patterns, ownership-aware alternatives, trait/generic/object-safety tradeoffs, API boundary design, and deciding when a pattern should not be used.
---

# Rust Design Patterns

本技能用于把设计模式（design pattern，可复用设计解法）翻译成 Rust 风格的实现决策。它综合 Refactoring.Guru 的 GoF（Gang of Four，四人帮经典模式）Rust 示例目录，以及 Rust Unofficial Patterns 对 idiom（惯用法）、pattern（模式）和 anti-pattern（反模式）的整理。

## 使用原则

1. 先判断设计压力：构造、生命周期、状态、插件化、协议适配、所有权、并发、错误边界、未来拆 crate 边界。
2. 优先用 Rust 语言特性解决问题：enum（枚举）、trait（特征）、generic（泛型）、closure（闭包）、RAII（资源获取即初始化）、Drop、iterator（迭代器）、newtype（新类型）、Result。
3. 用户点名经典模式时，先解释它在 Rust 中的等价实现，再指出是否有更简单的 Rust 惯用替代。
4. 如果模式会引入过度抽象，明确建议不要使用，并给出更小的组合式方案。
5. 在本项目中落地时，继续遵守 owner（归属模块）、最小可见性、`mod.rs` 只导出、未来易拆 crate 的规则。

## 工作流

1. 用 `references/pattern-selection.md` 选择候选模式，先排除 YAGNI（你不需要它）和过度抽象。
2. 如果用户需要 GoF 模式，读取 `references/gof-rust.md`，把 OO（面向对象）结构改写为 Rust 的 trait、enum、泛型、组合或 newtype。
3. 如果用户需要 Rust 原生方案，读取 `references/rust-idioms.md`，优先匹配 Rust Unofficial Patterns 中的惯用法与反模式提醒。
4. 修改代码前先确认 owner 模块和依赖方向，不新增全局 `utils` / `common` / `helper`。
5. 完成后按风险选择 `cargo check`、定向测试或 `cargo clippy` 验证。

## 输出要求

- 给出“为什么是这个模式”，而不是只给代码。
- 说明替代方案和不用某个模式的理由。
- 对 trait object（特征对象）、generic（泛型）、enum dispatch（枚举分发）、closure（闭包）之间的取舍要写清楚。
- 涉及公共 API 时补 rustdoc，并让示例路径和模块导航跟随当前结构。

## 参考链接

- Refactoring.Guru Rust 设计模式目录：<https://refactoring.guru/design-patterns/rust>
- Refactoring.Guru Abstract Factory Rust 示例：<https://refactoring.guru/design-patterns/abstract-factory/rust/example>
- Refactoring.Guru Adapter Rust 示例：<https://refactoring.guru/design-patterns/adapter/rust/example>
- Rust Unofficial Patterns 入口：<https://rust-unofficial.github.io/patterns/intro.html>
- Rust Unofficial Patterns 模式目录：<https://rust-unofficial.github.io/patterns/patterns/index.html>
- Rust Unofficial Patterns 惯用法目录：<https://rust-unofficial.github.io/patterns/idioms/index.html>
- Rust Unofficial Patterns 反模式目录：<https://rust-unofficial.github.io/patterns/anti_patterns/index.html>
- Rust Unofficial Patterns Newtype：<https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html>
- Rust Unofficial Patterns Builder：<https://rust-unofficial.github.io/patterns/patterns/creational/builder.html>
- Rust Unofficial Patterns Clone 反模式：<https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html>
