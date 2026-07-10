---
version: "alpha"
name: "compaction-status"
description: "Draft — context compaction / retry status (c493)."
tokens_from: "../DESIGN.md"
components:
  compaction-line:
    textColor: "{colors.muted}"
---

# Compaction status（草稿）

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

后置：上下文压缩 / 重试状态（c493）。

## 意向

短 dim 一行；忙碌时可并入 status（见 [`status.md`](./status.md)），勿常驻多行。
