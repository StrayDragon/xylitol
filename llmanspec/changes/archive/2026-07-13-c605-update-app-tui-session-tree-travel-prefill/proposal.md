---
change_id: c605-update-app-tui-session-tree-travel-prefill
title: "产品 stub：Enter 在 user 节点预填 editor"
status: full
priority: 605
depends_on: []
author: agent
track: B
---

# c605-update-app-tui-session-tree-travel-prefill

## Why

demo（c600）已对齐 pi：Enter + user → 正文进 editor。产品 c491 stub 仍只记 `travel → id`，双 Esc 验收时看不到「改这条再发」形态。在 **不** 接活树 / Driver 的前提下，stub 可用节点 `kind` + `label` 做同形预填。

## Purpose

放宽 stub 冻结中「Enter 仅记 id」的窄义：user 节点 Enter MUST 预填 editor；仍 MUST NOT 活树 / Driver navigateTree / fork。

## What Changes

1. **合约**：`modify_requirement` `ast2` — user 预填 + 关树 + travel 注记；非 user 不预填。
2. **实现**：`UiRoot` 树槽 Enter：`kind=user` → `editor.set_text(label)`；否则清空或不填；仍 `travel → {id}`。
3. **Harness / design / playground** 注脚对齐。

## Capabilities

- `app-tui-session-tree`
- `app-tui-design-playground`（可选注脚）

## Out of scope

- 活 `session_tree` / filter / fold 产品接线
- `Driver::navigateTree` / 真 transcript 重建
- Shift+F fork
- 改 demo（已由 c600 覆盖）

## Impact

- `src/app/tui/layout/root.rs`、`tests.rs`
- `design/session-tree.md`、`keybindings.md`、playground
