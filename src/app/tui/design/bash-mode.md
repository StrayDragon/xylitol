---
version: "alpha"
name: "bash-mode"
description: "Draft — product bash mode and external editor (c492 / c457)."
tokens_from: "../DESIGN.md"
components:
  bash-accent:
    textColor: "{colors.success}"
---

# Bash mode（草稿）

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

后置：产品 bash 模式与外部编辑器（c492 / c457）。

## 意向

对齐 pi：bash 模式用 **fg** 强调（非 tool bg 三态）；退出码可用 `{colors.error}` / `{colors.warning}`。
