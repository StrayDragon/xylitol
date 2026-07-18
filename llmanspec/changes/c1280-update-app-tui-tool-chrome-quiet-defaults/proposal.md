---
change_id: c1280-update-app-tui-tool-chrome-quiet-defaults
title: app-tui：工具 chrome 默认安静化（write/edit 不对用户倾倒参数墙）
status: full
priority: 1280
depends_on:
- c1260-update-app-tui-tool-stream-chrome
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: app
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: false
---

# c1280-update-app-tui-tool-chrome-quiet-defaults

## Why

1. 对照 pi：`write` 默认只显示路径 + 行数提示；`edit` 默认折叠 diff，而不是整份 `edits`/`content` JSON。
2. c1260 的 `human_tool_args_preview` 在缺 `path`（常见：仅 `edits[]`）时 **回退 compact JSON**，流式意图阶段仍把大块参数刷进主视线。
3. `ToolExecutionEnd` 成功时把机器结果 JSON（如 `{"path","success"}` / 含 `display_diff` 的整包）填进 `Tool.output`；Alt+E 展开后仍是 JSON 墙。edit 已另挂 `UiEntry::Diff`，Tool 行不应再倾倒同一份 JSON。
4. `docs/roadmaps/TUI视觉与信息表达.md`：默认不要 JSON 墙；完整参数/输出按需展开。

## Purpose

对齐 pi 默认展现密度：

1. **write**：默认 `write <path> (N lines)`（有 content 时计行）；无 path 时短占位 `write`，**MUST NOT** 倾倒 content JSON。
2. **edit**：默认 `edit <path>`（可从 `edits[0].path` 推断）；无 path 时 `edit`；**MUST NOT** 倾倒 `edits`/`oldText`/`newText` JSON。
3. **其它工具**：缺关键字段时用最短占位（工具名级），**MUST NOT** 把完整 args JSON 当默认 `args_preview`。
4. **成功输出**：write/edit 成功的机器 JSON **MUST NOT** 默认填入可展开 Tool.output（edit 成功 diff 走 `UiEntry::Diff`）；错误结果仍可见。
5. 展开手势沿用 Alt+E / ctrl+o；本 change 锁定 **默认折叠 + 无 JSON 回退** 契约。

## What Changes

- `src/app/tui/bridge`：`human_tool_args_preview` 安静化；`ToolExecutionEnd` 成功 quiet output
- live specs：`app-tui-transcript` att13 强化；`app-tui-bridge` atb11（quiet success output）
- bridge 单测：write/edit 缺 path / 含 edits / 含 content；成功 End 后 output 为空

## Capabilities

- `app-tui-bridge`（modify）
- `app-tui-transcript`（modify）

## Out of scope

- XML `<tool_call>` 核心抢救
- edit exact-match 失败重试 UX（另议）
- 主题/密度 M4 全盘
- c1265 fastrace
- llman SDD stage 推断（其它 agent 轨）

## Ethics

- risk_level: low
- prohibited_actions: 默认倾倒完整 write content / edit edits / 成功机器 JSON；假树 stub 扩活树
- required_evidence: bridge 单测 — write/edit 意图与成功 End 默认预览/output 不含大段 JSON
- escalation_policy: 无 path 时宁可短占位（`write` / `edit`）也不回退整 JSON

## Depends

- `c1260-update-app-tui-tool-stream-chrome`（已有 chrome 挂载点）
