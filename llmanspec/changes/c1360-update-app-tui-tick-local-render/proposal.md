---
id: c1360-update-app-tui-tick-local-render
stage: draft
depends_on:
  - c1350-update-edit-diff-tui-cap
---

# Proposal: Tick 局部刷新（spinner 不整树重算）

## Why

长会话 / 大 scrollback 时 busy spinner 变卡：host 每 ~16ms `Tick` **无条件** `request_render`，且 `UiRoot::render` 每次重跑整段 `render_scrollback`。差异写入终端只改 status 行，但 CPU 仍扫数千行 → spinner 卡顿。

## What Changes

1. **Host Tick 门闩**：仅当 `idle_tick()` 返回 dirty **或** 显式 `paint_dirty`（bang chunk 等）时才 `request_render`；idle 无动画 MUST NOT 每 tick 整帧。
2. **bang chunk**：`append_bash_chunk` 只标 `paint_dirty`，由 Tick 合并绘制（对齐 ath7「chunk 标 dirty，Tick try_render」）。
3. **UiRoot 上区缓存**：loaded-resources + scrollback + queue 在仅 status Loader 推进时复用上次行缓冲；`apply_ui_model` / fold / theme / width 变化时失效。

## Capabilities

- `app-tui-host`（Tick / paint_dirty）
- 可选触及 `app-tui-chrome`（status 形态不变）

## Impact

- busy 时 spinner 仍按 Loader interval 推进；大 transcript 下 Tick CPU 下降。
- 非目标：引擎 `previous_lines` 热缓冲封顶（→ c1370）；改 Loader 帧率语义。
