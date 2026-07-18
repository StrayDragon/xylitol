---
change_id: c1280-update-app-tui-tool-chrome-quiet-defaults
title: "app-tui：工具 chrome 默认安静化（write/edit 不对用户倾倒参数墙）"
status: purpose-draft
priority: 1280
depends_on:
  - c1260-update-app-tui-tool-stream-chrome
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: app
---

# c1280-update-app-tui-tool-chrome-quiet-defaults

## Why

1. 对照 pi（`.tmp/pi-tui-snapshot.txt` + `pi-session-…019f7390….html`）：`write` 默认只显示路径 +「N more lines… expand」；`edit` 默认是折叠 diff 行，而不是整份 `edits`/`content` JSON。
2. xylitol 在 c1260 已有 `human_tool_args_preview`，但 `edit`/`write` 若缺 `path` 字段（常见：仅 `edits[]`）会 **回退 compact JSON**，流式意图阶段仍把大块参数刷进主视线。
3. `docs/roadmaps/TUI视觉与信息表达.md`：默认不要 JSON 墙；完整参数按需展开。

## Purpose（延后；本文件仅 draft）

对齐 pi 默认展现密度：

1. **write**：默认 `write <path>` + 内容行数摘要（如 `… (15 lines)`）；正文默认折叠，展开才见全文。
2. **edit**：默认 `edit <path>`（无 path 时从 edits/旧文推断或 `edit`）；主视线优先 **短 diff 摘要**，不默认展开 `oldText`/`newText`/`edits` 全量。
3. 其它工具：缺关键字段时 **禁止** 把完整 args JSON 当默认预览；用工具名 + 最短可用 hint。
4. 展开手势与 pi 心智对齐（产品可先：已有 expandable / 后续 ctrl+o）；本 change 先锁定 **默认折叠契约**。

## What Changes（拟）

- `src/app/tui/bridge`：`human_tool_args_preview` / Tool 行渲染；write/edit 专用摘要
- 可能 `widgets`/scrollback：折叠正文与 diff 行预算
- live specs：`app-tui-transcript` / `app-tui-bridge` 增补「默认无 JSON 墙」场景
- 证据：对照 `.tmp/pi-tui-snapshot.txt` 同任务视觉密度

## Capabilities

- `app-tui-bridge`（modify）
- `app-tui-transcript`（modify）

## Out of scope

- XML `<tool_call>` 核心抢救（eeee 断流根因；另议，非本 draft）
- 主题/密度 M4 全盘
- c1265 fastrace

## Ethics

- risk_level: low
- prohibited_actions: 默认倾倒完整 write content / edit edits；假树 stub 扩活树
- required_evidence: bridge 单测 — write/edit 流式意图帧默认预览不含大段 content/edits JSON
- escalation_policy: 无 path 时宁可短占位（`write` / `edit`）也不回退整 JSON

## Depends

- `c1260-update-app-tui-tool-stream-chrome`（已有 chrome 挂载点）
- 软依赖：真机对照 pi snapshot（人工）

## Status note

**purpose-draft / 延后**：先锁产品意图，不进入 propose/apply，直到用户显式开闸。
