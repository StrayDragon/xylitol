---
change_id: c469-demo-session-tree-pan-history
title: "TreeSelector 水平 pan + agent_demo 活树/travel 回复链"
status: full
priority: 469
depends_on: ["c467-enhance-package-tui-tree-fold-label", "c456-demo-session-nav-keys"]
blocks: []
author: agent
track: A
note: "先实现后补规格（retroactive）；实现已在 ab54a95（不含 c468 steer）。"
---

# c469-demo-session-tree-pan-history

## Why

相对 pi：深 indent 选中行需水平平移；会话树须随用户提交/工具/助手增长；travel 到 user 节点时应带上其后的线性 assistant/tool 回复，否则回溯后「相关回复消失」。

## Purpose

包侧 `TreeSelector` 实现 pi 式水平 viewport pan；`agent_demo` 维护活 `session_tree` + travel 路径含线性回复链；harness 覆盖。

## What Changes

1. **包**：`render_horizontal_viewport`（固定 gutter，平移 body）；`set_active_id`；单测深 indent 选中可见。
2. **Demo**：`session_tree` / `history_payloads`；submit、tool、assistant 挂树；`travel_path_with_replies`；travel 后 idle（无 spinner）。
3. **文档**：`session-tree-vs-pi.md` / `session-tree.md` 对齐。

## Capabilities

- `package-tui-tree-selector`（水平 pan）
- `app-tui-session-tree`（demo 活树与 travel 回复链形态学）

## Out of scope

- 产品 `Driver` 真 travel/fork（c491 假树 stub 之后另批）
- steer/follow-up（c468 已归档）
- 产品 `UiRoot` 活树接线
