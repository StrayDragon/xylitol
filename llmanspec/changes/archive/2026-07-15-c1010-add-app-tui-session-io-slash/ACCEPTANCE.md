# ACCEPTANCE — c1010-add-app-tui-session-io-slash

## Automated

```bash
just fmt
cargo test --lib app::tui::harness::tests::h28_slash_session_compact_and_usage -- --nocapture
cargo test --lib app::tui::harness::tests::h29_slash_session_export_html_and_jsonl -- --nocapture
cargo test --lib app::tui::harness::tests::h30_slash_session_import_confirm_cancel_and_accept -- --nocapture
cargo clippy -p xylitol --lib -- -D warnings
llman sdd validate c1010-add-app-tui-session-io-slash --strict --no-interactive
```

## Human hand-test (debug build)

1. `cargo run` → idle TUI.
2. `/session-compact` → system note (compacted or unchanged); `/session-compact foo` → usage error, no compact.
3. `/session-export` → note with `export.html`; `/session-export /tmp/x.jsonl` → JSONL path in note.
4. `/session-import /path/to.jsonl` → Yes/No in editor slot; **No** or **Esc** → `Import cancelled`; **Yes** → switch + transcript rebuild + imported note.
5. `/compact`, `/export`, `/import` → unknown command.
6. While agent busy: all three → busy rejection notes.
