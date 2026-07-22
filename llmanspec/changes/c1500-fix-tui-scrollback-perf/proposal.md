---
change_id: c1500-fix-tui-scrollback-perf
title: TUI scrollback 增量绘制 + 可测性能闸（harness / e2e）
status: in-progress
priority: 1500
depends_on: []
author: agent
branch: feature/c1500-fix-tui-scrollback-perf
base_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
checkpointed: false
---

# c1500-fix-tui-scrollback-perf

## Why

1. 工具密集长会话（如 `ubg`）后期 **spinner 卡住**：每次流式/`apply_ui_model` 失效整段 upper 缓存后，`render_scrollback` 对**全部** Assistant 条目重新 `Markdown::new`，成本 O(历史)×帧率。
2. 既有 **ath24** 要求「仅 status Loader 推进时复用上区缓存」，但未约束「committed 条目不得因 streaming tail 变化而整表重 Markdown」。
3. 需要 **harness 可测**（无 TTY）+ **可选 tmux/PTY e2e**，防止回归成「能跑但卡死」。

## What Changes

### P0 — 绘制

- `ScrollbackPaintCache`：按 entry fingerprint 复用已渲染行；streaming tails 仍每帧重画
- `apply_ui_model`：仅当 entries / streaming / queue strip 等上区内容变化时 `bump_upper_gen`；**纯 status 文案变化不失效** upper（对齐 ath24）
- 视觉/语义不变（同一 fold/width/theme 下像素级可接受小差异，但块结构 MUST 不变）

### P1 — 验证

- Harness：构造多 Assistant + 多次 TextDelta，断言 **committed entry 重绘次数** 不随历史线性放大（或 `upper_rebuild` / 专用计数器有上界）
- 产品 `tests/tui_e2e`：可选 `#[ignore]` 用例（tmux 或 pty）在「大 scrollback 模拟」下仍能完成一轮 Fake 对话并 `/exit`（不卡死）；**不**强断言 OS CPU%）

## Out of scope

- Viewport 虚拟化（只画可见行）— 后置
- OTEL session 父子树（c1495 draft）
- c1490 Collector
- 改 Markdown 引擎本身

## Capabilities

- `app-tui-host`（ath24 收紧 / 新 ath）
- `app-tui-transcript`（绘制成本）
- 验证落点：`src/app/tui/` harness + `tests/tui_e2e/`

## Impact

- 长会话 streaming 时 spinner 可持续；idle/busy 主线程负载下降
- 回归由 harness 守住，e2e 作冒烟
