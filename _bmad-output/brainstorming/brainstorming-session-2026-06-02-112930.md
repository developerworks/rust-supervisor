---
stepsCompleted: [1, 2]
inputDocuments: []
session_topic: "Supervisor(监督器), Service(服务), Job(一次性任务) 等角色接入契约"
session_goals: "方向清单, API(应用程序接口) 草案, specification(规格) 文档, 实现任务拆分"
selected_approach: "Progressive Technique Flow(渐进技巧流程)"
techniques_used:
  - What If Scenarios(如果场景)
  - Morphological Analysis(形态分析)
  - Six Thinking Hats(六顶思考帽)
  - Solution Matrix(方案矩阵)
ideas_generated:
  - 角色契约必须优先降低使用者的 cognitive complexity(认知复杂度), 并尽量接近 Java(编程语言) 生态的心智模型.
  - 每个角色需要提供 3 种接入方式, 分别是 annotation-like macro(标记宏) 入口, explicit trait(显式特征) 入口, abstract default implementation(抽象默认实现) 入口, 并且 3 种入口应该放在不同目录中.
  - 契约覆盖范围必须包含当前所有 TaskRole(任务角色) 变体, 即 Service(服务), Worker(后台任务), Job(一次性任务), Sidecar(边车), Supervisor(监督器).
  - macro entry(宏入口) 默认使用形态 2, 即在 impl block(实现块) 上标记角色, 并通过生命周期方法表达角色流程.
  - macro entry(宏入口) 的形态 1 和形态 3 记录为 optional implementation(可选实现), 但不作为默认推荐路径.
  - 过程宏需要独立 proc-macro crate(过程宏包), 因此项目需要评估从单 crate(包) 结构升级为 workspace(工作区) 结构.
context_file: ""
---

# 头脑风暴会话结果

**Facilitator(引导者):** {{user_name}}
**Date(日期):** {{date}}

## 会话概览

**Topic(主题):** Supervisor(监督器), Service(服务), Job(一次性任务) 等 role(角色) entry contracts(接入契约).

**Goals(目标):** 产出 direction list(方向清单), API(应用程序接口) draft(草案), specification(规格) document outline(文档大纲), implementation task breakdown(实现任务拆分).

### 背景说明

当前示例 `examples/step_02_supervisor_with_service/main.rs` 把 `run_service` 作为 free function(自由函数) 使用. 本会话需要探索项目如何把 lifecycle requirements(生命周期要求), startup behavior(启动行为), readiness(就绪), heartbeat(心跳), cancellation(取消), restart(重启), shutdown(关闭) 等责任变成显式契约.

### 会话设置

本会话先保留多个设计方向, 再比较每一种契约应该落在哪一层. 可选边界包括 trait(特征), Spec(规格), builder(构建器), runtime(运行时), example(示例).

### 早期用户约束

首要约束是 cognitive complexity(认知复杂度). 这表示使用者应该可以理解 role contract flow(角色契约流程), 而不需要先学习大量无关的内部细节. 期望体验接近 Java(编程语言) ecosystem(生态), 即一个 role(角色) 通常暴露清晰命名的 interface(接口), 小范围 lifecycle(生命周期), 以及可预期的 framework wiring(框架装配).

用户最倾向 annotation-like macro(标记宏) 入口, 因为这个入口更容易上手. 同时, 项目也需要 explicit trait(显式特征) 和 abstract default implementation(抽象默认实现) 两种变体. 这些变体应该放在不同目录中, 以保持项目结构清晰, 并且服务并行开发.

## 技巧选择

**Approach(方法):** Progressive Technique Flow(渐进技巧流程)

**Journey Design(流程设计):** 从 exploration(探索) 系统推进到 action(行动).

**Progressive Techniques(渐进技巧):**

- **Phase 1(阶段 1) - Exploration(探索):** 使用 What If Scenarios(如果场景), 目标是最大化生成契约方向.
- **Phase 2(阶段 2) - Pattern Recognition(模式识别):** 使用 Morphological Analysis(形态分析), 目标是映射 lifecycle(生命周期) 和 API(应用程序接口) 维度.
- **Phase 3(阶段 3) - Development(深化):** 使用 Six Thinking Hats(六顶思考帽), 目标是从 facts(事实), benefits(收益), risks(风险), creativity(创造性), process(流程) 角度审视方案.
- **Phase 4(阶段 4) - Action Planning(行动计划):** 使用 Solution Matrix(方案矩阵), 目标是形成 specification(规格) 章节和 implementation(实现) 任务.

**Journey Rationale(流程理由):** 本会话必须产出 direction list(方向清单), API(应用程序接口) draft(草案), specification(规格) document outline(文档大纲), implementation task breakdown(实现任务拆分). 因此, 流程先从宽泛备选方向开始, 再逐步转化为具体工程工作.

## 当前角色盘点

**Source(来源):** `src/policy/task_role_defaults.rs`, `src/spec/child_builder.rs`, `src/spec/child.rs`, `examples/*`, `specs/005-2-task-role-defaults/*`.

当前项目有 5 个 `TaskRole`(任务角色) variants(变体), 这些角色都必须纳入 role contract(角色契约) 工作范围.

| Role(角色)   | Current meaning(当前语义)                                           | Contract focus(契约重点)                                                                                                                                                                                                    | Existing anchors(现有锚点)                                                                                                                |
| ------------ | ------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Service`    | Long-running service(长期运行服务), 需要保持在线.                   | 必须显式表达 initialization(初始化), readiness(就绪), heartbeat(心跳), long-running loop(长期运行循环), cancellation(取消), cooperative shutdown(协作关闭).                                                                 | `TaskRole::Service`, `ChildSpecBuilder::service`, `examples/service/service_task.rs`, `examples/step_02_supervisor_with_service/main.rs`. |
| `Worker`     | Bounded background worker(有界后台任务), 完成有限工作后停止.        | 必须显式表达 batch work(批量工作) 或 bounded work(有界工作), success completion(成功完成), retry on failure(失败重试), completion event(完成事件).                                                                          | `TaskRole::Worker`, `ChildSpecBuilder::worker`, `examples/worker/worker_task.rs`.                                                         |
| `Job`        | One-shot job(一次性任务), 成功运行一次后保持停止.                   | 必须显式表达 one-shot execution(一次性执行), success stop(成功后停止), failure retry budget(失败重试预算), timeout escalation(超时升级), no permanent restart(禁止永久重启语义).                                            | `TaskRole::Job`, `ChildSpecBuilder::job`, `examples/job/job_task.rs`, `semantic_conflicts_for_child`.                                     |
| `Sidecar`    | Auxiliary sidecar(辅助边车), 绑定到 primary service(主服务).        | 必须显式表达 primary binding(主任务绑定), linked lifecycle(绑定生命周期), dependency(依赖), own restart scope(自身重启范围), no sidecar chain(禁止边车链).                                                                  | `TaskRole::Sidecar`, `SidecarConfig`, `ChildSpecBuilder::sidecar`, `examples/sidecar/sidecar_task.rs`, `validate_sidecar_local`.          |
| `Supervisor` | Nested supervisor unit(嵌套监督器单元), 外层把它当成一个受监督单元. | 必须把 role contract(角色契约) 与 `TaskKind::Supervisor`(监督器执行种类) 分开, 并显式表达 nested tree ownership(嵌套树归属), readiness(就绪), restart budget(重启预算), cancellation(取消), shutdown propagation(关闭传播). | `TaskRole::Supervisor`, `ChildSpecBuilder::supervisor`, `examples/supervisor/supervisor_task.rs`, `examples/task_role_demo.rs`.           |

### 角色与执行种类边界

`TaskRole`(任务角色) 描述 business lifecycle semantics(业务生命周期语义). `TaskKind`(任务执行种类) 描述 runtime execution shape(运行时执行形态), 包括 `AsyncWorker`(异步工作者), `BlockingWorker`(阻塞工作者), `Supervisor`(监督器节点). 契约设计必须避免混淆这两个概念.

### 契约范围决定

第一批 implementation scope(实现范围) 应该覆盖上面 5 个角色. 每个角色应该有 3 种 entry style(入口形式), 并且分别放在不同目录中. 这 3 种入口是 macro entry(宏入口), trait entry(特征入口), template entry(模板入口). macro entry(宏入口) 应该保持为推荐的 low cognitive complexity(低认知复杂度) 路径.

### Macro Entry(宏入口) 默认形态决定

当前决定采用形态 2, 即在 impl block(实现块) 上使用 role macro(角色宏), 让结构体承载 lifecycle methods(生命周期方法). 这个形态比 free function(自由函数) 更能表达流程, 也比 derive macro(派生宏) 更少隐藏行为.

示例形态如下:

```rust
struct QuoteService;

#[service(id = "quote-service", name = "Quote Service")]
impl QuoteService {
    async fn init(&mut self, ctx: &ServiceContext) -> ServiceResult {
        ctx.ready();
        Ok(())
    }

    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult {
        ctx.wait_shutdown().await;
        Ok(())
    }

    async fn shutdown(&mut self, ctx: &ServiceContext) -> ServiceResult {
        Ok(())
    }
}
```

该形态的核心目标是让使用者看到 "角色 -> 生命周期方法 -> 框架装配" 这条简单路径, 而不是先理解 `TaskFactory`(任务工厂), `ChildSpec`(子任务规格), `TaskContext`(任务上下文) 等内部对象.

### Workspace(工作区) 结构影响

迁移前, 项目 `Cargo.toml` 是 single crate(单包) 结构, package(包) 名称为 `rust-tokio-supervisor`, library crate(库包) 名称为 `rust_supervisor`.

如果实现 `#[service] impl QuoteService { ... }` 这种 attribute macro(属性宏), 或实现 `#[derive(ServiceRole)]` 这种 derive macro(派生宏), Rust(系统编程语言) 要求这些过程宏放在独立的 proc-macro crate(过程宏包) 中. proc-macro crate(过程宏包) 只负责在编译期生成代码, 不适合承载普通 runtime type(运行时类型), 例如 `ServiceContext`, `ServiceResult`, `ChildSpec`.

因此, role contract(角色契约) 设计采用 workspace(工作区) 结构. 当前仓库 `rust-supervisor` 自身作为 workspace root(工作区根目录), 并在该仓库内部放置主库和过程宏库.

用户最新澄清: 不是保留 root(根目录) 下的当前 Rust crate(包) 文件, 而是把当前主 crate(包) 内容整体下沉到 `/Users/0x00/Documents/rust-supervisor-tools/rust-supervisor/rust-supervisor`, 再在 `/Users/0x00/Documents/rust-supervisor-tools/rust-supervisor` 下新增 `rust-supervisor-macros`.

工作流和 agent(代理) 相关目录不应该跟随主 crate(包) 下沉. 这些目录属于 workspace root(工作区根目录) 上下文, 例如 `_bmad`, `_bmad-output`, `.agent`, `.agents`, `.specify`, `AGENTS.md`. 它们应该服务整个 workspace(工作区), 而不是只服务 `rust-supervisor` 主 crate(包).

已采用结构如下:

```text
/Users/0x00/Documents/rust-supervisor-tools/rust-supervisor/
├── .agent/
├── .agents/
├── .specify/
├── _bmad/
├── _bmad-output/
├── AGENTS.md
├── Cargo.toml
├── Cargo.lock
├── clippy.toml
├── deny.toml
├── specs/
├── rust-supervisor/
│   ├── Cargo.toml
│   ├── src/
│   ├── examples/
│   ├── tests/
│   ├── docs/
│   ├── manual/
│   ├── scripts/
│   ├── artifacts/
│   ├── fixtures/
│   ├── README.md
│   └── README.zh.md
├── rust-supervisor-macros/
│   ├── Cargo.toml
│   └── src/
└── ...
```

该结构表示:

- `/Users/0x00/Documents/rust-supervisor-tools/rust-supervisor` 是 workspace root(工作区根目录).
- `/Users/0x00/Documents/rust-supervisor-tools/rust-supervisor/rust-supervisor` 是主 library crate(库包) 目录, 并承载当前项目的 `src`, `examples`, `tests`, `docs`, `manual`, `scripts`, `artifacts`, `fixtures`, README(说明文档) 等主包内容.
- `/Users/0x00/Documents/rust-supervisor-tools/rust-supervisor/rust-supervisor-macros` 是 proc-macro crate(过程宏包) 目录.
- root `Cargo.toml` 只做 workspace(工作区) 成员声明和 workspace package metadata(工作区包元数据), 不承载业务代码.
- root(根目录) 保留 `_bmad`, `_bmad-output`, `.agent`, `.agents`, `.specify`, `AGENTS.md`, `specs`, `clippy.toml`, `deny.toml` 等 workflow artifacts(工作流产物) 和 tool configuration(工具配置), 因为这些文件描述整个 workspace(工作区) 的开发流程, 规格, 代理上下文和质量门禁.
- 上一级 `/Users/0x00/Documents/rust-supervisor-tools` 不参与本次 workspace(工作区) 设计, 因此不会牵动其它独立 GitHub(代码托管平台) 仓库.
- 当前仓库的 Git(版本控制系统) 历史保留在同一个仓库内. 文件移动后, Git(版本控制系统) 可以通过 `git log --follow` 追踪单个文件的历史.
- 实际迁移时不能把目录本身移动到它自己的子目录中. 正确做法是先创建 `rust-supervisor/` 子目录, 再把当前主 crate(包) 相关文件和目录移动进去, 同时保留 `.git/` 和 workflow artifacts(工作流产物) 在 workspace root(工作区根目录).
- CI(持续集成) 和本地脚本命令需要同步改为 workspace-aware(工作区感知) 形式. 例如 package-specific cargo command(指定包命令) 使用 `-p rust-tokio-supervisor`, 主包脚本使用 `working-directory: rust-supervisor`.

下面两条路径是早期备选方案, 后续应以用户明确的目标结构为准.

最小改动路径:

```text
rust-supervisor/
├── Cargo.toml
├── src/
├── examples/
├── tests/
├── specs/
└── crates/
    └── rust-supervisor-macros/
        ├── Cargo.toml
        └── src/
```

该路径保留当前主 package(包) 在仓库根目录, 只新增 workspace(工作区) 成员 `crates/rust-supervisor-macros`. 这个路径改动小, 适合先验证 macro entry(宏入口).

完整拆分路径:

```text
rust-supervisor/
├── Cargo.toml
├── crates/
│   ├── rust-supervisor/
│   │   ├── Cargo.toml
│   │   └── src/
│   └── rust-supervisor-macros/
│       ├── Cargo.toml
│       └── src/
├── examples/
├── tests/
└── specs/
```

该路径把主 library crate(库包) 移到 `crates/rust-supervisor/`, 同时把过程宏放到 `crates/rust-supervisor-macros/`. 这个路径更适合长期并行开发, 但需要同步调整 examples(示例), tests(测试), package metadata(包元数据), docs(文档) 和 CI(持续集成) 命令.

职责划分:

- `rust-supervisor`: 主 library crate(库包), 保存 `ServiceContext`, `JobContext`, `RoleContract`, `ChildSpec`, `TaskFactory` 等运行时类型.
- `rust-supervisor-macros`: proc-macro crate(过程宏包), 保存 `#[service]`, `#[worker]`, `#[job]`, `#[sidecar]`, `#[supervisor_role]` 等宏实现.
- root `Cargo.toml`: workspace(工作区) 注册入口, 负责统一依赖版本和成员声明.

这种结构不是为了兼容导出, 而是为了满足 Rust(系统编程语言) 的过程宏编译模型, 并让 macro entry(宏入口) 与 runtime contract(运行时契约) 分开演进.

### Macro Entry(宏入口) 可选形态记录

以下 2 种形态需要记录为 optional implementation(可选实现), 防止后续设计遗漏. 它们不作为默认推荐路径.

#### 形态 1: 函数标记

形态 1 在 free function(自由函数) 上使用 role macro(角色宏). 这个形态最短, 适合极小示例或快速迁移, 但它会把 lifecycle(生命周期) 压缩到一个函数里, 因此不适合作为首要路径.

```rust
#[service(id = "quote-service", name = "Quote Service")]
async fn quote_service(ctx: ServiceContext) -> ServiceResult {
    ctx.ready();
    ctx.wait_shutdown().await;
    Ok(())
}
```

可选实现定位:

- 用于 demo(演示) 和 quick migration(快速迁移).
- 用于把现有 `run_service` 这类自由函数快速包装为角色.
- 不用于表达复杂 lifecycle(生命周期).

#### 形态 3: derive macro(派生宏)

形态 3 在 struct(结构体) 上使用 derive macro(派生宏), 自动生成 role contract(角色契约) 的 glue code(胶水代码). 这个形态声明最短, 但是行为更隐式, 使用者可能看不出运行逻辑在哪里.

```rust
#[derive(ServiceRole)]
#[service(id = "quote-service", name = "Quote Service")]
struct QuoteService;
```

可选实现定位:

- 用于高级使用者或高度约定化场景.
- 用于状态很少, 生命周期默认值很强的简单角色.
- 不作为默认推荐路径, 因为它会隐藏部分 lifecycle(生命周期) 流程.

## Specification(规格) 收敛结果

基于前面的角色讨论, 当前已经创建 `specs/007-role-contracts/` 规格目录. 该目录用于把 macro entry(宏入口), trait entry(特征入口), template entry(模板入口), runtime adapter(运行时适配器) 和 5 个角色契约固定下来.

新增规格文件如下:

```text
specs/007-role-contracts/
├── spec.md
├── api-draft.md
├── data-model.md
├── contracts/
│   ├── service-contract.md
│   ├── worker-contract.md
│   ├── job-contract.md
│   ├── sidecar-contract.md
│   └── supervisor-contract.md
└── tasks.md
```

该规格目录的收敛结论如下:

- `spec.md` 固定 role contract(角色契约) 的需求, 用户故事, 边界情况和成功标准.
- `api-draft.md` 固定使用者可见 API(应用程序接口), 包括 `#[service]`, `#[worker]`, `#[job]`, `#[sidecar]`, `#[supervisor_role]`.
- `data-model.md` 固定 `RoleMetadata`(角色元数据), `RoleContext`(角色上下文), `RoleResult`(角色结果), `RoleLifecyclePhase`(角色生命周期阶段), `RoleAdapter`(角色适配器) 和 `MacroInput`(宏输入).
- `contracts/` 目录按角色拆分 Service(服务), Worker(后台任务), Job(一次性任务), Sidecar(边车), Supervisor(监督器) 的生命周期契约.
- `tasks.md` 将实现拆为 7 个阶段, 并以 Service(服务) 最小闭环作为 MVP(最小可用产品).

实现顺序当前收敛为:

```text
Specification(规格)
-> Service Runtime Contract(服务运行时契约)
-> Service Macro Entry(服务宏入口)
-> Service Tests and Example(服务测试和示例)
-> Other Role Contracts(其他角色契约)
-> Template Entry(模板入口)
-> Quality Gates(质量门禁)
```

下一步如果进入实现, 应该先执行 `tasks.md` 中的 T005 到 T010, 即在 `rust-supervisor/src/role/` 中实现 `ServiceContext`(服务上下文), `ServiceRole`(服务角色特征), `ServiceResult`(服务结果) 和 `ServiceRoleAdapter`(服务角色适配器). 这一步先不写 macro(宏), 因为 trait entry(特征入口) 是宏入口的真实契约基础.
