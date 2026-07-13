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

## 产品已接线（c492 / c668）

| 行为 | 约定 |
|---|---|
| `!` / `!!` 前缀 | Editor 边框切 `{colors.success}`；去掉后恢复 muted |
| idle Enter `!cmd` | `Driver::execute_bash`（`exclude_from_context=false`）；**MUST NOT** `Driver::run` |
| idle Enter `!!cmd` | 同上且 `exclude_from_context=true` |
| 空 `!` / `!!` | 系统提示，不执行 |
| 结果 | live scrollback：`UiEntry::Bash` 块（`$ cmd` + 输出）；提交即 **pending** tint（`tool-pending-bg`）；成功 → `tool-success-bg`；失败 / Esc `(cancelled)` → `tool-error-bg`；**整行终端宽度** `apply_background_to_line` + 块内 padding_y=1 |
| 块间距 | 块与块之间 **一行** untinted Spacer |
| 长输出 | 默认末尾 5 行视口 + `ctrl+o to expand`；**Ctrl+O** 全局视口折叠/全文（与工具详情同键） |
| busy | `!…` 仍走 steer/普通文本；**不**另开 bash |
| Ctrl+G | **c650**：TTY 真 `$VISUAL`/`$EDITOR`（对齐 demo）；harness / 非 TTY 仍 stub（系统行 + `# $EDITOR stub`） |

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
| `agent_demo` / `src/app/tui` | 解析编辑器命令、写临时文件、spawn/wait、写回 Editor；产品 **c650** 对齐 demo 真路径，harness 仍 stub |
| `infra` / Driver | bash **执行**走 `Driver::execute_bash`；外部编辑器 **不参与** |

## 意向（对齐 pi / demo）

bash 交互块用 **tool-*-bg 三态 tint**（pending / success / error·cancelled）；边框仍用 `{colors.success}` fg 强调 `!` 模式。退出码与 `(cancelled)` 用 error tint + error fg。
