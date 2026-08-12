---
depends_on:
  - c1760-add-tui-activity-fold
  - c2020-add-package-tui-mouse-input
  - c2040-add-tui-mouse-click-fold-triangle
  - c2070-add-package-tui-dual-interaction-modes
status: superseded
absorbed_by: c2045-add-tui-fold-target-remaining
skip_specs_landing: true
---

# 多级 ActivityFold 适配鼠标 / 块级覆盖

> **状态**：**docs-only superseded**（2026-08-13）。段命中 / 统一 `FoldTarget` 由 [`c2045`](../../c2045-add-tui-fold-target-remaining/proposal.md) Wave B 兑现；本票 **无 Branch binding / 无 Specs landing / 无应用代码**。
>
> 前置：[`c1760`](../2026-08-12-c1760-add-tui-activity-fold/)（已归档）· [`c2040`](../2026-08-12-c2040-add-tui-mouse-click-fold-triangle/) · [`c2070`](../2026-08-12-c2070-add-package-tui-dual-interaction-modes/)。

> **一句话**：语义矩阵与性能 MUST 的决策史；实现归 c2045。

## Why

原为 c1760 多级折叠落地后的鼠标适配层。c2040 Q6=B 后，广义 `FoldTarget` 总装归 c2045，本票默认路径 A（吸收）→ docs-only。

## What Changes

**无代码 / 无 live specs。** 兑现对照：

| 本票意图 | 落点 |
|---|---|
| `FoldTarget::Segment(id)` + 摘要三角命中 | c2045 Wave B / att31 |
| 深挖 A 分层 | c1760 att25 + c2045 harness |
| 统一 hit 表 / 无第二管道 | c2045 att32 |
| 性能 O(可见) / ath25 | c2045 Wave B 抽检 |

## Capabilities

无（docs-only）。矩阵原文：[`research/multilevel-fold-interaction-matrix.md`](./research/multilevel-fold-interaction-matrix.md)。

## Impact

| 相关 | 关系 |
|---|---|
| c2045 | **absorbed_by** — Segment 实现 + 合约 |
| c1760 | 段状态提供者（已归档） |

## Out of scope

- 独立 apply / 平行 hit API（禁止）
- 复活 keyboard fold-leader

## Ethics

- risk_level: low
- prohibited_actions: 复活独立段命中管道
- required_evidence: c2045 Wave B verify PASS
