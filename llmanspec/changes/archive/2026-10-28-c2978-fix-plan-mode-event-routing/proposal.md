---
depends_on: []
title: 将计划模式事件发布到切换后的会话槽
needs_specs_change: true
branch: sdd/c2978-fix-plan-mode-event-routing
base_branch: main
base_sha: 0ecd10d911d1052f5bd851768d77a0bd89461a7f
---

# 将计划模式事件发布到切换后的会话槽

## Why

- `switch_session` / `new_session` 成功后，Host 需要向客户端发布 `planModeChanged`，让 ModeStrip 跟随新会话。
- 当前 unary 分派在 dispatch 之前按 payload 解析会话：这两个命令的 payload 没有 `session_id`，`session_id_from_payload` 回退到 boot 时的 `fallback_session`，于是 `planModeChanged` 被追加到旧槽 journal。
- 新会话的订阅者收不到该事件；若另一客户端仍挂在 fallback 会话，会收到别的会话的模式事件并错误翻转 ModeStrip，且额外消耗一个 seq。
- TUI 目前“看起来正常”是因为 `apply_switched_session` 又从会话条目初始化 ModeStrip，这与 `app-tui-fixed-zone`「ModeStrip 状态须仅由 `planModeChanged` 驱动，不得在端侧另造模式来源」的条款冲突。

## What Changes

- unary 分派在 `switch_session` / `new_session` 成功后，以 dispatch 返回的新会话 id 重新取槽，并把 `planModeChanged` 追加到该槽 journal。
- 事件发布不得落到 fallback 会话槽；其他会话的订阅者不得收到本事件。
- 明确 ModeStrip 初始状态来源（二选一，见 design）：
  - Host 在订阅/恢复时发布一次 `planModeChanged`，端侧只消费事件；
  - 或允许端侧以会话条目做一次初始快照，之后仅由事件驱动。
- 新增 serve 测试：两个会话槽 + 一次 `switch_session` / `new_session`，断言事件落在新会话 journal，旧槽 journal 无该事件。

## Capabilities

- `server-core`：会话切换/新建后的模式事件路由。
- `app-tui-fixed-zone`：ModeStrip 初始状态来源口径（视决策）。

## Impact

- `crates/xylitol-serve/src/occupancy/host.rs`：dispatch 后的槽解析与事件发布。
- `crates/xylitol-app-tui/src/host/session_ops.rs`：ModeStrip 初始化（视决策）。
- `tests/bdd` 或 serve 单测：事件落槽断言。

## Out of Scope

- 其他事件（`todoUpdated` / `workflowUpdated`）的会话路由；本次只修 `planModeChanged` 的 switch/new 路径。
- `set_plan_mode` 的既有路由（payload 带 `session_id`，行为正确）。

## Further Notes

- 复现证据与行号见 `design.md`「Evidence」。
- 该问题由 PR #30 review F1 记录；修复前 TUI 的 ModeStrip 正确性依赖端侧兜底。
