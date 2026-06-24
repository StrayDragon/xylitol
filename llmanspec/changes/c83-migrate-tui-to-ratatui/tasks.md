# c83-migrate-tui-to-ratatui — Tasks

- [ ] Rewrite `engine/mod.rs`: replace `compose_layout()` with ratatui-style `render(app, f)` function
- [ ] Rewrite `engine/transcript_renderer.rs`: convert from `Vec<String>` output to ratatui Widget rendering transcript entries (Paragraph/List + ScrollbarState)
- [ ] Rewrite `engine/composer_renderer.rs`: convert from `Vec<String>` output to ratatui Widget rendering composer input + status bar
- [ ] Delete `engine/diff.rs` and `engine/renderer.rs` (no longer needed)
- [ ] Rewrite `mod.rs` event loop: use ratatui `Terminal`, alternate screen, remove premature agent startup
- [ ] Prune `engine/ansi.rs`: keep markdown-rendering helpers, remove cursor/erase helpers
- [ ] Verify `state/` and `input/` unit tests still pass (112 tests, zero modifications)
- [ ] Run full BDD regression: `cargo test --test bdd -- --test-threads=1`
- [ ] Manual smoke test: TUI startup, typing, submission, scrolling, Ctrl+D quit
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c83-migrate-tui-to-ratatui --strict --no-interactive`
