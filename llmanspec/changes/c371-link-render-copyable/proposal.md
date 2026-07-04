---
change_id: c371-link-render-copyable
title: 链接渲染为可复制形式（text (url)）+ autolink
status: proposed
priority: 371
depends_on: []
author: agent
---

# c371-link-render-copyable

## Why

c370 vendor 了 ratatui-markdown，但用户反馈链接渲染不达预期——要求「方便用户习惯可以手动复制」。调查 vendored `inline.rs` 发现：

1. **`[text](url)` 解析已存在但渲染残缺**：`inline.rs:225-235` 已解析链接结构，但**丢弃了 url**（变量名 `_url`，只用 `link_text`）。`[GitHub](https://github.com)` 只渲染成 `GitHub`（带 underline），URL 不可见、无法手动选中复制。
2. **`<url>` autolink 完全缺失**：inline.rs 没有 `<` 分支，`<https://example.com>` 字面渲染（含尖括号），不是可复制的干净 URL。

用户场景明确：终端里看到链接要能**直接看到完整 URL 并手动复制**（不走 OSC8 可点击，因为很多终端不支持，且用户明确要手动复制）。

## What Changes

### 1. `[text](url)` 渲染成 `text (url)`（inline.rs:225-235）

codex 风格：text + ` (` + url + `)`。text 用 text_color，url 用 primary_color + UNDERLINED，括号用 muted_text_color 弱化。空 link_text 时只输出 url。

### 2. `<url>` autolink（inline.rs:198 后插入）

加 `if chars[i] == '<'` 分支：扫到 `>`，校验内容含 `://` 或 `mailto:` 前缀，渲染成 url（primary_color + UNDERLINED）。~15 行。

## Capabilities

- `app-tui`（修改）：新增 tui72（链接 MUST 渲染为可见 url 形式方便手动复制）。

## Impact

- **受影响代码**：`src/app/tui/vendor/ratatui_markdown/markdown/inline.rs`（仅此一文件）。
- **受影响规范**：`app-tui`（新增 tui72）。
- **风险**：低。纯 inline 渲染，render.rs/text.rs 消费点无需改（链接多 Span 自动流过现有 wrap 管线）。

## 不在本变更范围

- `[ref][id]` / `[ref]` 引用式链接（需文档级 ref 表，inline 解析器 per-line 无状态，改造面大）
- OSC8 可点击超链接（用户要手动复制）
- 给 RichTextTheme 加专用 get_link_color（vendor 改动面最小化，复用 primary_color）

## 调研证据

- **codex**：`markdown_render.rs:1564-1590` 把链接渲染成 `text (url)` 后缀，url 用 link 样式着色——正是「方便手动复制」的形式。
- **vendored inline.rs:200-248**：`[text](url)` 解析已完整（含嵌套 `[]`/`()` 处理），只差渲染时保留 url。
