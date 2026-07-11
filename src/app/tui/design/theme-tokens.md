---
version: "alpha"
name: "theme-tokens"
description: "Map DESIGN.md semantic tokens to terminal SGR closures."
tokens_from: "../DESIGN.md"
components:
  map-note:
    textColor: "{colors.on-surface}"
---

# Theme tokens

> Token 根源：全部语义色 / 间距 → [`../DESIGN.md`](../DESIGN.md) frontmatter。本文件只规定**映射规则**，不另立色板。

语义 token（主 `DESIGN.md` frontmatter）→ 终端 SGR 的映射层在**应用面**；包组件只收闭包。

## MUST

1. 颜色 MUST 经语义 token 映射，**MUST NOT** 在产品组件里散落原始 hex。
2. 包组件主题字段为 `Box<dyn Fn(&str) -> String>`（或等价），由本面注入。
3. Diff 使用 `{colors.diff-added}` / `{colors.diff-removed}` / `{colors.diff-context}`（见 [`diff-block.md`](./diff-block.md)）。
4. 工具块背景使用 `{colors.tool-pending-bg}` / `{colors.tool-success-bg}` / `{colors.tool-error-bg}`（见 [`expandable.md`](./expandable.md)）；bg 闭包只重置背景（`\x1b[49m`）。
5. 一屏最多一处 `{colors.accent}`（通常 busy spinner 或焦点边框）。
6. Markdown 标题分级色见 [`markdown.md`](./markdown.md)（`md-h1`…`md-h6` → accent / on-surface / muted）；链接 URL 可用 underline。
7. 查任意 `{colors.*}` 表达式：打开 [`../DESIGN.md`](../DESIGN.md) 的 `colors:` 段。

## 默认意向

Catppuccin Mocha 短色板；**产品 MVP 固定暗色**。

## Demo 扩展点（c458 · `agent_demo`）

| 项 | 约定 |
|---|---|
| 默认 | `theme:dark`（未开 auto 时忽略 COLORFGBG / OSC11） |
| 启用 | `XYLITOL_AGENT_DEMO_THEME_AUTO=1` 或 harness `set_theme_auto` |
| 解析 | 包 `terminal_colors`：`resolve_terminal_color_scheme`（explicit > OSC11 亮度 > CSI 997 > COLORFGBG > Dark） |
| 可见 | footer 含 `theme:dark` / `theme:light`；muted chrome 用 Mocha/Latte 真彩 |

产品 host **MUST NOT** 因本 demo 默认打开自动切换。
