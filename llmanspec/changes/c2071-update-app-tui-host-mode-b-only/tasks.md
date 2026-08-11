# Tasks: c2071-update-app-tui-host-mode-b-only

> **前置**：[`c2070`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/) 已归档。本分支 `sdd/c2071-update-app-tui-host-mode-b-only`。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 1 Specs landing ath30 | ✅ | ath30 → 产品默认 ApplicationOwned |
| 2 产品 host 默认 AO | ⬜ | 代码 + harness |
| 3 文档 / 人验签字 | ⬜ | AGENTS / D16 / 终端清单 |
| 4 validate / verify | ⬜ | — |

---

## 1. Specs landing — ✅

- [x] 1.1 Branch binding：`change start`（`sdd/c2071-…`）
- [x] 1.2 改写 live `ath30`：产品 MUST 固定/默认 ApplicationOwned；废「缺省 Inline」
- [x] 1.3 无 ath30 `.feature` GWT（unit scenario 保留；产品 harness 覆盖）
- [x] 1.4 库 `ptim01`/`ptim08`/`ptim14` 仍允许库暴露 Inline；**不**恢复热切

## 2. 产品 host 默认 AO — ⬜

- [ ] 2.1 `TuiRunOptions` 默认 `InteractionMode::ApplicationOwned`
- [ ] 2.2 产品单测 / Fake / PTY 期望跟默认 AO（改写原 `interaction_mode_defaults_to_inline`；harness H1–H9 **业务断言不变**，仅模式/finish 观测对齐 AO）
- [ ] 2.3 生命周期：构造走 `new_product_ui_with_meta_mode`；alt begin/end、mouse、suspend；ptim14 清单；`finish` 分发 AO teardown + dump（**禁止**为 AO 改 Driver/slash/键位语义）
- [ ] 2.4 Copied / 误触：ath31 信号已接 chrome；本批仅核对默认 AO 下路径
- [ ] 2.5（可选）avs1 / feature 文案：`finish_inline` → 模式无关的 `finish`/stop 表述

## 3. 文档与人验 — ⬜

- [ ] 3.1 `src/app/tui/AGENTS.md` + PI_DELTAS（D16 等）改「产品 AO-only」
- [ ] 3.2 终端签字清单（Ghostty / tmux / SSH）— 人验板；不挡 Specs；合并前按需签

## 4. 收口 — ⬜

- [ ] 4.1 `llman sdd validate --strict`；verify 双轴无 CRITICAL
- [ ] 4.2 确认未越界实现 fold / viewport slice / live 换模式

## 实现顺序

```text
1 Specs landing ath30 ✅
  → 2 host 默认 AO + harness
  → 3 文档 / 人验
  → 4 validate / verify → archive
```

## Open Questions（已钉）

| # | 钉 |
|---|---|
| lab/env 逃生回 Inline | **不做**产品开关；库 Inline 仅 lab/demo |
| dump opt-out | **不**暴露用户开关；沿用库默认 dump-on |
| 终端签字 | 任务 3.2；不挡 Specs landing / apply 起步 |
