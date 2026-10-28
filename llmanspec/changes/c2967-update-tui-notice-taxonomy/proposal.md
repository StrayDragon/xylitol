---
depends_on: []
title: Classify TUI notices into transient toast, client confirmation and durable scroll
needs_specs_change: true
---

# Classify TUI notices into transient toast, client confirmation and durable scroll

## Why

The TUI currently has three notice surfaces (toast, top-right client stack, scroll notice) without a written rule for which information belongs where. The heuristic-token notice is redundant with the footer `~` marker and adds noise on every process start. Durable outcomes can be lost when they are shown only as a transient toast.

## What Changes

- **Remove the heuristic estimate notice** entirely: `HEURISTIC_ESTIMATE_NOTICE`, `maybe_heuristic_estimate_notice`, the once-per-process flag and all call sites. The footer `~` already marks heuristic provenance.
- **Codify notice taxonomy**:
  - **Toast**: blocking rejection / soft gate / immediate failure that needs action now (reload wait, busy-session switch refusal, link down/recovered). One slot, `Error:` prefix, TTL, never conversation content.
  - **Client stack**: immediate confirmations only (Copied, Copied Last Msg). `✓` prefix. No informational essays.
  - **Scroll notice**: durable navigation/outcome/error that should remain readable in the transcript (rename/delete/import/export/fork results, model/thinking failures, compaction/retry details).
  - **System message**: durable information that must enter agent context; not a notice surface.
- Audit call sites and move any durable outcome currently shown as toast to scroll notice.
- Update `r1227` / `r1930` and add a `notice-taxonomy` rule.

## Capabilities

- `app-tui-fixed-zone`: toast / client stack rules and taxonomy.
- `app-tui-transcript`: durable outcomes use scroll notices.
- `app-tui-host`: event-to-notice classification.

## Impact

- `layout/root/mod.rs`: remove heuristic notice state and method.
- `host/mod.rs` / `effects`: remove heuristic calls.
- Tests: delete heuristic-notice tests; add taxonomy classification tests.
- DESIGN.md / AGENTS: notice taxonomy summary.

## Out of Scope

- New notice UI surfaces or animations.
- Moving existing scroll notices into a separate panel.
- Print surface notice behavior.

## Further Notes

- The footer `~` and `?%` already encode heuristic provenance; a separate notice duplicates that signal.
- Taxonomy is normative: a durable outcome must not be toast-only, and a transient gate must not create scroll noise.
