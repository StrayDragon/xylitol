---
change_id: c468-demo-steer-followup
title: "agent_demo：忙碌 Enter=steer / Alt+Enter=follow-up 假队列"
status: full
priority: 468
depends_on: ["c461-expose-steer-followup-seam"]
blocks: ["c480-add-app-tui-input"]
author: agent
track: A
note: "先实现后补规格（retroactive）；实现已在 ab54a95。"
---

# c468-demo-steer-followup

## Why

产品键位（`app-tui-input` ati3 / `keybindings.md`）约定：流中 Enter = **steer**，Alt+Enter = **follow-up**。c461 已暴露 Driver/协议 seam，但产品 `UiRoot` 尚未接线。需要在 `agent_demo` 用假队列验证形态学，避免忙碌时二次 Enter 打断当前轮（旧行为会 `queue_simulated_turn` 清脚本）。

## Purpose

在 `agent_demo` 实现 steer / follow-up 假队列：忙碌 Enter 入 steer 且不打断；Alt+Enter 入 follow-up 至空闲再开新轮；footer 显示队列计数；harness 覆盖。

## What Changes

1. `FakeCodingAgentApp`：`steer_queue` / `follow_up_queue`；`is_turn_busy` 时 Enter→steer，Alt+Enter→follow-up。
2. steer 入队时写入 transcript + 活树 `[steer]` 节点，**不清** `scheduled_actions`。
3. 当前轮结束后 `drain_message_queues`：先 steer 再 follow-up。
4. footer：`steer:N` / `follow-up:N` 提示。
5. harness：`steer_does_not_abort_busy_turn`、`follow_up_queues_while_busy`。
6. 回写 `design/queue-steer.md` / `keybindings.md` / `session-tree-vs-pi.md`（demo 已验证）。

## Capabilities

- `app-tui-input`（增补：demo 形态学验证 ati3 键位语义）

## Out of scope

- 产品 `Driver::steer` / `follow_up` 接线（c480 / c465）
- 真实 ReAct 迭代注入（c461 runtime 已有合约；demo 仅假队列）
- session tree pan / travel / 活树增长（同 commit 其它项，非本 change 合约）
