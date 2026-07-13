---
change_id: c655-update-app-tui-footer-context
title: "Footer：有数据时显示 context%"
status: purpose-draft
priority: 655
depends_on: ["c625-update-app-tui-design-next-wave"]
author: agent
track: A
---

# c655-update-app-tui-footer-context

## Why

用户不知道上下文还剩多少；DESIGN 已预留 `· context%`，需 Driver 有数据时显示、无则省略。

## Purpose

footer 字段序 `cwd · model · context%?`；无数据不伪造。

## What Changes（实现时）

1. 经 Driver / GetState（或等价）取上下文占用；映射到 footer。
2. 窄宽截断规则遵循 [`footer.md`](../../../src/app/tui/design/footer.md)。
3. harness：有/无数据两种渲染。

## Capabilities

- `app-tui-chrome`（modify footer）
- 可能轻触 Driver 只读状态（不 reach infra）

## Design SSOT

- [`footer.md`](../../../src/app/tui/design/footer.md)
- [`DESIGN.md`](../../../src/app/tui/DESIGN.md) Layout / Next wave

## Impact

- footer 渲染；只读状态

## Out of scope

- 精确 tokenizer 百分百对齐各厂商（允许最佳努力 + 文档诚实）

## Ethics

- risk_level: low
- prohibited_actions: 无数据时显示假 `0%`/`100%` 墙
- required_evidence: harness 双态

## Depends

- **c625**；与 c630 松耦合（footer model 字段已有）
