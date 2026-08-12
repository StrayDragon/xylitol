---
depends_on:
  - c2040-add-tui-mouse-click-fold-triangle
blocks: []
---

# 广义 FoldTarget：剩余可折块鼠标独立点（可收薄 c2050）

> **状态**：purpose-draft（2026-08-12 自 c2040 深挖 Q6=B 拆出）。**硬前置** [`c2040`](../c2040-add-tui-mouse-click-fold-triangle/proposal.md)。正式化前可修订边界。
>
> **一句话**：在 c2040（Tool/Diff/Ask/Thinking L1 三角 + per-id）之上，统一广义 `FoldTarget`，使**凡可折叠块**均可鼠标独立点开；并评估**收薄/吸收**现 [`c2050`](../c2050-update-activity-fold-mouse-leader/proposal.md) 段级命中模型。

## Why

c2040 只收口 L1 三角四类块。产品终局是「任意可折块都能鼠标定点」，还缺：

- Bash / Compaction / Ctrl+O viewport 等今日无三角或不同态机的块
- L2/L3 activity 段点击（今日草案在 c2050）
- 跨块类型的统一目标模型，避免每加一类再开平行覆盖表

本草案把「剩余面 + FoldTarget 总装」收成一条车道，避免 c2040 膨胀，也避免 c2050 与「任意块」故事分叉两套 API。

## What Changes（意向；propose 时拆）

1. **广义 `FoldTarget`**：Entry(id, kind) | Segment(id) | Viewport(…) 等；与全局键盘 default + overrides 同构。
2. **剩余可折块**：为需要者补三角/命中列（或显式「无三角但可点头」政策）；接同一 hit 表与 paint gen。
3. **与 c2050**：propose 时二选一钉死——(a) 本 change **吸收**段级点击后 archive/docs-only 收薄 c2050；(b) c2050 仅留段语义、命中实现下沉本 change。默认倾向 (a) 若时序上 c1760 已先落地。
4. **非目标（本草案阶段）**：不实现 c2040 范围内四类块；不改差分引擎；不复活 keyboard fold-leader。

## Capabilities（意向）

- `app-tui-transcript` — FoldTarget + 剩余块命中
- 可能触及 `app-tui-host`（Mouse 路由不变则可能 skip）

## Impact

| 相关 | 关系 |
|---|---|
| c2040 | **depends_on**；L1 三角与 per-id 地基 |
| c1760 | 段级内容依赖；时序上 SHOULD 先于本 change 的段部分 |
| c2050 | **可收薄/吸收**（Q6=B）；propose 时钉迁移清单 |

## Open Questions（延后钉）

1. 吸收 c2050 的时机：c1760 归档前还是后？
2. Bash / Compaction 是否补三角，还是整行可点？
3. Ctrl+O viewport 是否纳入「可折」语义，还是另键另命中？

## Ethics

- risk_level: low（草案）
- prohibited_actions: 未归档 c2040 就膨胀实现本草案主路径；静默双写第二套 hit 表

## Further Notes

- 决策来源：c2040 深挖 Q6=B；综合调研见 c2040 `research/synth-pi-zellij-xylitol-fold-hit.md`
