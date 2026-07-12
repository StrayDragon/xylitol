---
change_id: c492-add-app-tui-bash-mode
title: "app-tui ! bash 模式"
status: draft
priority: 492
depends_on:
  - "c485-add-app-tui-vertical-slice"
  - "c457-demo-bash-mode-external-editor"
author: agent
track: B
---

# c492-add-app-tui-bash-mode

> **status: draft**（已从 purpose-draft 升格；待 `llman-sdd-apply`）

## Why

demo（c457）已验证 `!` 边框强调与 Ctrl+G 外部编辑器边界；`Driver::execute_bash` / `Command::Bash` / `dispatch` 已存在。产品 TUI idle Enter 仍把 `!cmd` 当普通 prompt 交给 `Driver::run`，缺少 bash 模式边框与执行→live scrollback 路径。

## What Changes

1. **边框**：产品 Editor 文本 trim 后以 `!` / `!!` 开头时，操作区边框 MUST 切 `{colors.success}`；去掉前缀后 MUST 恢复 muted（对齐 demo ati8 / `design/bash-mode.md`）。
2. **执行**：idle Enter 且文本为 bash 前缀时 MUST 走 `dispatch(Command::Bash)` / `Driver::execute_bash`，MUST NOT 调用 `Driver::run`。`!!` → `exclude_from_context=true`；`!` → `false`。
3. **呈现**：执行结果 MUST 写入 live scrollback（复用既有 `UiEntry`，优先 System/专用 bash 行）；exit_code≠0 时 MUST 用 error/warning 前景强调；busy agent 轮次中 `!` Enter 行为与 steer 正交（本切片：busy 时 `!` 仍按 steer/普通文本，不另开 bash——或明确拒绝；见 design）。
4. **Ctrl+G（SHOULD）**：产品 host 接线外部编辑器（复用 `with_terminal_suspended` 边界；harness stub）；不阻塞 bash 执行主路径若时间紧可拆 follow-up，但本 change tasks 含最小 stub。
5. **非目标**：改 Driver API、流式 bash 工具块三态（LLM tool bash 已有）、Codex TranscriptView、真 PTY `$EDITOR` E2E（stub 即可）。

## Capabilities

- `app-tui-input`：产品 `!`/`!!` 边框与 idle bash 提交
- `app-tui-transcript`：bash 结果进 live scrollback 的呈现约束（轻量）

## Impact

- 触达：`src/app/tui/{host,ui_root,theme,mod}.rs`、可选 `bridge`、合成 harness、`design/bash-mode.md` / keybindings。
- 风险：低；seam 已通。注意 `!` 与 slash `/`、steer 的优先级。
