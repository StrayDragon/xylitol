---
version: "alpha"
name: "footer"
description: "Single-line dim footer — cwd · model · optional context%."
tokens_from: "../DESIGN.md"
components:
  footer:
    textColor: "{colors.muted}"
    height: "{spacing.footer-rows}"
---

# Footer

> Token 根源：`{colors.*}` / `{spacing.*}` → [`../DESIGN.md`](../DESIGN.md)。

## MUST

1. 恰好 **1 行** dim：`cwd (branch) · model · context%`；放不下截断右侧，**MUST NOT** 增高。
2. 快捷键提示：默认不列清单；需要时 `/help` 或极短 `?`。若旁注键位，MUST 括号包裹完整和弦（见 [`keybindings.md`](./keybindings.md)）。
3. **MUST NOT** 常驻多行 debug / 快捷键墙。

颜色：`{colors.muted}`；高度：`{spacing.footer-rows}`。
