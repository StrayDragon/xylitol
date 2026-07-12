---
version: "alpha"
name: "editor"
description: "Bottom operation zone — bordered multiline editor / selector slot."
tokens_from: "../DESIGN.md"
components:
  editor-border:
    textColor: "{colors.muted}"
  operation-zone:
    border: "{components.editor-border}"
---

# Editor（操作区）

> Token 根源：`{colors.*}` / `{components.*}` → [`../DESIGN.md`](../DESIGN.md)。
> **c480**：接线 Editor + slash MVP + 全局键；选择器替换本槽。

## MUST

1. 多行草稿；反色假光标；默认隐藏硬件光标。
2. **上下 `─` 边框保留**，明确「这里是输入/操作区」（对齐 `agent_demo`）。
3. Ctrl+P / 设置 / 会话树 / slash 选择器：**替换 editor 槽**，Esc 还原；**MUST NOT** blit 到 transcript 顶部。
4. 键位见 [`keybindings.md`](./keybindings.md)。
5. **Slash MVP（c480）**：至少 `/exit`、`/model`；补全走包 `CompletionSource`；未知命令短错误进 scrollback 或 editor 提示，不崩。
6. idle 占位文案可静默（无强制 placeholder 墙）；有草稿时不盖住内容。

边框色：`{colors.muted}`（`{components.editor-border}`）。
