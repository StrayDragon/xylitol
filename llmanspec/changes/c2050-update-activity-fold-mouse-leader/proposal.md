---
depends_on:
  - c1760-add-tui-activity-fold
  - c2020-add-package-tui-mouse-input
  - c2040-add-tui-mouse-click-fold-triangle
  - c2070-add-package-tui-dual-interaction-modes
---

# 多级 ActivityFold 适配鼠标 / 块级覆盖

> **状态**：Designed / pre-start（**未分支**）。前置 [`c2070`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/proposal.md) + `c1760` + [`c2040`✓](../archive/2026-08-12-c2040-add-tui-mouse-click-fold-triangle/)。段级点击依赖 **c2020** 管道 + Mode B。
>
> **Start readiness**：`blocked_on_c1760`（`ready_for_start=false`）。详见 [`design.md`](./design.md) / [`tasks.md`](./tasks.md)。
>
> **2026-08-12**：c2040 Q6=B → [`c2045`](../c2045-add-tui-fold-target-remaining/proposal.md)。**默认路径 A**：段命中被 c2045 吸收 → 本票 **docs-only archive**（无 start/Specs/代码）。**Fallback B**：仅当 c2045 design 明文排除 Segment。禁止两套段命中 API。

> **一句话**：让 `c1760` 的段级 L2/L3 与 L1 块级 **鼠标定点**、全局键盘折叠共用一套目标模型与性能边界（**无** keyboard leader 编号）；实现默认归 c2045，本票钉语义矩阵。

## Why

`c1760` 已设计多级折叠（L0–L3）与 `expandNearest`/`collapseNearest`，并**显式后置鼠标（M4）**、保留行距缝。`c2020`/`c2040` 补齐引擎鼠标与 L1 **点击三角 + per-block 覆盖**（原 `c2030` leader 已废弃）。若不在多级折叠落地时统一：

- 段摘要行无法被点击；
- L2/L3 与 L1 全局/覆盖态冲突（`c1760` 深挖 A）；
- hit-test 只认 entry 不认 segment，Web 同源动作对不齐。

本 change **不重复实现** `c1760` 主交付，而是规定/落地「适配层」：同一套 `fold.toggle(target)` 在 L1 块与 L2/L3 段上的规则，以及性能约束。

## What Changes

> 实现落点：默认 **c2045 吸收**；下列为本票合约意图（路径 B 时本票落地）。

1. **目标模型统一**：在 c2040 `FoldTarget` 上扩展 `Segment(id)`（广义总装归 c2045）；Thinking 点折已在 c2040，摘要行不点 Thinking。
2. **点击**：摘要行**三角列** → **对该段一级**升/降（与单目标版栈对称；非整行；非纯显隐 bool）。
3. **分层规则继承 `c1760` 深挖 A**：段处于 L2/L3 时，L1 全局/覆盖不穿透该段外观。
4. **性能**：
   - hit 表只覆盖**当前视口可见**头/摘要；
   - 与 `ScrollbackPaintCache` / 未来 `c1505` 切片同代；
   - toggle 段级 MUST 失效该段行范围，禁止全历史重 Markdown；
   - 与 `c1370` 热缓冲：少画（L2/L3）降低压力，但不替代封顶。
5. **跨面**：动作语义进 Web 同源板；物理点击为 TUI/Web 增强，不写进公共 MUST 和弦。

## Capabilities（意向）

- `app-tui-transcript` — 段/块统一目标
- 文档 — `Web与TUI同源.md` M1b 兑现时同步

## Impact

| 相关 change | 关系 |
|---|---|
| `c1760` | 主交付多级折叠；**硬阻塞**本票 start |
| `c2020` / `c2040` ✓ | Mouse + L1 三角 / 覆盖 / `FoldHitTable` |
| `c2045` | **默认吸收** Segment 命中 → 本票 docs-only |
| `c1505` / `c1370` / `c1535` | 性能并列；适配 MUST 不恶化端到端 |

## 依赖与排序（frontmatter SSOT）

```text
Wave 0 ✓  c2020 · c2070/c2071
Wave 1 ✓  c2040 L1 三角
Wave 1'   c1760 多级折叠 MVP          ← 硬阻塞本票
Wave 2    c2045 FoldTarget 总装（默认含 Segment）
          c2050：默认 docs-only；fallback 才独立 apply
```

历史备选（已降级）：若 c1760 末段自行做完段点击，亦可 docs-only archive——但 **现行默认吸收方是 c2045**（Q6=B），避免与「任意块」故事分叉。

## Out of scope

- 重做 `c1760` L2/L3 文案与自动降级策略
- 实现 `c1505`/`c1370`/`c1535`
- 复活 keyboard fold-leader / 数字编号（已废弃）
- 块焦点常驻 EditorSlot

## 验证（自动化 + 人类）

| 层 | 自动化 | 人类 |
|---|---|---|
| Harness | 同屏 L1 块 + L2 摘要：点击各打中正确 `FoldTarget`；L2 段内 L1 不穿透（深挖 A） | 旧 turn 摘要行点标记升/降一级 |
| Paint / 性能 | 混合屏 toggle 不回退 ath25 miss 上界；hit 表 O(可见) | 长会话点摘要无整屏闪 |
| 跨面文档 | Web 同源板动作 id 与 TUI 一致 | — |

**人类最短路径**：c1760 落地后造 L2 段 → 点摘要标记 → 与 `Alt+Shift+E` 栈行为不矛盾。

## Ethics

- risk_level: medium
- prohibited_actions: L2/L3 与 L1 语义互相吞掉；伪造 Worked for；因 hit 表导致每帧全量 invalidate
- required_evidence: 混合 L1+L2 屏上点击打中正确目标；miss 计数不回退 ath25
- escalation_policy: 与 `c1760` 已拍板和弦冲突时升级确认

## Open Questions（已钉）

见 [`design.md`](./design.md)；摘要：点标记=段一级；Thinking 摘要不可点；默认被 c2045 吸收→docs-only；`blocked_on_c1760`。

## Further Notes

- 规划壳：[`design.md`](./design.md) · [`tasks.md`](./tasks.md)
- 调研：[`research/multilevel-fold-interaction-matrix.md`](./research/multilevel-fold-interaction-matrix.md)
- 主案：[`../c1760-add-tui-activity-fold/proposal.md`](../c1760-add-tui-activity-fold/proposal.md)
- 吸收方：[`../c2045-add-tui-fold-target-remaining/proposal.md`](../c2045-add-tui-fold-target-remaining/proposal.md)
- L1 样板：[`../archive/2026-08-12-c2040-add-tui-mouse-click-fold-triangle/`](../archive/2026-08-12-c2040-add-tui-mouse-click-fold-triangle/)
- **2026-08-11**：`c2030` fold-leader 废弃；本 change 不再 depends 编号模式
