---
change_id: c453-demo-conditional-display
title: "agent_demo 统一条件展示（折叠/展开策略原型）"
status: full
priority: 453
depends_on: ["c451-add-package-tui-diff"]
author: agent
track: A
---

# c453-demo-conditional-display

## Why

thinking / tool / diff 都需默认折叠、键位展开；策略要在 demo 验证后再锁进产品。

## Purpose

在 `agent_demo` 统一条件展示原型（默认折叠、Ctrl+T / Alt+E、配置档），并回写 `design/expandable.md`。

## What Changes

1. 统一 Expandable 块状态机（应用层，不进包）：Thinking / Tool / Diff 共用折叠语义。
2. 流式 thinking 自动展开再折叠（对齐现有 demo）。
3. Diff 块接入 c451（已有）；Alt+E 同时切换 tool+diff。
4. 回写 `design/expandable.md` 锁定 demo 验证过的默认策略。
5. 不锁持久化 settings 格式（future）。

## Out of scope

- 产品面默认策略最终锁定（可在 c470 升格）
- 包内通用 Expandable 组件
