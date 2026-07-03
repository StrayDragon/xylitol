---
change_id: c355-tui-markdown-render
title: TUI Markdown 渲染组件（用户输入与 LLM 回复共用）
status: proposed
priority: 355
depends_on: []
author: agent
---

# c355-tui-markdown-render

## Why

当前 TUI 的正文渲染（`render.rs:79-80`）把 `RenderedLine::UserInput` / `AssistantText` 的内容当作**裸字符串**塞进 `Line::styled`，无任何 markdown 解析。后果：

- **代码块**没缩进/背景/语言标签，```` ```rs ```` 和正文混作一团
- **列表/标题/引用/粗体**无视觉结构，LLM 回复的可读性远低于 print 模式（print 走 markdown 渲染）
- **用户输入与 LLM 回复不一致**：user 加 `❯ ` 前缀但同样不解析 markdown，`- item` 列表在用户输入里也是平铺

三个对标项目都做了 markdown 渲染，且都基于现成 parser（codex: pulldown-cmark + syntect；pi: marked；kimi-code: pi-tui Markdown）。pi 进一步让 user/assistant **共用同一个 Markdown 类**，靠样式参数差异化（`assistant-message.ts:103` vs `user-message.ts:31`）。

本变更新增 `MarkdownRenderer` 组件，消费已 finalize 的字符串产出 `Vec<Line<'static>>`（ratatui buffer 路径，沿 tui42/tui51）。**用户输入与 LLM 回复共用此组件**，差异只在样式 token（`palette.user_prompt()` vs `palette.assistant()`），保证两者视觉结构一致。

## What Changes

### 1. 新增 `src/app/tui/components/markdown.rs` — MarkdownRenderer

```rust
pub struct MarkdownStyle { /* text/code_block_bg/code_inline/heading/list/quote 样式 token */ }
pub fn render_markdown(text: &str, width: u16, style: &MarkdownStyle) -> Vec<Line<'static>>
```

基于 `pulldown-cmark`（Cargo.toml 加依赖，`tui` feature gate）。消费已 finalize 的字符串 + `MarkdownStyle`，产出 `Vec<Line<'static>>`（ratatui buffer 路径，沿 tui42/tui51）。MVP 支持元素（spec tui61）：标题（#）、**粗体**/`code` inline、```` ``` 代码块 ````（语言标签 + 背景色，**无 syntect 高亮**）、`- 无序列表`、`> 引用`、CJK 段落换行（复用现有 `unicode-width`）。

### 2. user/assistant 共用 MarkdownRenderer（spec tui60）

`RenderedLine::UserInput` 和 `AssistantText` 的渲染路径都经 `render_markdown(...)`，差异只在注入的 `MarkdownStyle`（`user_prompt()` palette vs `assistant()` palette）。用户输入的 `❯ ` 前缀作为首行独立 Span（不进 markdown 解析），避免 `# 标题` 被当成 heading。

### 3. RenderedLine 渲染路径从单行改多行

`to_line(&self) -> Line` 改为 `to_lines(&self, width: u16) -> Vec<Line<'static>>`（或新增 `to_lines` 保留 `to_line` 给非 markdown 变体）。`TranscriptLine::rows()` / `commit_height` / `render_commit_lines_into_buf` 同步调整为消费 `Vec<Line>`。

### 4. 流式渲染不变（finalize 后才渲染 markdown）

mutable 顶行的流式文字仍走纯文本（现状，spec tui61 streaming-stays-plain）；TextDelta 经 `insert_before` 提交到 scrollback 时才经 markdown 渲染。流式增量 markdown 留后续。

## Capabilities

- `app-tui`（修改）：新增 markdown 渲染约束（tui60 user/assistant 共用 + pulldown-cmark；tui61 MVP 元素范围 + 不做 syntect/表格/流式增量）。

## Capabilities

- `app-tui`（修改）：新增 markdown 渲染约束（user/assistant 共用、pulldown-cmark、TestBackend 可验证、不做语法高亮）

## Impact

- **受影响代码**：
  - 新增 `src/app/tui/components/markdown.rs`（MarkdownStyle + render_markdown + pulldown-cmark Event 遍历状态机）
  - `src/app/tui/render.rs`：`RenderedLine::UserInput` / `AssistantText` 渲染路径改经 `render_markdown`，产出多行
  - `src/app/tui/components/transcript_line.rs`：`rows()` / `row_count()` 消费 `Vec<Line>`（原消费单 `Line`）
  - `Cargo.toml`：加 `pulldown-cmark`（`tui` feature gate）
- **受影响规范**：`app-tui`（新增 tui60/tui61）。
- **风险**：低-中。pulldown-cmark 是纯解析无 IO。`RenderedLine` 从单行变多行影响 commit 路径（`commit_height` / `render_commit_lines_into_buf`），需回归 CJK 换行（tui41 的 `commit_cjk_long_line_wraps_by_display_width` 等测试必须仍绿）。

## 调研证据（三家对比）

- **codex**：`pulldown-cmark` + 自维护状态机（`markdown_render.rs:825-863`）+ syntect 高亮。流式用「纯文本流式 → 结束后 AgentMarkdownCell 重渲替换」。user 不解析（`UserHistoryCell` 纯文本 + textwrap）。
- **pi**：`marked.lexer` Token AST + `MarkdownTheme` 函数式注入（每个元素是 `(text) -> string` ANSI 函数）。user/assistant **共用 Markdown 类**，靠参数差异化（`assistant-message.ts:103` vs `user-message.ts:31`）。流式靠全量重解析 + (text,width) 缓存。表格 170+ 行（列宽加权收缩）。
- **kimi-code**：复用 pi-tui Markdown + `createMarkdownTheme`。assistant 走 Markdown，user 走纯 `Text`（不共用）。`renderCache(width→lines)` 避免重算。

**xylitol 取舍**：抄 pi 的「user/assistant 共用 + MarkdownStyle 注入」（一致性），用 codex 的 pulldown-cmark（Rust 原生），流式先跳过（pi/codex 的流式方案都复杂），表格/高亮先跳过（pi 表格 170 行，codex syntect 重）。

## 不在本变更范围

- 语法高亮（syntect）——独立后续
- 表格渲染——独立后续
- 流式增量 markdown——独立后续
- 列表选择器/审批浮层/命令面板——c356/c357
