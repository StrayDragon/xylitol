# c377 Design

> 两个独立改动，本文记关键决策。

## 改动 1：cursor 字节偏移

TuiApp 加 `input_cursor: usize`（字节偏移，Rust String::insert/remove 按字节）。
- push_char(c)：`input.insert(cursor, c)`，cursor += c.len_utf8()
- backspace：找 cursor 前一字符边界（char_indices 逆序），remove，cursor 后移
- cursor_left/right：char_indices 找前/后字符边界移动 cursor
- cursor_home/end：cursor=0/input.len()
- take_input：重置 cursor=0

cursor_x(input, cursor_byte)：`UnicodeWidthStr::width(&input[..cursor_byte])`（CJK-aware）。

## 改动 3：围栏感知段落 commit

StreamBuffer 加 `in_fence: bool`。改 drain 逻辑：
- 逐行扫描 buffer（自 committed_len 起）
- 每行检查 `trim().starts_with("```")`：切换 in_fence
- 围栏外遇空行：flush 当前累积为一段落
- 围栏内：累积行（含空行）直到闭 ```，整体 flush
- 未换行的尾部留 pending_tail（mutable 区，不变）

handle_xy_event TextDelta：drain 返回的每个段落包成 `AssistantText(paragraph)`（而非每行一个）。

## 围栏检测简化

- 只处理 ```（3+ backtick），不处理 ~~~
- `trim().starts_with("```")` 即围栏标记（开/闭切换）
- 不处理缩进围栏（>3 空格）和 blockquote 前缀（MVP）

## 为什么不像 c375/c376 失败

| | c375（失败） | c376（失败） | c377（本变更） |
|---|---|---|---|
| commit 时机 | TurnEnd 整段 | 每次 delta 行差量 | 段落边界（空行/围栏闭合） |
| mutable 显示 | 整段（挤一起） | 重渲渲染行 | 纯文本尾部（不变） |
| TAIL_HEIGHT | 终端/2（布局乱） | 终端/2 | 6（不变） |
| 复杂度 | 低但不可用 | 高（重渲+行差量） | 低（~20 行围栏状态） |

## 不在本变更范围

- 多行编辑器、~~~ 围栏、表格 holdback
