---
change_id: c675-update-app-tui-status-spacer
title: "Status spinner 紧贴 input，上方保留一行空白（对齐 demo）"
status: full
priority: 675
depends_on: ["c475-add-app-tui-chrome"]
author: agent
track: B
wave: polish
---

# c675-update-app-tui-status-spacer

## Why

产品 busy status 与 `agent_demo` / [`status.md`](../../../src/app/tui/design/status.md) 不一致：demo **保留** `Loader::render` 的前导空行，使 spinner **紧贴 editor 上方、上空一行呼吸**；产品 `UiRoot::render_status_slot` 当前 **故意 filter 掉空行**（注释：drop empties so busy is 1 row）。

## Purpose

1. busy：status 槽 = **前导空行 +** `spinner + 短词`，紧贴 input。
2. idle：**1 行空白**呼吸间距（input 不贴 transcript）。
3. harness 覆盖 busy / idle 两种间距。

## What Changes

1. `render_status_slot`：busy **停止** strip 空行；idle 返回一行 `""`（对齐 demo `status_lines`）。
2. harness：busy 帧 editor 上为 blank→spinner；idle 帧仍有一行 blank。
3. 对照 `status.md`（已写 MUST；若文案过时则同步）。

## Capabilities

- `app-tui-chrome`（modify）

## Design SSOT

- [`status.md`](../../../src/app/tui/design/status.md)
- demo `agent_demo.rs` `status_lines`

## Out of scope

- Esc abort 停轮（→ **c670**，另案）
- 改 Loader 包 API

## Ethics

- risk_level: low
- prohibited_actions: 双行文案 status；假 Ready
- required_evidence: harness 前导空行
- escalation_policy: 无

## Depends

- **c475**（已归档）
