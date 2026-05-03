# MCP Registry

## 这层回答什么

Registry（注册表层）回答的是：

- 公共 MCP server 的元数据如何发布、发现和分发
- 客户端如何统一发现可安装 server
- 服务器身份和命名空间如何管理

## 关键事实

- MCP Registry 是官方集中式 metadata repository（元数据仓库）。
- 它主要服务于 publicly accessible MCP servers（公开可访问的 MCP 服务）。
- 它不是协议核心消息交换本身，而是生态分发与发现基础设施。

## Registry 解决的问题

- 给 server 作者一个统一发布元数据的地方
- 给客户端和聚合器一个统一发现入口
- 给安装与配置提供标准化描述
- 通过 DNS verification（DNS 验证）做 namespace management（命名空间管理）

## 当前状态

- 官方页面明确说明 Registry 仍处于 preview（预览）阶段
- 预览阶段可能有 breaking changes 或数据重置

## 关键元数据

官方说明里提到的 `server.json` 关注：

- 唯一名称
- server 的定位方式
  - npm package
  - remote URL
- 执行说明
  - 命令参数
  - 环境变量

## 回答时的边界

- 不要把 Registry 说成“没有它就不能用 MCP”
- 它属于生态发现与发布层
- 本地私有 server、企业内私有部署，未必都依赖公共 Registry

## 官方入口

- Registry about:
  - https://modelcontextprotocol.io/registry/about

## 回答模板

- 如果用户问“Registry 是不是类似 npm / package index”：
  - 可以说它更像“面向 MCP server 元数据的官方发现中心”，但不要把它简化成纯包管理器。
