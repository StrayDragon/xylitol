---
version: "alpha"
name: "expandable"
description: "Xylitol Terminal component UX — see DESIGN.md index."
---

# Expandable（thinking / tool）

对齐 pi interactive：折叠一行摘要，展开完整内容。实现落在**应用面** transcript 子块，不是包内通用 Chat 组件。

## MUST

1. 默认 **详略得当**：折叠时一行（或短摘要）；展开后显示完整 thinking / 工具输出。
2. 折叠态仍须复制友好：摘要行本身可读，**MUST NOT** 只有图标。
3. 快捷键切换（thinking toggle、tools expand）由产品面绑定；包组件只渲染给定展开态。
4. 折叠行旁 MUST 提示对应快捷键（demo 榜样：`thinking  ^T`、`tool/diff  Alt+E`），避免只靠 footer 快捷键墙。

## 策略（可后续细化）

默认折叠哪些、是否记住、是否全局一键展开工具 → 接线后可再议；本文件锁定「需要可展开」+「块旁键位提示」。

Diff 块也可套同一展开壳（见 [`diff-block.md`](./diff-block.md)）；展开策略与 tool 块可共用状态机（c453）。
