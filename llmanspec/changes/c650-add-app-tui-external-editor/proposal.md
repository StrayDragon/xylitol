---
change_id: c650-add-app-tui-external-editor
title: "产品 Ctrl+G：真 $EDITOR（TTY）/ harness stub"
status: purpose-draft
priority: 650
depends_on: ["c625-update-app-tui-design-next-wave"]
author: agent
track: A
---

# c650-add-app-tui-external-editor

## Why

长提示在终端 Editor 里改成本高；demo 已用 `with_terminal_suspended` + `$EDITOR`，产品仍 stub。

## Purpose

产品 Ctrl+G：TTY 真 `$VISUAL`/`$EDITOR`；非 TTY / harness 保持 stub（可测、不挂 CI）。

## What Changes（实现时）

1. 对齐 demo：suspend → tempfile → spawn → `set_text` → resume。
2. env 覆盖策略对齐 [`bash-mode.md`](../../../src/app/tui/design/bash-mode.md)。
3. harness：仍断言 stub，**不** spawn 真编辑器。

## Capabilities

- `app-tui-input`（modify）
- 包边界不变（仅用已有 `with_terminal_suspended`）

## Design SSOT

- [`bash-mode.md`](../../../src/app/tui/design/bash-mode.md) § 外部编辑器
- [`keybindings.md`](../../../src/app/tui/design/keybindings.md)

## Impact

- `src/app/tui/host` / layout；**不**把 `$EDITOR` 逻辑塞进 `xylitol-tui` 包

## Out of scope

- GUI 编辑器集成；computer-use

## Ethics

- risk_level: medium（真终端副作用）
- prohibited_actions: CI 默认 spawn `$EDITOR`；包内实现 tempfile/spawn
- required_evidence: harness stub 绿；手动 TTY 备注可接受

## Depends

- **c625**；与树 power / models **无硬依赖**
