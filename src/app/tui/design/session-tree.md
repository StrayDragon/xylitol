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

**优先产品路径**（替代 Codex 式 transcript 浏览）：双 Esc 打开会话树。包侧：`TreeSelector`（c454）；demo 原型（c456）；产品接线（c491）。

## MUST

1. **空编辑器**双 Esc（时间窗与 demo/c456 一致）打开会话树；**MUST NOT** 做成 Codex 式独立 transcript 浏览面。
2. 树 **替换 editor 槽**（`showSelector`），保证贴底可见；**MUST NOT** blit 到内容绝对顶部。
3. 选中用 **reverse**；勿另立 selection 面板底色。
4. Esc 关闭树并还原 editor；流中单 Esc 仍为 abort（见 [`keybindings.md`](./keybindings.md)）。
5. travel / fork 经应用面 `Driver`；包组件只负责树 UI。

## 实现指针

| 层 | Change |
|---|---|
| 包 `TreeSelector` | **c454 已归档**；`agent_demo` 双 Esc 冒烟已通 |
| demo 键位/文档收紧 | c456（可选） |
| 产品 `src/app/tui` | c491 |
