# Verify — c2070-add-package-tui-dual-interaction-modes

**Date:** 2026-08-12 (final polish)
**Branch:** `sdd/c2070-add-package-tui-dual-interaction-modes`
**Human:** alt-screen demo H1–H5 + host loop + Editor/transcript selection + mutex — **PASS**
**Example build:** `agent_demo_alt` / `agent_demo` — **no warn**

## Gates

| Gate | Result |
|---|---|
| `interaction_modes_test` | 19 (incl. editor/transcript mutex + hit-priority) |
| BDD `package_tui_interaction_modes` | 8/8 (prior wave) |
| PTY (prior wave): submit CJK + product bang×3 | 4/4 (`--test-threads=1`) |
| Product default still Inline (`ath30`) | yes — AO-only → c2071 |
| blocks (fold / viewport slice) not implemented | yes — host hook only (`set_transcript_hit_priority`) |
| `llman sdd validate --strict` | green |

## 合约轴

- **CRITICAL:** none
- **WARNING:** ath30/ath31 GWT live under `package-tui-interaction-modes.feature` (cross-capability; measurable)
- Library ptim01–15 covered; product ath30 still default Inline
- Final polish: selection mutual exclusion; fold hit host seam for c2040

## 标准轴

- **CRITICAL:** none
- **SUGGESTION:** none remaining for pub naming — `Inline` / `ApplicationOwned` SSOT (AGENTS §硬约束 8).

## Downstream alignment (pre-archive)

- c2071 proposal: ptim14 API table (construct / dump / no hot-switch)
- c2040 proposal: hard-deps c2070 + `set_transcript_hit_priority` wiring note

## Verdict

**Ready to archive.** Product ApplicationOwned-only remains [`c2071`](../c2071-update-app-tui-host-mode-b-only/).
