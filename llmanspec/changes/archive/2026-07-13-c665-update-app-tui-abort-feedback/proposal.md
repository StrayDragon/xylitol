---
change_id: c665-update-app-tui-abort-feedback
title: "产品 TUI：abort / 工具取消的 status 反馈"
status: full
priority: 665
depends_on: ["c660-fix-agent-tool-abort-process"]
author: agent
track: B
---

# c665-update-app-tui-abort-feedback

## Why

内核取消（c660）可靠后，用户仍需立刻看见「已中止」，且 Esc 须能打断进行中的 `!`/`!!` bash（host 不再独占 await）。

## Purpose

1. Esc abort（agent 流或 bang）：scrollback 一行 System `Aborted`，status 回 idle（0 行）；不堆墙。
2. bang 执行中 Esc：调用 `Driver::abort`，bash 以 cancelled 结束；输入循环不阻塞。

## What Changes

1. Host：abort 时立即 `note_user_abort`；bang 期间标 busy + status `Running`；host loop `select!` 并发输入与 `execute_bash`。
2. `Driver::execute_bash` 改为 `&self`（与 abort 共享借用，使 select 可行）。
3. Bridge：`Error("aborted")` 展示为 `Aborted`；若已有本地 Aborted 注记则不重复。
4. Harness：busy Esc → 见 Aborted；bang hang + Esc → abort + cancelled。

## Capabilities

- `app-tui-bridge`（modify）
- `app-tui-input`（modify：bang Esc）
- `app-tui-chrome`（modify：abort 反馈形态）

## Design SSOT

- [`status.md`](../../../src/app/tui/design/status.md) · [`errors.md`](../../../src/app/tui/design/errors.md)
- 本 change `design.md`

## Out of scope

- computer-use；重做 abort 键位；Settings/Plate

## Ethics

- risk_level: low
- prohibited_actions: 多行 debug 墙；假 0%/假 status
- required_evidence: harness abort 反馈 + bang Esc 取消
