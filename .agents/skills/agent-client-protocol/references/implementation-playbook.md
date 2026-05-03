# 实现落地手册

这份文件是基于官方 ACP 资料的工程化落地建议，不等同于规范原文。它的目标是帮助开发工程师把 ACP 做成“可维护、可测试、可互操作”的软件。

## 1. 先确定你在实现哪一侧

### A. ACP Client

典型形态：

- 编辑器或 IDE 插件
- CLI 前端
- 浏览器或桌面端开发界面

核心职责：

- 建立 transport
- 发起 `initialize`
- 建立和切换 session
- 把用户输入转成 `session/prompt`
- 接收并渲染 `session/update`
- 处理 `session/request_permission`
- 暴露文件系统、终端或其他宿主能力

### B. ACP Agent

典型形态：

- 本地子进程 agent
- 远端代理包装层
- 既有 coding agent 的 ACP facade

核心职责：

- 正确实现初始化和 capability 协商
- 管理会话状态
- 把模型输出映射为 ACP 更新流
- 管理工具调用、权限前置和停止原因
- 保证取消和恢复语义正确

### C. ACP Adapter / Proxy

典型形态：

- ACP 到已有内部协议
- ACP 到 SaaS agent API
- ACP 到本地既有 agent CLI

核心职责：

- 双向语义翻译
- 错误和 stop reason 映射
- 会话标识和工具调用标识映射
- 历史回放和加载语义对齐

## 2. 推荐架构切分

无论做哪一侧，都建议把实现切成四层：

1. Transport layer
   - 只处理 JSON-RPC 编解码、连接读写、消息收发
2. Protocol layer
   - 只处理 ACP 方法、通知、schema 和 capability 判定
3. Session engine
   - 维护会话状态机、权限等待、工具调用、取消传播
4. Product adapter
   - 连接模型、宿主 UI、文件系统、终端、MCP 配置、持久化

这样可以避免把“读写 socket”“协议语义”“产品逻辑”揉成一个大对象。

## 3. 状态机建议

### Connection state

- disconnected
- connecting
- initialized
- closed

### Session state

- new
- active
- awaiting_permission
- running_tool
- cancelling
- completed
- failed

### Turn state

- idle
- prompting
- streaming_updates
- waiting_permission
- running_tool
- stopping
- finished

工程建议：

- 连接状态、会话状态、轮次状态分开建模
- 不要用几个布尔值硬拼状态
- 会话与轮次的取消要分开处理

## 4. 权限与信任边界

ACP 的一个重点不是“能不能调工具”，而是“谁有权决定调工具”。

实现时要先回答：

- 哪些工具操作必须请求用户授权
- 哪些权限可以自动接受
- 哪些权限决策可以按 session 记忆
- 用户取消时，是否会终止当前轮所有未完成工具

工程建议：

- 把 permission request（权限请求）做成独立组件
- 每条权限记录至少保留：
  - `session_id`
  - `tool_call_id`
  - `tool_kind`
  - `title`
  - `options`
  - `decision`
  - `decision_time`

## 5. MCP 集成建议

ACP 常常和 MCP 一起出现，但两者职责不同。

推荐做法：

- 在 session 建立时显式传入 MCP server 配置
- 把 MCP 连接失败视为独立错误面，而不是和 ACP 握手错误混在一起
- 宿主如果需要“把自己的能力再暴露给 agent”，优先通过 MCP server 或小型代理完成

避免做法：

- 在一个对象里同时硬编码 ACP 和 MCP 所有状态
- 把 MCP server 生命周期和 ACP connection 生命周期完全耦死

## 6. 取消与超时

ACP 取消语义比“断开连接”更细。

你需要明确：

- 用户取消当前轮时，是否保留 session
- 取消是否中断模型请求
- 取消是否中断工具执行
- 取消后的补发 `session/update` 允许到什么时点
- 超时是转成协议错误，还是语义化 `cancelled`

工程建议：

- 为每个 prompt turn 分配独立 cancellation token（取消令牌）
- transport 断连和 turn cancel 不要共用一个错误枚举值
- 对工具执行层提供 cooperative cancellation（协作式取消）

## 7. 观测点建议

最少记录这些字段：

- `connection_id`
- `session_id`
- `turn_id`
- `tool_call_id`
- `request_id`
- `protocol_version`
- `client_name`
- `agent_name`
- `mode_id`
- `stop_reason`
- `permission_outcome`
- `latency_ms`

建议日志分级：

- `info`
  - 初始化完成
  - 会话创建/加载
  - 权限最终决策
  - 一轮完成及停止原因
- `warn`
  - capability 不匹配
  - 会话恢复失败但可降级
  - 工具调用取消或回退
- `error`
  - 协议解码失败
  - 必填字段缺失
  - 未处理的内部状态错误
- `debug` / `trace`
  - 完整消息流
  - 增量更新序列
  - 权限请求上下文

## 8. 测试矩阵

### 协议面

- `initialize` 成功
- 版本不匹配
- capability 缺失
- 未声明能力时的拒绝路径
- `session/new` 成功
- `session/load` 在未声明 `loadSession` 时被禁止

### 会话面

- 多 session 并存
- 同一连接下多个 prompt turn 顺序执行
- 历史回放顺序正确
- session 取消不误伤其他 session

### 交互面

- 权限允许
- 权限拒绝
- 权限请求期间取消
- 工具运行期间取消
- `session/update` 与最终 `session/prompt` 响应顺序正确

### 兼容面

- 未识别 `_meta`
- 忽略未知扩展字段
- 旧 client 对新 capability 的降级
- 桥接现有 agent 时 stop reason 映射正确

## 9. 发布前验收

发布或对外宣称“ACP compatible（兼容 ACP）”前，至少核对：

- 官方基线路径是否全通
- 负向测试是否覆盖
- 权限 UI 是否能解释当前动作
- `session/cancel` 是否总能收口为 `cancelled`
- 日志里是否能串起一个完整 turn
- 文档里是否明确哪些是 ACP 核心能力，哪些是 vendor 扩展
