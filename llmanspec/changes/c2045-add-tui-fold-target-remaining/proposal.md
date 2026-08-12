---
depends_on:
- c2040-add-tui-mouse-click-fold-triangle
blocks:
- c2050-update-activity-fold-mouse-leader
branch: sdd/c2045-add-tui-fold-target-remaining
base_sha: 839990764660a6b9f9014d1a88ee2d35aafeccd6
checkpointed: false
---

# 广义 FoldTarget：剩余可折块鼠标独立点（吸收 c2050）

> **状态**：Wave A+B applied（2026-08-13）。硬前置 [`c2040`](../archive/2026-08-12-c2040-add-tui-mouse-click-fold-triangle/) · [`c1760`](../archive/2026-08-12-c1760-add-tui-activity-fold/)（均已归档）。live：`app-tui-transcript` att29–att32；host skip（ath33）。c2050 → docs-only [`archive/2026-08-13-c2050-…`](../archive/2026-08-13-c2050-update-activity-fold-mouse-leader/)。
>
> **一句话**：在 c2040 L1 四类三角之上扩展统一 `FoldTarget`/`FoldHitTable`，补齐 Compaction + Ctrl+O viewport + Segment 摘要标记鼠标点；**docs-only 收薄 c2050**。

## Why

c2040 只收口 Tool/Diff/Ask/Thinking。产品终局是「凡可折块都能鼠标定点」，还缺：

- Compaction（今日无三角、全局 `compaction_expanded`）
- Ctrl+O 输出 viewport（全局 `tools_output_expanded`；Bash 无 L1 折态）
- L2/L3 activity 段点击（原草案在 c2050）

本 change 把「剩余面 + FoldTarget 总装」收成一条车道，避免与 c2050 分叉两套 hit API。

## What Changes

1. **扩展 `FoldTarget`**：保留 c2040 四变体；新增 `Compaction`、`OutputViewport`、`Segment(id)`（见 `design.md` D6）。
2. **Wave A（c2040 后即可）**：
   - Compaction **补三角**；点三角 = flip 全局 `compaction_expanded`（不碰 tools overrides）。
   - Ctrl+O **纳入** `OutputViewport`；点可见 hint 带 = flip `tools_output_expanded`。
   - Bash **不补** L1 三角（无块级折态；走 Viewport）。
3. **Wave B（硬闸 c1760 归档）**：`Segment` 命中；点摘要标记 = 该段一级 toggle；分层继承 c1760 深挖 A。
4. **吸收 c2050（D1）**：段级点击与统一目标叙事由本 change 交付；c2050 **不再**独立 apply → docs-only archive。frontmatter `blocks: c2050`。
5. **非目标**：不重做 c2040 四类；不改差分引擎；不复活 keyboard fold-leader；不实现 c1760 段状态机本身。

## Capabilities

- `app-tui-transcript` — FoldTarget 扩展 + 剩余/段命中
- `app-tui-host` — 仅当 latch/路由缺口需要（预期 skip）

## Impact

| 相关 | 关系 |
|---|---|
| c2040 | **depends_on**（已归档）；L1 三角 + per-id + latch 地基 |
| c1760 | Wave B 软/任务闸；段状态与行距缝的提供者 |
| c2050 | **被吸收**（`blocks`）；完成后 docs-only |

## Open Questions

| # | 原问 | 钉（2026-08-12） |
|---|---|---|
| 1 | 吸收 c2050 时机：c1760 前还是后？ | **吸收策略立即生效**（c2050 勿并行实现）。**Segment 实现**在 c1760 归档后（Wave B）。Wave A 不等 c1760。 |
| 2 | Bash / Compaction 补三角还是整行可点？ | **Bash：不补三角**（走 Viewport）。**Compaction：补三角列**（非整行）。 |
| 3 | Ctrl+O 是否纳入「可折」？ | **纳入** `FoldTarget::OutputViewport`；命中 = hint 带（非三角例外）。 |

无未决项挡 `change start`。细则决策表见 `design.md`。

## Specs landing

| 项 | 状态 |
|---|---|
| Branch binding | ✅ `sdd/c2045-add-tui-fold-target-remaining` |
| `app-tui-transcript` | ✅ att29–att32（`feature: false` unit） |
| `app-tui-host` | ✅ skip（ath33 已覆盖） |
| att23–att28 共存 | ✅ 未改；c1760 cherry-pick 保留 |
| Wave B 实现闸 | c1760 归档（specs 已先写 att31） |
| 应用代码 | ⬜ 本阶段禁止；下一步 apply Wave A |

下一步：`llman-sdd-apply` Wave A；Wave B 等 c1760 归档。

## Ethics

- risk_level: low–medium
- prohibited_actions: 第二套 hit 表；c1760 前假装 Segment；点 Compaction 误清 tools 覆盖
- required_evidence: Wave A harness + c2040 回归；Wave B 混合 L1+L2 屏
- escalation_policy: 改 Alt+E×compaction **键盘**连带 → 另案确认

## Further Notes

- 决策来源：c2040 深挖 Q6=B；综合调研 `../archive/2026-08-12-c2040-add-tui-mouse-click-fold-triangle/research/synth-pi-zellij-xylitol-fold-hit.md`
- c2050 原矩阵：`../c2050-update-activity-fold-mouse-leader/research/multilevel-fold-interaction-matrix.md`（Wave B 吸收时删/改 leader 残留条款）
- 设计 SSOT：[`design.md`](./design.md)；任务：[`tasks.md`](./tasks.md)
