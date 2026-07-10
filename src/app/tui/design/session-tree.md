---
version: "alpha"
name: "session-tree"
description: "Draft — double-Esc session tree (c491); package tree selector c454/c456."
tokens_from: "../DESIGN.md"
components:
  tree-line:
    textColor: "{colors.on-surface}"
  tree-muted:
    textColor: "{colors.muted}"
---

# Session tree（草稿）

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

后置：双 Esc 打开会话树（c491）。包侧 tree selector：c454 / c456。

## 意向

替换 editor 槽或短 overlay；选中用 reverse，勿另立 selection 面板底。
