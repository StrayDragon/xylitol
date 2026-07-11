---
change_id: c550-update-package-tui-expandable-edges
title: "package-tui-expandable-output：Head 模式与零宽边角"
status: full
priority: 550
depends_on: []
author: agent
track: P
---

# c550-update-package-tui-expandable-edges

## Why

ExpandableOutput 已覆盖 Tail 贴尾与 earlier 提示；Head 截断与极端宽度（0/1）缺少明确合约与回归，工具/bash 长输出在 demo 里偶发边界闪烁时难定位。

## Purpose

为 `package-tui-expandable-output` 补齐 Head 提示与窄/零宽安全渲染合约。

## What Changes

1. Head 模式 more-lines 提示位置与文案合约（对照既有 peo2）。
2. width=0/1 或空文本 MUST 不 panic、输出有界。
3. 扩单测；agent_demo 仅在需要时点一下（非必须改 demo）。

## Capabilities

- `package-tui-expandable-output`（修改）

## Impact

- `packages/xylitol-tui/src/components/expandable_output.rs` 及测试
- 对照 `design/expandable.md`

## Out of scope

- 产品 tool tint / Diff header 策略（已有 design）
- 改折叠快捷键语义

## Ethics

- risk_level: low
- prohibited_actions: 不把产品 tint 逻辑打进包组件
- required_evidence: expandable 单测绿；validate 通过
