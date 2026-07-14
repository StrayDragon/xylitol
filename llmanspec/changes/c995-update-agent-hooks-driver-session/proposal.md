---
change_id: c995-update-agent-hooks-driver-session
title: "Hook：Driver 侧 session tree / switch / shutdown"
status: purpose-draft
priority: 995
depends_on: ["c735-update-agent-hooks-pi-parity"]
author: agent
track: A
wave: hooks-pi-parity-followup
domain: c995
---

# c995-update-agent-hooks-driver-session

> **status: purpose-draft** — 承接 `c735` `future.md` Driver/session 缺口。升格 full 前不 apply。
> **depends_on c735**：须待 c735 归档后再 apply。

## Why

c735 在 agent 层已挂 `session_start` / compact / `before_fork`，但 **Driver** 面的 session-tree、travel、`/new`·`/resume` 式 switch、以及进程/会话结束时的 `session_shutdown` 仍无 `hook_bus` 旁路。扩展无法对齐 pi 的 tree/switch/shutdown 钩子。

## Purpose

1. 为 `Driver`（至少 `InProcessDriver`）暴露或注入 `XyHookBus`（或等价 seam），使应用面 session 操作可发脚本 hook。
2. MUST 接线（observe；cancel 语义若做须可测）：
   - `session_before_tree` / `session_tree`（`session_tree` / `travel_session_tree`）
   - `session_before_switch`（新会话 / resume / 切换，对齐产品入口）
   - `session_shutdown`（会话结束或替换前清理）
3. 空 `HookDispatcher` 零开销；Blocked 对 observe 事件 fail-open（与 c735 一致），若实现 cancel 则须明确文档与 BDD。
4. 不改 session JSONL 格式；不把抓包装进本 change（见 `c999`）。

## What Changes（升格后预期）

- Driver / TUI host 或 dispatch 路径挂 hook
- `agent-hooks` delta + BDD
- 更新 c735 future 勾销对应行

## Capabilities

- `agent-hooks`（modify）
- 可能触及 `app-tui-host` / Driver seam（升格时声明）

## Out of scope

- `model_select`（`c996`）
- bang/`input`（`c997`）
- Completions HTTP 三缝（`c998`）
- Track B 抓包（`c999`）

## Ethics

- risk_level: low
- prohibited_actions: 默认阻塞所有 session switch
- required_evidence: BDD 或 harness 证明 tree/switch/shutdown 至少各触发一次
- escalation_policy: cancel 语义与 pi 不一致时先对齐产品面再实现

## Depends

- **c735-update-agent-hooks-pi-parity**（归档后）
