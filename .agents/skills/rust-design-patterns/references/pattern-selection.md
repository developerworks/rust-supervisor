# Rust 设计模式选择矩阵

本参考用于在写代码前判断“需要哪种模式，或者是否根本不需要模式”。先看设计压力，再选择 Rust 原生表达，最后才套用经典 GoF 名称。

## 先排除

- 如果只是把几行重复代码挪走，优先提取函数，不要引入 trait 层次。
- 如果候选实现集合是封闭的，优先 enum（枚举）和 `match`，不要默认 trait object（特征对象）。
- 如果只需要一次性回调，优先 closure（闭包）或函数参数，不要默认 Command（命令模式）。
- 如果只是构造参数多，先看简单 `new` / `Default` / 配置结构体，再决定是否需要 Builder（建造者模式）。
- 如果只是为了“方便全局访问”，不要默认 Singleton（单例模式）；优先显式依赖注入。
- 如果只是为了绕开 borrow checker（借用检查器），不要靠盲目 `clone()`，先调整所有权边界。

## 设计压力到模式

| 设计压力 | 首选 Rust 表达 | 可选模式 | 避坑 |
| --- | --- | --- | --- |
| 构造复杂对象，有很多可选项 | `Default` + builder struct | Builder | 不要为两三个字段引入复杂 typed builder |
| 必填字段顺序和状态必须编译期保证 | typestate（类型状态）builder | Builder + typestate | 只在不变式重要时使用 |
| 构造一组相关实现 | trait + associated type（关联类型）或泛型 | Abstract Factory | 闭集可用 enum，别强行 OO 类层次 |
| 运行期可替换算法 | `dyn Trait` 或 boxed closure | Strategy | 静态分发优先泛型，动态分发才用 trait object |
| 编译期可替换算法 | generic type parameter（泛型参数） | Strategy | 注意 monomorphization（单态化）带来的编译体积 |
| 适配外部 API 或协议字段 | newtype + `From` / `TryFrom` | Adapter | 显式转换放 owner 模块的 `mapping.rs` |
| 保存资源并自动释放 | RAII guard + `Drop` | RAII | `Drop` 不应隐藏可能失败的重要业务动作 |
| 需要可组合处理链 | iterator pipeline、middleware、函数链 | Chain of Responsibility | 处理顺序要测试覆盖 |
| 对外暴露简化入口 | 小门面模块 + selective re-export | Facade | `mod.rs` 仍只负责导出，不承载实现 |
| 多观察者事件通知 | channel、callback registry、broadcast | Observer | 明确背压、取消订阅和生命周期 |
| 状态驱动行为 | enum state machine 或 typestate | State | 简单状态机不需要 trait object |
| 请求需要排队、撤销、记录 | command struct + `execute` | Command | 一次性函数调用不需要封装成命令对象 |
| 树形结构统一处理 | enum tree 或 trait object tree | Composite | 避免过度动态派发 |
| 给对象叠加行为 | wrapper newtype、middleware | Decorator | 不要滥用 `Deref` 做伪继承 |
| 缓存或共享不可变大状态 | `Arc`, interner, cache owner | Flyweight | 明确共享状态的 owner 和失效策略 |
| 控制访问、懒加载、远程调用 | wrapper service/client | Proxy | 不要隐藏网络、IO 或权限失败 |
| 跨 FFI 边界 | opaque handle + wrapper type | Object-Based API | unsafe 收口到小模块 |

## 本项目落地检查

- 先判定 owner（归属模块），再决定文件位置。
- 不新增全局集中点；模式辅助代码应贴近 owner。
- trait 转换归 `mapping.rs`，常量归 `constants.rs`，serde 兼容归 `serde_functions.rs`。
- 如果引入新 public API（公共接口），补 rustdoc 和单测。
- 如果重构只为结构归位，不顺手改变默认值、协议兼容或运行时语义。

## 主要资料

- Refactoring.Guru: <https://refactoring.guru/design-patterns/rust>
- Rust Unofficial Patterns: <https://rust-unofficial.github.io/patterns/intro.html>
