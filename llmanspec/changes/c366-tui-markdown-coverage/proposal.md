---
change_id: c366-tui-markdown-coverage
title: 补齐 markdown 管线覆盖（ThinkingText + 引用视觉前缀）
status: proposed
priority: 366
depends_on: []
author: agent
---

# c366-tui-markdown-coverage

## Why

c355 把 `RenderedLine::UserInput` / `AssistantText` 接入了 `MarkdownRenderer`，但有两个可见缺口在 c355 范围外：

1. **ThinkingText 仍走纯文本**：`render.rs:81` 的 `RenderedLine::ThinkingText(text) => Line::styled(text.clone(), p.thinking())` 没经 `render_markdown`。但 thinking（reasoning）内容常含代码块、列表、内联代码——尤其 reasoning 模型会输出「分析步骤」式的 markdown。当前这类内容被平铺成一行裸字符串，可读性差，且与 assistant 正文渲染不一致。

2. **引用块无视觉前缀**：c355 的引用渲染只对 `BlockQuote` 应用 `quote`（dim）样式，不加 `>` 或竖线前缀。实测（c355 visualize 测试）：`> 这是一段引用` 渲染后是「这是一段引用」纯 dim 文本，在终端里与普通正文几乎无法区分（dim 灰度差异在多数终端不明显）。codex/pi/kimi-code 三家都对引用加了视觉前缀（codex 用 `│`，pi 用 `>`）。

本变更是对 c355 渲染管线的收口：让所有「正文类」`RenderedLine` 变体统一走 markdown，并让引用有可识别的视觉结构。

## What Changes

### 1. ThinkingText 接入 MarkdownRenderer

`RenderedLine::to_lines` 的 `ThinkingText` 分支从 `vec![self.to_line()]` 改为经 `render_markdown(text, width, &MarkdownStyle::for_thinking(&p))`。新增 `MarkdownStyle::for_thinking`：以 `p.thinking()`（dim italic）为 text 基色，heading/code/list/quote 复用 assistant 的结构样式但整体偏 dim，保持「思考是次要内容」的视觉层级。

### 2. 引用块加视觉前缀

`markdown.rs` 的 `BlockQuote` 渲染：在引用的每一行首加 `▎ `（左竖线 + 空格，半角宽度，比 `>` 更清爽且不与列表 `-`/`*` 混淆）前缀 span，用 `style.quote` 着色。缩进随引用嵌套层级递增（二级引用 `▎ ▎ `）。

## Capabilities

- `app-tui`（修改）：新增约束（tui65 ThinkingText 经 markdown；tui66 引用块有视觉前缀）。

## Impact

- **受影响代码**：
  - `src/app/tui/render.rs`：`RenderedLine::to_lines` 的 `ThinkingText` 分支
  - `src/app/tui/components/markdown.rs`：新增 `MarkdownStyle::for_thinking`；`Renderer` 的 BlockQuote 处理加前缀
- **受影响规范**：`app-tui`（新增 tui65/tui66）。
- **风险**：低。纯渲染层，TestBackend 可全覆盖。ThinkingText 多行化影响 `commit_height`（已在 c355 验证过多行路径安全）。

## 不在本变更范围

- 代码语法高亮（syntect/tree-sitter）——独立后续变更，依赖库选型
- 流式增量 markdown——独立后续
- 表格渲染——独立后续

## 调研证据

- **codex**：引用用 `│` 左竖线前缀（`markdown_render.rs` blockquote 分支）；thinking 经 markdown 渲染（`AgentMarkdownCell` 不区分 thinking/text 的 markdown 解析，只差样式）。
- **pi**：引用保留 `>` 前缀；thinking 与 assistant 共用 Markdown 类，样式参数差异化。
- **kimi-code**：引用用 dim + `▏` 前缀。

**xylitol 取舍**：引用前缀用 `▎`（轻量左竖线，比 codex 的 `│` 视觉更轻，比 pi 的 `>` 不与引用语法符号重复）；ThinkingText 经 markdown 复用 c355 已就绪的 `render_markdown`，只加 `for_thinking` 样式构造器。
