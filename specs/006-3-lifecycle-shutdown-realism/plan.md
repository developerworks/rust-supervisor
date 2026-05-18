# Implementation Plan(实现计划): 真实生命周期与无孤儿关停

**Branch(分支)**: `[006-3-lifecycle-shutdown-realism]` | **Date(日期)**: 2026-05-17 | **Spec(规格)**: `specs/006-3-lifecycle-shutdown-realism/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-3-lifecycle-shutdown-realism/spec.md`

## Summary(摘要)

本切片是 006 系列的第一优先实现. 核心目标是将 RuntimeControlState 从"状态标记型"升级为"真实生命周期治理型". 具体而言: 将 children: HashMap<ChildId, ManagedChildState> 替换为 slots: HashMap<ChildId, ChildSlot>. ChildSlot 数据结构包含 status(状态), generation(代次), attempt(尝试计数), restart_count(重启计数), cancellation_token(取消令牌), join_handle(异步等待句柄), last_exit(最近一次退出摘要), last_ready_at(最近一次就绪时间戳), last_heartbeat_at(最近一次心跳时间戳), restart_window(重启窗口), pending_restart(待重启指示器). 所有运行时命令 (start, restart, pause, resume, remove, quarantine, shutdown_tree) 必须真实操作 ChildSlot 中的取消令牌与 join handle, 禁止仅在内存字段状态机中改写标签.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, rust-version 1.88
**Primary Dependencies(主要依赖)**: Tokio(异步运行时) 已提供 CancellationToken, JoinHandle. 本切片不新增 crate.
**Storage(存储)**: N/A(不适用). ChildSlot 驻留在运行时内存中.
**Testing(测试)**: cargo test; 验收夹具必须钉死 RNG seed 与注入时钟 (依赖 tokio dev test-util). 并发测试使用 loom(并发模型测试) 在 CI 夜间队列完成; 日常开发验证以标准测试为主.
**Target Platform(目标平台)**: Linux(操作系统) 与 macOS(操作系统) 开发者工作站.
**Project Type(项目类型)**: Tokio(异步运行时) supervisor runtime(监督器运行时).
**Performance Goals(性能目标)**: ChildSlot 查找与取消令牌传播在微秒级完成, 不影响控制循环主路径延迟.
**Constraints(约束)**: 禁止兼容导出. src/ Rust 注释英文. 规格正文中文且术语 English(中文说明). ChildSlot 字段命名以 data-model.md 冻结为准.
**Scale/Scope(规模和范围)**: 单进程内一棵或多棵监督树, 每个 child id(子任务标识) 对应一个 ChildSlot. 并发请求通过队列化或幂等键仲裁.

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查.*

- **Module Ownership(模块所有权)**: ChildSlot 数据结构与运行时命令实现落在 src/runtime/ 模块. main.rs 只做参数解析与依赖拼装.
- **Supervision Contract(监督契约)**: 本切片全面改变监督行为: 生命周期状态, 启动/停止/重启/取消/关闭语义, 调用者可见错误均由 ChildSlot 中的实际执行上下文驱动.
- **Test Gate(测试关口)**: tasks.md 中测试任务先于实现任务. 验收测试覆盖: 并发重启冲突拒绝, 取消令牌传播, 超时中止, 孤儿进程检测.
- **Observable Failures(可观察失败)**: 并发违例返回 structured error(结构化错误) 并携带 ChildSlot 的 generation 与 RunningInstanceId. 关停过程暴露 ShutdownPhase 枚举.
- **Small Increment(小增量)**: 不新增异步运行时或持久化层. 仅在现有 RuntimeControlState 上替换数据结构与操作实现.
- **Chinese Writing(中文写作)**: 本文件与派生物使用中文叙述, 英文术语括注.

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/006-3-lifecycle-shutdown-realism/
├── spec.md              # 功能规格
├── plan.md              # 本文件
├── research.md          # Phase 0(研究阶段) 输出: ChildSlot 并发安全模型研究
├── data-model.md        # Phase 1(设计阶段) 输出: ChildSlot 字段定义与生命周期枚举
├── quickstart.md        # Phase 1(设计阶段) 输出: src/ 阅读顺序锚点
├── contracts/
│   ├── child-slot-api.md        # ChildSlot 公开方法契约
│   └── shutdown-phase-enum.md   # ShutdownPhase 枚举取值与迁移表
└── tasks.md             # Phase 2(任务阶段) 输出
```

### Source Code(源代码, 仓库根目录)

```text
src/
├── main.rs              # 仅参数解析与依赖拼装
├── runtime/
│   ├── mod.rs           # 模块入口, 重新导出 RuntimeControlState
│   ├── child_slot.rs    # ChildSlot 数据结构与构造方法
│   ├── control_loop.rs  # 控制循环, 调度生命周期指令到 ChildSlot
│   ├── shutdown.rs      # shutdown_tree 扇出与超时管理
│   └── admission.rs     # AdmissionSet 并发不变式
├── types/
│   ├── mod.rs           # 类型定义入口
│   ├── child_id.rs      # ChildId 类型
│   ├── running_instance_id.rs  # RunningInstanceId 类型
│   └── shutdown_phase.rs      # ShutdownPhase 枚举
└── lib.rs               # 公开 API 最小集合, 禁止兼容导出

tests/
├── lifecycle_integration.rs    # 全生命周期冒烟测试
├── concurrent_restart_test.rs  # 并发重启冲突测试
├── shutdown_orphan_test.rs     # 孤儿进程检测测试
└── join_timeout_test.rs        # join 超时与 abort 测试
```

**Structure Decision(结构决定)**: 采用 Rust 单 crate(包) 结构. runtime/ 模块持有 ChildSlot 所有权; types/ 模块持有共享类型定义. 这种分离避免循环依赖.

## Complexity Tracking(复杂度跟踪)

> **只有 Constitution Check(宪章检查) 存在违反项时, 才填写本节.**

| Violation(违反项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
|---|---|---|
| N/A(不适用) | - | - |
