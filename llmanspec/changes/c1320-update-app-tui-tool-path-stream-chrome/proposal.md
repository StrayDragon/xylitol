---
change_id: c1320-update-app-tui-tool-path-stream-chrome
title: app-tui：write/edit/read 路径流式 chrome 与 read 行域
status: full
priority: 1320
depends_on:
- c1300-update-app-tui-write-edit-process-chrome
author: agent
track: stream-tool-ux
wave: tui-stream-align
domain: app
branch: feat/tui-dev
base_sha: 680e83b0baf487deed2bfe8f17a19c96c0c4f975
checkpointed: false
---

# c1320-update-app-tui-tool-path-stream-chrome

## Why

对照 pi（`packages/coding-agent` write/edit/read + `renderToolPath`）与产品截图：

1. **edit / write 流式阶段**常只见 `edit` / `write (N lines)`，路径要等执行结束或完整 args 才出现；pi 在每次 `updateArgs` 即刷新 `edit|write {path|…}`，路径一旦出现在 partial JSON 就进 header。
2. **write** 在 content 已流、path 尚未到（或 partial 解析短暂丢 path）时只显示 `(N lines)`，割裂过程确认。
3. **read** 缺 pi 的行域后缀 `:offset` / `:start-end`（`offset`+`limit` → `start+(limit-1)`）。
4. **edit「行信息」**：pi **不**把行号塞进 header（测试明确禁止 `:1`）；行号在 **diff 正文**（`±NNN` / display gutter）。产品已有 `display_diff` + 行号 gutter（c1300 合块）；本变更不另造 header 行域。

## Purpose

1. **路径尽早**：MessageUpdate / ToolCall 意图阶段，write/edit/read 的 `args_preview` MUST 在 path（或 `file_path`/`file`）一旦可解析时立即显示缩短路径；空 path 用 `…` 占位（对齐 pi `…`），MUST NOT 等 ToolExecutionEnd 才首次出现路径。
2. **粘性 path**：同 tool id 后续 upsert 若新 args 缺 path 但旧行已有 path，MUST 保留已见路径（防 partial JSON 在 content 增长时丢 path）。
3. **End 回填**：ToolExecutionEnd 成功 JSON 含 `path` 时，若 header 仍无路径，MUST 回填进 `args_preview`。
4. **read 行域**：有 `offset`/`limit` 时 header MUST 附 `:start` 或 `:start-end`（warning/muted 可用既有 theme；最小先纯文本后缀）。
5. **edit 行信息**：继续靠合块 `display_diff` 行号 gutter；MUST NOT 在 edit header 加 `:N` 行域（对齐 pi）。

## What Changes

- `src/app/tui/bridge/preview.rs`：`human_tool_args_preview` — `file_path` 别名；空 path → `…`；read `:range`；write 有 content 无 path 时 `write … (N lines)` 而非裸 `write (N lines)`（若策略选粘性则由 upsert 层保证）
- `src/app/tui/bridge` upsert / ToolExecutionEnd：粘性 path + result.path 回填
- live specs：`app-tui-bridge` / `app-tui-transcript`（扩展 atb10/att13 或新 req）
- 单测：流式 path 渐进、粘性、read range、End 回填

## Capabilities

- `app-tui-bridge`（modify）
- `app-tui-transcript`（modify）

## Out of scope

- tool-*-bg 色值改「真蓝」（DESIGN tokens；另案）
- write 语法高亮增量（pi incremental highlight）
- compact read skill/docs/resource 分类头（pi `formatCompactReadCall`）
- c1310 archive / infra result 形状

## Ethics

- risk_level: low
- prohibited_actions: 默认倾倒 edits/content JSON；假树扩活
- required_evidence: 单测 — MessageUpdate 渐进 path；缺 path 粘性；read `:120-329`；edit header 无 `:1`
- escalation_policy: 若 partial JSON 系统性晚于 content 才给出 path，仍靠粘性+End 回填；不改模型 schema

## Depends

- `c1300-update-app-tui-write-edit-process-chrome`（write viewport / edit 合块基线）
