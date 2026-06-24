# c82-fix-tui-core Tasks

- [x] Fix event loop starvation: add `agent_done` flag, use `pending()` for exhausted agent stream
- [x] Fix user message display: create `TranscriptEntry::User` in `MessageStart { role: "user" }`
- [x] Add scroll state to Transcript: `scroll_offset`, `scroll_up`, `scroll_down`, `reset_scroll`
- [x] Implement scroll-aware rendering in `transcript_renderer.rs`
- [x] Add scroll keybindings and actions (PgUp/PgDn, Up/Down in Transcript focus)
- [x] Wire model name from CLI through to composer status bar
- [ ] Manual smoke test: typing, submission, scrolling, Ctrl+C/D quit
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c82-fix-tui-core --strict --no-interactive`
