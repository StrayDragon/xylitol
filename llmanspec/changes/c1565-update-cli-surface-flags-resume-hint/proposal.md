---
change_id: c1565-update-cli-surface-flags-resume-hint
title: 表面旗标下沉到 tui/print + 退出时条件打印 resume 提示
status: purpose-draft
priority: 1565
depends_on: []
author: agent
---

# c1565-update-cli-surface-flags-resume-hint

> **阶段**：purpose-draft。拆自一批 UX；**正交**于 c1560（历史种子）、c1570（Ctrl+C）。
> **决策已收敛** — Open Questions 已清空。

## Why

1. 顶层全局 `--session/--model/--list-models/--trust/--no-trust/--config/--no-color` 与「表面动词 `tui` / `print`」分层不一致；裸 `xylitol --session` 易踩空，且全局旗标会**同时污染**所有表面。
2. 正常 `/exit` 或退出后，没有一行可复制的 resume 命令；未落盘 session 也不该伪造提示。

## 代码事实

- 旗标在顶层 `CliArgs`；`xylitol tui` / `tui run` / `print` 自身无这些 arg
- bootstrap 总分配 Uuid；`store.exists` 为真才算「有 session 可 resume」
- TUI 退出：`finish_inline` 后无 resume 文案
- `--config` 进 loader；`--no-color` 已挂 CLI（print/TUI 共用顶层）

## Decisions（已锁）

### D1. 统一体验 = **每表面自带旗标**（非顶层全局）

从顶层 **移除**表面相关选项；按表面挂载：

**`xylitol tui`（建议挂在 `Tui` 命令上，使 `tui --session` 与 `tui run --session` 等价）：**

- `--session`
- `--model`
- `--list-models`
- `--trust` / `--no-trust`
- `--config`
- `--no-color`

**`xylitol print`：**

- `--session`
- `--model`
- `--config`
- `--no-color`

TTY 裸跑 `xylitol`（无子命令）仍默认进 TUI（`ce12`），但 **不再**接受上述顶层旗标 → 显式 override / resume 走 `xylitol tui …`（或 `print --session` 等）。

`resources` / `tokenizer` / `server`：**不**吞这些表面旗标（`ce16` 管理动词保持顶层、不进 tui 子树）。

### D2. 颜色旗标形状

**A（已锁）**：`tui` 与 `print` 均挂 `--no-color`（原语义最小迁移；从顶层下沉到表面）。

### D3. 退出 resume 提示

- 正常退出（`/exit`、idle 确认退出的 Ctrl+C）在终端 restore 之后，若 `store.exists(session_id)`，向 **stderr** 打印：
  - `Resume by $ xylitol tui --session <session uuid>`
- 未持久化 → **MUST NOT** 打印
- 硬失败路径不强制盖住真实错误

## What Changes（意向）

- clap：顶层剥离；`Tui` / `Print` 接收表面旗标；单测/help 断言
- 修订 `cli-entry`（及 preflight 文案里的 `--model` 指向）
- TUI 退出路径条件打印 resume 行 + 测例

## Non-Goals

- Editor 历史种子（c1560）
- Ctrl+C busy 语义（c1570）
- 把 ops 动词挪进 `tui` 子树
- 改名为 `--color` 互斥对（明确不做；保持 `--no-color`）

## Open Questions

（已清空。）

## Related

- `cli-entry`（`ce12`/`ce16`）、`cli-print`、`app-tui-host`（退出）
- 提示文案依赖本 change 的 `tui --session` 形状（与 c1560 无硬依赖）
- 并行 draft：`c1560`、`c1570`

## Ethics

- `ethics.risk_level`: low（CLI 破窗；文档/脚本若依赖顶层旗标需迁移）
- `ethics.prohibited_actions`: draft 阶段写应用代码；未 promote 前改 live MUST 大扫除
- `ethics.required_evidence`: `CliArgs` help 单测；resume 有/无 exists 两例
- `ethics.escalation_policy`: 外部脚本依赖顶层 `--model` 时，changelog 明示迁移

## Next

现有 SDD 流水线告一段落后再 `/llman-sdd-propose` promote → attach → apply
