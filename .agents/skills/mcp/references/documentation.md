# MCP Documentation

## 这层回答什么

Documentation（文档层）回答的是：

- MCP 是什么
- 应该从哪条路径开始学
- 如何连接已有 server
- 如何实现 server / client / app / skill
- 官方概念应该怎么理解

它主要承担教学、引导和实践落地职责，不等于权威规范。

## 文档站的典型作用

- 帮开发者快速入门
- 提供面向角色的路径导航
- 解释概念与术语
- 给出实现建议、教程和官方入口

## 默认阅读路线

### 1. 完全不了解 MCP

- 从 Intro 开始
- 再看 Architecture
- 再根据目标分流：
  - Use MCP
  - Build Servers
  - Build Clients
  - Build Skills

### 2. 已经知道概念，开始实现

- 先看对应实现路径文档
- 再回到 Specification 确认规范要求

### 3. 已经开始设计新能力

- 文档层只够做背景理解
- 接下来要切去：
  - Extensions
  - SEPs
  - Community

## 文档层与规范层的边界

- 文档层：
  - 强调“怎么理解”和“怎么做”
  - 可读性优先
- 规范层：
  - 强调“必须 / 应该 / 可以”
  - 一致性和可互操作性优先

回答时要避免把教程中的示例写法误说成协议强制要求。

## 官方重点入口

- Intro:
  - https://modelcontextprotocol.io/docs/getting-started/intro
- Architecture:
  - https://modelcontextprotocol.io/docs/learn/architecture
- SDKs:
  - https://modelcontextprotocol.io/docs/sdk
- Server concepts:
  - https://modelcontextprotocol.io/docs/learn/server-concepts

## 回答模板

- 如果用户问“我应该看哪一页”：
  - 先按目标分类，再给路径，而不是只甩首页。
- 如果用户问“官方文档和规范区别是什么”：
  - 直接回答“文档负责教你，规范负责约束你”。
