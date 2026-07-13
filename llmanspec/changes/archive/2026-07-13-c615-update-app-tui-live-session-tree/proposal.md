---
change_id: c615-update-app-tui-live-session-tree
title: "产品 TUI：MessageHistory 活树 + Driver travel"
status: full
priority: 615
depends_on: ["c610-add-driver-session-tree-kind"]
author: agent
track: B
---

# c615-update-app-tui-live-session-tree

## Why

c491 假树 + c605 stub 预填已验证槽与键位，但数据与 leaf 仍不连真 session。demo（c600）与 pi 的 travel 形态需落到产品：双 Esc 看真历史、Enter 经 Driver 旅行。

## Purpose

在 **c610** API 就绪后，解冻产品 Tree 槽：渲染 `session_tree(MessageHistory)`，Enter 走 `travel_session_tree`，按 `SessionTreeTravel` 更新 UI；废除假树为唯一数据源。

## What Changes

1. **合约**：改写 `ast2` / `ati18`；新增映射要求 `ast6`；playground `adp5` 注脚改为活树。
2. **实现**：`UiRoot`/host 打开树时拉 MessageHistory → 映射 `TreeNode`（kind + 纯正文 label）；Enter → Driver travel → 关树、预填、刷新 transcript/状态。
3. **DESIGN / AGENTS / playground**：去掉「禁止 Driver travel / 假树冻结」表述，改为活树 SSOT。
4. **Harness**：合成测覆盖开树见活数据、user Enter 预填（ScriptedDriver 桩）。

## Capabilities

- `app-tui-session-tree`
- `app-tui-input`
- `app-tui-design-playground`

## Out of scope

- 产品 filter / fold / 水平 pan 全量对齐 demo（可后续）
- Shift+F：同 session 分叉 vs `Driver::fork_session` 文件 fork（见 future.md）
- 实现第二种 `SessionTreeKind`

## Impact

- `src/app/tui/layout/root.rs`、`host.rs`、`harness.rs`、`tests.rs`
- `AGENTS.md`、`design/session-tree.md`、`keybindings.md`、playground
