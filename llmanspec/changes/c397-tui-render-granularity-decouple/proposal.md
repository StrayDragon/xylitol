---
change_id: c397-tui-render-granularity-decouple
title: TUI 渲染粒度与 commit 粒度解耦（整条消息 raw_source 渲染 + 行差值 commit）
status: proposed
priority: 397
depends_on: [c396-tui-markdown-style-fix]
author: agent
---

# c397-tui-render-granularity-decouple

## Why

`_HANDOFF.md` 第二节论证：xylitol 当前「渲染粒度 = commit 粒度 = 段落」的模型是留白不一致、列表/引用连续性断裂的**根因**。每个段落独立 `render_markdown`，渲染器看不到前文，只能用「无条件空行」hack（c395 follow-up d877b63），结果空行过多或错位。

c396 修复了样式表（标题/引用/列表/主题），但架构耦合不动——因为 HANDOFF 第五节明确建议「不要边改样式边改架构」。本变更承载第二梯队：**解耦渲染粒度与 commit 粒度**，从根上消除跨段落视觉断裂。

codex 的不变量（`controller.rs:1085-1200` 测试 `controller_loose_vs_tight_with_commit_ticks_matches_full`）：**流式逐 delta + tick 的输出 == 一次性渲染整条源**。xylitol 目前无法满足。本变更的目标是让 xylitol 也满足这个不变量（在简化模型下）。

## What Changes

### 1. StreamBuffer 改为累积 raw_source + 整体 render（app.rs）

当前 `StreamBuffer::drain_complete_paragraphs` 按 fence/段落切分，每段独立 commit + 独立 render。改为：

- `StreamBuffer` 维护 `raw_source: String`（append-only）+ `committed_rendered_line_count: usize`。
- 每次 `push(delta)`：append 到 `raw_source`，对**完整 `raw_source`** 调 `render_markdown(raw_source, width, style)` 得到 `full_rendered: Vec<Line>`。
- 差值行 = `full_rendered[committed_rendered_line_count..]`（减去尾部 N 行 mutable 区，N = tail budget）。
- commit 粒度仍是行（甚至段落）：把差值的**稳定行**（最后一个换行边界之前）commit 到 scrollback，advance `committed_rendered_line_count`。
- 不稳定尾部行留在 mutable 区（`full_rendered[committed..]`），每帧重绘。

**核心收益**：渲染器永远看完整条消息源，`needs_newline` / 列表连续性 / 引用 prefix 重放自动恢复——c396 的样式修复能在流式场景下正确工作，不再依赖「无条件空行」hack。

### 2. mutable region 动态高度（terminal.rs + render.rs）

当前 `TAIL_HEIGHT: u16 = 6` 固定（mutable 2 + status 1 + panel 3）。改为：

- mutable 行数 = `uncommitted_rendered_lines.len()`（动态，上限 viewport 高度 - status - panel 最小值）。
- codex 用 `desired_height`（chatwidget/rendering.rs:6-40）按内容决定。xylitol 简化：mutable 区域高度 = min(uncommitted 行数, viewport_height - 4)（4 = status 1 + panel 3 最小）。
- `Viewport::Inline` 的初始高度按需设置；mutable 区行数变化时 ratatui 自动重排（inline viewport 支持每帧不同 mutable 高度）。

> **简化承诺**：xylitol 不做 codex 的 commit-tick 动画（逐行 dequeue 节流）、不做 finalize 二次 canonicalize（resize 重渲）。差值行一旦稳定立即 commit，mutable 区每帧整体重画。这够用，且大幅降低复杂度。

### 3. 删除「无条件空行」hack（markdown_render.rs）

架构解耦后，renderer 看到完整文档，可用 codex 的 `needs_newline` 机制（block 边界精确 push 一个空行）。删除 c395 follow-up（d877b63）的无条件 `self.lines.push(Line::raw(""))`，改为 `if !self.lines.is_empty() { push_blank_line() }`。

### 4. 流式 == 整体渲染 不变量测试（app.rs 测试）

新增类似 codex `controller_loose_vs_tight_with_commit_ticks_matches_full` 的测试：把同一段 markdown（含标题/列表/引用/代码块/表格）分别用「流式逐 delta」和「一次性整串」两种方式喂给 StreamBuffer，断言最终 committed 行序列**逐行相等**（文本 + 样式）。

## Capabilities

- `app-tui`：流式管线渲染粒度（modify tui50/tui75，add tui80/tui81）。

## Impact

- **代码**：`src/app/tui/app.rs`（StreamBuffer 重构，主改）、`terminal.rs`（动态 TAIL_HEIGHT）、`render.rs`（commit/mutable 路径适配）、`markdown_render.rs`（删 hack）。`markdown.rs` seam 不变。
- **测试**：新增流式==整体不变量测试；现有 StreamBuffer 测试（app.rs:500-588）需适配新语义（drain 返回行而非段落字符串）；mutable_line 测试需适配多行动态高度。
- **spec**：modify tui50（mutable last line → mutable region 动态行）、tui75（stream-incremental-highlight 升级为整源 render + 差值 commit）；add tui80（渲染粒度=整条消息）、tui81（流式==整体渲染不变量）。
- **风险**：中。触及流式管线核心 + viewport。需仔细回归 streaming/wrapping/CJK/turn-end-flush 测试。
- **不在范围**：stable 区可替换（codex AgentMarkdownCell，resize 自适应，HANDOFF 第三梯队 #7）、CustomTerminal fork（#8）、commit-tick 动画。

## 依赖

- `depends_on: [c396-tui-markdown-style-fix]`：c396 的样式表（标题分级/列表/引用前缀）依赖渲染器看到完整文档才能在流式下正确工作；先解耦架构（c397）再修样式，样式会在段落切分下被打断，需要返工。先样式（c396，单文件低风险）立即改善观感，再架构（c397）根治。
