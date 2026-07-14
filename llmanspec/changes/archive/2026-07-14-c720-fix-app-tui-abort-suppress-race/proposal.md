---
change_id: c720-fix-app-tui-abort-suppress-race
title: "Busy Esc 立刻抑制 Xy：消除 abort 与 drain 之间的 UI 竞态"
status: full
priority: 720
depends_on: ["c715-update-app-tui-qa-baseline"]
author: agent
track: QA
wave: stage-qa-s1
---

# c720-fix-app-tui-abort-suppress-race

## Why

Busy Esc 仅置 `pending_abort`；`suppress_xy` / `note_user_abort` 要等下一轮 `drain_pending`。主环无 biased 时，窗口内迟到 delta 仍可 `apply_xy_event`，造成假忙或正文闪现。ath8 覆盖「note 之后」；缺口在 **latch→note**。

## Purpose

1. Busy Esc（非 overlay）在 `HostSession::step` **同步**臂装 `suppress_xy_until_stream_end`（或等价），迟到 Xy 在 drain 前即丢弃。
2. 保留 `pending_abort`，使 `drain_pending` 仍调用 `Driver::abort` + `note_user_abort`（Aborted 文案 / 清 streaming / `suppress_idle_esc`）。
3. Bang Esc 仍 `note_bash_cancelled`，不混用 Aborted。

## What Changes

1. `host.rs`：`try_busy_input` Esc 臂装 suppress（不提前 `take` abort）。
2. `note_user_abort`：对已臂装 suppress / 已写 Aborted 做幂等。
3. harness + BDD：Esc → 注入 Xy → 再 drain → 不复活（收紧 c715 基线）。

## Capabilities

- `app-tui-host`（modify ath8；add ath11）
- `app-tui-input`（ati31）

## Out of scope

- bang 环拓扑合一（c725）；host 文件拆分（c730）；主环改 biased（可选，非必须）

## Ethics

- risk_level: medium
- prohibited_actions: Esc 后仍 apply 该轮 TextDelta；把 bang cancelled 改成 Aborted
- required_evidence: harness esc-before-drain；相关 BDD；c715 BASE 仍绿
- escalation_policy: 若 Driver::abort 必须先于 UI note 才能安全，升设计复议

## Depends

- **c715** MUST 先 apply 并 archive（llman depends_on）
