---
change_id: c457-demo-bash-mode-external-editor
title: "agent_demo：! bash 模式边框 + Ctrl+G 外部编辑器原型"
status: full
priority: 457
depends_on: ["c455-add-package-tui-input-listener"]
author: agent
track: A
---

# c457-demo-bash-mode-external-editor

## Why

产品后置 bash/外部编辑器，但库与交互需提前验证（边框色、键位不与 Editor 冲突；Ctrl+G 已从 Alt+G glyphs 让出）。

## Purpose

demo：编辑器文本以 `!` 开头时边框切 bash 强调色；Ctrl+G 触发外部编辑器 stub（不强制真 spawn `$EDITOR`）；回写 `design/bash-mode.md`。

## What Changes

1. 包：`Editor::set_border_color`（或等价）供 host 切换边框主题闭包。
2. demo：检测 `!` 前缀 → 绿色边框；否则恢复 muted。
3. demo：Ctrl+G → 系统行 + stub 回写（可追加 `# $EDITOR stub`）；harness 断言。
4. 文档：`bash-mode.md` / `keybindings.md`。

## Capabilities

- `package-tui-editor`（边框可切换）
- `app-tui-input`（demo bash 前缀与 Ctrl+G）

## Out of scope

- 产品 `Driver::execute_bash`（c492）
- 真 `$EDITOR` PTY 往返（stub 即可）
