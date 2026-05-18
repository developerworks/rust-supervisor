# Implementation Plan(实现计划): 平台边界, 说明文档与看板 IPC 安全强化

**Branch(分支)**: `006-1-platform-docs-ipc-security` | **Date(日期)**: 2026-05-17 | **Spec(规格)**: `specs/006-1-platform-docs-ipc-security/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-1-platform-docs-ipc-security/spec.md`

## Summary(摘要)

本切片处理两条横切线: 平台边界条件编译与 IPC 安全强化. 当前代码大量使用 Unix-only API(Unix 平台接口) 但没有 feature gate(功能开关) 保护. 必须先确定 Unix-only 策略 (条件编译或 feature gate), 再补齐 9 项 IPC 控制点 (C1-C9) 的默认配置模板与检查表. README 必须固定写明三目录架构和平台支持矩阵. 交付件是检查表与默认配置文件模板, 不涉及运行时控制循环改动.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, rust-version 1.88
**Primary Dependencies(主要依赖)**: tokio(异步运行时) 已包含 UnixListener(Unix 域套接字监听器); serde(序列化), confique(配置), tracing(结构化追踪) 间接满足. 如需 OS 凭证读取绑定库, 须在 Phase 0(研究阶段) 论证 syscall(系统调用) 覆盖矩阵.
**Storage(存储)**: N/A(不适用). IPC 控制点默认值驻留在内存配置结构中.
**Testing(测试)**: `cargo test` 覆盖构造放行样本与拒绝样本各一组; 验证拒绝路径返回结构化错误与审计条目.
**Target Platform(目标平台)**: Unix-like(类 Unix 系统) 为主要目标; 非类 Unix 组合以支持矩阵裁剪标记标明限制.
**Project Type(项目类型)**: Rust library(库) + config(配置模板).
**Performance Goals(性能目标)**: IPC 控制点检查必须在微秒级完成.
**Constraints(约束)**: 禁止兼容导出. 平台条件编译策略必须在 Phase 0(研究阶段) 书面确定. 如需 OS 凭证读取绑定库, 必须在 Phase 0 论证.
**Scale/Scope(规模和范围)**: 单进程内一套 IPC 监听路径; 9 项控制点默认值一次加载, 运行时按请求校验.

## Constitution Check(宪章检查)

_GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查._

- **Module Ownership(模块所有权)**: 平台 support(支持) 层落在 `src/platform/` 或 feature-gated 模块; IPC 安全默认值落在 `src/ipc/security/` 模块; config(配置) 模型落在 `src/config/` 模块. `src/main.rs` 只做参数解析与依赖拼装.
- **Supervision Contract(监督契约)**: N/A(不适用). 本切片不改变监督状态机迁移集合, 但迁移触发指令必须经过 IPC 控制点校验. 调用失败时不得留下半启动实例; IPC 拒绝必须携带结构化错误; 关停指令共用授权链路.
- **Test Gate(测试关口)**: `tasks.md` 中测试任务先于实现任务. 验收测试覆盖 9 项控制点的放行与拒绝样本. 最终运行 `cargo test` 全量.
- **Observable Failures(可观察失败)**: 每次拒绝必须暴露稳定的 tracing(结构化追踪) target 名称前缀与结构化错误载荷, 可在值班手册附录检索.
- **Small Increment(小增量)**: 不引入新异步运行时或持久化层. 如需 OS 凭证绑定库, 必须在 Phase 0 论证.
- **Chinese Writing(中文写作)**: 本文件与所有派生物 (research.md, data-model.md, quickstart.md, contracts/, tasks.md) 使用中文写作. 英文术语写成 `English(中文说明)`.

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/006-1-platform-docs-ipc-security/
├── spec.md              # 功能规格
├── plan.md              # 本文件
├── research.md          # Phase 0(研究阶段) 输出: 平台条件编译策略研究
├── data-model.md        # Phase 1(设计阶段) 输出: IPC 控制点数据模型
├── quickstart.md        # Phase 1(设计阶段) 输出: IPC 安全接入阅读顺序
├── contracts/
│   └── ipc-control-points.md  # C1-C9 控制点接口契约
└── tasks.md             # Phase 2(任务阶段) 输出
```

### Source Code(源代码, 仓库根目录)

```text
src/
├── main.rs              # 仅参数解析与依赖拼装
├── config/
│   └── ipc_security.rs  # IPC 控制点默认配置加载
├── ipc/
│   └── security/
│       ├── mod.rs       # 模块入口
│       ├── peer_identity.rs   # C1-C2: socket owner + peer credentials
│       ├── authz.rs           # C3: command authorization 模型
│       ├── replay.rs          # C4: replay protection
│       ├── limits.rs          # C5-C6: request size + rate limit
│       ├── audit.rs           # C7: audit persistence
│       ├── idempotency.rs     # C8: command idempotency key
│       └── allowlist.rs       # C9: external command allowlist
├── platform/
│   └── mod.rs           # Unix-only 条件编译或 feature gate 声明
└── lib.rs               # 公开 API 最小集合, 禁止兼容导出

tests/
└── ipc_security_integration.rs  # 9 项控制点放行/拒绝验收
```

**Structure Decision(结构决定)**: 采用 Rust 单 crate(包) 结构, IPC 安全模块按控制点编号拆分, 便于单文件审阅与独立测试.

## Complexity Tracking(复杂度跟踪)

> **只有 Constitution Check(宪章检查) 存在违反项时, 才填写本节.**

| Violation(违反项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
| ----------------- | ---------------------- | ---------------------------------------------------------- |
| N/A(不适用)       | -                      | -                                                          |
