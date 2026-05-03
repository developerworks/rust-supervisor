---
name: agent-client-protocol
description: "Use when the user needs authoritative help with ACP (Agent Client Protocol): designing or implementing ACP clients, agents, adapters, or ACP-enabled products; interpreting protocol semantics; handling initialization, sessions, prompt turns, permissions, modes, transports, MCP bridging, interoperability debugging, and release validation."
---

# Agent Client Protocol

这份 skill 里的 ACP 专指 Agent Client Protocol（代理客户端协议）。

用它来回答“如何基于 ACP 做软件”，而不是只回答“ACP 是什么”。目标是让回答既站在官方协议语义上，又能落到客户端、代理端、适配层、测试、日志、验收和发布。

## 何时使用

- 用户要设计或实现 ACP Client（客户端）/ Agent（代理）/ Adapter（适配器）/ Proxy（桥接代理）/ ACP-enabled product（支持 ACP 的产品）
- 用户要弄清 `initialize`、`session/new`、`session/load`、`session/prompt`、`session/update`、`session/cancel`、`session/request_permission`
- 用户要处理 capability negotiation（能力协商）、authentication（认证）、session persistence（会话持久化）、stop reason（停止原因）、cancellation（取消）、permission UX（权限交互）、mode switching（模式切换）、MCP server forwarding（MCP 服务转发）、transport（传输）选择、vendor interoperability（厂商互操作）
- 用户要 review（评审）或调试 ACP 兼容性问题，或给 ACP 软件写测试、日志、文档、验收清单
- 用户要在 Rust 项目中落地 ACP，选择 crate、组织状态机、处理异步并发

## 回答原则

1. 先分清问题层次：
   - 协议要求
   - 官方实现或官方文档行为
   - 生态约定或 vendor（厂商）特性
   - 工程建议
2. 明确标注：
   - “协议 MUST/SHOULD/MAY” 是规范语义
   - “建议这样实现” 是工程建议
   - “某个产品支持这个模式” 是产品特性，不自动等于 ACP 核心规范
3. 优先使用官方 ACP 站点和官方厂商文档；遇到时效性问题先回官方页面核对。
4. 默认从 state machine（状态机）、capabilities（能力）、message flow（消息流）、cancellation safety（取消安全）、permission boundary（权限边界）、observability（可观测性）组织答案。
5. 只要进入实现讨论，就主动补：
   - 模块边界
   - 关键数据结构
   - 错误模型
   - `tracing` 点位
   - 单元测试与互操作测试
   - 兼容性验收

## 先判断用户在做什么

1. Client
   - 编辑器、IDE、CLI、Web UI、桌面前端
   - 重点看协议入口、权限交互、文件/终端能力、会话管理
2. Agent
   - 本地子进程、远端代理、编码代理、包装现有 agent 的 ACP facade（门面）
   - 重点看 initialize、session lifecycle（会话生命周期）、prompt turn（提示轮次）、tool/status 更新、取消、模式
3. Adapter / Proxy
   - ACP 到自有协议、ACP 到既有 agent API、ACP 到编辑器宿主
   - 重点看语义映射、兼容边界、错误翻译、历史回放
4. Product integration
   - 把 ACP 嵌进产品工作流
   - 重点看 trust boundary（信任边界）、权限 UX、MCP 配置注入、会话恢复、观测与回归

## 使用导航

- 想看只包含权威入口的资料表：
  - 读 [references/official-sources.md](./references/official-sources.md)
- 想确认协议核心语义、基线能力、消息顺序：
  - 读 [references/protocol-core.md](./references/protocol-core.md)
- 想做客户端、代理端、桥接层实现方案：
  - 读 [references/implementation-playbook.md](./references/implementation-playbook.md)
- 想做兼容性评审、排错、测试与发布验收：
  - 读 [references/interoperability-review.md](./references/interoperability-review.md)
- 想在 Rust 里直接落地 ACP：
  - 读 [references/rust-guide.md](./references/rust-guide.md)

## 默认工作流

### 1. 先给结论

一句话回答当前问题，例如：

- 这是协议要求
- 这是某个 vendor 扩展
- 这是客户端职责，不该放到 agent
- 这是会话状态机设计问题，不只是 JSON 编解码问题

### 2. 再给协议定位

至少回答这些问题：

- 当前动作属于 initialization（初始化）、session setup（会话建立）还是 prompt turn（提示轮次）
- 发起方是谁，接收方是谁
- 这是 request-response（请求响应）还是 notification（通知）
- 依赖哪些 capability 或前置协商
- 失败时应该返回协议错误、语义 stop reason，还是用户可见提示

### 3. 再给工程落地

默认补齐：

- 模块划分
- 状态机
- 消息结构
- 错误与超时
- 取消路径
- 日志/指标
- 测试矩阵

### 4. 有时效性就回官方核对

这些问题默认要回官方页面核对：

- 当前 protocol version（协议版本）或 page layout（页面结构）
- 某个 session/mode/config feature（特性）是否已稳定
- 官方库、示例、已知集成、兼容矩阵
- 某家厂商是否支持 ACP，以及支持的是哪种 transport

## 强制检查清单

回答 ACP 实现问题时，默认检查：

- 是否在创建 session 前完成了 `initialize`
- 是否把未声明的 capability 当成“不支持”
- 是否把 `session/update` 和 `session/prompt` 最终响应区分开
- 是否把取消当成错误而不是语义化 `cancelled`
- 是否把 vendor 行为误说成 ACP 核心规范
- 是否在权限请求、文件系统、终端、MCP 转发处说清 trust boundary
- 是否为 session persistence / `session/load` 写了验收路径
- 是否为绝对路径、1-based line number（从 1 开始的行号）、content capability、mode/config 兼容写了测试

## 常见误区

- 不要把 GitHub Copilot CLI 的 `TCP` 支持说成 ACP 核心传输规范本身。
- 不要把示例 SDK 的调用方式说成协议必选行为。
- 不要忽略 `session/cancel` 的取消安全语义。
- 不要在没有 capability 的前提下调用对应方法。
- 不要把旧会话恢复、历史回放、会话继续这几件事混为一谈。
- 不要只测 happy path（理想路径）；必须测拒绝授权、取消、断开重连、版本不匹配、缺能力、部分更新、工具失败。

## 推荐输出结构

除非用户指定别的格式，优先按这个顺序回答：

1. 结论
2. 协议归属
3. 状态机或消息流
4. 代码或模块方案
5. 风险点
6. 测试与验收
7. 若需要，再给官方链接
