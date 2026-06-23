# c82-fix-tui-core Tasks

- [x] Fix event loop starvation: add `agent_done` flag, use `pending()` for exhausted agent stream
- [x] Fix user message display: create `TranscriptEntry::User` in `MessageStart { role: "user" }`
- [x] Add scroll state to Transcript: `scroll_offset`, `scroll_up`, `scroll_down`, `reset_scroll`
- [x] Implement scroll-aware rendering in `transcript_renderer.rs`
- [x] Add scroll keybindings and actions (PgUp/PgDn, Up/Down in Transcript focus)
- [x] Wire model name from CLI through to composer status bar
- [x] Fix composer cursor on empty draft: move CURSOR_MARKER before the placeholder so the cursor sits right after the prompt (r4 empty-cursor)
- [x] Fix differential render ghosting: no leftover/stale characters across composer text changes (r5 no-leftover)
- [x] Fix composer dropping typed whitespace (spaces/tabs): render draft verbatim instead of via wrap_text which skips whitespace runs; fix strip_ansi to handle BEL-terminated CURSOR_MARKER so width math is correct
- [x] Add unit tests covering composer cursor marker placement, differential render overwrites, and whitespace preservation (guards r4/r5)
- [ ] Manual smoke test: typing, submission, scrolling, Ctrl+C/D quit
- [x] `just fmt && just lint && just test`
- [ ] `llman sdd validate c82-fix-tui-core --strict --no-interactive`
