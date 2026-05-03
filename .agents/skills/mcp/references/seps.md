# MCP SEPs

## 这层回答什么

SEPs（Specification Enhancement Proposals，规范增强提案）回答的是：

- MCP 的重大变化如何被提出
- 提案如何进入审查、接受、落地
- 某项能力当前是提案、已接受标准，还是已完成扩展

## 关键事实

- SEP 是 MCP 讨论重大规范变化的主要机制。
- 每份 SEP 都会给出：
  - 技术提案
  - 设计动机
  - 取舍理由
  - 状态

## 常见用途

- 提议新的核心协议能力
- 提议新的扩展
- 调整流程或治理机制
- 给重大设计决策留下正式记录

## 阅读 SEP 时先看什么

1. SEP 编号
2. 标题
3. 状态
4. 类型
5. 创建时间

状态和类型决定它是“历史资料”“活跃提案”还是“已成标准”。

## 扩展与 SEP 的关系

- 官方扩展通常通过 SEP 路径推进
- SEP-2133 专门定义了 Extensions 相关框架
- 因此“扩展层”与“SEP 层”密切相连，但不是一回事：
  - Extensions 是能力分类
  - SEP 是演进机制

## 回答时的边界

- 不要把 SEP 说成“已经是协议正文”
- SEP 可能是已最终接受，也可能仍在过程中
- 如果用户问当前权威要求，最终还是要回到 Specification 或最终接受的扩展文档

## 官方入口

- SEPs index:
  - https://modelcontextprotocol.io/seps/index
- Community view of SEPs:
  - https://modelcontextprotocol.io/community/seps
- SEP-2133 Extensions:
  - https://modelcontextprotocol.io/seps/2133-extensions

## 回答模板

- 如果用户问“怎么推动一个想法进入 MCP”：
  - 回答“通常要经过 SEP 流程；如果是扩展，还要结合扩展轨道与参考实现要求。”
