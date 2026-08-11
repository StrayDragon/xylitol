# Handoff — c2070 dual interaction modes

**Branch:** `sdd/c2070-add-package-tui-dual-interaction-modes`
**Demos:** `just demo-tui` (Inline) · `just demo-tui-alt-screen` (ApplicationOwned) · `just demo-tui-host-loop`
**Last sync:** 2026-08-12 — concept vocab + no live mode-switch; ready to archive when asked

## Naming (code SSOT)

| Prefer | Avoid (informal) |
|---|---|
| `InteractionMode::Inline` | Mode A |
| `InteractionMode::ApplicationOwned` / alt-screen | Mode B |
| `set_append_session_to_main_scrollback_on_exit` | `*_exit_dump` |

术语 ↔ 代码：[`research/emulator-vs-app-selection-oneof.md`](./research/emulator-vs-app-selection-oneof.md) §1。

## Product decision (2026-08-12)

- **No mid-session live switch** Inline ↔ ApplicationOwned. Mode is bound at `HostSession` / `TUI` construction (`new_product_ui_with_meta_mode` / `with_interaction_mode`).
- Removed product API `HostSession::apply_interaction_mode` (was restack hot-switch).
- 「退出后主屏仍能翻到会话」靠 `finish` / `finish_application_owned` dump，**不是**切回 Inline。
- Product default stays **Inline** until [`c2071`](../c2071-update-app-tui-host-mode-b-only/) (AO-only + dump UX).

## Done

- Library ApplicationOwned foundation + facade + ptim14 checklist + host example
- Human verify PASS; PTY bang/submit fixed
- Vocab: docs / live specs use Inline·ApplicationOwned; finish() dispatcher; dock/clipboard/suspend polish
- Product: start-time mode only; ath30/ptim01 forbid hot-switch

## Next

1. `llman-sdd-archive` for c2070 (when you ask)
2. c2071: product default ApplicationOwned + exit-leave-scrollback UX

## Do not

- Flip ath30 default to ApplicationOwned on this change (that is c2071)
- Kill Inline engine / library dual construction entry
- Reintroduce `XYLITOL_AGENT_DEMO_MODE` or `apply_interaction_mode`
- Reintroduce mid-session mode restack as a product feature
