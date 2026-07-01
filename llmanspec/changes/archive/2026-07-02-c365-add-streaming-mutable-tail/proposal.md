---
change_id: c365-add-streaming-mutable-tail
title: 流式文字增量生长在 scrollback 最后一行（mutable-last-line），tail 区只留 thinking+输入
status: proposed
priority: 365
depends_on:
  - c360-revise-c341-tui-render-harness
author: agent
---

# c365-add-streaming-mutable-tail

## Why

c360 落地后真终端冒烟发现流式布局问题（记于 c360/future.md）：当前 `draw_tail_frame` 把流式 pending 文字渲染在 thinking 指示器**上方**（顺序：流式文字 → thinking → 输入框），每来一个 chunk 在 thinking 上方打字，换行后整行上行进 scrollback。这不符合用户期望的 codex/pi 体验。

用户期望（参考 codex StreamController 双区域模式）：**流式文字增量生长在 scrollback 的最后一行（边打字边在该行追加），换行时该行固定并准备下一行；tail 区固定只显示 thinking 指示器 + 输入框**。即"打字机效果发生在上行区，不是在 thinking 上方"。

### 调研证据（codex StreamController 深度调研）

codex 的 mutable-last-line 机制核心只有两块（`docs/tui-research/` 未单独成文，本节为 c365 调研结论）：

1. **换行门控 buffer**（`codex-rs/tui/src/markdown_stream.rs:87-96`）：流式 token 累积进 buffer，只在 buffer 含 `\n` 时才把"上次边界到末个换行"的部分提交为稳定行。未换行的尾部永远留 buffer，作为 mutable tail。

2. **tail 每帧重绘**（`codex-rs/tui/src/chatwidget/streaming.rs:436-471`）：mutable tail 不进 scrollback，而在底部 viewport 区域每帧用 `current_tail_lines()` 重画——因为整屏每帧重绘，旧 tail 被新内容覆盖，视觉上原地生长。

codex 的其它复杂度（table_holdback 表格保留 / AdaptiveChunkingPolicy 动画平滑 / consolidation finalize 重排 / 独立 commit_tick 线程）都是为 markdown 表格稳定性 + 120fps 动画服务的，**与纯文本 mutable-last-line 无关，xylitol 简化版全部省略**（调研结论 §7）。

### 为什么是独立变更而非 c360 内联

c360 是渲染基础设施变更（harness + widget 解禁 + RenderedLine seam），已归档。流式 mutable-last-line 是流式渲染**架构**调整（commit 时机从"换行批量"改为"增量生长"），触及 `draw_tail_frame` 布局 + TextDelta 处理 + 可能需 `InlineTerminal` 新增 mutable-last-line 能力，属独立工作单元。

## What Changes

1. **新增流式状态机 `StreamBuffer`**（换行门控，对标 codex `MarkdownStreamCollector` 极简版）：`buffer: String` + `committed_len: usize`。`push(delta)` 累积 token；`drain_complete_lines() -> Vec<String>` 返回新换行边界内的完整行（推进 committed_len）；`pending_tail() -> &str` 返回未换行的尾部（mutable last line）。

2. **TextDelta 处理改为增量 commit**：每来一个 TextDelta，`drain_complete_lines()` 产出的完整行**立即** `commit_to_scrollback`（稳定区，进终端原生 scrollback）。未换行的尾部留 `pending_tail()`，每帧重绘。

3. **`draw_tail_frame` 移除 pending 文字渲染**：tail 区不再显示 `current_streaming_line`。流式时 tail 只渲染：thinking 指示器（顶部）+ 输入框（底部）。mutable 文字在 scrollback 最后一行生长，不在 tail 区。

4. **mutable-last-line 渲染**：流式时，`pending_tail()` 的内容需要**在 scrollback 最后一行增量显示**。实现方式：每帧用 `pending_tail()` 在 viewport 顶部行重绘（viewport 上方紧贴已 commit 的稳定行）——即 tail 区顶部那一行就是 mutable last line，下方是 thinking + 输入。

5. **TurnEnd 处理**：turn 结束时，`pending_tail()` 残留（未换行的最后一句）补 commit 到 scrollback，清空 buffer。

6. **复用 c360 的 RenderedLine seam**：增量 commit 的完整行经 `RenderedLine::AssistantText` 走（不破坏 tui42 边界分离）。

## Capabilities

- `app-tui`（修改）：新增流式渲染布局 spec（tail 不含 pending，pending 在 scrollback 最后一行生长）。

## Impact

- **受影响代码**：
  - `src/app/tui/app.rs`（新增 StreamBuffer 状态，TextDelta 处理改增量 commit）
  - `src/app/tui/render.rs`（draw_tail_frame 移除 pending 渲染 + 新增 mutable-last-line 渲染）
  - `src/app/tui/mod.rs`（TextDelta 分发逻辑调整）
- **受影响规范**：`app-tui`。
- **风险**：中。触及流式渲染热路径（用户最直接感知的 UX）。缓解：c360 的 TestBackend harness 是安全网，扩展 harness 覆盖 mutable-last-line 行为（增量生长 + 换行固化 + TurnEnd flush）。

## 反降级护栏（防止本变更被降级）

- [x] 流式 TextDelta 的完整行 MUST 增量 commit 到 scrollback（每来一个换行边界即 commit，非 turn 结束才批量）。
- [x] 未换行的尾部（pending_tail）MUST 在 panel 上方紧贴显示（每帧重绘），视觉上"打字机在上行区"。
- [x] mutable last line MUST 在 ratatui buffer 内渲染（Viewport::Inline），透明背景融进 scrollback；MUST NOT 用 escape 直写终端绕过 buffer（raw_render.rs 已删除）。
- [x] mutable last line 超宽 MUST wrap 到多行（CJK 按显示宽度），超出 tail capacity 的顶部行丢弃（不丢内容，换行后进 scrollback）。
- [x] TurnEnd MUST 把残留 pending_tail commit 到 scrollback（无丢失）。
- [x] 渲染层 MUST 组件化为 components/ 下可复用 widget（TranscriptLine/MutableLine/InputPrompt/BottomPanel/Tail/Spinner），每个 TestBackend 可独立验证；draw_tail_frame 退化为 Tail::render；commit_to_scrollback 改吃 &[RenderedLine]。
- [x] Thinking MUST 是正文（灰色 ThinkingText），thinking_buf 独立流式 + commit；不在面板内。
- [x] 面板 MUST 固定 3 行（border+input+border），idle 不填满 tail 区。
- [x] 输入框 MUST NOT 含 ❯ 前缀。
- [x] 现有 harness 测试 + 既有 TUI 测试 MUST 回归通过。
- [x] 本变更 MUST NOT 引入 codex 的 table_holdback / chunking / consolidation / 动画线程。
- [x] 本变更 MUST NOT 破坏 tui42 边界分离。
- [x] 本变更 MUST NOT 预先实现多行编辑 / StatusBar（后续独立变更）。
