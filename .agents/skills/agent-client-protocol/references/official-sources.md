# 官方资料入口

这份文件只列权威来源，优先级从上到下递减。

## 一、ACP 官方站点

这些页面应被视为协议与官方实践入口：

- Introduction
  - https://agentclientprotocol.com/get-started/introduction
  - 适合回答 ACP 是什么、解决什么问题、本地/远端大方向
- Architecture
  - https://agentclientprotocol.com/get-started/architecture
  - 适合回答 client、agent、MCP 注入、实时通知、信任边界
- Protocol Overview
  - https://agentclientprotocol.com/protocol/overview
  - 适合回答消息模型、典型调用顺序、双向调用职责
- Initialization
  - https://agentclientprotocol.com/protocol/initialization
  - 适合回答版本协商、capability、authMethods、clientInfo、agentInfo
- Session Setup
  - https://agentclientprotocol.com/protocol/session-setup
  - 适合回答 `session/new`、`session/load`、历史回放、MCP server 参数
- Prompt Turn
  - https://agentclientprotocol.com/protocol/prompt-turn
  - 适合回答 `session/prompt`、`session/update`、取消、stop reason
- Tool Calls
  - https://agentclientprotocol.com/protocol/tool-calls
  - 适合回答工具调用、状态更新、权限请求
- Content
  - https://agentclientprotocol.com/protocol/content
  - 适合回答 prompt/content block 结构
- File System
  - https://agentclientprotocol.com/protocol/file-system
  - 适合回答客户端文件能力
- Terminals
  - https://agentclientprotocol.com/protocol/terminals
  - 适合回答终端相关能力
- Agent Plan
  - https://agentclientprotocol.com/protocol/agent-plan
  - 适合回答计划更新与可视化
- Session Modes
  - https://agentclientprotocol.com/protocol/session-modes
  - 适合回答 mode、模式切换、规划态与执行态
- Session Config Options
  - https://agentclientprotocol.com/protocol/session-config-options
  - 适合回答可配置会话选项
- Slash Commands
  - https://agentclientprotocol.com/protocol/slash-commands
  - 适合回答命令入口
- Extensibility
  - https://agentclientprotocol.com/protocol/extensibility
  - 适合回答 `_meta`、自定义方法、扩展协商
- Transports
  - https://agentclientprotocol.com/protocol/transports
  - 适合回答协议传输层
- Schema
  - https://agentclientprotocol.com/protocol/schema
  - 适合回答最终字段形状、可选项、默认值、能力对象

## 二、ACP 官方库与语言入口

- Rust library
  - https://agentclientprotocol.com/libraries/rust
  - 适合 Rust 项目落地与 trait 边界判断
- TypeScript library
  - https://agentclientprotocol.com/libraries/typescript
- Python library
  - https://agentclientprotocol.com/libraries/python
- Java library
  - https://agentclientprotocol.com/libraries/java
- Kotlin library
  - https://agentclientprotocol.com/libraries/kotlin

## 三、官方生态与兼容入口

- Agents
  - https://agentclientprotocol.com/get-started/agents
  - 适合回答有哪些 agent 已支持 ACP
- Clients
  - https://agentclientprotocol.com/overview/clients
  - 适合回答有哪些 client / IDE / 前端在用 ACP

## 四、官方演进资料

- RFDs index
  - https://agentclientprotocol.com/rfds
  - 适合回答尚在演进中的功能、未来方向、兼容窗口

## 五、官方厂商实现资料

- GitHub Copilot CLI ACP server
  - https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server
  - 适合回答 GitHub 官方如何暴露 ACP server，以及其 `stdio` / `TCP` 支持是产品特性还是协议层要求

## 使用约束

- 先看 ACP 官方站点，再看厂商文档。
- 厂商文档可以说明“某个产品怎么做”，不能直接覆盖 ACP 官方协议语义。
- 需要字段级细节时，优先回 `Schema` 页面，而不是只看 prose（说明性文字）页面。
- 需要时效性判断时，不要只靠本地记忆，回上面这些官方页面核对。
