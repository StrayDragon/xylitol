---
change_id: c1570-update-tui-ctrl-c-busy-abort
title: busy 时 Ctrl+C 等同 Esc abort；idle 保持清空/再退出
status: purpose-draft
priority: 1570
depends_on: []
author: agent
---

# c1570-update-tui-ctrl-c-busy-abort

> **决策已全部锁定**。延后于 c1560；正交其它 UX/bug draft。

## Why

busy 时空 editor 一次 Ctrl+C 会退出；应等同 Esc abort。

## Decisions（已锁）

| 状态 | Ctrl+C |
|---|---|
| **agent busy** 且无 overlay | 等同 Esc：`Driver::abort`；**MUST NOT** 退出 |
| **bash busy（`!`/`!!`）** 且无 overlay | 等同 Esc：abort bang；**MUST NOT** 退出 |
| **idle** + editor 非空 | 先清空 |
| **idle** + editor 空 | 退出 TUI |
| overlay 开 | 先关槽（不 abort） |

## Open Questions

（已清空。）

## Related

- `app-tui-input` `ati2`、`design/keybindings.md`

## Ethics

- `ethics.risk_level`: low
- `ethics.required_evidence`: busy/idle/bash Ctrl+C harness
