---
change_id: c1570-update-tui-ctrl-c-busy-abort
title: busy 时 Ctrl+C 等同 Esc abort；idle 保持清空/再退出
status: purpose-draft
priority: 1570
depends_on: []
author: agent
---

# c1570-update-tui-ctrl-c-busy-abort

> **阶段**：purpose-draft。拆自一批 UX；**正交**于 c1560、c1565。
> **决策已收敛** — Open Questions 已清空。

## Why

现合约 `ati2` / design：Ctrl+C 在 editor 非空时清空、为空时退出，**不区分 idle/busy**。busy 时空 editor 一次 Ctrl+C 会整窗退出；用户期望第一次只停止对话（等同 Esc），避免误杀。

## 代码事实

- `UiRoot::on_ctrl_c`：overlay 关槽 → 非空清空 → 空则 `quit_flag`；**busy 不拦**
- busy Esc：`app.interrupt` → `pending.abort` → `Driver::abort`（`ati2` 前半已要求）

## Decisions（已锁）

| 状态 | Ctrl+C |
|---|---|
| **busy**（`is_busy`）且无 overlay | **等同 Esc**：abort + 既有 abort UI/suppress；**MUST NOT** 退出进程 |
| **idle** + editor **非空** | **先清空**（保留现状；用户已选） |
| **idle** + editor **空** | **退出** TUI |
| overlay 开 | 先关槽（现状） |

- 修订 `ati2` + `design/keybindings.md`
- harness：busy Ctrl+C → abort 且不 quit；idle 非空 → clear；idle 空 → quit

## What Changes（意向）

- Host / `on_ctrl_c`（或 busy 路径抢先消费 Ctrl+C）接线 abort
- specs + harness / BDD

## Non-Goals

- 改 Esc 语义、双 Esc 开树
- CLI 旗标 / resume 文案 / 历史种子

## Open Questions

（已清空 — idle 非空「先清空」已确认。）

## Related

- `app-tui-input`（`ati2`）、`app-tui-host`
- `src/app/tui/design/keybindings.md`
- 并行 draft：`c1560`、`c1565`

## Ethics

- `ethics.risk_level`: low
- `ethics.prohibited_actions`: draft 阶段写应用代码
- `ethics.required_evidence`: busy/idle Ctrl+C harness
- `ethics.escalation_policy`: 无

## Next

现有 SDD 流水线告一段落后再 `/llman-sdd-propose` promote（无阻塞 OQ；可与 c1560/c1565 并行）
