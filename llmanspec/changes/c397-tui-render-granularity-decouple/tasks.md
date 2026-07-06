# c397-tui-render-granularity-decouple — Tasks

> 依赖 c396（样式表）。架构解耦：渲染粒度=整条消息，commit 粒度=行。
> 风险中：触及流式管线核心 + viewport 布局。每步后跑回归。

## 1. StreamBuffer 重构：raw_source 累积 + 整源 render（app.rs）

- [ ] 1.1 `StreamBuffer` 字段改为 `raw_source: String` + `committed_rendered_len: usize` + `last_full_render: Vec<Line>`。删除 `buffer/committed_len/in_fence` 旧字段。
- [ ] 1.2 `push(delta, width, style)`：append 到 raw_source，全量 `render_markdown(raw_source, width, style)`，存 `last_full_render`。
- [ ] 1.3 `stable_rendered_len()`：对 `raw_source[..=last_newline]` 单独 render 取行数；无换行返回 0。
- [ ] 1.4 `drain_new_stable_lines() -> Vec<Line>`：返回 `last_full_render[committed..stable_len]`，advance committed。
- [ ] 1.5 `pending_rendered() -> &[Line]`：返回 `last_full_render[committed..]`（mutable 区）。
- [ ] 1.6 `finalize() -> Vec<Line>`：返回 `last_full_render[committed..]`（残余），重置 raw_source。
- [ ] 校验：`cargo test -p xylitol --lib app::stream_buffer`（新增单测覆盖 push/drain/finalize）。

## 2. handle_xy_event 适配新 StreamBuffer（app.rs）

- [ ] 2.1 `RenderedLine` 新增 `Rendered(Vec<Line<'static>>)` 变体（携带预渲染行）。
- [ ] 2.2 `RenderedLine::to_lines` 对 `Rendered` 直接返回行（绕过 markdown.rs 二次 render）。
- [ ] 2.3 `TextDelta` 分支：`stream_buf.push + drain_new_stable_lines` → `RenderedLine::Rendered`。
- [ ] 2.4 `ThinkingDelta` 分支同上（thinking_buf 同构改造）。
- [ ] 2.5 `TurnEnd`：finalize 残余 → `RenderedLine::Rendered`。
- [ ] 校验：`cargo test -p xylitol --lib app::tests`（textdelta/turn_end/end_stream 等适配新语义）。

## 3. mutable region 多行 + 适配 widget（render.rs / mutable_line.rs / tail.rs）

- [ ] 3.1 `MutableLine` widget 改接收 `&[Line<'static>]`（渲染行）而非 `&str`；多行渲染，wrap 由 renderer 已处理。
- [ ] 3.2 `Tail` widget 布局：mutable 区高度 = min(pending 行数, N-4)，N=TAIL_HEIGHT；超出顶部丢弃。
- [ ] 3.3 `pending_tail()` → `pending_rendered()` 签名变更，调用方适配。
- [ ] 3.4 评估是否需提升 `TAIL_HEIGHT`（6→8 或 10）给 mutable 更多预算；若 6 够用则保持。
- [ ] 校验：`cargo test -p xylitol --lib render::cursor_tests` + `render::commit_harness`（CJK/wrapping/streaming 回归）。

## 4. 删除无条件空行 hack（markdown_render.rs）

- [ ] 4.1 `Tag::CodeBlock` 前空行加守卫 `if !self.lines.is_empty()`。
- [ ] 4.2 `flush_code_block` 后空行加守卫。
- [ ] 4.3 注释更新：移除「streaming 段落切分临时缓解」，改为「renderer 看完整文档，按需加空行」。
- [ ] 校验：`cargo test -p xylitol --lib markdown_render` + `markdown`（code_block 留白断言更新）。

## 5. 流式 == 整体渲染 不变量测试（app.rs）

- [ ] 5.1 新增 `streaming_matches_full_render`：含标题/列表/引用/代码块的 md，逐字 stream vs 一次性 render，断言行序列（文本+样式）相等。
- [ ] 5.2 新增 `streaming_preserves_cross_paragraph_context`：段落+代码块留白在流式下正确（无需无条件空行）。
- [ ] 校验：`cargo test -p xylitol --lib streaming_invariant -- --test-threads=1`。

## 6. 现有测试适配

- [ ] 6.1 `stream_push_drains_complete_lines_on_newline`（app.rs:500）：drain 返回 `Vec<Line>`，断言改渲染行文本。
- [ ] 6.2 `fence_aware_commit_accumulates_code_block_until_close`（app.rs:542）：新模型下代码块天然整源 render，断言更新。
- [ ] 6.3 `outside_fence_lines_commit_immediately`（app.rs:581）：适配行级 commit 语义。
- [ ] 6.4 mutable_line / tail widget 测试适配新签名。
- [ ] 校验：`cargo test -p xylitol --lib`（全绿）。

## 7. 全量校验 + 归档

- [ ] 7.1 `just fmt` + `just lint`（clippy 无新告警）。
- [ ] 7.2 `just test`（含新增不变量测试 + 回归全绿）。
- [ ] 7.3 `cargo run -- --help`（无回归）。
- [ ] 7.4 `just qa`。
- [ ] 7.5 手动：流式长文档（含多段落/列表/引用/代码块），观察留白一致、列表连续、无多余空行；resize 不崩（已 commit 行不重排是预期）。
- [ ] 7.6 `_HANDOFF.md` 第二/三节相关项标记完成或删除 HANDOFF（架构已解耦）。
- [ ] 7.7 `llman sdd validate c397-tui-render-granularity-decouple --strict` 通过后 archive。
