---
version: "alpha"
name: "bash-mode"
description: "Product ! / !! bash (c492) + demo accent / Ctrl+G (c457+)."
tokens_from: "../DESIGN.md"
components:
  bash-accent:
    textColor: "{colors.success}"
---

# Bash mode

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

## 产品已接线（c492）

| 行为 | 约定 |
|---|---|
| `!` / `!!` 前缀 | Editor 边框切 `{colors.success}`；去掉后恢复 muted |
| idle Enter `!cmd` | `Driver::execute_bash`（`exclude_from_context=false`）；**MUST NOT** `Driver::run` |
| idle Enter `!!cmd` | 同上且 `exclude_from_context=true` |
| 空 `!` / `!!` | 系统提示，不执行 |
| 结果 | live scrollback：`$ cmd` + 输出；非 0 exit → `UiEntry::Error`（error fg） |
| busy | `!…` 仍走 steer/普通文本；**不**另开 bash |
| Ctrl+G | 最小 stub（系统行 + `# $EDITOR stub`）；harness 不 spawn `$EDITOR` |

## Demo 已验证（c457+ · `agent_demo`）

| 行为 | 约定 |
|---|---|
| `!` 前缀 | 编辑器边框切 `{colors.success}`（`Editor::set_border_color`） |
| 去掉 `!` | 边框恢复 muted |
| Ctrl+G（TTY） | 真 `$VISUAL`/`$EDITOR`（缺省 nano/notepad）：`TUI::with_terminal_suspended` → tempfile → spawn → 写回 `set_text` |
| Ctrl+G（harness / 非 TTY） | **stub**（系统行 + `# $EDITOR stub`）；`XYLITOL_AGENT_DEMO_EDITOR_STUB=1` 强制 stub；`XYLITOL_AGENT_DEMO_REAL_EDITOR=1` 强制真路径 |

## 分层边界（外部编辑器）

| 层 | 职责 |
|---|---|
| `packages/xylitol-tui` | **仅**挂起/恢复终端：`with_terminal_suspended`（`Terminal::stop`/`start` + `request_render(true)`）；**MUST NOT** 内置 `$EDITOR` / tempfile |
| `agent_demo` / `src/app/tui` | 解析编辑器命令、写临时文件、spawn/wait、写回 Editor（产品 MVP = stub） |
| `infra` / Driver | bash **执行**走 `Driver::execute_bash`；外部编辑器 **不参与** |

## 意向（对齐 pi）

bash 模式用 **fg** 强调（非 tool bg 三态）；退出码用 `{colors.error}`（产品 Error 行）。
