# Handoff — c2070 dual interaction modes (Mode B)

**Branch:** `sdd/c2070-add-package-tui-dual-interaction-modes`
**Demo Inline:** `just demo-tui` → `--example agent_demo`
**Demo Mode B:** `just demo-tui-alt-screen` → `--example agent_demo_alt` (**no** `XYLITOL_AGENT_DEMO_MODE`)
**Shared impl:** `packages/xylitol-tui/examples/agent_demo_impl.rs`
**Last sync:** 2026-08-12 — **产品 B-only 迁出 → [`c2071`](../c2071-update-app-tui-host-mode-b-only/)**；本分支专攻库基础

## Strategy lock (human 2026-08-12)

| Layer | Decision |
|---|---|
| Product app | **B-only** — **deferred [`c2071`](../c2071-update-app-tui-host-mode-b-only/)**（勿在本分支改 ath30） |
| Library | Dual entry / separately managed — **this branch** |
| Demo | **Two examples** — done |
| E2E | Mode B PTY minimal — done; tmux SHOULD |

## Next (c2070 only)

1. **7.8** 双入口结构极致化（ApplicationOwned 抽离、teardown 命名、dump opt-out、Editor 单一 remap）
2. **7.3** ptim14 库 host 接入清单（产品可抄）
3. **7.9**（可选）最小 host 环 example
4. **7.4** validate / verify → archive
5. 然后另开分支做 **c2071**

## Do not

- 本分支改 live `ath30` / 产品默认 InteractionMode
- Reintroduce `XYLITOL_AGENT_DEMO_MODE`
- Kill library Inline engine
- Share `CARGO_TARGET_DIR` across worktrees
