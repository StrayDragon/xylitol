---
change_id: c463-revise-app-tui-transcript-scope
title: "app-tui-transcript：降级为 live scrollback（禁 Codex 浏览面）"
status: full
priority: 463
depends_on: []
author: agent
track: B
---

# c463-revise-app-tui-transcript-scope

## Why

产品决策：不做 Codex 式 TranscriptView；历史/分支 UX 走双 Esc 会话树。`app-tui-transcript` 壳仍写「对话 transcript / 可展开栈」，会误导后续实现。

## Purpose

改写 capability purpose 与 requirements：仅约束 **live 行写入引擎 scrollback**；显式禁止 Codex 式浏览面；可展开栈不再作为产品硬 MUST（demo 可继续验证）。

## What Changes

1. `purpose` → live scrollback 呈现（非浏览面）。
2. `att1` 收紧为 live 输出 + 禁截断冒充滚动。
3. `remove` `att2`（产品强制 Expandable 栈）。
4. `add` `att6`：MUST NOT 实现 Codex 式 TranscriptView / 专用浏览面。
5. `att4` 与 `design/expandable.md` 对齐（Diff 仅 header tint）。
6. 同步 `design/transcript.md` / `AGENTS.md` 已有叙述（若有漂移再补）。

## Capabilities

- `app-tui-transcript`

## Out of scope

- 实现 live 行渲染代码
- 会话树（c454/c456/c491）
- 删除 capability 目录本身（壳保留，语义降级）
