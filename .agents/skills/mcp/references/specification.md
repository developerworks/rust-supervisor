# MCP Specification

## 这层回答什么

Specification（规范层）回答的是：

- 协议真正要求了什么
- 哪些行为是 MUST / SHOULD / MAY
- 初始化、协商、消息结构、生命周期的权威定义是什么

这是 MCP 的 authoritative source（权威来源）。

## 关键事实

- MCP 规范是协议要求的正式定义。
- 它基于 JSON-RPC 2.0。
- 它强调：
  - stateful connections（有状态连接）
  - capability negotiation（能力协商）
  - client / server 交互语义
- 规范文本使用 RFC 2119 / RFC 8174 风格词汇：
  - MUST
  - SHOULD
  - MAY

## 回答时的基本立场

- 只要用户问“协议到底要求什么”，优先回规范。
- 只要用户问“某 SDK 这样做是不是 MCP 要求”，要把 SDK 行为和规范要求分开。
- 只要用户问互操作性问题，也优先回规范。

## 规范关注的典型内容

- 参与者模型
- 初始化握手
- 能力声明
- 工具 / 资源 / 提示的协议行为
- 通知与请求响应模式
- 客户端和服务端各自职责
- 扩展如何协商

## 规范与文档的区别

- Documentation：
  - 更像“实现导览”和“学习材料”
- Specification：
  - 更像“协议合同”

如果二者表述粒度不同，以规范层优先。

## 规范与扩展的区别

- Core specification（核心规范）：
  - 定义所有合规实现共享的基础协议
- Extensions（扩展）：
  - 为可选能力建立标准化外延
  - 不是默认强制实现

## 官方入口

- Specification:
  - https://modelcontextprotocol.io/specification/2024-11-05

## 回答模板

- 如果用户问“这是不是 MCP 标准的一部分”：
  - 先回答“属于 core spec / extension / 不是规范层”三选一。
- 如果用户问“实现必须支持吗”：
  - 优先看是否是 core requirement（核心要求），还是 optional extension（可选扩展）。
