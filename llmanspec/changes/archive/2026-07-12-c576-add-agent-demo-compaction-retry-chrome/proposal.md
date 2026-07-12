---
change_id: c576-add-agent-demo-compaction-retry-chrome
title: "agent_demo：Compacting / Retry 状态预览（非 overlay）"
status: archived
priority: 576
depends_on: ["c493-add-app-tui-compaction-retry-ui"]
author: agent
track: P
---

# c576-add-agent-demo-compaction-retry-chrome

> **status: archived**

## Why

c493 已在产品 bridge 落地 Compacting / Retry 单行 status，但活路径难触发。需在 `agent_demo` 用假触发验收观感，再考虑产品 `/compact` 等接线。

capturing overlay 焦点演示 **不做**：产品交互优先 TreeSelector / ChoicePrompt 等 editor 槽；overlay 非产品主控件（见 `design/overlay.md`）。

## What Changes

1. `agent_demo`：Alt+K / plate `compact-status` / `/compact` → Compacting → Working → Ready + System 说明。
2. Alt+Y / plate `retry-status` / `/retry` → Retry 1/3 → fail 说明 → Working → Ready。
3. **MUST NOT** 在本 change 加 overlay focus-restore 演示入口。
4. 更新 `design/overlay.md` / DESIGN：默认不用 capturing overlay。

## Capabilities

- `package-tui-agent-demo`：增补 Compacting / Retry chrome 可触发预览（pad5）

## Out of scope

- 产品 `src/app/tui` `/compact` slash、runtime emit AutoRetry
- 改 c575 包引擎 focus-restore（已归档；产品可不接线）
- capturing overlay 演示

## Impact

- `packages/xylitol-tui/examples/agent_demo.rs`、相关 harness 测、`src/app/tui/design/overlay.md`
