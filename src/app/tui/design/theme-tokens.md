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

包可选便利层（c570）：`xylitol_tui::Palette` + `fg_rgb`/`bg_rgb` + 固有方法 `markdown_theme` / `diff_theme` / `choice_prompt_theme`——**不是** JSON 主题市场 / TS Theme 服务。

## MUST

1. 颜色 MUST 经语义 token 映射，**MUST NOT** 在产品组件里散落原始 hex。
2. 包组件主题字段为 `Box<dyn Fn(&str) -> String>`（或等价），由本面注入。
3. Diff 使用 `{colors.diff-added}` / `{colors.diff-removed}` / `{colors.diff-context}`（见 [`diff-block.md`](./diff-block.md)）。
4. 工具块背景使用 `{colors.tool-pending-bg}` / `{colors.tool-success-bg}` / `{colors.tool-error-bg}`（见 [`expandable.md`](./expandable.md)）；bg 闭包只重置背景（`\x1b[49m`）。
5. 一屏最多一处 `{colors.accent}`（通常 busy spinner 或焦点边框）。
6. Markdown 标题分级色见 [`markdown.md`](./markdown.md)（`md-h1`…`md-h6` → accent / on-surface / muted）；链接 URL 可用 underline。
7. 查任意 `{colors.*}` 表达式：打开 [`../DESIGN.md`](../DESIGN.md) 的 `colors:` 段。

## 内建两套（c570）

| Scheme | 色板 | 用途 |
|---|---|---|
| Dark | Catppuccin Mocha = `DESIGN.md` | **产品 MVP 固定**；demo 默认 |
| Light | Catppuccin Latte 对齐 | demo / 未来 host **opt-in** |

`Palette::dark()` / `::light()` / `Palette::from(TerminalColorScheme)`（`SemanticPalette` 为别名）。扩展新 flavor = 新 `Palette` 常量；固有方法不变。

## Demo 扩展点（`agent_demo`）

| 项 | 约定 |
|---|---|
| 默认 | `theme:dark`（未开 auto 时忽略 COLORFGBG / OSC11） |
| 启用 | `XYLITOL_AGENT_DEMO_THEME_AUTO=1` 或 harness `set_theme_auto` |
| 解析 | `COLORFGBG`（默认）；harness `feed_terminal_color_reply` / `apply_theme_detect` 可注入 OSC11/CSI |
| 禁止 | **不要**在 crossterm 事件环里写 `OSC11_BG_QUERY`——reply 会当按键漏进 stdin（可误开 Ctrl+G 编辑器） |
| 命令 | `/theme` · `/theme dark` · `/theme light` · `/theme toggle`；Ctrl+P → `theme-toggle` |
| 可见 | footer `theme:dark` / `theme:light`；**markdown / diff / tool-bg / muted / ChoicePrompt** 均随 `theme_mode` 全量换肤 |
| Playground | 右上 Dark/Light；`?scheme=light`；token 来自 `colors` + `colors_light` |

产品 host **MUST NOT** 因本 demo 默认打开自动切换。
