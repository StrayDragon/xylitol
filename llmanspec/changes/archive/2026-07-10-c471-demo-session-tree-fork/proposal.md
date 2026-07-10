---
change_id: c471-demo-session-tree-fork
title: "agent_demo：会话树内 fork（同会话分叉）"
status: full
priority: 471
depends_on: ["c469-demo-session-tree-pan-history"]
author: agent
track: A
---

# c471-demo-session-tree-fork

## Why

pi 用 `/fork` 从历史 user 消息开新会话；xylitol 单会话树里需要等价形态：从某节点分叉继续，旧子树保留为兄弟分支。travel（Enter）会带上回复链；fork 必须停在选中点以便改写后再提交。

## Purpose

`agent_demo` 树开时 **Shift+F**：fork 到选中节点（重建 path、leaf=选中、不跟线性回复链）；若为 user 节点则填入编辑器；下次 Enter 提交在该 leaf 下挂新子节点形成分叉。

## What Changes

1. `fork_from_history(id)`：path 重建、leaf=id、可选 editor 预填、关树、系统行。
2. 键位 Shift+F；树 header / keybindings / vs-pi 文档。
3. harness：fork 后 leaf 与 editor；再 submit 出现兄弟分支。

## Capabilities

- `app-tui-session-tree`（demo fork 形态学）

## Out of scope

- 多会话文件 / `Driver::fork`（产品另批）
- branch summary 提示
- 产品 c491 stub 扩展（仍冻结）
