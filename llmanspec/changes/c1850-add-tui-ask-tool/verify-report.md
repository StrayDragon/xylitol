# Verify — c1850-add-tui-ask-tool

Date: 2026-08-02

## Gate

| Check | Result |
|---|---|
| `readyToImplement` | true |
| `llman sdd validate --strict --no-check` | pass |
| `cargo test --lib ask` | 7 pass |
| `cargo test --test bdd -- app_tui_ask` | 7 pass |
| `cargo test -p xylitol-tui choice_prompt` | 14 pass |
| Trust path | unchanged (bootstrap `trust_gate.rs`) |

## Specs ↔ code

| Req | Verdict |
|---|---|
| ata1 TUI-only registration | PASS — `install_ask_tool` / `default_tools_with_ask`; Print omits |
| ata2 Choice slot | PASS — `UiRoot::mount_ask_choice` / Choice slot render+input |
| ata3 skip success | PASS — gateway returns `to_ask_payload_json` skipped |
| ata4 answered payload | PASS — answered JSON via ChoiceResult |
| ata5 scrollback rail | PASS — `UiEntry::Ask` + rail paint |
| ata6 ≠ Trust | PASS — Trust gate separate |
| t28 ask not in default_tools | PASS — unit + BDD |
| pcp06–08 | PASS — package tests already green |

## OTEL / Langfuse

| Item | Verdict |
|---|---|
| Per-tool ask span | NOT required |
| Inheritance | PASS — `tool_exec::run_one` wraps all tools with `ToolExecuteSpan` (`tool_name=ask`, `langfuse.observation.type=tool`) |
| Doc | Comment on `src/infra/tools/ask.rs` |

## CRITICAL

None.

## WARNING

- Full live agent→ask→Choice E2E harness not added (BDD is seam-level) — acceptable for MVP.
- Full `just qa` / `--lib` parallel flake noise noted in tasks; ask-related gates green.

## SUGGESTION

- Optional later: Langfuse event `waiting_user` on ask hang; WS `AnswerQuestion` alignment.

## Conclusion

Verify **PASS** — ready to finalize/archive.
