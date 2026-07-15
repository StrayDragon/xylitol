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
> **c475 MVP**：`cwd · model`；**c1035**：带 provenance 的 `used N`/`~N`/`?`（无则省略）；branch 仍可选。

## MUST

1. 恰好 **1 行** dim。字段序：`cwd · model`；可选 `· used <friendly> tokens`（**c1035**）/ `· branch`。
2. `context%` 仅当 Driver/会话能提供上下文占用比例时显示；无数据 **MUST NOT** 伪造 `0%` 墙。
3. 放不下截断右侧（优先保留 cwd 左端与 model），**MUST NOT** 增高。
4. 快捷键提示：默认**不**写进 footer（勿 `enter submit · double Esc…` 墙）；需要时 `/help` 或旁注括号和弦（见 [`keybindings.md`](./keybindings.md)）。
5. 队列摘要若展示：短前缀 `q:sN|fM ·` 可贴 footer 最左，仍保持单行。
6. **MUST NOT** 常驻多行 debug / 快捷键墙；**MUST NOT** 把 Working 文案塞进 footer（那是 status）。

颜色：`{colors.muted}`；高度：`{spacing.footer-rows}`。
