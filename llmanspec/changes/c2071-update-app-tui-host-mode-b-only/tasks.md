# Tasks: c2071-update-app-tui-host-mode-b-only

> **延后**：待 [`c2070`](../c2070-add-package-tui-dual-interaction-modes/) 库收口归档后再 Branch binding / Specs landing。
> 本文件为规划壳；**禁止**在 `sdd/c2070-…` 分支把 `ath30` **默认翻成** ApplicationOwned（禁热切等收窄已在 c2070 落地）。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 1 Specs landing ath30 | ⬜ | ath30 → 产品默认 ApplicationOwned |
| 2 产品 host 默认 AO | ⬜ | 代码 + harness |
| 3 文档 / 人验签字 | ⬜ | AGENTS / D16 / 终端清单 |
| 4 validate / verify | ⬜ | — |

---

## 1. Specs landing — ⬜

- [ ] 1.1 Branch binding：`change start`（干净树、自默认分支；**勿**绑到 c2070 分支）
- [ ] 1.2 改写 live `ath30`：产品 MUST 固定/默认 ApplicationOwned；废「缺省 Inline」
- [ ] 1.3 同步相关 `.feature` / scenario binding（若有）
- [ ] 1.4 确认库 `ptim01`/`ptim08`/`ptim14` 仍允许库暴露 Inline；**勿**恢复热切叙事

## 2. 产品 host 默认 AO — ⬜

- [ ] 2.1 `TuiRunOptions` 默认 `InteractionMode::ApplicationOwned`
- [ ] 2.2 产品单测 / Fake / PTY 期望跟默认 AO
- [ ] 2.3 生命周期：构造走 `new_product_ui_with_meta_mode`；alt begin/end、mouse、suspend；ptim14 清单（**不**抄 demo 私有胶；**不**再引入 `apply_interaction_mode`）
- [ ] 2.4 Copied / 误触：若本批一并接产品 chrome，钉落点；否则显式 defer 并保持 ath31 信号可用

## 3. 文档与人验 — ⬜

- [ ] 3.1 `src/app/tui/AGENTS.md` + PI_DELTAS（D16 等）改「产品 AO-only」
- [ ] 3.2 终端签字清单（Ghostty / tmux / SSH 等）谁签、是否挡合并

## 4. 收口 — ⬜

- [ ] 4.1 `llman sdd validate --strict`；verify 双轴无 CRITICAL
- [ ] 4.2 确认未越界实现 fold / viewport slice / live 换模式

## 实现顺序

```text
c2070 archive（库双入口 + ptim14）
  → 1 Specs landing ath30
  → 2 host 默认 B + harness
  → 3 文档 / 人验
  → 4 validate / verify → archive
```
