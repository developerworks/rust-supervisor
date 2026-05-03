# GoF 模式到 Rust 的翻译

Refactoring.Guru 的 Rust 目录覆盖 23 个 GoF（Gang of Four，四人帮经典模式）模式。本参考不照搬面向对象类层次，而是说明在 Rust 中通常如何表达。

## Creational Patterns（创建型模式）

| GoF 模式 | Rust 中的常见表达 | 使用时机 | 慎用点 |
| --- | --- | --- | --- |
| Abstract Factory（抽象工厂） | trait + associated types、泛型工厂、闭集 enum factory | 需要成组创建相关对象且实现族可替换 | 不要为了“工厂感”制造深 trait 层次 |
| Builder（建造者） | `TBuilder`、`Default`、链式 setter、typestate builder | 参数多、可选项多、构造有校验 | 简单对象用 struct literal 或 `new` 即可 |
| Factory Method（工厂方法） | trait 方法返回具体类型、associated type、closure factory | 调用方不应知道具体构造细节 | Rust 没有经典继承，别照搬 superclass 术语 |
| Prototype（原型） | `Clone`、`Arc::clone`、模板配置复制 | 从已有值派生新值 | clone 必须有意为之，不能只是躲 borrow checker |
| Singleton（单例） | `OnceLock` / `LazyLock` / 显式注入共享 handle | 真正全局、不可变或受控初始化资源 | 本项目默认反对全局状态和隐藏依赖 |

## Structural Patterns（结构型模式）

| GoF 模式 | Rust 中的常见表达 | 使用时机 | 慎用点 |
| --- | --- | --- | --- |
| Adapter（适配器） | newtype、`From` / `TryFrom`、薄 wrapper | 外部协议、库 API、领域类型不一致 | 转换 owner 要正确，避免全局 conversions |
| Bridge（桥接） | trait boundary + composition（组合） | 抽象与实现需要独立变化 | 先确认是否只需要一个 trait 参数 |
| Composite（组合） | enum tree、trait object tree | 树结构需要统一遍历或操作 | 简单数据树优先 enum |
| Decorator（装饰器） | wrapper type、middleware stack | 给行为增加日志、缓存、校验、限流 | 不用 `Deref` 伪装继承 |
| Facade（外观） | 模块根 selective re-export、service facade | 给复杂子系统提供稳定入口 | 门面只做边界，不把逻辑塞进 `mod.rs` |
| Flyweight（享元） | interned value、共享不可变状态、cache | 大量重复小对象共享状态 | 明确缓存失效和并发语义 |
| Proxy（代理） | wrapper client/service、lazy loader、权限检查层 | 控制访问、懒加载、远程边界 | 不隐藏 IO、权限和失败模式 |

## Behavioral Patterns（行为型模式）

| GoF 模式 | Rust 中的常见表达 | 使用时机 | 慎用点 |
| --- | --- | --- | --- |
| Chain of Responsibility（责任链） | middleware、iterator chain、handler list | 请求按顺序经过多个处理器 | 背压、短路和错误传播要明确 |
| Command（命令） | command struct、closure、enum command | 需要排队、记录、撤销、重放 | 一次性调用优先函数 |
| Iterator（迭代器） | `Iterator` trait、adapter pipeline | 遍历集合且隐藏内部结构 | 注意所有权、借用和惰性求值 |
| Mediator（中介者） | coordinator service、channel hub | 多组件交互需要降耦 | 中介者容易变成大泥球 |
| Memento（备忘录） | snapshot struct、state diff | 需要保存/恢复状态 | 快照边界和隐私要清晰 |
| Observer（观察者） | channel、callback registry、broadcast stream | 多订阅者事件通知 | 取消订阅、生命周期、背压必须设计 |
| State（状态） | enum state machine、typestate、trait object state | 状态影响行为 | 简单状态优先 enum |
| Strategy（策略） | trait、generic、closure、enum dispatch | 算法可替换 | Rust 中很多策略可直接用 trait/generic 表达 |
| Template Method（模板方法） | trait default methods、函数组合 | 固定流程中个别步骤可替换 | 不要模拟继承层次 |
| Visitor（访问者） | enum + `match`、visitor trait | 数据结构稳定、操作经常新增 | Rust 中闭集 AST 常用 `match` 更直接 |

## 判断动态分发还是静态分发

- 用 generic（泛型）：实现集合编译期已知、性能敏感、调用频繁。
- 用 `dyn Trait`（特征对象）：实现集合运行期选择、插件化、需要异构容器。
- 用 enum dispatch（枚举分发）：实现集合封闭、需要清晰穷尽匹配。
- 用 closure（闭包）：行为很小、只需要一次性注入或局部替换。

## 主要资料

- Refactoring.Guru Rust examples: <https://refactoring.guru/design-patterns/rust>
- Abstract Factory Rust example: <https://refactoring.guru/design-patterns/abstract-factory/rust/example>
- Adapter Rust example: <https://refactoring.guru/design-patterns/adapter/rust/example>
