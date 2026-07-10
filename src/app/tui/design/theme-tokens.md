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
6. 查任意 `{colors.*}` 表达式：打开 [`../DESIGN.md`](../DESIGN.md) 的 `colors:` 段。

## 默认意向

Catppuccin Mocha 短色板；用户主题切换后置（c458 demo / 产品 chrome）。
