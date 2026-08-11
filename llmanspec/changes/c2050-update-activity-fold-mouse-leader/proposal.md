---
depends_on:
  - c1760-add-tui-activity-fold
  - c2020-add-package-tui-mouse-input
  - c2030-add-tui-fold-leader-digit-toggle
  - c2040-add-tui-mouse-click-fold-triangle
---

# 多级 ActivityFold 适配鼠标 / Leader / 块级覆盖

> **一句话**：让 `c1760` 的段级 L2/L3 与 L1 块级定点折叠、鼠标点击、leader 编号共用一套目标模型与性能边界。

## Why

`c1760` 已设计多级折叠（L0–L3）与 `expandNearest`/`collapseNearest`，并**显式后置鼠标（M4）**、保留行距缝。并行草案 `c2020`–`c2040` 补齐引擎鼠标、L1 leader 定点、点击三角。若不在多级折叠落地时统一：

- 段摘要行无法被编号/点击；
- L2/L3 与 L1 全局/覆盖态冲突（`c1760` 深挖 A）；
- hit-test / 编号只认 entry 不认 segment，Web 同源动作对不齐。

本 change **不重复实现** `c1760` 主交付，而是规定/落地「适配层」：同一套 `fold.toggle(target)` / leader 可见目标集在 L1 块与 L2/L3 段上的规则，以及性能约束。

## What Changes

1. **目标模型统一**：`FoldTarget` = Entry(id) | Segment(id) |（可选）Thinking 全局仍走 Ctrl+T，或纳入同一平面（propose 钉）。
2. **Leader 可见集**：L2/L3 摘要行可编号；L0/L1 细账中可折叠头也可编号；**同屏混合**时编号仍「近输入优先」，总数封顶 10。
3. **点击**：摘要行标记 → `activity` 升/降一级或 toggle 段显隐（与 `expandNearest` 单目标版对齐，避免与栈键混淆——propose 钉「点标记 = toggle 该段一级」）。
4. **分层规则继承 `c1760` 深挖 A**：段处于 L2/L3 时，L1 全局/覆盖不穿透该段外观；leader 数字若指向被遮挡块 → 跳过或先升段（钉一种）。
5. **性能**：
   - hit / 编号表只覆盖**当前视口**；
   - 与 `ScrollbackPaintCache` / 未来 `c1505` 切片同代；
   - toggle 段级 MUST 失效该段行范围，禁止全历史重 Markdown；
   - 与 `c1370` 热缓冲：少画（L2/L3）降低 `previous_lines` 压力，但不替代封顶。
6. **跨面**：动作语义进 Web 同源板；物理点击为 TUI/Web 增强，不写进公共 MUST 和弦。

## Capabilities（意向）

- `app-tui-transcript` — 段/块统一目标
- 键位 / 输入 — leader 扩展
- 文档 — `Web与TUI同源.md` M1b 兑现时同步

## Impact

| 相关 change | 关系 |
|---|---|
| `c1760` | 主交付多级折叠；本 change 适配交互增强 |
| `c2020`–`c2040` | 硬依赖；提供引擎与 L1 定点 |
| `c1505` / `c1370` / `c1535` | 性能并列；适配 MUST 不恶化端到端；少画可缓解但非替代 |

## 依赖与排序（frontmatter SSOT）

```text
Wave 0（无互依赖，可并行深挖）
  c2020 鼠标地基
  c2030 L1 leader + per-block
  c1760 多级折叠 MVP          ← depends c1755✓

Wave 1
  c2040 点击三角              ← depends c2020 + c2030

Wave 2
  c2050（本 change）          ← depends c1760 + c2020 + c2030 + c2040
```

若 `c1760` propose 时交互增强已齐，可将本草案 **吸收进 c1760 末 tasks** 后 archive 本 id（docs-only）；否则保持独立闭环。

## Out of scope

- 重做 `c1760` L2/L3 文案与自动降级策略
- 实现 `c1505`/`c1370`/`c1535`
- 块焦点常驻 EditorSlot

## 验证（自动化 + 人类）

| 层 | 自动化 | 人类 |
|---|---|---|
| Harness | 同屏 L1 块 + L2 摘要：leader 编号与点击各打中正确 `FoldTarget`；L2 段内 L1 不穿透（深挖 A） | 旧 turn 摘要行点标记升/降一级 |
| Paint / 性能 | 混合屏 toggle 不回退 ath25 miss 上界；hit 表构建 O(可见)（单测或计数） | 长会话点摘要无整屏闪 |
| 跨面文档 | Web 同源板动作 id 与 TUI 一致（文档闸 / 审阅） | — |

**人类最短路径**：c1760 落地后造 L2 段 → leader 见摘要编号 → 点击标记 → 与 `Alt+Shift+E` 栈行为不矛盾。

依赖差分结论：见 [`../c2020-add-package-tui-mouse-input/research/diff-engine-mouse-fit.md`](../c2020-add-package-tui-mouse-input/research/diff-engine-mouse-fit.md)。

## Ethics

- risk_level: medium
- prohibited_actions: L2/L3 与 L1 语义互相吞掉；伪造 Worked for；因 hit 表导致每帧全量 invalidate
- required_evidence: 混合 L1+L2 屏上 leader/点击各打中正确目标；微基准或 miss 计数不回退 ath25
- escalation_policy: 与 `c1760` 已拍板和弦冲突时升级确认

## Further Notes

- 调研：[`research/multilevel-fold-interaction-matrix.md`](./research/multilevel-fold-interaction-matrix.md)
- 主案：[`../c1760-add-tui-activity-fold/proposal.md`](../c1760-add-tui-activity-fold/proposal.md)
- 性能并列：`c1505` / `c1370` / `c1535`
