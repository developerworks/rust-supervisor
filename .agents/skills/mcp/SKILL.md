---
name: mcp
description: >-
  Explain the Model Context Protocol (MCP) from an authoritative, standards-aware
  perspective, including documentation structure, core architecture, specification,
  extensions, registry, SEPs, governance, working groups, and community processes.
---

# MCP

用来解释 Model Context Protocol（模型上下文协议，MCP）的全局知识面，而不是只回答“它是什么”。

这份 skill 的目标视角是：

- 像长期维护 MCP 的核心参与者一样回答
- 明确区分文档、规范、扩展、注册表、提案流程、社区治理
- 当用户问“现在怎么做”时，能把问题准确导向对应官方层次

## 何时使用

- 用户问 MCP 是什么、为什么存在、解决什么问题
- 用户问 MCP 的 host / client / server（宿主 / 客户端 / 服务端）职责边界
- 用户问 `stdio`、`Streamable HTTP`、JSON-RPC（远程过程调用协议 JSON-RPC）之间的关系
- 用户问 tools / resources / prompts（工具 / 资源 / 提示模板）分别属于什么层次
- 用户问 Documentation、Specification、Extensions、Registry、SEPs、Community 分别是什么
- 用户问“我该看文档站、规范、还是 SEP”
- 用户问如何提交 MCP 提案、如何参与工作组、如何理解官方治理
- 用户问某个能力属于 core spec（核心规范）、extension（扩展）还是 ecosystem（生态层）
- 用户问如何从“使用 MCP”升级到“实现 / 设计 / 治理 MCP”

## 回答原则

1. 先判断用户问题属于哪一层：
   - Documentation（文档 / 教学与落地）
   - Specification（规范 / 权威要求）
   - Extensions（扩展 / 可选能力）
   - Registry（注册表 / 元数据发现）
   - SEPs（规范增强提案）
   - Community（社区 / 治理 / 工作组）
2. 先给结论，再解释该层在 MCP 体系中的角色。
3. 明确说清“这是不是规范要求”，不要把教程、参考实现、扩展仓库混说成强制标准。
4. 用户问到实现或治理细节时，优先读取对应 `references/*.md`，不要只靠入门类比硬答。
5. 涉及时效性或当前支持矩阵时，优先回到官方站点核对。

## MCP 六层知识图

把 MCP 理解成六个相关但不同的层次：

1. Documentation
   - 回答“怎么理解、怎么开始、怎么接入、怎么实现”
   - 这是学习路径和实践指导层
2. Specification
   - 回答“协议真正要求了什么”
   - 这是权威规范层
3. Extensions
   - 回答“哪些能力不是核心规范，但被标准化为可选能力”
   - 这是扩展演化层
4. Registry
   - 回答“公开 MCP server 元数据如何被发现、发布、安装”
   - 这是生态分发层
5. SEPs
   - 回答“规范和扩展是如何被提出、讨论、接受的”
   - 这是协议演进层
6. Community
   - 回答“谁在治理、谁能提案、工作组怎么运作、如何参与”
   - 这是治理协作层

## 默认解释框架

### 用户只是问“什么是 MCP”

- 先给一句话：
  - MCP 是 AI 应用连接外部上下文和能力的开放标准协议。
- 再给类比：
  - 它像“AI 世界的 USB-C 接口”。
- 再补关键限制：
  - MCP 只标准化“上下文和能力如何交换”，不规定模型怎么推理，也不规定产品怎么做记忆或代理编排。

### 用户问架构

- 优先解释：
  - Host 是承载 AI 体验的应用
  - Client 是 host 内部与单个 server 建连的组件
  - Server 是暴露上下文与能力的程序
- 再解释：
  - data layer（数据层）定义协议语义
  - transport layer（传输层）定义消息如何传输

### 用户问“官方怎么定义”

- 优先去 `references/specification.md`
- 明确区分：
  - 规范性要求
  - 文档性解释
  - SDK 封装细节

### 用户问“怎么新增一个能力”

- 先判断属于：
  - core spec change（核心规范修改）
  - extension（扩展）
  - registry metadata（注册表元数据）
  - ecosystem-only convention（纯生态约定）
- 再转到：
  - `references/extensions.md`
  - `references/seps.md`
  - `references/community.md`

## 使用导航

- 想知道官方文档站怎么组织、该从哪条路线入门：
  - 读 [references/documentation.md](./references/documentation.md)
- 想知道什么才是规范、规范和教程差在哪：
  - 读 [references/specification.md](./references/specification.md)
- 想知道扩展是什么、怎么创建、怎么协商：
  - 读 [references/extensions.md](./references/extensions.md)
- 想知道 MCP Registry 是什么、解决什么问题：
  - 读 [references/registry.md](./references/registry.md)
- 想知道 SEP 是什么、怎么提、状态怎么看：
  - 读 [references/seps.md](./references/seps.md)
- 想知道治理、工作组、兴趣组、参与路径：
  - 读 [references/community.md](./references/community.md)

## 常见判定规则

### 什么时候回答“这属于规范”

当问题涉及：

- 初始化握手
- JSON-RPC 消息
- capabilities（能力协商）
- 生命周期
- 工具 / 资源 / 提示的协议行为
- 客户端 / 服务端必须或应该做什么

优先按 Specification 回答。

### 什么时候回答“这属于扩展”

当问题涉及：

- 可选能力
- 某些客户端才支持的额外特性
- Apps、额外认证机制、实验能力
- `extensions` 字段协商

优先按 Extensions 回答。

### 什么时候回答“这属于治理或提案”

当问题涉及：

- 谁能决定
- 如何进入标准
- 工作组是否需要
- 怎样从想法变成正式能力

优先按 SEPs + Community 回答。

## 关键认知约束

- 不要把 Documentation 说成 Specification。
- 不要把 SDK 行为自动当成规范要求。
- 不要把 Extensions 说成“所有客户端都必须支持”。
- 不要把 Registry 说成“协议核心必需组件”。
- 不要把 SEPs 说成“实现文档”；它们是提案与演进机制。
- 不要把 Community 只理解成聊天群；它包含正式治理、维护者层级、工作组和兴趣组。

## 推荐回答风格

- 结论要短，结构要清晰。
- 对外部链接优先给官方站点。
- 用户问“当前状态”时，提醒自己回官方页面核对。
- 只要进入治理、扩展、注册表、SEP 细节，就不要停留在 intro 级解释。

## 官方入口

- Intro: https://modelcontextprotocol.io/docs/getting-started/intro
- Architecture: https://modelcontextprotocol.io/docs/learn/architecture
- Specification: https://modelcontextprotocol.io/specification/2024-11-05
- Extensions overview: https://modelcontextprotocol.io/docs/extensions/overview
- Registry: https://modelcontextprotocol.io/registry/about
- SEPs index: https://modelcontextprotocol.io/seps/index
- Governance: https://modelcontextprotocol.io/community/governance
- Working and Interest Groups: https://modelcontextprotocol.io/community/working-interest-groups
