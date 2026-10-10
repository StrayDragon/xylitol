---
depends_on: []
---

# 左右边栏（占用列 vs 抽屉）

## Why

opencode 宽屏 42 列占用、窄屏 overlay drawer；crush 30 列占用（窄屏隐藏）。pi TUI **无**边栏（槽替换 editor）——与 xylitol 同构。ratatui 侧栏是 `Layout::horizontal([Length(30), Fill(1)])` 出两个 `Rect`；xylitol 是垂直 `Vec<String>` + 底 `dock_rows` 全宽，**没有** Constraint 求解器，也不该为边栏引入（D12、wrap 缓存、DESIGN 无双栏）。

`DESIGN.md` 硬规则 2：**无双栏**。会话树 / 模型 / resume = **槽**；任务 = **待办栏**。本票默认 **park**。

若将来做 overlay 抽屉，鼠标会被今日「AO 选区先于 overlay」吃掉，必须先有 [c2858](../c2858-add-package-tui-pointer-hit-pipeline/proposal.md) 的 capture。占用列则是 wrap 宽问题，不靠指针管线，但会打爆 AO 行缓存（D19），**不要做宽度动画**。

## What Changes（仅当明确改 DESIGN）

- 占用列：transcript+dock 变窄；禁止动画占用宽。
- overlay 抽屉：右缘 clip；`HitSpace::Overlay` 先于选区（依赖 c2858）。与通知栈抢右缘、与槽 Esc 冲突必须写清。
- 词表：不得叫待办栏或槽。

## 非目标（默认）

- **不实施。** 元数据继续 `/session-resume` 与会话树槽。
- 不抄 ratatui `Constraint` 当默认布局，不抄 crush 30 列。
- 不引入阴影/卡片 backdrop。
- 不把垂直 tabs 冒充边栏（c2880）。

## Capabilities（仅 promote 后）

- `app-tui-fixed-zone`（先改 DESIGN）
- overlay 鼠标：`c2858` 已提供 capture，不必新 `package-tui-layout`

## Impact

- 占用列：Diff SBS / 待办栏 100 列阈值更少命中。
- 抽屉：盖住通知栈右上与 transcript 命中。

## Open Questions

1. **是否修订「无双栏」？**（推荐：**否**，park。）
2. 若要做：占用列 vs overlay 抽屉？（推荐仅会话详情抽屉，且 `depends_on: [c2858]`。）

## Further Notes

- [research/sidebar-peers.md](./research/sidebar-peers.md)
- ratatui split 说明见 [c2858 research](../c2858-add-package-tui-pointer-hit-pipeline/research/pointer-pipeline.md)
