---
change_id: c1530-update-app-tui-session-resume-row-layout
title: Resume 列表预览按终端比例 + Ctrl+U 切换完整 session id
status: in-progress
priority: 1530
depends_on: []
author: agent
branch: c1530-update-app-tui-session-resume-row-layout
base_sha: 62f3ec3222a03dc9e17f9b39b481522ff607439e
checkpointed: false
---

# c1530-update-app-tui-session-resume-row-layout

## Why

Resume 列表现需在「可读预览」与「排查用完整 session id」之间折中。常驻 UUID 挤占预览；纯隐藏则排障不便。默认隐藏 id、快捷键按需展开，并把预览宽度改为按终端比例软顶。

## What Changes

1. **预览列**：去掉固定 ~40 列硬顶；预览预算 = 扣除 id（若显示）与 meta 后的剩余，软顶约终端宽度的 **60%**；超长 `…` 截断。
2. **Session id 列（默认隐藏）**：`Ctrl+U`（`app.session.toggleId`）toggle 完整 id 显隐；**显示时 MUST 不截断**；无 name/`first_message` 时预览用 `—`，不得用 id 顶替预览列。
3. **合约**：更新 `atm10` / `ati29` + feature 场景；同步 `design/session-resume.md`、`keybindings.md`、playground 静图（默认 / id-on）。
4. **实现**：`SessionResumePanel` 布局 + 键位 + harness/单测。

## Out of scope

- 改 scope/sort/rename/delete 语义
- 改 list_sessions / Driver seam
- 默认常显 UUID（已否决）

## Capabilities

- `app-tui-commands`（atm10）
- `app-tui-input`（ati29）

## Impact

- Resume 默认更宽预览；排障时一键展开完整 id
- Header 提示增加 `(ctrl+u)` id 开关

## Ethics

- risk_level: low
- prohibited_actions: 显示态截断 UUID；把 id 仅塞进不可见调试通道；占用已有 ctrl+s/n/p/r/d 语义
- required_evidence: 单测/harness 覆盖默认无完整 id、Ctrl+U 后完整 id 可见；design playground check 绿
