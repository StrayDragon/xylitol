---
change_id: c395-self-research-markdown-renderer
title: 自研轻量 markdown renderer（pulldown-cmark + syntect，无边框纯样式）
status: proposed
priority: 395
depends_on: []
author: agent
---

# c395-self-research-markdown-renderer

## Why

c370 vendor 的 ratatui-markdown 带来完整能力，但渲染风格（边框 ╭─│╰─、树连接器 ├─└─、表格框线 ┌─┬─┐）不符合期望。用户要：只保留语法高亮 + 终端友好的加粗/斜体等样式，**不画任何边框/树连接器**；像 codex/pi 那样自研 component（底层库可复用）。

vendored 代码 ~3500 行，大量在 box-drawing 常量 + 表格边框渲染 + 树连接器逻辑——这些都是用户不要的。自研 ~300 行 pulldown-cmark 驱动的 renderer 即可覆盖需求，且更可控。

## What Changes

### 1. 自研 markdown_render.rs（pulldown-cmark Writer 状态机）

新增 `src/app/tui/components/markdown_render.rs`：基于 pulldown-cmark 的 Event 迭代器，状态机产出 `Vec<Line>`。
- 标题：加粗（H1 下划线）
- 粗体/斜体/删除线：Modifier
- inline code：颜色无背景
- 代码块：syntect 高亮（无框无标签）
- 引用：斜体 + dim（**不画 │/▎**）
- 列表：简单缩进或 `-`（**不画 ├─└─**）
- 表格：对齐空格（**不画 ┌─┬─┐**）
- 链接：`text (url)`（保留 c371 契约）

### 2. 复用 syntect_bridge + segment

把 vendored 的 `highlight/syntect_bridge.rs`（SyntectHighlighter 算法体）+ `highlight/segment.rs`（segments_to_lines）搬到 `components/`，删 trait 抽象（inherent 方法）。

### 3. 删除 vendored ratatui-markdown

搬出复用文件后，删整个 `src/app/tui/vendor/` 目录。

### 4. 保持 adapter 签名

`components/markdown.rs` 的 `render_markdown(text, width, &MarkdownStyle) -> Vec<Line>` 不变（render.rs 不用改，14 个行为测试保持绿）。

## Capabilities

- `app-tui`（修改）：修订 tui70（从 vendored 改为自研 renderer）。

## Impact

- 新增 `markdown_render.rs` + 搬入 `syntect_bridge.rs`/`segment.rs`（~600 行）
- 删除 `vendor/ratatui_markdown/`（~3500 行）
- 加 `pulldown-cmark = "0.10"` 依赖（tui-gated）
- 风险：中。自研 renderer 覆盖范围；但保持签名 + 行为测试降低风险。

## 调研证据

- codex `markdown_render.rs:86-105` MarkdownStyles（标题加粗、code cyan、blockquote green、链接 cyan+underline）。
- codex 用 pulldown-cmark 0.10（codex-rs/Cargo.toml:332）。
- vendored syntect_bridge.rs 不依赖 markdown 解析（纯 syntect 算法），可直接复用。
