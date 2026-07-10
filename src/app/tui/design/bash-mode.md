---
version: "alpha"
name: "bash-mode"
description: "Demo bash accent + external editor (c457+); product execute still c492."
tokens_from: "../DESIGN.md"
components:
  bash-accent:
    textColor: "{colors.success}"
---

# Bash mode

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

## Demo 已验证（c457+ · `agent_demo`）

| 行为 | 约定 |
|---|---|
| `!` 前缀 | 编辑器边框切 `{colors.success}`（`Editor::set_border_color`） |
| 去掉 `!` | 边框恢复 muted |
| Ctrl+G（TTY） | 真 `$VISUAL`/`$EDITOR`（缺省 nano/notepad）：`TUI::with_terminal_suspended` → tempfile → spawn → 读回 `set_text` |
| Ctrl+G（harness / 非 TTY） | **stub**（系统行 + `# $EDITOR stub`）；`XYLITOL_AGENT_DEMO_EDITOR_STUB=1` 强制 stub；`XYLITOL_AGENT_DEMO_REAL_EDITOR=1` 强制真路径 |

## 分层边界（外部编辑器）

| 层 | 职责 |
|---|---|
| `packages/xylitol-tui` | **仅**挂起/恢复终端：`with_terminal_suspended`（`Terminal::stop`/`start` + `request_render(true)`）；**MUST NOT** 内置 `$EDITOR` / tempfile |
| `agent_demo` / 未来 `src/app/tui` | 解析编辑器命令、写临时文件、spawn/wait、写回 Editor |
| `infra` / Driver | **不参与**（非 agent 工具调用） |

产品 host 驱动路径同样：暂停读输入 → `with_terminal_suspended`（或等价 `terminal.stop/start`）→ 恢复后强制全量重绘。

## 产品后置（c492）

- `Driver::execute_bash` / 真 bash 执行与退出码着色
- 产品面 Ctrl+G 接线（复用上表边界，非再造包 API）

## 意向（对齐 pi）

bash 模式用 **fg** 强调（非 tool bg 三态）；退出码可用 `{colors.error}` / `{colors.warning}`。
