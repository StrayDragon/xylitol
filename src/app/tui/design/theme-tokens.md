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
> **c475 / c1115**：产品 host 默认 `Palette::dark()`；用户可经 **`/theme`**（slash）切换内建 `dark` / `light`；**MUST NOT** 默认开启 theme auto。

语义 token → 终端 SGR 在**应用面**；包组件只收闭包。

包便利层（c570）：`xylitol_tui::Palette` + `fg_rgb`/`bg_rgb` + `markdown_theme` / `diff_theme` / `choice_prompt_theme`——**不是** JSON 主题市场。

闸门：`just sync-tui-tokens` / `just check-tui-tokens`（playground tokens + `Palette` ≡ DESIGN）。

## MUST

1. 颜色 MUST 经语义 token 映射，**MUST NOT** 在产品组件里散落原始 hex。
2. 包组件主题字段为 `Box<dyn Fn(&str) -> String>`（或等价），由本面注入。
3. Diff / 工具块 / Markdown 色规则见 [`diff-block.md`](./diff-block.md)、[`expandable.md`](./expandable.md)、[`markdown.md`](./markdown.md)。
   - Edit 嵌在 `tool-*-bg` 时：词级用 `xylitol_tui::word_wash_bg(block_bg, polarity)`（红/绿轻量混亮，默认 mix≈**0.32**），**MUST NOT** reverse。
4. 一屏最多一处 `{colors.accent}`（通常 busy spinner 或焦点边框）。
5. 产品 host **MUST** 默认 `Palette::dark()`；**MUST NOT** 默认开启 theme auto / OSC11 / COLORFGBG 探测。
6. 产品 **MAY** 经用户发起的 **`/theme`**（无参开 Themes 槽；有参 `dark`|`light`|`toggle`）热切换内建色板（c1115 / `HostSession::reload_themes`）。
7. **MUST NOT** 将 demo **Ctrl+P** `theme-toggle` 设为产品默认键位。
8. 查任意 `{colors.*}`：打开 [`../DESIGN.md`](../DESIGN.md) 的 `colors:` 段。

## 内建两套（c570）

| Scheme | 色板 | 用途 |
|---|---|---|
| Dark | Catppuccin Mocha = `DESIGN.md` | **产品默认**；demo 默认 |
| Light | Catppuccin Latte 对齐 | demo / playground / 产品 `/theme light` |

## Demo 扩展点（`agent_demo` = 活实验场）

| 项 | 约定 |
|---|---|
| 默认 | `theme:dark`（未开 auto 时忽略 COLORFGBG / OSC11） |
| 启用 | `XYLITOL_AGENT_DEMO_THEME_AUTO=1` 或 harness `set_theme_auto` |
| 禁止 | **不要**在 crossterm 事件环里写 `OSC11_BG_QUERY` |
| 命令 | `/theme` · Ctrl+P → `theme-toggle`（**产品只接 slash，不抄 Ctrl+P**） |
| Playground | 右上 Dark/Light 仅审色；产品默认仍 dark，用户 slash 可切 |

产品 host **MUST NOT** 因 demo 默认打开自动切换。
