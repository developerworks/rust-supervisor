# Rust 落地指南

这份文件面向 Rust 工程师，结合 ACP 官方 Rust 库入口给出实现建议。

## 1. 官方 Rust 入口

ACP 官方站点提供 Rust 库入口：

- https://agentclientprotocol.com/libraries/rust

官方说明的关键点：

- `agent-client-protocol` crate 同时提供 Agent 和 Client 两侧实现入口
- 你需要根据自己在做的是哪一侧，实现 `Agent` trait 或 `Client` trait
- 官方示例二进制可以作为最小起点

## 2. 推荐模块划分

如果你在 Rust 项目里做 ACP，优先按职责拆分：

- `transport/`
  - JSON-RPC 编解码、流读写、连接生命周期
- `protocol/`
  - ACP schema、方法路由、capability 判定
- `session/`
  - session state、turn state、取消、权限等待
- `permissions/`
  - 权限策略与用户决策映射
- `mcp_bridge/`
  - MCP 配置注入、代理、自托管桥接
- `runtime/`
  - tokio task 编排、shutdown、重连
- `observability/`
  - tracing、metrics、debug dump

## 3. 类型设计建议

优先把这些对象做成明确类型：

- `ProtocolVersion`
- `ClientCapabilities`
- `AgentCapabilities`
- `SessionId`
- `TurnId`
- `ToolCallId`
- `StopReason`
- `PermissionOutcome`
- `SessionState`
- `TurnState`

这样可以把很多不变式交给类型系统，而不是靠注释约定。

## 4. 错误处理建议

推荐分层：

- `TransportError`
- `ProtocolError`
- `CapabilityError`
- `SessionError`
- `PermissionError`
- `AdapterError`

应用层可以用 `anyhow` 补上下文，但协议层和核心状态机层最好保留结构化错误。

## 5. 异步并发建议

ACP 很容易踩到并发坑，尤其是：

- 一个连接多个 session
- 一个 session 多个并发更新
- 取消与工具执行同时发生
- 权限请求阻塞 turn 继续推进

建议：

- 用 message passing（消息传递）管理 turn 生命周期，避免到处共享可变状态
- 不要跨 `await` 持锁
- 用有界通道承接流式更新，防止前端慢消费者拖垮整个 session
- 为每个 turn 建独立取消令牌

## 6. `tracing` 字段建议

Rust 实现里，建议在核心链路统一带上：

- `session_id`
- `turn_id`
- `tool_call_id`
- `request_id`
- `protocol_version`
- `mode_id`
- `stop_reason`

常见埋点：

- `initialize` 收发
- capability 判定
- `session/new` / `session/load`
- `session/prompt`
- `session/update`
- `session/request_permission`
- `session/cancel`

## 7. 测试建议

### 单元测试

- 版本协商
- capability 缺失时的分支
- stop reason 映射
- 取消传播
- 权限决策映射

### 集成测试

- 启动一个最小 Agent，验证 Client 端握手
- 启动一个最小 Client，验证 Agent 端完整 turn
- `session/load` 重放历史
- 取消时最终返回 `cancelled`

### 回归测试

- 未识别 `_meta` 字段
- vendor 扩展字段透传
- 无 `terminal` 能力时的降级
- 工具调用中途取消

## 8. 文档建议

Rust 项目里，公共 ACP 接口优先补 `rustdoc`：

- 模块职责
- 状态机含义
- 取消语义
- 权限边界
- vendor 扩展边界

如果你在做库而不是应用，务必在文档里明确：

- 哪些是 ACP 核心语义
- 哪些是实现细节
- 哪些扩展字段会被透传或忽略
