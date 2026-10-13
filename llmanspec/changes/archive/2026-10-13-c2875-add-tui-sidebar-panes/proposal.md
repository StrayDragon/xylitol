---
title: 左右边栏（占用列 vs 抽屉）
depends_on: []
branch: sdd/c2875-add-tui-sidebar-panes
base_branch: main
base_sha: af742d55647f6fad8a3b1fbcc8515d21863ce50b
---

# 左右边栏（占用列 vs 抽屉）

## Why

opencode 宽屏 42 列占用、窄屏 overlay drawer；crush 30 列占用（窄屏隐藏）。pi TUI **无**边栏（槽替换 editor）——与 xylitol 同构。ratatui 侧栏是 `Layout::horizontal([Length(30), Fill(1)])` 出两个 `Rect`；xylitol 是垂直 `Vec<String>` + 底 `dock_rows` 全宽，**没有** Constraint 求解器，也不该为边栏引入（D12、wrap 缓存、DESIGN 无双栏）。

`DESIGN.md` 硬规则 2：**无双栏**。会话树 / 模型 / resume = **槽**；任务 = **待办栏**。本票定案（2026-10-13，用户追认）：不修订「无双栏」，实施 overlay 抽屉形态（现标准词「覆盖弹出框」）。

若将来做 overlay 抽屉，鼠标会被今日「AO 选区先于 overlay」吃掉，必须先有 [c2858](../c2858-add-package-tui-pointer-hit-pipeline/proposal.md) 的 capture。占用列则是 wrap 宽问题，不靠指针管线，但会打爆 AO 行缓存（D19），**不要做宽度动画**。

## What Changes

- overlay 抽屉（已实施）：右缘 clip；本期不注册指针命中。与通知栈 z 序（通知栈在上）与槽 Esc 分级（忙碌透传中止）在 design 定案。
- 占用列（未由本票实施）：后由 c2905 侧栏承担。
- 词表：不得叫待办栏或槽。

## 非目标

- 元数据继续 `/session-resume` 与会话树槽（抽屉只读展示，不替代入口）。
- 不抄 ratatui `Constraint` 当默认布局，不抄 crush 30 列。
- 不引入阴影/卡片 backdrop。
- 不把垂直 tabs 冒充边栏（c2880）。

## Capabilities（仅 promote 后）

- `app-tui-fixed-zone`（先改 DESIGN）
- overlay 鼠标：`c2858` 已提供 capture，不必新 `package-tui-layout`

## Impact

- 占用列：Diff SBS / 待办栏 100 列阈值更少命中。
- 抽屉：盖住通知栈右上与 transcript 命中。

## Open Questions（已定案）

1. **是否修订「无双栏」？** 已定案：**否**（瞬态覆盖弹出框非常驻分栏，DESIGN 硬规则 2 增例外注记）。
2. 占用列 vs overlay 抽屉？已定案：仅 overlay 抽屉（`c2858` 指针管线在库）；占用列后由 c2905 侧栏承担。

## Further Notes

- [research/sidebar-peers.md](./research/sidebar-peers.md)
- ratatui split 说明见 [c2858 research](../c2858-add-package-tui-pointer-hit-pipeline/research/pointer-pipeline.md)
