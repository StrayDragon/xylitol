---
change_id: c1055-update-app-tui-footer-cost-breakdown
title: "Footer：费用与 ↑↓ / cache 分项（pi 风格账单感）"
status: purpose-draft
priority: 1055
depends_on:
  - "c1035-update-app-tui-footer-token-usage"
author: agent
track: A
---

# c1055-update-app-tui-footer-cost-breakdown

> **status: purpose-draft** — 待产品明确要账单感展示，且 c1035 footer token 文案已落地后 promote。

## Why

c1035 只做「used tokens」诚实展示。pi 全量 footer 还包含累计费用、输入/输出分项与 cache 命中感；xylitol 若需要应对齐，应独立变更以免拖垮 token 可信度主线。

## Purpose

1. 基于 session 内累计 `XyUsage`（及 cost rates）展示费用与分项。
2. 与「当前上下文 used tokens」语义分离，避免混用账单累计与 leaf 上下文占用。
3. 窄宽截断遵循 footer DESIGN。

## Capabilities（promote 时）

- `app-tui-chrome`（modify）

## Out of scope

- 重新实现 accounting 降级链

## Depends

- **c1035-update-app-tui-footer-token-usage**
