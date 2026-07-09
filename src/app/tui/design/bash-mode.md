---
version: "alpha"
name: "bash-mode"
description: "Demo-verified bash accent + external-editor stub (c457); product execute still c492."
tokens_from: "../DESIGN.md"
components:
  bash-accent:
    textColor: "{colors.success}"
---

# Bash mode

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

## Demo 已验证（c457 · `agent_demo`）

| 行为 | 约定 |
|---|---|
| `!` 前缀 | 编辑器边框切 `{colors.success}`（`Editor::set_border_color`） |
| 去掉 `!` | 边框恢复 muted |
| Ctrl+G | 外部编辑器 **stub**：系统行 + 文案标记 `# $EDITOR stub`；**不** spawn `$EDITOR` |

## 产品后置（c492）

- `Driver::execute_bash` / 真 bash 执行与退出码着色
- 真 `$EDITOR` PTY 往返

## 意向（对齐 pi）

bash 模式用 **fg** 强调（非 tool bg 三态）；退出码可用 `{colors.error}` / `{colors.warning}`。
