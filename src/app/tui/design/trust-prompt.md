---
version: "alpha"
name: "trust-prompt"
description: "Draft — project trust prompt then yolo (c490)."
tokens_from: "../DESIGN.md"
components:
  trust-body:
    textColor: "{colors.on-surface}"
  trust-warning:
    textColor: "{colors.warning}"
---

# Trust prompt（草稿）

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

后置：项目信任提示（c490）。信任后 yolo（无逐工具审批 UI）。

## 意向

短 overlay 确认（见 [`overlay.md`](./overlay.md)）；警告用 `{colors.warning}`。
