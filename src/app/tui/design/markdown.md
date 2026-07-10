---
version: "alpha"
name: "markdown"
description: "Copy-friendly Markdown — fg inline, bg deferred to full-width pad."
tokens_from: "../DESIGN.md"
components:
  md-body:
    textColor: "{colors.assistant}"
  md-heading:
    textColor: "{colors.accent}"
  md-link:
    textColor: "{colors.accent}"
  md-code:
    textColor: "{colors.success}"
  md-quote:
    textColor: "{colors.muted}"
---

# Markdown

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

助手正文与可复制链接。包：`Markdown` + `MarkdownTheme.highlight_code` 回调。

## MUST

1. 标题用 `#` 前缀字符；列表用 `1.` / `-`。
2. 链接渲染为 `text (url)` 可复制形式（勿只留不可选中的 OSC 隐藏）。
3. 代码块：**语法高亮即可**；**MUST NOT** 边框、语言标签条、行号墙。
4. 高亮库（syntect 等）注入在主 crate / demo，**MUST NOT** 打进 `xylitol-tui` 默认依赖（c452）。
5. 段落间最多一空行。
6. **fg / bg 分相**（吸取 pi-tui）：元素着色走 fg 闭包；若需消息底色，在行宽 padding 阶段再套 `bgColor`（`apply_background_to_line`），保证背景铺满终端宽。
