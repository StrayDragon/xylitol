# c365-add-streaming-mutable-tail — Tasks

> 流式 mutable-last-line（buffer 路线 + 组件化）：文字增量生长在 tail 顶行（buffer 内，透明 bg 融进 scrollback），换行即 insert_before 固化进 scrollback，tail 只留 thinking+输入。换行门控 StreamBuffer（codex 极简版）。渲染层拆 5 个可复用 widget，每个 TestBackend 可独立验证。

## 0. 新增 StreamBuffer 换行门控状态机

- [x] 在 `src/app/tui/app.rs` 新增 `StreamBuffer { buffer: String, committed_len: usize }`
- [x] 实现 `push(delta)` / `drain_complete_lines() -> Vec<String>` / `pending_tail() -> &str` / `finalize() -> Option<String>`
- [x] TuiApp 用 StreamBuffer 替代旧的 `pending: String` + `flush_complete_pending`

## 1. TextDelta 改增量 commit（返回 RenderedLine 语义修正）

- [x] `handle_xy_event` 返回 `Vec<RenderedLine>`（**全是 finalized 行**：流式完整行 AssistantText、ToolSummary、Status），不再返回 `Vec<Line>`
- [x] TextDelta 分支：push 后 drain_complete_lines，完整行即时返回为 `RenderedLine::AssistantText`（mod.rs 立即 commit）
- [x] pending_tail **不进返回值**（留给 MutableLine widget 每帧读 app.state 渲染）
- [x] TurnEnd：finalize() 残留尾部作为 `RenderedLine::AssistantText` 返回
- [x] Submit 时 user message 也走 `RenderedLine::UserInput`（和非流式 commit 同路 insert_before）

## 2. 删 escape 路线，回 ratatui-buffer

- [x] 删 `src/app/tui/raw_render.rs`（escape 直写 + DECSTBM）
- [x] `terminal.rs` 删 `redraw_mutable` / `clear_mutable` / `commit_line_raw` 及 `size_and_viewport_top`
- [x] `terminal.rs` `commit_to_scrollback` 改吃 `&[RenderedLine]`（不是 `&[Line]`），内部用 `TranscriptLine` widget 渲染进 insert_before buffer
- [x] `mod.rs` `Msg::Xy` 分支简化：无条件 `commit_to_scrollback(&lines)`（不再按 is_streaming 分流 escape vs insert_before）
- [x] 删 escape 时期的反向测试 `streaming_text_not_in_ratatui_buffer_*`，替换为正向（buffer 里有 mutable 文字）

## 3. 组件化：widget 拆分（src/app/tui/components/）

- [x] `components/mod.rs` re-export
- [x] `components/transcript_line.rs`：`TranscriptLine` widget，`&RenderedLine` + width → Buffer（wrap + CJK + 样式）；insert_before 与 TestBackend 共用
- [x] `components/mutable_line.rs`：`MutableLine` widget，pending_tail + width + area → tail buffer 顶区（wrap 多行，透明 bg `Color::Reset`，紧贴面板 top-anchored）
- [x] `components/status_indicator.rs`：`StatusIndicator` widget，spinner_idx + status → `Working`/工具状态 label（执行进度，原 ThinkingIndicator 改名+拆分）
- [x] `components/thinking_block.rs`：`ThinkingBlock` widget，reasoning 显示（`Thinking…` 占位，未来收 ThinkingDelta 可展开热切换）
- [x] `components/input_prompt.rs`：`InputPrompt` widget，input buffer + area → 底行（`❯` + 内容 + 光标，CJK 显示宽度）
- [x] `components/bottom_panel.rs`：`BottomPanel` widget，带 border + panel_bg 的 chrome 容器，组合 StatusIndicator + ThinkingBlock + InputPrompt；idle 填充整个 tail 区（无空终端行）
- [x] `components/tail.rs`：`Tail` widget，组合 MutableLine（顶）+ BottomPanel（底）；`draw_tail_frame` 退化为 `Tail::render`
- [x] `app.rs` 新增 `reasoning` 字段 + 处理 `ThinkingDelta`（累积进 reasoning，不产 scrollback 行）+ `reasoning()` 取值
- [x] `theme.rs` 新增 `panel_bg` / `panel_border` token
- [x] `TAIL_HEIGHT` 调到 6（mutable 1 + 面板 5）

## 4. harness 覆盖（每个 widget 独立 TestBackend 可测）

- [x] `TranscriptLine`：ASCII / CJK / 长 wrap 多行
- [x] `MutableLine`：wrap 多行 + 透明 bg 断言
- [x] `StatusIndicator`：`Working` 默认 label / 工具状态覆盖
- [x] `ThinkingBlock`：无 reasoning 显示 `Thinking…` 占位 / 有 reasoning 显示文本
- [x] `BottomPanel`：idle border+input / streaming status+thinking+input / 内部 panel_bg 填充 / height helper
- [x] `InputPrompt`：`❯` + 内容 + 光标位置（含 CJK）
- [x] `Tail` 组合：idle 面板填充无空行 / streaming mutable 顶+面板底 / 透明 vs panel_bg / TurnEnd 无残留
- [x] StreamBuffer 状态机单测（app.rs 内，c365 原有保留）

## 5. 校验

- [x] `cargo test --features tui --lib tui::` 全过（66 passed）
- [x] `cargo test --features tui` 全套绿（lib 541 + integration 87）
- [x] `cargo test --test bdd -- --test-threads=1` 无回退（87 passed）
- [x] `just qa` 绿（fmt + clippy + test + docs + prek；含 `cargo clippy --features tui -- -D warnings` 零 warning）
- [x] `llman sdd validate c365-add-streaming-mutable-tail --strict --no-interactive` 通过（自举，见下）

真终端冒烟（用户验证，非代码任务）：`cargo run --features tui`，确认文字在正文区生长、换行固化、到宽度自动换行、thinking+输入在底部不受影响。
