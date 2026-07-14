---
change_id: c670-fix-app-tui-abort-clears-stream
title: "Esc abort 发出后 MUST 真正停轮：不得继续出 assistant 正文"
status: full
priority: 670
depends_on: ["c665-update-app-tui-abort-feedback"]
author: agent
track: B
wave: bugfix
---

# c670-fix-app-tui-abort-clears-stream

## Why

Esc 后已写 System `Aborted`，但 agent EventStream 仍被泵入 bridge：`ThinkingDelta`/`TextDelta` 经 `set_busy_status` 把 UI 拉回 busy，并继续拼出完整 assistant——**假中止**。

## Purpose

1. `note_user_abort` MUST 清空 `streaming_thinking` / `streaming_assistant`。
2. abort 后至本轮 EventStream 结束：host MUST **丢弃**后续 `XyEvent`（含 delta / AgentEnd 正文），不得再 `apply_xy_event` 推进该轮。
3. `Driver::abort` 仍 MUST 调用（既有）；下一轮 submit MUST 可正常开始。

## What Changes

1. `UiModel::note_user_abort`：清 streaming 缓冲。
2. `HostSession`：`suppress_xy_until_stream_end`；`HostEvent::Xy` 在 latch 时丢弃；`on_run_stream_closed` / `on_run_started` 清 latch。
3. harness：abort 后泵入 ThinkingDelta+TextDelta+AgentEnd → 无 assistant 正文、保持 idle。

## Capabilities

- `app-tui-bridge` / `app-tui-host`（modify）

## Out of scope

- bang `(cancelled)`；status 空行（c675）；改 ScriptedDriver 停流（UI 门闩即可测）

## Ethics

- risk_level: medium
- prohibited_actions: 只画 Aborted 仍 apply delta
- required_evidence: harness 停轮
- escalation_policy: 若生产流取消仍不足，另开 agent change

## Depends

- **c665**（已归档）
