---
change_id: c355-tui-markdown-render
title: TUI Markdown 渲染组件（用户输入与 LLM 回复共用）
status: draft
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

## What Changes（草案，full 化时细化）

1. 新增 `src/app/tui/components/markdown.rs`，定义 `MarkdownRenderer`：
   - 入口 `pub fn render_markdown(text: &str, width: u16, style: MarkdownStyle) -> Vec<Line<'static>>`
   - `MarkdownStyle { text: Style, code_block_bg: Option<Color>, ... }` 由调用方注入（user/assistant 各一份）
   - 基于 `pulldown-cmark`（Cargo.toml 加依赖；tui21 允许 fit-driven 库依赖）
2. 支持的元素（阶段 1）：标题（#）、**粗体**/`code`、```` ``` 代码块 ````（背景色 + 缩进 + 语言标签，**不做语法高亮**）、`- 无序列表`、`> 引用`、段落换行（CJK 宽度，复用现有 unicode-width）
3. `RenderedLine::UserInput` / `AssistantText` 的 `to_line` 改为经 `MarkdownRenderer` 产出**多行**（当前是单行 `Line`，需调整为 `Vec<Line>` 或在 commit 路径展开）
4. 流式渲染：**阶段 1 只做 finalize 后的全量渲染**。mutable 顶行的流式文字仍走纯文本（现状），TurnEnd 提交到 scrollback 时才经 markdown 渲染。流式增量 markdown（codex 的「流式纯文本→结束后重渲替换」或 pi 的「全量重解析+缓存」）留后续
5. 不做（阶段 1）：语法高亮（syntect）、表格、OSC8 超链接、流式增量解析

## Capabilities

- `app-tui`（修改）：新增 markdown 渲染约束（user/assistant 共用、pulldown-cmark、TestBackend 可验证、不做语法高亮）

## Impact

- 新增 `src/app/tui/components/markdown.rs` + `pulldown-cmark` 依赖
- 修改 `render.rs`（RenderedLine 渲染路径）、`Cargo.toml`
- 风险：低-中。pulldown-cmark 是纯解析无 IO。RenderedLine 从单行变多行可能影响 commit 路径，需回归 CJK 换行（tui41）

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

## 约束草案（full 化时落 spec.toon）

- user/assistant MUST 共用 MarkdownRenderer，差异只在 MarkdownStyle
- MUST 基于 pulldown-cmark（不自写 parser）
- 每个 markdown 元素渲染 MUST 可经 TestBackend 独立验证（tui41）
- 渲染函数 MUST 消费 UI-only 数据（字符串 + MarkdownStyle），不 match XyEvent（tui42）
- 阶段 1 MUST NOT 引入 syntect（代码块只做结构化：背景+缩进+语言标签）
- 阶段 1 MUST NOT 做流式增量 markdown（finalize 后全量渲染即可）
