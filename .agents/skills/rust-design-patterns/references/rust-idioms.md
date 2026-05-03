# Rust Unofficial Patterns 参考

Rust Unofficial Patterns 把内容分成 idioms（惯用法）、design patterns（设计模式）、anti-patterns（反模式）和 functional patterns（函数式模式）。在 Rust 项目中，很多经典模式不需要照搬 OO（面向对象）结构，因为语言本身已经提供了更小的表达方式。

## 优先考虑的惯用法

| 惯用法 | 适用场景 | 本项目约束 |
| --- | --- | --- |
| Borrowed types for arguments（参数使用借用类型） | API 接受 `&str` / `&[T]` 等更宽输入 | 不无故要求调用方交出所有权 |
| Constructor（构造函数） | 简单构造或少量必填字段 | 优先 `new`，复杂才上 Builder |
| Default trait（默认值特征） | 有稳定默认值的配置或状态 | 可配置项不要伪装成常量 |
| RAII guards（RAII 守卫） | 进入作用域获取资源，离开作用域释放 | 失败动作不要藏进 `Drop` |
| `mem::take` / `mem::replace` | 在枚举状态迁移中取走 owned value（拥有值） | 先保证状态语义清楚 |
| On-stack dynamic dispatch（栈上动态分发） | 少量分支选择不同实现但不想 heap allocate（堆分配） | 简单闭集优先 enum |
| Privacy for extensibility（用隐私保持扩展性） | 对外 API 保持稳定，内部可替换 | 默认最小可见性 |
| Newtype（新类型） | 单位、安全边界、封装内部类型、协议适配 | 字段默认私有，转换放 owner |
| Fold（折叠） | 从迭代器累计构造复杂值 | 累计状态要命名准确 |
| Compose structs（组合结构体） | 复用行为或状态块 | 优先组合，不堆叠抽象 |
| Prefer small crates（偏好小 crate） | 未来拆 crate 或引入依赖 | 与本项目“未来易拆 crate”目标一致 |
| Contain unsafety（收口 unsafe） | FFI 或底层优化 | unsafe 必须隔离到小模块并补文档 |
| Custom traits for bounds（自定义 trait 简化约束） | 类型约束过长影响可读性 | trait 名必须表达真实语义 |

## 常见反模式

- Clone to satisfy the borrow checker（为通过借用检查器而克隆）：只有当 clone 表达真实共享、复制或快照语义时才保留。
- Deref polymorphism（Deref 多态）：不要用 `Deref` 模拟继承或隐藏 API 边界。
- `#[deny(warnings)]` 作为库默认策略：可能导致依赖或编译器升级时破坏使用者构建。
- 全局 mutable singleton（可变全局单例）：隐藏依赖和测试耦合，优先显式传入依赖。
- Stringly typed state（字符串状态）：能用 enum 或 newtype 表达的不变式，不靠字符串约定。
- Premature trait object（过早 trait object）：静态已知或闭集分发时，优先泛型或 enum。

## Rust 化模式落地提示

- Builder 要服务构造复杂度或不变式，不是为了“看起来像模式”。
- Strategy 在 Rust 中常常只是 trait、泛型、闭包或 enum dispatch。
- Adapter 通常是 newtype + 显式转换，而不是继承适配类。
- Visitor 只有在数据结构稳定但操作频繁新增时才有价值；闭集 enum 通常直接 `match`。
- Facade 在本项目中通常表现为模块根 selective re-export，不代表可以把实现塞进 `mod.rs`。
- Prototype 通常是 `Clone`，但 clone 必须是业务语义，不是借用问题遮羞布。

## 主要资料

- Rust Unofficial Patterns introduction: <https://rust-unofficial.github.io/patterns/intro.html>
- Pattern index: <https://rust-unofficial.github.io/patterns/patterns/index.html>
- Idioms index: <https://rust-unofficial.github.io/patterns/idioms/index.html>
- Newtype: <https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html>
- Builder: <https://rust-unofficial.github.io/patterns/patterns/creational/builder.html>
- Clone anti-pattern: <https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html>
- Prefer small crates: <https://rust-unofficial.github.io/patterns/patterns/structural/small-crates.html>
