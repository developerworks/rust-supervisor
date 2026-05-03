# 互操作评审与排错清单

这份文件适合在 review（评审）、调试、验收或线上排障时使用。

## 1. 第一轮判定

先问五个问题：

1. 失败发生在 initialization、session setup 还是 prompt turn
2. 是 protocol mismatch（协议不匹配）、schema mismatch（字段不匹配）、state mismatch（状态不匹配）还是 product bug（产品 bug）
3. 双方是否真的协商过对应 capability
4. 出问题的是 ACP 核心语义，还是某个 vendor 特性
5. 当前失败能否用官方 schema 或官方示例复现

## 2. 常见故障分类

### A. 初始化故障

表现：

- `initialize` 失败
- 双方版本不一致
- `clientInfo` / `agentInfo` 缺失引发 UI 或日志异常

排查：

- Client 发送的协议版本是否正确
- Agent 返回版本是否被 Client 接受
- 被省略的 capability 是否被错误当成支持

### B. 会话故障

表现：

- `session/new` 创建失败
- `session/load` 被错误调用
- 多 session 互相串话

排查：

- `cwd` 是否绝对路径
- `mcpServers` 参数是否满足当前 agent 能力
- `loadSession` 是否先在 initialization 响应中声明

### C. 流式更新故障

表现：

- `session/update` 不显示
- 最终响应提前返回
- 工具调用状态不闭合

排查：

- 是否把 notification 当成 request 在等返回
- 是否在 tool call 结束前提前发了最终 `session/prompt` 响应
- 是否遗漏了 `tool_call_update`

### D. 取消故障

表现：

- 用户点击取消后 UI 卡住
- Agent 返回错误而不是 `cancelled`
- 工具仍然继续写文件

排查：

- Client 是否给所有待决权限返回 `cancelled`
- Agent 是否捕获了底层取消异常并映射为 `cancelled`
- 工具层是否支持协作式取消

### E. 权限故障

表现：

- 工具无提示直接运行
- 权限弹窗没有足够上下文
- 权限选择后没有继续执行

排查：

- 该工具是否真的要求用户授权
- `session/request_permission` 是否带足够的 `toolCall` 信息
- Client 是否正确返回所选选项

## 3. 最小抓包与日志建议

排错时最少收集：

- 原始 JSON-RPC 消息顺序
- 连接建立与断开时间
- `session_id`
- `request_id`
- `tool_call_id`
- capability 协商结果
- stop reason
- permission outcome

如果日志系统支持结构化字段，优先结构化记录，不要只留拼接字符串。

## 4. 兼容性评审问题单

做 code review 或发布验收时，按下面清单问：

- 是否把省略 capability 视为不支持
- 是否把所有路径都限制为绝对路径
- 是否保持 1-based 行号
- 是否把取消语义单独建模
- 是否区分 transport error 和 protocol error
- 是否允许未知 `_meta` 字段安全透传或忽略
- 是否把 vendor 扩展隔离在独立适配层
- 是否为 `session/load`、权限拒绝、工具失败写了测试

## 5. 典型验收场景

### 基线场景

- 初始化成功
- 新建 session
- 发送一个文本 prompt
- 收到流式 `session/update`
- 收到最终 `stopReason = end_turn`

### 权限场景

- 触发一个需要权限的工具
- Client 拒绝
- Agent 正确收口，不继续执行工具

### 取消场景

- 工具执行中发送 `session/cancel`
- 待决权限全部变成 `cancelled`
- Agent 最终返回 `cancelled`

### 恢复场景

- 声明 `loadSession`
- 关闭前端
- 重新连接并 `session/load`
- 历史以 `session/update` 重放

### 降级场景

- Client 不支持 `terminal`
- Agent 仍能完成纯文本或只读流程
- UI 明确提示能力受限，不崩溃

## 6. 结论模板

当你要给用户输出评审结论时，建议固定分成三类：

1. 协议违规
   - 明确指出违反了哪个阶段、哪个能力约束或哪个消息顺序
2. 工程风险
   - 协议允许，但实现方式会导致互操作、恢复或调试困难
3. 兼容建议
   - 不改变语义的前提下，如何提升可观测性、测试覆盖和跨产品兼容
