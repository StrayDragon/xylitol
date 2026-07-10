# Theme tokens

语义 token（主 `DESIGN.md` frontmatter）→ 终端 SGR 的映射层在**应用面**；包组件只收闭包。

## MUST

1. 颜色 MUST 经语义 token 映射，**MUST NOT** 在产品组件里散落原始 hex。
2. 包组件主题字段为 `Box<dyn Fn(&str) -> String>`（或等价），由本面注入。
3. Diff 使用 `diff-added` / `diff-removed` / `diff-context`（见 [`diff-block.md`](./diff-block.md)）。
4. 一屏最多一处 `accent`（通常 busy spinner 或焦点边框）。

## 默认意向

Catppuccin Mocha 短色板；用户主题切换后置（c458 demo / 产品 chrome）。
