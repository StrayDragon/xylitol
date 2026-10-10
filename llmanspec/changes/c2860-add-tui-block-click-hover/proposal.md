---
depends_on:
  - c2858-add-package-tui-pointer-hit-pipeline
---

# 整块可点折叠与悬停可发现性

## Why

单击折叠在 ApplicationOwned 已经接好，但可发现性差：r1352 / att22 强制三角列-only。opencode / crush 整块点、pi `MouseRegion` 整块 result 更易用。roadmap M4 与 `designing/tui-lab/modules/hover-highlight/` 未晋级。

引擎层做不到「Up 才 toggle / Moved 才 hover」——见前置 [c2858](../c2858-add-package-tui-pointer-hit-pipeline/proposal.md)。本票是 **产品几何与合约**：扩 hit 矩形、翻 ath33、停丢 Moved、晋级悬停 tint。

ratatui 对照：widget 不收鼠标；app 对 Rect 命中。xylitol 产品应继续 paint 时往 `HitRegionList` `push`（今日 `FoldHitTable` 同拍），不要把 transcript 改成活 Component 树。

## What Changes

- 命中从三角列放宽为 **header 全宽**（三角仍绘）；activity 信封用已有 `row_spans`。正文拖选仍优先。
- 产品相位切到 c2858 的 `UpIfEmptySelection`：有选区或发生过拖则不 toggle（对齐 opencode `getSelectedText()` 守卫）。crush 400ms delay 不作首切片。
- 产品 fan-in **在探针武装时停丢 Moved**（今日 `src/app/tui/mod.rs` ~582 无条件丢，hover 否则到不了引擎）。
- 悬停：可交互块淡 tint（非反色、非 selection invert）；拖选中不显示。id 未变不排帧。
- `FoldTarget` 仍只在产品；经 `id` 接到包 `HitRegionList`。`ExpandableOutput` 不改 click 合约。
- 晋级 hover-highlight lab；不逐工具 hover 信封内部。

## 非目标

- 不把 transcript 改回每块 Component / 不移植 pi `MouseRegion` 当主路径。
- 不改 Inline；不经 `XYLITOL_TUI_MOUSE` 兼得原生选区 + 点选。
- 不把 Alt+E / Ctrl+T / Ctrl+O 改成「仅鼠标」。
- 不在本票扩展引擎相位（那是 c2858）。

## Capabilities

- `app-tui-transcript`（r1352 / att22：三角列-only → header 可点）
- `app-tui-host`（ath33 Down-swallow → Up + 空选；ath28 Moved 丢弃的 carve-out）

## Impact

- harness「点正文不 toggle」改成「点 header toggle / 点正文起选」。
- 空单击今日 Down 已 `dragging=true`、Up 走 clear；切相位后空单击必须 **toggle 且不 OSC52**。

## Open Questions

1. header 全宽（推荐）还是整块 painted rows？
2. 悬停 tint 用哪枚 DESIGN token（lab 已写淡底，非反色）？

## Further Notes

- 产品对照：[research/click-fold-peers.md](./research/click-fold-peers.md)
- 引擎前置：[../c2858-add-package-tui-pointer-hit-pipeline/research/pointer-pipeline.md](../c2858-add-package-tui-pointer-hit-pipeline/research/pointer-pipeline.md)
