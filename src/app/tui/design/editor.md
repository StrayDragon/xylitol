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

## MUST

1. 多行草稿；反色假光标；默认隐藏硬件光标。
2. **上下 `─` 边框保留**，明确「这里是输入/操作区」（对齐 agent_demo 图 2）。
3. Ctrl+P / 设置等：**替换 editor 槽**为 `SelectList` / `SettingsList`，Esc 还原；**MUST NOT** blit 到 transcript 顶部。
4. 键位见 [`keybindings.md`](./keybindings.md)（Ctrl+C 清/退、流中 Enter=steer 等）。

边框色：`{colors.muted}`（`{components.editor-border}`）。
