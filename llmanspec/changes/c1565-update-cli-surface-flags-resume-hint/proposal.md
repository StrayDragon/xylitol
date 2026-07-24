---
change_id: c1565-update-cli-surface-flags-resume-hint
title: 表面旗标下沉到 tui/print + 退出时条件打印 resume 提示
status: purpose-draft
priority: 1565
depends_on: []
author: agent
---

# c1565-update-cli-surface-flags-resume-hint

> **决策已全部锁定**。延后于 c1560 full/apply；正交 c1570/c1580/c1585。

## Why

顶层全局旗标与表面动词分层不一致；退出后缺可复制 resume 行。

## Decisions（已锁）

### D1. 每表面自带旗标

顶层 **硬移除**（clap 报错）：`--session` `--model` `--list-models` `--trust` `--no-trust` `--config` `--no-color`。

**`xylitol tui`**（挂在 `Tui`，`tui --session` ≡ `tui run --session`）：上述全部（含 trust / list-models）。

**`xylitol print`**：`--session` `--model` `--config` `--no-color`（无 list-models / trust）。

TTY 裸跑仍默认 TUI（`ce12`），但不再接受顶层表面旗标。

### D2. 颜色

两侧均挂 `--no-color`（原语义）。

### D3. Resume 提示

TUI **与** print 正常退出且 `store.exists(session_id)` → stderr：
`Resume by $ xylitol tui --session <uuid>`；未持久化不打印。

## Open Questions

（已清空。）

## Related

- `cli-entry`、`cli-print`、`app-tui-host`

## Ethics

- `ethics.risk_level`: low（硬移除破窗；changelog 明示）
- `ethics.prohibited_actions`: draft 写应用代码
- `ethics.required_evidence`: help/parse 单测；resume 有/无 exists
- `ethics.escalation_policy`: 无
