# Tasks: c2071-update-app-tui-host-mode-b-only

> **延后**：待 [`c2070`](../c2070-add-package-tui-dual-interaction-modes/) 库收口归档后再 Branch binding / Specs landing。
> 本文件为规划壳；**禁止**在 `sdd/c2070-…` 分支改 live `ath30`。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 1 Specs landing ath30 | ⬜ | ath30 → 产品 B-only |
| 2 产品 host 默认 B | ⬜ | 代码 + harness |
| 3 文档 / 人验签字 | ⬜ | AGENTS / D16 / 终端清单 |
| 4 validate / verify | ⬜ | — |

---

## 1. Specs landing — ⬜

- [ ] 1.1 Branch binding：`change start`（干净树、自默认分支；**勿**绑到 c2070 分支）
- [ ] 1.2 改写 live `ath30`：产品 MUST 固定 ApplicationOwned；废「默认 Mode A」
- [ ] 1.3 同步相关 `.feature` / scenario binding（若有）
- [ ] 1.4 确认库 `ptim01`/`ptim08`/`ptim14` 措辞仍允许库暴露 Inline（不误读成「库删 A」）

## 2. 产品 host 默认 B — ⬜

- [ ] 2.1 `TuiRunOptions`（或等价）默认 `InteractionMode::ApplicationOwned`
- [ ] 2.2 产品单测 / Fake / PTY 期望跟默认 B
- [ ] 2.3 Mode B 生命周期接线：alt begin/end、mouse、suspend；走 c2070 文档化的 ptim14 host 清单（**不**抄 demo 私有胶）
- [ ] 2.4 Copied / 误触：若本批一并接产品 chrome，钉落点；否则显式 defer 并保持 ath31 信号可用

## 3. 文档与人验 — ⬜

- [ ] 3.1 `src/app/tui/AGENTS.md` + PI_DELTAS（D16 等）改「产品 B-only」
- [ ] 3.2 终端签字清单（Ghostty / tmux / SSH 等）谁签、是否挡合并

## 4. 收口 — ⬜

- [ ] 4.1 `llman sdd validate --strict`；verify 双轴无 CRITICAL
- [ ] 4.2 确认未越界实现 fold / viewport slice

## 实现顺序

```text
c2070 archive（库双入口 + ptim14）
  → 1 Specs landing ath30
  → 2 host 默认 B + harness
  → 3 文档 / 人验
  → 4 validate / verify → archive
```
