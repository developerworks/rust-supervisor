# Feature Specification(功能规格): 角色接入契约

**Feature Branch(功能分支)**: `[007-role-contracts]`
**Created(创建日期)**: 2026-06-02
**Status(状态)**: Frozen(已冻结)
**Input(输入)**: 用户要求把 `Supervisor`(监督器), `Service`(服务), `Worker`(后台任务), `Job`(一次性任务), `Sidecar`(边车) 从自由函数接入改为显式契约, 并降低使用者的 cognitive complexity(认知复杂度).

## Dependency Note(依赖说明)

本功能依赖 `specs/005-2-task-role-defaults/spec.md` 中已经定义的 `TaskRole`(任务角色) 语义. 本功能不重新定义角色默认策略, 只定义使用者如何用更清晰的 role contract(角色契约) 接入现有 runtime(运行时).

本功能依赖新的 workspace(工作区) 结构. `rust-supervisor` 主包保存 runtime contract(运行时契约), `rust-supervisor-macros` 宏包保存 proc-macro(过程宏) 实现.

## User Scenarios & Testing(用户场景和测试)

### User Story 1(用户故事 1) - 使用宏声明服务角色 (Priority(优先级): P1)

作为应用作者, 我希望用 `#[service]` 直接声明 `Service`(服务) 的 lifecycle methods(生命周期方法), 以便我不用先理解 `TaskFactory`(任务工厂), `TaskContext`(任务上下文) 和 `ChildSpec`(子任务规格) 的内部装配细节.

**Why this priority(为什么是这个优先级)**: 当前示例中的 `run_service` 是自由函数, 它能运行, 但是它没有把初始化, 就绪, 运行和关闭表达为显式契约.

**Independent Test(独立测试)**: 使用 `examples/step_02_supervisor_with_service/main.rs` 的等价场景, 验证 `#[service]` 生成的子任务规格能被 runtime(运行时) 启动, 报告 readiness(就绪), 响应 shutdown(关闭), 并通过 `cargo test --workspace` 与宏编译测试.

### User Story 2(用户故事 2) - 使用显式特征接入角色 (Priority(优先级): P1)

作为不想使用 macro(宏) 的应用作者, 我希望可以直接实现 `ServiceRole`(服务角色特征), `WorkerRole`(后台任务角色特征), `JobRole`(一次性任务角色特征), `SidecarRole`(边车角色特征) 和 `SupervisorRole`(监督器角色特征), 以便我可以在 IDE(集成开发环境) 中看到完整契约.

**Why this priority(为什么是这个优先级)**: trait(特征) 入口是宏入口的真实契约基础, 也是编译器错误和文档说明最稳定的来源.

**Independent Test(独立测试)**: 为每个角色写一个不使用 macro(宏) 的最小实现, 验证 adapter(适配器) 能生成现有 runtime(运行时) 可消费的 `TaskFactory`(任务工厂).

### User Story 3(用户故事 3) - 使用模板默认实现减少样板代码 (Priority(优先级): P2)

作为需要多个相似角色的应用作者, 我希望可以继承 template(模板) 入口的默认生命周期实现, 以便只覆盖关键方法, 而不重复写空的 `init`, `shutdown` 或 `complete`.

**Why this priority(为什么是这个优先级)**: 模板入口能支持 Java(编程语言) 生态中常见的 abstract base class(抽象基类) 心智模型, 但它不能取代宏入口和特征入口.

**Independent Test(独立测试)**: 对每个角色准备一个只实现必选方法的模板示例, 验证默认生命周期钩子不会改变现有角色默认策略.

## Edge Cases(边界情况)

- 当 `#[service]` 缺少 `run` 方法时, macro(宏) 必须产生可读的 compile error(编译错误), 并指出缺少的生命周期方法.
- 当 `#[worker]` 缺少 `work` 方法时, macro(宏) 必须产生可读的 compile error(编译错误).
- 当 `Job`(一次性任务) 的 `child_spec()` 被额外配置为 permanent restart(永久重启) 语义时, 现有 runtime policy(运行时策略) 必须拒绝或生成明确的配置冲突诊断.
- 当 `#[sidecar]` 缺少 `primary` 参数时, macro(宏) 必须产生可读的 compile error(编译错误).
- 当 `#[supervisor_role]` 缺少 `build_tree` 方法时, macro(宏) 必须产生可读的 compile error(编译错误).
- 后续如果支持多个 attribute macro(属性宏) 组合检查, 当使用者同时在同一个 `impl block`(实现块) 上标记多个角色宏时, macro(宏) 必须拒绝该写法.
- 当 context(上下文) 暴露能力不足以支持角色生命周期时, 应优先扩展对应 role context(角色上下文), 不得把完整 `TaskContext`(任务上下文) 暴露给使用者.

## Requirements(需求)

### Functional Requirements(功能需求)

- **FR-001**: 系统必须提供 `#[service]`, `#[worker]`, `#[job]`, `#[sidecar]`, `#[supervisor_role]` 五个 attribute macro(属性宏) 入口.
- **FR-002**: 系统必须提供 `ServiceRole`(服务角色特征), `WorkerRole`(后台任务角色特征), `JobRole`(一次性任务角色特征), `SidecarRole`(边车角色特征) 和 `SupervisorRole`(监督器角色特征) 五个 explicit trait(显式特征) 入口.
- **FR-003**: 系统必须提供 template(模板) 入口, 让使用者可以只覆盖必选 lifecycle method(生命周期方法).
- **FR-004**: macro(宏) 入口必须通过 adapter(适配器) 生成现有 runtime(运行时) 可消费的 `TaskFactory`(任务工厂), 并且使用者不需要直接构造 `TaskFactory`(任务工厂).
- **FR-005**: `Service`(服务) 必须使用 `init -> run -> shutdown` 生命周期, 其中 `run` 是必选方法.
- **FR-006**: `Worker`(后台任务) 必须使用 `init -> work -> complete` 生命周期, 其中 `work` 是必选方法.
- **FR-007**: `Job`(一次性任务) 必须使用 `init -> run -> complete` 生命周期, 其中 `run` 是必选方法, 并且成功完成后不得自动按 permanent restart(永久重启) 语义运行.
- **FR-008**: `Sidecar`(边车) 必须使用 `init -> run -> shutdown` 生命周期, 其中 `run` 和 `primary` 是必选项.
- **FR-009**: `Supervisor`(监督器) 必须使用 `build_tree -> run -> shutdown` 生命周期, 其中 `build_tree` 是必选方法.
- **FR-010**: 所有 role context(角色上下文) 必须只暴露该角色需要的能力, 并且不得直接暴露完整 `TaskContext`(任务上下文).
- **FR-011**: 所有 role result(角色结果) 必须能映射为现有 `TaskResult`(任务结果), 并保留结构化错误的能力.
- **FR-012**: 所有宏生成代码必须遵守现有模块所有权, 不得新增 compatibility exports(兼容导出).

### Key Entities(关键实体)

- **`RoleContract`(角色契约)**: 使用者看到的角色生命周期声明, 包含 role metadata(角色元数据), context(上下文), result(结果) 和 lifecycle methods(生命周期方法).
- **`RoleContext`(角色上下文)**: 对 `TaskContext`(任务上下文) 的窄接口封装, 每个角色有自己的上下文类型.
- **`RoleResult`(角色结果)**: 每个角色的结果类型别名或结构化结果, 可映射到 `TaskResult`(任务结果).
- **`RoleAdapter`(角色适配器)**: 把 `RoleContract`(角色契约) 转换成 `TaskFactory`(任务工厂) 的内部层.
- **`RoleMacro`(角色宏)**: 编译期解析使用者的 `impl block`(实现块), 并生成 role trait(角色特征) 与 adapter(适配器) 所需代码.

## Constitution Alignment(宪章一致)

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本功能改变使用者声明受监督单元的入口, 但不改变 `TaskRole`(任务角色) 的默认监督行为. 每个角色的生命周期阶段必须写入契约文档.
- **Failure behavior(失败行为)**: 角色方法返回错误时, adapter(适配器) 必须把错误转换为现有 runtime(运行时) 可观察的失败结果.
- **Shutdown behavior(关闭行为)**: `Service`(服务), `Sidecar`(边车) 和 `Supervisor`(监督器) 必须明确 shutdown(关闭) 钩子. `Worker`(后台任务) 和 `Job`(一次性任务) 必须明确 cancellation(取消) 查询能力.

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: runtime contract(运行时契约) 位于 `rust-supervisor/src/role/`. proc-macro(过程宏) 位于 `rust-supervisor-macros/src/`.
- **Compatibility exports(兼容导出)**: None(无). 本功能不新增兼容导出.
- **Diagnostics(诊断)**: macro(宏) 的 compile error(编译错误) 必须指出角色, 缺失方法或参数, 以及可执行的修复方向.
- **Dependency impact(依赖影响)**: 宏包预计需要 `syn`(Rust 语法解析库), `quote`(代码生成库) 和 `proc-macro2`(过程宏辅助库). 计划阶段必须说明这些依赖不可避免, 因为 attribute macro(属性宏) 需要解析 `impl block`(实现块) 并生成代码.

## Success Criteria(成功标准)

- **SC-001**: 使用者可以用 `#[service]` 重写 `examples/step_02_supervisor_with_service/main.rs` 中的自由函数服务示例, 并保持运行行为一致.
- **SC-002**: 5 个角色都拥有 macro entry(宏入口), trait entry(特征入口) 和 template entry(模板入口) 的文档草案和至少 1 个编译示例.
- **SC-003**: 对缺少必选方法或参数的宏用例, compile-fail(编译失败) 测试覆盖率达到 100%.
- **SC-004**: `cargo check --workspace --all-targets`, `cargo test --workspace` 和宏编译测试全部通过.
- **SC-005**: 使用者在最小示例中不需要直接写 `TaskFactory`(任务工厂), `TaskContext`(任务上下文) 或 `ChildSpec`(子任务规格), 也能完成角色接入.

## Assumptions(假设)

- 当前实现已经覆盖 `Service`(服务), `Worker`(后台任务), `Job`(一次性任务), `Sidecar`(边车) 和 `Supervisor`(监督器) 5 个角色. `Service`(服务) 最小闭环只是实施顺序, 不是最终范围限制.
- 形态 2 是默认 macro entry(宏入口), 即 attribute macro(属性宏) 标在 `impl block`(实现块) 上.
- 形态 1 的自由函数宏和形态 3 的 derive macro(派生宏) 是 optional implementation(可选实现), 不进入第一批默认路径.
- `TaskRole`(任务角色) 和 `TaskKind`(任务执行种类) 必须继续分开. 前者表达业务生命周期语义, 后者表达 runtime(运行时) 执行形态.
