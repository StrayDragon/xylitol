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

> **流程**：仅 `purpose-draft`；**后置**——禁止与 c1630 apply 缠做；禁止提前 full。
> **建议**：c1630 **且** c1640 归档后再 promote（便于「刚 auto」与 footer 刷新同验）。

## Why

触发 SSOT 已是 reserve（c1630）。Footer 仍可能要 **派生**「约 x%」读数；该读数 **绝不是** 触发闸。

## 需求锁定

### R1 — 派生展示（已决精神）

- 若展示占用比：MUST 为 `tokens / context_window`（或文档化的等价派生），数据 MUST 与 footer 同源估计（c1/c16）。
- MUST 诚实标注 provenance（Api / Heuristic 等）；MUST NOT 把 Heuristic 标成官方用量。

### R2 — 与触发隔离（硬约束）

- MUST NOT 恢复 `compaction_threshold` 或任何百分比触发配置。
- MUST NOT 用「显示用 %」改变 `should_compact` / reserve 公式。
- 可选「接近 `window - reserve`」弱提示：仅 UI，不改触发。

### R3 — 非目标

| 禁止 |
|---|
| cache hit 同屏叙事（另见 roadmap） |
| 改 domain 触发合约 |
| 在未归档 c1630 前 apply |

## 验收锚点（promote 时）

| id | Then |
|---|---|
| derived-only | footer % 与同源估计一致 |
| no-threshold-config | 配置/schema 无百分比闸字段 |
| trigger-unchanged | reserve 公式行为与 c1630 一致 |

## Open Questions（promote 时再拍；草案不锁死）

- 展示「已用 %」还是「距 reserve 余量」或两者？
- 视觉：footer vs compact-status 行——走 `design/footer.md` / playground。

## Capabilities

`app-tui-chrome` / footer；（只读）domain-compaction

## Ethics

- risk_level: low
- prohibited_actions: % 写回触发；伪造 Api 用量；提前 full / 与 c1630 缠做
- required_evidence: harness；触发仍 reserve
- refusal_contract: 未归档 c1630 不 apply
- escalation_policy: 视觉争议只进 design，不改触发
