---
change_id: c595-add-package-tui-tree-node-kind
title: "TreeSelector：结构化 kind 标签（库机制）"
status: full
priority: 595
depends_on: []
author: agent
track: P
---

# c595-add-package-tui-tree-node-kind

## Why

会话树行上的 `user:` / `assistant:` / `tool:` 目前由 host 预烘焙进 `TreeNode.label`，包无法主题化前缀、也无法按 kind 做稳定过滤/搜索。应对齐「库收结构化标签、host 只填正文」的开闭形态，并为后续 travel（c600）提供可判定的 user 节点。

## Purpose

`TreeNode` 增加可选 `kind`；`TreeSelector` 用主题闭包渲染 kind 前缀；搜索纳入 kind；demo / DESIGN / playground 改为「kind 着色前缀 + 纯正文 label」，废除把 role 写进 label 的旧示意。

## What Changes

1. **包**：`TreeNode.kind: Option<String>`（host 约定如 `user`/`assistant`/`tool`，包不硬编码枚举）；`TreeSelectorTheme::kind_prefix`；渲染顺序：`[annotation]?` + kind 前缀 + `label`；增量搜索 MUST 匹配 kind。
2. **Demo**：样例/活树节点设 `kind`，label 仅正文；filter `UserOnly` 等改看 `kind`。
3. **Design SSOT**：`session-tree.md` / `session-tree-vs-pi.md` / playground 树静图用 token 色画 kind 前缀；单一表现，不再写「文案预渲染进 label」。

## Capabilities

- `package-tui-tree-selector`：节点 kind + 主题前缀 + 搜索
- `app-tui-design-playground`：树槽静图对齐 kind 表现

## Out of scope

- travel / editor 预填（→ **c600**）
- 产品 c491 stub 扩活树
- 把 FilterMode 枚举塞进包（仍 `include_node`）
- 改 annotation（Shift+L）语义

## Impact

- `packages/xylitol-tui/src/components/tree_selector.rs`、re-export、单测
- `packages/xylitol-tui/examples/agent_demo.rs`、相关 harness
- `src/app/tui/design/session-tree.md`、`session-tree-vs-pi.md`、`design/playground/index.html`
