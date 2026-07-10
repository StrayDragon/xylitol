---
version: "alpha"
name: "keybindings"
description: "Resolved product key chords — parenthesized hints in UI."
tokens_from: "../DESIGN.md"
components:
  key-hint:
    textColor: "{colors.muted}"
---

# Keybindings

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

已决议产品键位（实现：c480 / InputListener c455）。包 demo 应提前对齐以便验证。

## MUST（全局 / 输入）

| 键 | 行为 |
|---|---|
| Esc | 流中：**abort** 当前模型/工具流 |
| Ctrl+C | 编辑器非空：**清空**；已空：**退出** TUI |
| Enter（流中） | **steer**（插入引导，不打断当前轮的队列语义见 c461） |
| Alt+Enter | **follow-up**（排队到本轮结束后） |
| 双 Esc | 打开 **会话树**（**优先路径**；见 [`session-tree.md`](./session-tree.md)，c454→c456→c491） |
| Ctrl+O | **树关**：工具详情视口折叠/全文；**树开**：循环 session-tree filter |
| Ctrl+T | **树关**：thinking 展开/折叠；**树开**：filter → no-tools |
| Alt+E | tool/diff **块**展开/折叠（有无详情；与 Ctrl+O 视口正交） |
| Ctrl+D / U / L / A | **树开**：filter → default / user / labeled / all |
| Ctrl/Alt+←→ | **树开**：fold 或分支段跳转 |
| Shift+L | **树开**：编辑节点 annotation |
| Shift+T | **树开**：切换 annotation 时间戳显示 |
| Shift+F | **树开**：**fork**（停在选中节点、预填 user 文案；下次提交成兄弟分支；异于 Enter travel） |

## MUST（编辑器槽）

| 键 | 行为 |
|---|---|
| Esc（选择器打开时） | 关闭选择器，还原 editor |
| Ctrl+P 等 | 打开命令/设置（替换 editor 槽） |

## 规则

1. 全局键经 `InputListener` **先于** Editor 焦点消费（c455）。
2. **MUST NOT** 让 Ctrl+C 泄漏进 Editor 变成字面 `c`。
3. footer **默认不**罗列完整快捷键墙；细节 `/help`。
4. UI 旁注快捷键 MUST 用括号包裹完整和弦（如 `(Ctrl+T)`、`(Alt+E)`），**MUST NOT** 使用 `^T` 缩写作为用户可见提示；提示色用 `{colors.muted}`。
