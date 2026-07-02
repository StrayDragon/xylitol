# c365-add-streaming-mutable-tail — Tasks

> 流式 mutable-last-line（buffer 路线 + 组件化）：文字增量生长在 tail 顶行（buffer 内，透明 bg 融进 scrollback），换行即 insert_before 固化。Thinking 是正文（灰色），面板固定 3 行。渲染层拆 6 个可复用 widget。

## 0. StreamBuffer + 事件处理

- [x] `StreamBuffer { buffer, committed_len }` → `push` / `drain_complete_lines` / `pending_tail` / `finalize`
- [x] `handle_xy_event` 返回 `Vec<RenderedLine>`（finalized 行）
- [x] `ThinkingDelta` → thinking_buf 流式 + commit `ThinkingText`（灰色）
- [x] 首个 `TextDelta` → flush thinking_buf + 切 `thinking_phase=false`
- [x] `TextDelta` → stream_buf 流式 + commit `AssistantText`
- [x] `TurnEnd` → flush 双 buf 残留
- [x] `MutableKind` enum（Thinking/Text/Tool）→ `pending_tail() -> Option<(&str, MutableKind)>`
- [x] `tool_status` 工具状态标

## 1. 删 escape 路线

- [x] 删 `raw_render.rs`
- [x] `terminal.rs` 删 `redraw_mutable` / `clear_mutable` / `commit_line_raw`
- [x] `commit_to_scrollback` 改吃 `&[RenderedLine]`

## 2. 组件化

- [x] `transcript_line.rs` — `RenderedLine → Buffer`（含 `ThinkingText` 灰）
- [x] `mutable_line.rs` — 未换行尾部，caller 传 `Style`，透明 bg，top-anchored
- [x] `input_prompt.rs` — 输入框，无 `❯`，CJK 光标
- [x] `bottom_panel.rs` — bordered + panel_bg，只含 InputPrompt，固定 3 行
- [x] `tail.rs` — 组合 MutableLine（顶）+ BottomPanel（底）
- [x] `spinner.rs` — 单 glyph spinner（底层可复用，当前未接线）

## 3. harness

- [x] `TranscriptLine` — ASCII / CJK / wrap
- [x] `MutableLine` — wrap 多行 + 透明 bg
- [x] `BottomPanel` — border+input / panel_bg 填充 / height=3
- [x] `InputPrompt` — 无 `❯` + CJK 光标
- [x] `Tail` — idle 面板 3 行固定底部 / thinking placeholder / thinking 内容 / text 内容 / 透明 vs panel_bg / TurnEnd 无残留
- [x] `Spinner` — glyph 渲染
- [x] StreamBuffer 单测

## 4. 校验

- [x] `cargo test --features tui` 全绿
- [x] `cargo test --test bdd` 无回退
- [x] `just qa` 绿
- [x] `llman sdd validate` 通过

真终端冒烟（用户验证）：`cargo run --features tui`
