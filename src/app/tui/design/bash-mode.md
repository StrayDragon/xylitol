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
| 结果 | live scrollback：`UiEntry::Bash` 块（`$ cmd` + 输出）；提交即 **pending** tint（`tool-pending-bg`）；输出 chunk 到达时 **同一块内增量刷新**（保持 pending）；成功 → `tool-success-bg`；失败 / Esc `(cancelled)` → `tool-error-bg`；**整行终端宽度** `apply_background_to_line` + 块内 padding_y=1 |
| 块间距 | 块与块之间 **一行** untinted Spacer |
| 长输出 | 默认末尾 5 行视口 + `ctrl+o to expand`；**Ctrl+O** 全局视口折叠/全文（与工具详情同键） |
| busy（agent） | `!…` 仍走 steer/普通文本；**不**另开 bash |
| busy（交互 bang 仍在跑） | 再提交 `!`/`!!` **硬拒绝**：提示 + 保留编辑器文本；**MUST NOT** 第二次 `execute_bash`、**MUST NOT** 排队 |
| Ctrl+G | **c650**：交互 TTY 且已配置非空 `$VISUAL`/`$EDITOR` → 真外部编辑器；harness / 非 TTY → stub（系统行 + `# $EDITOR stub`）；未配置或失败 → `UiEntry::Error`（**无**静默 nano/notepad；与 demo 可默认 nano **分叉**） |

## Demo 已验证（c457+ · `agent_demo`）

| 行为 | 约定 |
|---|---|
| `!` 前缀 | 编辑器边框切 `{colors.success}`（`Editor::set_border_color`） |
| 去掉 `!` | 边框恢复 muted |
| Ctrl+G（TTY） | 真 `$VISUAL`/`$EDITOR`（缺省 nano/notepad）：`TUI::with_terminal_suspended` → tempfile → spawn → 写回 `set_text` |
| Ctrl+G（harness / 非 TTY） | **stub**（系统行 + `# $EDITOR stub`）；`XYLITOL_AGENT_DEMO_EDITOR_STUB=1` 强制 stub；`XYLITOL_AGENT_DEMO_REAL_EDITOR=1` 强制真路径 |

### 产品 vs demo（c650）

| | 产品 `src/app/tui` | demo `agent_demo` |
|---|---|---|
| 未配置 `$VISUAL`/`$EDITOR` | **`UiEntry::Error`**，保留原文；**MUST NOT** 默认 nano | 可默认 nano/notepad |
| 实现位置 | 产品独立模块（不抽进包、不与 demo 共享） | demo 内辅助函数 |
| 包 | 仅 `with_terminal_suspended` | 同左 |

## 分层边界（外部编辑器）

| 层 | 职责 |
|---|---|
| `packages/xylitol-tui` | **仅**挂起/恢复终端：`with_terminal_suspended`（`stop`/`start`/`refresh_size` + soft pending；**保留** `previous_lines` 供差分，**不**立刻 `do_render`、**不**整屏 clear）；**MUST NOT** 内置 `$EDITOR` / tempfile |
| `agent_demo` / `src/app/tui` | 解析编辑器命令、写临时文件、spawn/wait、写回 Editor；产品 **c650** 对齐 demo 真路径，harness 仍 stub |
| `infra` / Driver | bash **执行**走 `Driver::execute_bash`；外部编辑器 **不参与** |

## 意向（对齐 pi / demo）

bash 交互块用 **tool-*-bg 三态 tint**（pending / success / error·cancelled）；边框仍用 `{colors.success}` fg 强调 `!` 模式。退出码与 `(cancelled)` 用 error tint + error fg。
