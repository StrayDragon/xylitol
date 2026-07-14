---
change_id: c650-add-app-tui-external-editor
title: "产品 Ctrl+G：真 $EDITOR（TTY）/ harness stub"
status: full
priority: 650
depends_on: ["c625-update-app-tui-design-next-wave"]
author: agent
track: A
---

# c650-add-app-tui-external-editor

## Why

长提示在终端 Editor 里改成本高；demo 已验证 `with_terminal_suspended` + `$EDITOR` 形状，产品仍永远 stub。

## Purpose

产品 Ctrl+G：交互 TTY 且已配置 `$VISUAL`/`$EDITOR` 时走真外部编辑器；harness / 非 TTY 仍 stub；未配置或失败走 `UiEntry::Error`，不崩、不静默默认 nano。

## What Changes

1. 产品 host：TTY 真路径（suspend → tempfile → spawn → `set_text` → resume）；判定与 stub 门闸对齐 `bash-mode.md`（产品侧独立实现，**不**与 demo/包共享模块）。
2. 严格解析：仅非空 `$VISUAL` 否则 `$EDITOR`；皆无 → `UiEntry::Error`；**MUST NOT** 默认 nano/notepad。
3. 失败（缺配置 / IO / spawn / 非零退出）一律 `UiEntry::Error` 短行 + 保留原文；不 panic。
4. harness：继续断言 stub，**不** spawn 真编辑器；补缺配置/失败 → Error 的可测路径（不必真 TTY）。
5. 同步 `bash-mode.md` / `keybindings.md`：产品严格无默认编辑器（与 demo 可保留 nano 默认的差异写明）。

## Capabilities

- `app-tui-input`（modify `ati17`）

## Design SSOT

- [`design.md`](./design.md)
- [`bash-mode.md`](../../../src/app/tui/design/bash-mode.md) § 外部编辑器
- [`keybindings.md`](../../../src/app/tui/design/keybindings.md)
- [`errors.md`](../../../src/app/tui/design/errors.md)

## Impact

- `src/app/tui/host`（及产品侧 tempfile/spawn 辅助）；**不**改 `packages/xylitol-tui` 行为（仅调用已有 `with_terminal_suspended`）

## Out of scope

- 包内 `$EDITOR`/tempfile/spawn；与 `agent_demo` 代码抽共享
- GUI 编辑器；computer-use
- 改 demo 的 nano 默认策略（demo 可继续独立）

## Ethics

- risk_level: medium（真终端副作用）
- prohibited_actions: CI/harness 默认 spawn `$EDITOR`；包内实现 tempfile/spawn；未配置时静默默认编辑器；panic 代替 Error 行
- required_evidence: harness stub 绿；缺配置 → `UiEntry::Error` 可测；手动 TTY 备注可接受

## Depends

- **c625**（已归档）；与树 power / models **无硬依赖**
