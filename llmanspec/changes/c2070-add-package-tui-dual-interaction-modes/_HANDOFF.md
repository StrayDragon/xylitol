# Handoff — c2070 dual interaction modes

**Branch:** `sdd/c2070-add-package-tui-dual-interaction-modes`
**Demos:** `just demo-tui` (Inline) · `just demo-tui-alt-screen` (ApplicationOwned) · `just demo-tui-host-loop`
**Last sync:** 2026-08-12 — human PASS + 7.4 verify green → ready to archive

## Naming (code SSOT)

| Prefer | Avoid (informal) |
|---|---|
| `InteractionMode::Inline` | Mode A |
| `InteractionMode::ApplicationOwned` / alt-screen | Mode B |
| `set_append_session_to_main_scrollback_on_exit` | `*_exit_dump` |

## Done

- Library ApplicationOwned foundation + facade + ptim14 checklist + host example
- Human verify PASS; PTY bang/submit fixed (`wait_for_raw` + fake `api_key`)
- Product B-only deferred → [`c2071`](../c2071-update-app-tui-host-mode-b-only/)
- Verify: [`_VERIFY.md`](./_VERIFY.md)

## Next

1. Commit remaining work on this branch
2. `llman-sdd-archive` for c2070 (when you ask)
3. Later: c2071 product default ApplicationOwned

## Do not

- Flip ath30 / product default on this change
- Kill Inline engine
- Reintroduce `XYLITOL_AGENT_DEMO_MODE`
