---
change_id: c467-enhance-package-tui-tree-fold-label
title: "TreeSelector：fold / 分支跳转 / annotation label"
status: full
priority: 467
depends_on: []
author: agent
track: A
---

# c467-enhance-package-tui-tree-fold-label

> c456 之后相对 pi 的下一批树操作。

## Why

深会话树需要折叠子树与按分支段跳转；用户 annotation（label）是 travel 前的定位线索。缺这些则 demo 仍难验收真实树工作流。

## Purpose

包 `TreeSelector`：⊞/⊟ fold、Ctrl/Alt+←→（fold 或分支段跳转）、可选 `annotation`/`annotation_at` 与 Shift+L / Shift+T；demo 接线并回写 vs-pi / session-tree / keybindings。

## What Changes

1. 包：`folded_nodes`、fold 标记、分支段跳转、`tui.tree.*` 键；`TreeNode.annotation`；`on_label_edit`；时间戳显示开关。
2. Demo：假树 fold/label 编辑槽；Shift+T 切换时间戳。
3. 文档与 harness。

## Out of scope

- 水平平移（另批）
- 产品 session 持久化 label（c491）
- FilterMode 进包（仍 `include_node`）
