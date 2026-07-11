---
change_id: c560-update-package-tui-tree-edges
title: "package-tui-tree-selector：无匹配与过滤后选中稳定"
status: full
priority: 560
depends_on: []
author: agent
track: P
---

# c560-update-package-tui-tree-edges

## Why

TreeSelector 已具备过滤/搜索/fold/平移；缺「无匹配空态」与「过滤后选中 id 尽量稳定」的短合约，demo 会话树在切 filter 时偶发选中跳动，开闸前宜收口。

## Purpose

为 `package-tui-tree-selector` 增补空态提示与选中稳定合约。

## What Changes

1. 无可见节点时 MUST 渲染可辨空态（主题 no_match 或等价文案）。
2. include_node / 搜索变更后，若原 selected id 仍可见 MUST 保持；否则落到合理默认（首项或 0）。
3. 扩 tree_selector 单测。

## Capabilities

- `package-tui-tree-selector`（修改）

## Impact

- `packages/xylitol-tui/src/components/tree_selector.rs` 及测试
- demo 仅在回归需要时点一下

## Out of scope

- 产品 FilterMode 枚举进包
- 改键位表默认值（除非测需要）

## Ethics

- risk_level: low
- prohibited_actions: 不引入主 crate session 类型
- required_evidence: tree_selector 单测绿；validate 通过
