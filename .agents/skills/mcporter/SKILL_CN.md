---
name: mcporter
description: 使用 mcporter CLI 直接列出、配置、鉴权并调用 MCP 服务器/工具（HTTP 或 stdio），包括临时服务器、配置编辑以及 CLI/类型生成。
---

# mcporter

主页: http://mcporter.dev

使用 `mcporter` 直接与 MCP 服务器交互。

快速开始
- `mcporter list`
- `mcporter list <server> --schema`
- `mcporter call <server.tool> key=value`

调用工具
- 选择器语法: `mcporter call linear.list_issues team=ENG limit:5`
- 函数语法: `mcporter call "linear.create_issue(title: \"Bug\")"`
- 完整 URL: `mcporter call https://api.example.com/mcp.fetch url:https://example.com`
- Stdio: `mcporter call --stdio "bun run ./server.ts" scrape url=https://example.com`
- JSON 载荷: `mcporter call <server.tool> --args '{"limit":5}'`

鉴权与配置
- OAuth 鉴权: `mcporter auth <server | url> [--reset]`
- 配置: `mcporter config list|get|add|remove|import|login|logout`

守护进程
- `mcporter daemon start|status|stop|restart`

代码生成
- CLI: `mcporter generate-cli --server <name>` 或 `--command <url>`
- 检查: `mcporter inspect-cli <path> [--json]`
- TypeScript: `mcporter emit-ts <server> --mode client|types`

说明
- 默认配置文件: `./config/mcporter.json`（可通过 `--config` 覆盖）。
- 如需机器可读结果，优先使用 `--output json`。
