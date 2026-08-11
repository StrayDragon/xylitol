# Handoff — c2070 dual interaction modes (Mode B)

**Branch:** `sdd/c2070-add-package-tui-dual-interaction-modes`
**Demo Inline:** `just demo-tui` → `--example agent_demo`
**Demo Mode B:** `just demo-tui-alt-screen` → `--example agent_demo_alt` (**no** `XYLITOL_AGENT_DEMO_MODE`)
**Shared impl:** `packages/xylitol-tui/examples/agent_demo_impl.rs`
**Last sync:** 2026-08-12 — demo split + Mode B PTY gates (alt/OSC52/dump/wheel/dock/suspend)

## Strategy lock (human 2026-08-12)

| Layer | Decision |
|---|---|
| Product app | **B-only** (ath30 Specs landing still pending) |
| Library | Dual entry / separately managed |
| Demo | **Two examples** — done |
| E2E | Mode B PTY minimal (alt/mouse/dump + OSC52 drag) — done; expand matrix later |

## Next

1. Specs landing ath30 → product B-only (7.5) + host wire
2. 7.3 ptim14 docs / AGENTS host checklist
3. 7.8 dual-entry structural debt (optional parallel)
4. 7.4 validate / verify
5. Expand e2e: dock clamp / wheel sticky / suspend (SHOULD)

## Do not

- Reintroduce `XYLITOL_AGENT_DEMO_MODE`
- Kill library Inline engine
- Share `CARGO_TARGET_DIR` across worktrees
