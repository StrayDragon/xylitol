---
version: "alpha"
name: "session-tree"
description: "Double-Esc session tree — primary history/branch UX (c454/c456/c491)."
tokens_from: "../DESIGN.md"
components:
  tree-line:
    textColor: "{colors.on-surface}"
  tree-muted:
    textColor: "{colors.muted}"
---

# Session tree

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

**优先产品路径**（替代 Codex 式 transcript 浏览）：双 Esc 打开会话树。包侧：`TreeSelector`（c454+c456）；demo 原型（c456）；产品接线（c491）。

## MUST

1. **空编辑器**双 Esc（时间窗与 demo/c456 一致）打开会话树；**MUST NOT** 做成 Codex 式独立 transcript 浏览面。
2. 树 **替换 editor 槽**（`showSelector`），保证贴底可见；**MUST NOT** blit 到内容绝对顶部。
3. 选中用 **reverse**；勿另立 selection 面板底色。
4. Esc：有搜索串时先清搜索；否则关闭树并还原 editor；流中单 Esc 仍为 abort（见 [`keybindings.md`](./keybindings.md)）。
5. travel / fork 经应用面 `Driver`；包组件只负责树 UI。
6. **搜索**：对 `label` 增量过滤（与 `include_node` AND）；槽内提示 `Type to search` / 当前 query。
7. **翻页**：←→ 与 PgUp/PgDn 按 `max_visible` 翻页。
8. **Filter**（demo/产品谓词，非包枚举）：Ctrl+D/T/U/L/A；树开时 Ctrl+O 循环；状态行 `(i/n) [filter]`。

## 实现指针

| 层 | Change |
|---|---|
| 包 `TreeSelector` | c454 归档 + **c456**（搜索 / ←→ / status_suffix） |
| 相对 pi 差距 | [`session-tree-vs-pi.md`](./session-tree-vs-pi.md) |
| 产品 `src/app/tui` | c491 |
