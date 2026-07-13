---
change_id: c600-update-app-tui-session-tree-travel
title: "demo travel 对齐 pi：user → leaf=父 + editor 预填"
status: full
priority: 600
depends_on: ["c595-add-package-tui-tree-node-kind"]
author: agent
track: P
---

# c600-update-app-tui-session-tree-travel

## Why

pi `navigateTree`：Enter 选中 **user** 时 leaf 落到父节点、用户正文进入 editor（空编辑器时）；非 user 则 leaf=选中、不预填。xylitol `agent_demo` 的 `travel_to_history` 仍走「回复链末端 + 不填 editor」，与 pi 及用户期望的「改这条再发」不一致。fork（Shift+F）已有预填，但 travel 缺口仍在。

## Purpose

改写 demo travel 语义与 `ast4`：user（`kind=user`）→ 父 leaf + 预填；非 user → leaf=选中、清空或不填；同步 DESIGN / playground / vs-pi 为唯一 SSOT。

## What Changes

1. **Demo**：`travel_to_history` 按 kind 分支；user 用 payload 正文 `set_text`（可剥 `[steer] `）；transcript 重建到**父路径**（不含被选 user 及其子回复链）；非 user 重建到选中节点（不再强制跟线性回复链）。
2. **合约**：`modify_requirement` 替换 `ast4`（废除 travel-user-keeps-reply）。
3. **Design / playground**：Enter travel 说明改为 pi 语义；树槽注脚体现「Enter user → 填 input」。
4. **Harness**：覆盖 user travel 预填与 leaf=父；非 user 不预填。

## Capabilities

- `app-tui-session-tree`：demo travel 语义
- `app-tui-design-playground`：travel 静图/文案对齐

## Out of scope

- 产品 c491 stub / Driver 真 travel
- branch summary 对话框（pi summarization prompt）
- 改 fork（ast5）语义
- 包 `TreeSelector` API（kind 已在 c595）

## Impact

- `packages/xylitol-tui/examples/agent_demo.rs`、`tests/agent_demo_test.rs`
- `llmanspec/specs/app-tui-session-tree`（经 delta）
- `src/app/tui/design/session-tree.md`、`session-tree-vs-pi.md`、`keybindings.md`、playground
