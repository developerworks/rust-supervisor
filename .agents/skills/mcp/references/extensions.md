# MCP Extensions

## 这层回答什么

Extensions（扩展层）回答的是：

- 哪些能力不属于核心规范，但被正式定义为可选能力
- 扩展如何命名、协商、演进
- 如何把新能力从想法变成官方扩展

## 关键事实

- MCP extensions 是 core spec 之外的 optional additions（可选附加能力）。
- 扩展适合：
  - modular features（模块化能力）
  - specialized features（专门场景能力）
  - experimental ideas（实验性能力）
- 扩展标识符格式是：
  - `{vendor-prefix}/{extension-name}`
- 官方扩展使用：
  - `io.modelcontextprotocol/...`

## 官方扩展仓库规则

- 官方扩展仓库位于 MCP GitHub 组织下
- 仓库前缀通常为：
  - `ext-`
- 当前典型例子：
  - `ext-auth`
  - `ext-apps`

## 协商方式

- 客户端与服务端在初始化能力中通过 `extensions` 字段声明支持情况
- 扩展默认不应被假定为总是可用
- 扩展一般需要显式 opt-in（显式启用）

## 演进规则

- 扩展独立于 core spec 演进
- 向后兼容很重要
- 推荐：
  - 用 capability flags（能力标志）
  - 用 settings 内部版本字段
- 如果必须 breaking change（破坏性变更），更安全的做法是换新 identifier（新标识）

## 创建官方扩展的常见路径

1. 提出 SEP
2. 至少做一个官方 SDK 的 reference implementation（参考实现）
3. 经维护者审查
4. 发布到官方扩展仓库
5. 被其他 client / server / SDK 采纳

## 扩展与核心规范的边界

- 如果某能力是所有 MCP 实现都必须共享的基础行为，更像 core spec 问题
- 如果某能力是可选的、分场景的、更易独立演进的，更像 extension 问题

## 官方入口

- Extensions overview:
  - https://modelcontextprotocol.io/docs/extensions/overview
- Apps extension:
  - https://modelcontextprotocol.io/docs/extensions/apps
- Auth extensions:
  - https://modelcontextprotocol.io/extensions/auth/overview
- SEP-2133 Extensions:
  - https://modelcontextprotocol.io/seps/2133-extensions

## 回答模板

- 如果用户问“这该不该进 core”：
  - 先看它是不是所有实现都需要的基础协议行为。
- 如果用户问“怎么做自定义扩展”：
  - 先提醒命名规则和 vendor prefix（厂商前缀），再解释官方扩展和第三方扩展的差异。
