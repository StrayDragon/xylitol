---
change_id: c1460-fix-mcp-tool-display
status: purpose-draft
depends_on: []
branch: feat/c1460-fix-mcp-tool-display
base_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
checkpointed: true
checkpoint_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
---

# c1460-fix-mcp-tool-display

## Why

MCP 工具（`mcp:{server}:{tool}`）在 TUI scrollback / Print 面上目前几乎是「机器 JSON 墙」：

- **调用参数**：header `args_preview` 对未知工具几乎空或弱摘要；Print 的 `ToolExecutionStart` 只打工具名、丢掉 args——排错时看不到「怎么调的」。
- **返回结果**：adapter 用 `serde_json::to_string` 压成单行；TUI 不经 `humanize`；Print 只取前 3 行（单行 JSON 等于整坨糊在一行）。

内置 `read`/`bash`/`write`/`edit` 已有 humanize；`ls`/`grep`/`find` 本来就是纯文本。缺口集中在 MCP（及同类未登记 humanize 的 JSON 信封）。

本期**刻意不做** CallToolResult `content[].text` 抽取——先保证：能看见调用参数 + 结果 JSON pretty；视口仍走既有 Ctrl+O。

## What Changes

- TUI：`mcp:` 工具块 **header 不重复参数**；body 为 `args:`（pretty 请求）+ `result:`（pretty JSON，不抽 content）；忽略 Update 对流式 raw 的追加以免重复。
- Print：`ToolExecutionStart` 对 `mcp:` 打印 pretty args；`ToolExecutionEnd` 对 JSON 结果 pretty（预览至多 24 行）。
- 共享：`src/app/tool_display.rs`（`is_mcp_tool_name` / `pretty_json_*` / `mcp_tool_body`），TUI bridge 与 Print 共用。
- **非目标（本期）**：抽取 MCP text 内容、改 Alt+E/Ctrl+O 语义、为每个 MCP server 定制 chrome。

## Capabilities

- `app-tui-transcript` / `app-tui-bridge`（呈现）
- `cli-print`（Print 摘要）

## Impact

- 用户可见：MCP 块可读性↑；排错能对照 args/result。
- 合约：本期以 **purpose-draft 留档 + 实现合入** 收口；未新增 live MUST（后续若要合约化再 promote）。
- 风险：低；仅展示层，不改 MCP 协议或 ReAct。
