---
change_id: c425-port-editor-core
title: "port editor core — VisualLine+stickyColumn+pageScroll+history+PasteBurst 补齐（pi editor.ts → Rust）"
status: draft
priority: 425
depends_on: []
author: agent
---

# c425-port-editor-core

## Why

`editor.ts`（pi 2333 行 → xy 408 行，缺口 -83%）是最远的核心组件。xy 现有 408 行覆盖了基础编辑基元（insert/delete/undo/yank/history 基础版/jump/paste），但缺失整个**VisualLine 系统**——多行编辑的垂直光标移动、pageScroll、光标快照到原子段边界（paste markers）的逻辑——以及改进的 history 导航和 PasteBurst（c415）的 5 个调用点。没有 VisualLine，move_cursor 只能处理简单行内移动，多行上下移动时无法正确处理 wrap 后的视觉列。

### 证据

pi `editor.ts` 缺失部分：

1. **VisualLine 系统**（~200 行）：
   - `buildVisualLineMap(width)` → `Array<{logicalLine, startCol, length}>`：将逻辑行+wrap 展开为视觉行列表（:1716-1752）
   - `findCurrentVisualLine` / `findVisualLineAt`：光标→视觉行索引映射（:1754-1772）
   - `moveToVisualLine`：带 sticky column 的视觉行间移动，含光标快照逻辑（:1357-1504）
   - `computeVerticalMoveColumn`：sticky column 决策表（P/S/T/U flags）（:1461-1490）

2. **Sticky column**（`preferredVisualCol`, `snappedFromCursorCol`）：垂直移动时保持光标应去的视觉列，共约 20 处引用

3. **pageScroll**（~20 行）：用 `buildVisualLineMap` + `moveToVisualLine` 替代 xy 当前硬编码 `move_cursor(-5,0)`

4. **History 导航改进**（~40 行）：
   - `navigateHistory(direction)` 对齐 pi（setTextInternal + cursorPlacement）
   - `exitHistoryBrowsing()` 分散在 12 个调用点
   - xy 已有 `hist_nav` / `history_draft`，需要重构成对齐 pi 的语义

5. **PasteBurst 接入**（~30 行）：
   - `insert_ch` → `paste_burst.on_plain_char(clock.now())`
   - `newline` / submit path → `paste_burst.should_insert_newline_instead_of_submit(clock.now())`
   - 复位/清空 → `paste_burst.reset()`
   - xy 的 `clock` 需要注入（`Box<dyn Clock>` 或 `Instant` 参数——沿用 c415 验证的 `Instant` 参数注入模式）

6. **渲染改进**：max_vis 用终端行数 30% 计算（对齐 pi），而非硬编码 `5.max(lines.len().min(10))`

7. **Paste-marker-aware 分段**（可选，~40 行）：`segment_with_markers` → `word_wrap_line` 接受 `Option<&[GraphemeData]>`

### 往期验证

- c415 `PasteBurst` 已就位（5 方法 + 4 常量），`Instant` 参数注入模式已验证
- c405 第 1+4 层测试 harness 就位
- `word_wrap_line` 已存在（基本版），只缺 paste-marker-aware

## What Changes

1. **VisualLine struct + build_visual_line_map**：
   - 新增 `VisualLine { logical_line: usize, start_col: usize, len: usize }`
   - `build_visual_line_map(&self, width: usize) -> Vec<VisualLine>`
   - `find_current_visual_line(&self, vls: &[VisualLine]) -> usize`
   - `find_visual_line_at(&self, vls: &[VisualLine], line: usize, col: usize) -> usize`

2. **move_to_visual_line + compute_vertical_move_column**：
   - `move_to_visual_line(vls, from_vl, to_vl)`：带 sticky column 决策表 + 光标快照
   - `compute_vertical_move_column(current_col, src_max, tgt_max) -> usize`：P/S/T/U 决策表

3. **Sticky column 字段**：
   - `Editor` 添加 `preferred_visual_col: Option<usize>`, `snapped_from_cursor_col: Option<usize>`
   - `set_cursor_col` 清除两个字段
   - `move_cursor` 重写为使用 `build_visual_line_map` + `move_to_visual_line`

4. **pageScroll**：
   - 使用 `build_visual_line_map` + 终端行数 30% → 替换当前 `move_cursor(-5,0)` / `move_cursor(5,0)`

5. **History 导航重构**：
   - `navigate_history(direction)` 替代 `hist_nav(dir)`，对齐 pi 的 `setTextInternal` + `cursorPlacement`
   - `exit_history_browsing()` 在 12 个编辑动作入口调用

6. **PasteBurst 接入**：
   - `Editor` 添加 `paste_burst: PasteBurst` 字段
   - `insert_ch` → `paste_burst.on_plain_char(now)`
   - submit/newline 入口 → `paste_burst.should_insert_newline_instead_of_submit(now)` → 插入换行而非提交
   - 非普通字符/非 Enter → `paste_burst.reset()`

7. **渲染 max_vis 计算**：
   - 接受 `terminal_rows: usize`（从 TUI 注入或构造参数），`max_vis = (terminal_rows * 30 / 100).max(5)`

8. **lib.rs 导出** `VisualLine`

9. **测试（c405 第 1+4 层）**：
   - 第 1 层（TuiTestHarness 或直接 `Editor` 单测）：VD-01 基础垂直移动、VD-02 wrap 后上下移动、VD-03 pageScroll、VD-04 sticky column、VD-05 PasteBurst Enter 抑制、VD-06 history 导航保留 draft
   - 第 4 层（proptest）：随机按键序列不变量（光标在界内、undo 还原）

## Capabilities

- `editor`（新建）

## Impact

- `packages/xylitol-tui/src/components/editor.rs`（重写，~1200 行）
- `packages/xylitol-tui/src/lib.rs`（导出 `VisualLine`）
- `packages/xylitol-tui/tests/editor_test.rs`（新建，~20 测试）
- `llmanspec/specs/editor/spec.toon`（新建）

## Non-goals

- **不在 editor 中集成 autocomplete**——那是 c430（editor-autocomplete）的范围
- **不修改 word_wrap_line 为 paste-marker-aware**——可延后到 c430 或独立小变更
- **不改变 Editor 的 pub API 签名破坏性**——`on_submit`/`on_change`/`disable_submit` 保留
