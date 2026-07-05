# c377 — Tasks

## 改动 1：左右光标

- [x] 1.1 app.rs：TuiApp 加 `input_cursor: usize`。push_char/backspace 改在 cursor 处操作。新增 cursor_left/right/home_end + input_cursor()。take_input 重置。
- [x] 1.2 input.rs：加 Left/Right/Home/End 键码处理。
- [x] 1.3 input_prompt.rs：cursor_x 接 cursor_byte 参数（算 input[..cursor] 宽度）。tail.rs input_cursor_position 传 app.input_cursor()。
- [x] 1.4 测试：cursor 插入/删除/移动；CJK cursor 边界。

## 改动 3：围栏感知段落 commit

- [x] 3.1 app.rs StreamBuffer：加 `in_fence: bool`。drain_complete_lines → drain_complete_paragraphs（围栏外空行切段；围栏内累积到闭合）。
- [x] 3.2 handle_xy_event TextDelta/ThinkingDelta：段落包成单个 AssistantText/ThinkingText。
- [x] 3.3 测试：代码块流式 commit 后高亮（PreRendered/AssistantText 含完整围栏）。

## 阶段 4：校验 + 归档

- [x] 4.1 `just qa`（all-features）+ arch_guard + strict validate。
- [x] 4.2 归档 c377。
