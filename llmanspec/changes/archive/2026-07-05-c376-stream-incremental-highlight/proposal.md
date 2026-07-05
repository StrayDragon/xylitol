---
change_id: c376-stream-incremental-highlight
title: 流式期间及时上行 + 全程高亮（行差量 commit）
status: proposed
priority: 376
depends_on: []
author: agent
---

# c376-stream-incremental-highlight

## Why

c375 延迟到 TurnEnd 整段 commit 导致流式期间不换行、内容挤一起，用户体验差。用户要求 codex 式行为：流式期间每行及时上行进 scrollback + 全程高亮。

spike 验证：vendored parser 即使代码块围栏未闭合（流式中）也正确高亮——所以每次 TextDelta 全量重渲，已稳定的行（换行边界后）带高亮 commit，未换行尾部留 mutable，能实现「及时上行 + 全程高亮」。

## What Changes

1. 每次 TextDelta 全量 `render_markdown(finalized)`，按 source 最后换行边界判稳定，commit 新增的稳定行（带高亮）
2. mutable 区显示渲染后的尾部行（`Vec<Line>`，带高亮，非纯文本）
3. TurnEnd commit 剩余全部行
4. 修订 c375 的 tui75（从「延迟到 TurnEnd」改成「换行边界稳定 + 行差量 commit」）

## Capabilities

- `app-tui`（修改）：修订 tui75。

## Impact

- `src/app/tui/app.rs`（handle_xy_event：全量重渲 + 行差量）、`mutable_line.rs`（接收 `&[Line]`）、`tail.rs`
- 风险：中。流式渲染架构再改。性能（每次全量重渲，syntect 毫秒级，可接受）。
