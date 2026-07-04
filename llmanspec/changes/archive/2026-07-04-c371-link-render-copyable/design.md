# c371 Design — 链接渲染修复

> 范围极小（只改 inline.rs），本文只记两个决策。

## 决策 1：`text (url)` 形式而非 OSC8

用户明确要「方便用户习惯可以手动复制」。OSC8（`\x1b]8;;url\x1b\\text\x1b]8;;\x1b\\`）是终端可点击链接，但：
- 很多终端不支持（尤其 SSH/远程场景）
- 用户要手动复制，OSC8 的 url 在很多终端里复制时丢失
- codex 也用 `text (url)` 可见形式（`markdown_render.rs:1564-1590`）

选 `text (url)`：text + ` (` + url + `)`，url 完整可见、可手动选中复制。

## 决策 2：复用 primary_color，不加专用 link token

vendored theme.rs 的 RichTextTheme trait 没有 get_link_color。两个选项：
- A（采纳）：复用 `get_primary_color()`（Cyan）+ UNDERLINED，括号用 `get_muted_text_color()`。零侵入，不改 trait/ThemeConfig/ThemeBuilder。
- B（否决）：加 `get_link_color` 默认方法 + ThemeConfig 字段 + ThemeBuilder 方法。更整洁但要改 vendor 四处协同点，改动面与收益不匹配（Cyan 已是链接色事实标准）。

## 不在本变更范围

- `[ref]` 引用式链接（需 ref 表，独立后续）
- OSC8 可点击超链接
