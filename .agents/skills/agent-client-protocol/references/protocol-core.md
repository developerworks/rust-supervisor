# 协议核心语义

这份文件总结 ACP 的核心协议面。除特别说明外，这里优先描述官方文档中的规范语义；如果某段是工程归纳，会明确标记为“实现建议”。

## 1. 通信模型

- ACP 采用 JSON-RPC 2.0。
- 协议里同时存在 request-response（请求响应）和 notification（通知）。
- 双方都可能发起请求，不是单向 API。
- `session/update` 是通知，不应该等待结果。
- `session/prompt` 是请求，最终必须有响应。

## 2. 角色分工

### Agent

Agent 是使用生成式 AI 自主修改代码或执行开发任务的一侧。

基线要求：

- `initialize`
- `session/new`
- `session/prompt`
- `session/cancel`
- `session/update`

可选能力：

- `authenticate`
- `session/load`
- `session/list`
- `session/set_mode`
- 其他通过 capability 或扩展声明的能力

### Client

Client 是用户和 agent 之间的界面层，常见形态包括代码编辑器、IDE、CLI 前端或其他交互 UI。

常见职责：

- 发起 `initialize`
- 发起 `session/new` / `session/load`
- 发起 `session/prompt`
- 接收 `session/update`
- 处理 `session/request_permission`
- 暴露 `fs/*`、`terminal/*` 等能力给 agent

## 3. 初始化阶段

在任何 session 建立之前，Client 必须先发 `initialize`。

初始化阶段要解决四件事：

1. 协议版本协商
2. capability 协商
3. 认证方式发现
4. 双方实现信息交换

关键规则：

- Client 必须发送自己支持的最新协议版本。
- Agent 如果支持该版本，应返回同一版本；否则返回自己支持的最新版本。
- 如果 Client 不支持 Agent 返回的版本，应断开连接并提示用户。
- 省略的 capability 必须按“不支持”处理，不能乐观假设存在。

## 4. Capability 语义

能力协商是 ACP 的关键边界。

### Client 侧常见能力

- `fs.readTextFile`
- `fs.writeTextFile`
- `terminal`

### Agent 侧常见能力

- `loadSession`
- `promptCapabilities`
  - `image`
  - `audio`
  - `embeddedContext`
- `mcpCapabilities`
  - `http`
  - `sse`
- `sessionCapabilities`

基线要求：

- 所有 agent 都必须支持 `session/new`、`session/prompt`、`session/cancel`、`session/update`
- 所有 agent 都必须支持文本和资源链接这两类基础 prompt 内容

## 5. Session Setup

### `session/new`

创建新会话时，Client 需要提供：

- `cwd`
- `mcpServers`

Agent 成功后必须返回唯一 `sessionId`。

### `session/load`

只有当 `loadSession` 能力为真时，Client 才能调用 `session/load`。

加载已存在会话时，Agent 应：

- 恢复上下文和对话历史
- 连接指定 MCP server
- 通过 `session/update` 重新流出会话历史

实现建议：

- 把 connection state（连接态）和 session state（会话态）拆开，避免同一物理连接承载多个会话时相互污染。

## 6. Prompt Turn

一个 prompt turn 是从 `session/prompt` 开始，到 Agent 完成该轮响应为止的完整交互周期。

典型流程：

1. Client 发送 `session/prompt`
2. Agent 通过 `session/update` 持续报告文本、计划、工具调用和状态更新
3. 如果需要，Agent 发起 `session/request_permission`
4. 完成后，Agent 对原始 `session/prompt` 返回结果，包含 `stopReason`

重点区分：

- `session/update` 是流式过程反馈
- `session/prompt` 的结果是该轮结束信号

## 7. Stop Reason

常见 stop reason：

- `end_turn`
- `max_tokens`
- `max_turn_requests`
- `refusal`
- `cancelled`

实现建议：

- 把 stop reason 当作会话语义的一部分，不要在内部只保留一个“结束字符串”。
- 对外暴露时保持枚举化，避免 magic string（魔法字符串）散落。

## 8. 取消语义

Client 可以通过 `session/cancel` 取消一轮正在进行的 prompt turn。

取消时的关键点：

- Client 应尽快把当前轮未结束的工具调用标记为取消
- Client 必须对所有待处理的 `session/request_permission` 返回 `cancelled`
- Agent 应尽快停止模型请求和工具调用
- Agent 最终必须对原始 `session/prompt` 返回 `cancelled`

实现建议：

- 不要把取消传播成普通异常并直接回传给上层 UI
- 要在 transport 层、会话层、工具执行层之间定义明确的取消传播规则

## 9. 权限请求

当工具调用需要用户授权时，Agent 可以发起 `session/request_permission`。

Client 的职责：

- 向用户展示上下文、风险和可选项
- 根据用户选择返回结果
- 在会话取消时返回 `cancelled`

实现建议：

- 权限请求对象应该带会话标识、工具调用标识、动作摘要、默认项和审计上下文
- 权限系统最好做成独立协调器，而不是散落在 UI 代码里

## 10. 文件与终端能力

文件系统和终端能力属于 Client 向 Agent 提供的宿主能力。

关键边界：

- Agent 不应假设文件和终端永远可用
- 能力是否可用，要以 initialization 协商结果为准
- 协议要求文件路径使用绝对路径
- 行号使用 1-based 表示

## 11. MCP 注入与桥接

ACP 常常和 MCP 一起出现，但两者职责不同。

推荐理解：

- ACP 负责 Client 与 Agent 之间的交互
- MCP 负责 Agent 与外部工具/资源服务器之间的能力接入

关键点：

- 如果宿主自己也想暴露 MCP 能力，可以通过 MCP server 配置或代理桥接接入
- 不要把 ACP 和 MCP 复用到同一个 socket 上然后指望对方自动区分

## 12. 扩展机制

ACP 支持扩展，但扩展必须以兼容为前提。

官方文档强调的做法：

- 用 `_meta` 挂附加数据
- 自定义方法名以下划线前缀开头
- 在 initialization 时声明自定义 capability

实现建议：

- 扩展字段要和核心字段解耦
- 未识别扩展要能安全忽略
- 任何扩展都不应破坏基线协商和核心消息流
