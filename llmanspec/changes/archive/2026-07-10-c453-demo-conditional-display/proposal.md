---
change_id: c453-demo-conditional-display
title: "agent_demo 统一条件展示（折叠/展开策略原型）"
status: full
priority: 453
depends_on: ["c451-add-package-tui-diff"]
author: agent
track: A
note: "先实现后补规格（retroactive）；折叠/键位已在 agent_demo 落地，本变更补归档。"
---

# c453-demo-conditional-display

## Why

thinking / tool / diff 都需默认折叠、键位展开；策略要在 demo 验证后再锁进产品。

## Purpose

在 `agent_demo` 统一条件展示原型（Thinking / Tool / Diff 共用展开态；Ctrl+T / Alt+E；流式 thinking 自动展开再折叠），并回写 `design/expandable.md`。

## What Changes

1. 统一 Expandable 块状态机（应用层，不进包）：Thinking / Tool / Diff 共用折叠语义。
2. 流式 thinking 自动展开；结束后可折叠（对齐 demo）。
3. Diff 块接入 c451；Alt+E 同时切换 tool+diff；Ctrl+T 切换 thinking。
4. 块旁括号键位提示（`(Ctrl+T)` / `(Alt+E)`）。
5. 回写 `design/expandable.md`。
6. 不锁持久化 settings 格式（future）。

## Capabilities

- `app-tui-transcript`（增补：可展开块折叠策略与键位）

## Out of scope

- 产品面默认策略最终锁定（会话树优先后另议；**不**经已搁置的 c470 Codex TranscriptView）
- 包内通用 Expandable 组件（详情视口见 c466 ExpandableOutput）
- 工具 bg 三态（c462）
