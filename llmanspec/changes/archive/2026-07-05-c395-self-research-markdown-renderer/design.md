# c395 Design — 自研 markdown renderer

> 参考 codex markdown_render.rs（Writer 状态机），精简到 ~300 行。

## 核心架构

`Writer` 消费 `pulldown_cmark::Event` 迭代器，维护：
- `lines: Vec<Line<'static>>` 输出
- `current_spans: Vec<Span<'static>>` 当前行的 span 累积
- `inline_style: Style` 当前 inline 样式（bold/italic/code 叠加）
- `in_blockquote: bool` 引用深度
- `code_lang: Option<String>` + `code_buf: String` 代码块缓冲

## 样式映射（codex 风格，终端友好）

| 元素 | 样式 |
|---|---|
| H1 | bold + underline |
| H2 | bold |
| H3 | bold + italic |
| 粗体 | BOLD modifier |
| 斜体 | ITALIC modifier |
| 删除线 | CROSSED_OUT |
| inline code | fg Cyan |
| 代码块 | syntect 高亮（无框无背景无标签） |
| 引用 | italic + fg DarkGray（不画 │） |
| 列表 | `- ` 前缀 + 缩进（不画 ├─└─） |
| 链接 | text + ` (` + url(cyan+underline) + `)` |
| 表格 | 列对齐空格（不画 ┌─┬─┐） |

## 复用文件

- `syntect_bridge.rs`：`SyntectHighlighter`（inherent `highlight(lang, code) -> Vec<StyleSegment>`）
- `segment.rs`：`segments_to_lines(source, segments, prefix, style, width) -> Vec<Line>`

## 不画边框的实现

vendored renderer 在 render.rs 用 box_chars（╭│╰─┌┐├└）画代码块/表格边框、用 list_prefix 树连接器画列表。自研 renderer 完全不用这些常量——代码块只输出高亮行、列表只输出 `- ` 前缀、引用只加 italic、表格只用空格对齐。

## 不在本变更范围

- CustomTerminal viewport 动态高度（c381/c396）
- 流式全程高亮（c377 围栏感知已解决）
