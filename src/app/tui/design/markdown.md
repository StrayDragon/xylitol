# Markdown

助手正文与可复制链接。包：`Markdown` + `MarkdownTheme.highlight_code` 回调。

## MUST

1. 标题用 `#` 前缀字符；列表用 `1.` / `-`。
2. 链接渲染为 `text (url)` 可复制形式（勿只留不可选中的 OSC 隐藏）。
3. 代码块：**语法高亮即可**；**MUST NOT** 边框、语言标签条、行号墙。
4. 高亮库（syntect 等）注入在主 crate / demo，**MUST NOT** 打进 `xylitol-tui` 默认依赖（c452）。
5. 段落间最多一空行。
