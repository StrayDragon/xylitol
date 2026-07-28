---
change_id: c1680-add-tui-compaction-percent-display
title: TUI 上下文占用百分比派生展示（后置）
status: purpose-draft
priority: 1680
depends_on:
  - c1630-update-compaction-reserve-formula
author: agent
---

# c1680-add-tui-compaction-percent-display

## Why

触发 SSOT 改为 reserve 公式后，产品仍可能希望 footer / chrome **派生**显示「约占用 x%」（`tokens / contextWindow`），且诚实标注 provenance。该展示**不得**再成为触发闸。本 change **后置**：等 c1630（及建议的 c1640）落地后再做视觉提案，避免与公式迁移缠在一起。

## What Changes（意向）

- Footer（或 compact status）展示派生百分比 / 余量，文案区分 Api vs Heuristic 等 provenance。
- 可选：接近 `window - reserve` 时弱提示，但仍 **MUST NOT** 用独立百分比阈值触发压缩。
- design playground / `design/footer.md` 更新。
- **明确非目标**：恢复 `compaction_threshold` 配置；用百分比替换 reserve 触发。

## Capabilities

| Capability | 变更 |
|---|---|
| `app-tui-chrome` / footer | 派生 % 展示 |
| （只读）`domain-compaction` | 消费同一估计入口，不改触发 |

## Impact

- **破坏性**：低（纯展示）。
- **默认体验**：更易读占用；触发心智仍为 reserve。
- **非目标**：本草案阶段不实现。

## Depends / 后续

```text
c1630 ──► c1680 (本，后置)
```

建议在 c1640 之后再 promote，以便「auto 刚发生」与百分比刷新一起验收。

## Open Questions

- 显示「已用 %」还是「距 reserve 余量」？promote 时再拍。
- 是否与 cache hit 叙事同屏（见 roadmap 上下文缓存）——默认本 change 不做 cache。

## Ethics

- risk_level: low
- prohibited_actions: 把派生 % 写回触发配置；把 Heuristic 标成官方用量
- required_evidence: harness 快照/断言 footer 与估计同源；触发仍走 reserve
- refusal_contract: 不在未归档 c1630 前 apply 本 change
- escalation_policy: 视觉争议走 design playground，不改触发合约
