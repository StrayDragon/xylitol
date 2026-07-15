# ACCEPTANCE — c1015-add-app-tui-session-panel-slash

## Automated

```bash
just fmt
cargo test --lib app::tui::harness::tests::h31_slash_session_dumps_stats -- --nocapture
cargo test --lib app::tui::harness::tests::h32_slash_session_resume_list_switch_and_esc -- --nocapture
cargo clippy -p xylitol --lib -- -D warnings
llman sdd validate c1015-add-app-tui-session-panel-slash --strict --no-interactive
```

## Human hand-test (debug build)

1. `cargo run` → idle TUI with an active session.
2. `/session` → scrollback/system block with Session Info + Messages counts (not an ops menu); `/session info` → usage error.
3. `/session-resume` → editor-slot session SelectList (name + id when named); **Esc** closes without switch.
4. Pick another session → **Enter** → `switched → session …` note + transcript rebuild; tree/other slots closed.
5. `/resume` → unknown command (old name invalid).
6. While agent busy: `/session` and `/session-resume` → busy rejection notes.

## E2E (optional)

`just test-tui-e2e-pty` — smoke that slash completion lists `session` / `session-resume` and bare commands do not crash the PTY loop.
