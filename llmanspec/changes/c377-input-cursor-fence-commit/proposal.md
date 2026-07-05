---
change_id: c377-input-cursor-fence-commit
title: 左右光标移动 + 围栏感知段落 commit（流式代码块高亮）
status: proposed
priority: 377
depends_on: []
author: agent
---

# c377-input-cursor-fence-commit

## Why

回退 c375/c376 后 TUI 可用，但三个能力缺失（用户确认必须）：
1. **对话框不能左右移动光标**——input MVP 单行只能在末尾输入（c340 设计限制），用户无法编辑中间内容
2. **流式期间代码块无高亮**——逐行 commit 导致每行单独 render_markdown，围栏上下文丢失

## What Changes

### 1. 左右键移动光标（独立、低风险）

TuiApp 加 `input_cursor: usize`（字节偏移）。push_char 在 cursor 处插入；backspace 删 cursor 前；新增 cursor_left/right/home/end。input.rs 加 Left/Right/Home/End 键处理。input_prompt.rs cursor_x 接 cursor 位置算 `input[..cursor]` 显示宽度。

### 2. 围栏感知段落 commit（流式高亮关键）

StreamBuffer 加围栏状态机（~20 行，借鉴 codex FenceTracker）：围栏外按空行切段落；围栏内累积到闭 ``` 整体 commit。每个段落作为单个 AssistantText 经 render_markdown（围栏完整，高亮触发）。

不改 mutable 显示（仍纯文本尾部）、不改 TAIL_HEIGHT（仍 6）、不重渲整段（区别于失败的 c376）。

## Capabilities

- `app-tui`（修改）：新增 tui76（input cursor 左右移动）、tui77（围栏感知段落 commit）。

## Impact

- `src/app/tui/app.rs`（TuiApp cursor + StreamBuffer 围栏）、`input.rs`（左右键）、`input_prompt.rs`（cursor_x）、`tail.rs`（传 cursor）
- 风险：低-中。cursor 字节偏移边界；围栏检测对缩进/~~~ 不处理（MVP）。

## 调研证据

- codex FenceTracker（table_detect.rs:149-195）：~50 行围栏状态机，advance(line)+kind()。
- vendored parser（parser.rs:106-121）：代码块内空行是内容（不切段）；257-262 未闭合围栏 EOF 兜底 emit code_block。
- spike（c376）：完整围栏正确高亮（fn/let 均 colored）。
