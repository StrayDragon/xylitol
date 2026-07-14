---
change_id: c995-update-agent-hooks-driver-session
title: "库缝：session tree / switch / shutdown hooks"
status: full
priority: 995
depends_on: ["c990-add-test-hooks-wiring-bdd"]
author: agent
track: A
wave: hooks-wiring-bdd
domain: c995
---

# c995-update-agent-hooks-driver-session

## Why

c735 已挂 `session_start` / compact / `before_fork`；Driver 的 tree / switch / shutdown 仍无 `XyHookBus` 旁路。

## Purpose

1. 库权威路径 MUST：
   - `Driver::session_tree` 打开/构建前 → `session_before_tree`（**Blocked = 取消**，可测）
   - tree 成功返回后 → `session_tree`（observe fail-open）
   - `Driver::travel_session_tree`：before_tree（cancel）+ 成功后 session_tree（observe）；payload 含目标 entry
   - `Driver::switch_session` 前 → `session_before_switch`（cancel；reason `resume`）；切换成功后对旧会话 `session_shutdown`（reason `resume`）
2. **新建会话**路径（若存在显式 new）：`session_before_switch` reason=`new` + 旧会话 `session_shutdown` reason=`new`
3. **`session_shutdown` on process quit**：库层若无统一 teardown API → **MAY**（design 标明）；不得假装已接 TUI-only quit 为库合约 MUST
4. 空 bus 零开销；启用 wiring：`切换会话 target` / `打开会话树`

## What Changes

- `InProcessDriver` session APIs + agent 协作点
- `agent-hooks` + `test-hooks-wiring`；BDD cancel + observe

## Capabilities

- `agent-hooks`
- `test-hooks-wiring`

## Out of scope

- model_select（c996）、user_bash（c997）、Completions（c998）
- 强绑 TUI overlay

## Ethics

- risk_level: low
- prohibited_actions: 默认 block 所有 switch；把仅 TUI quit 写成已交付库 MUST
- required_evidence: tree/switch cancel 与 observe 各至少一测
- escalation_policy: quit shutdown 无库点时保持 MAY 并记 future

## Depends

- **c990-add-test-hooks-wiring-bdd**
