---
depends_on: []
title: 修复停靠区悬停、chip 取色与推理摘要通道
needs_specs_change: true
branch: sdd/c3030-fix-dock-hover-and-thinking-stream
base_branch: main
base_sha: ee8b4d08e7aba116010cbca4337117cb65897bfd
---

## Why

四个真机回归叠加，破坏鼠标优先的固定区体验与思考块可见性。

1. 待办栏摘要行可点但悬停无高亮，用户无法感知可点。队列条同样无悬停反馈。
2. `[详情] [侧栏] [重置折叠]` chip 行写死 muted，不跟随 thinking 档位边框色，与本轮之前的编辑区边框变色契约不一致。
3. 待办栏摘要行紧贴流式回复文字，打字机效果时版面拥挤。
4. 兼容 Responses 端在带 reasoning.effort 时只发 `response.reasoning_summary_text.delta`，而 provider 只映射 `response.reasoning_text.delta`，于是 xhigh 档思考块整块消失；off 档因不带 effort 反而回落到原始推理通道，出现「off 有、xhigh 没有」的反差。

## What Changes

- 引擎：`apply_hover_tint` 改为分 transcript / dock 两路映射，Dock 与越界 Chrome 命中都能高亮 dock 行；新增 `HoverRowSource`。
- 产品端：为队列条与待办栏摘要行登记 hover 命中区（不捕获指针），接入 `sync_pointer_hits`。
- chip 行取色改走 `operation_zone_paint()`：分隔虚线与未激活 chip 跟随 thinking 档位边框色，激活 chip 仍 accent 反色，bash 模式跟随 bash 强调色。
- 待办栏在摘要行上方加一行空白分隔；该行计入 Fixed-Zone Footprint，总可见行仍受 `min(6, term_rows/4)` 上限约束。
- provider：`response.reasoning_summary_text.delta` 与 `response.reasoning_text.delta` 同样映射为 `XyChunk::ThinkingDelta`。

## Further Notes

上游已实测确认不是网关 bug：同一网关在 `reasoning.summary="auto"` 时只发摘要通道，省略 summary 时发原始推理通道（见 `research/upstream-reasoning-channels.md`）。
