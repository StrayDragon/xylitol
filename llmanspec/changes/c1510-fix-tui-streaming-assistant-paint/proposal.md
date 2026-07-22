---
change_id: c1510-fix-tui-streaming-assistant-paint
title: TUI 流式 assistant Markdown 增量绘制
status: in-progress
priority: 1510
depends_on: []
author: agent
branch: feature/c1510-fix-tui-streaming-assistant-paint
base_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
checkpointed: false
---

# c1510-fix-tui-streaming-assistant-paint

## Why

c1500 已让**已提交** entry 在 TextDelta 下 cache hit。流式路径仍对整段 `streaming_assistant + "…"` 每帧 `Markdown::new` 全量解析——长回复流式时 spinner/主环仍可能被尾部 Markdown 拖住。

Ctrl+O 高度缩略只作用于 tool/bash/write/diff（`ExpandableOutput`），与 assistant Markdown **正交**；本变更 MUST NOT 改动该语义。

## What Changes

1. **Streaming assistant 增量 paint**：对 `streaming_scrollback_tails` 的 assistant 分支，按「稳定 Markdown 前缀 + 仅重绘后缀」复用已渲染行（对标 Codex `StreamingRender` 思路，落在产品 `scrollback`）。
2. **Harness**：长流式正文多次 TextDelta → 全量 Markdown 解析次数有上界（随稳定块增长，不随每个 delta 线性）。
3. **回归**：既有 Ctrl+O / att16 / expandable harness 保持绿灯；assistant 可见内容与全量解析一致（允许 `…` 尾标）。

## Out of scope

- c1505 viewport slice / 终端 scrollback 前缀冻结
- 改 `packages/xylitol-tui` Markdown 引擎内核（除非产品侧不够用再最小下沉）
- thinking 流式增量（可后置；本 change 仅 assistant）
- 改 Alt+E / Ctrl+O / 硬截断合约

## Capabilities

- `app-tui-host`（新 ath26）
- 验证：`src/app/tui/` harness；Ctrl+O 既有测

## Impact

- 长 assistant 流式时主环更稳；工具块 Ctrl+O 行为不变

## Ethics

- risk_level: low
- prohibited_actions: 为增量绘制削弱 Ctrl+O / 硬截断；静默改 expandable 合约
