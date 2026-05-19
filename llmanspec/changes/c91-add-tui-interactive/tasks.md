# c91-add-tui-interactive Tasks

- [ ] **1 — Async input queue**: modify `app.rs` `handle_key` to not disable input when running; add `queued_prompts: Vec<String>` to App; on agent completion, pop & auto-submit next; add `[Q: N]` indicator to StatusBar
  - Verify: `cargo check --features ui-tui`
- [ ] **2 — Wire selectors to real data**: modify `App::new()` to accept `&AppConfig` and `SessionService`; populate model_selector from `config.agent.models.keys()`, session_selector from `session_service.list_sessions()` (async), theme_selector from syntect `ThemeSet::load_defaults()`
  - Verify: `/model` opens list with real model names
- [ ] **3 — Ctrl+G editor integration**: in `input.rs` handle key chord; write buffer to temp file; spawn `$EDITOR` (or `vim`); wait for exit; read file back into buffer
  - Verify: `cargo test` + manual Ctrl+G test
- [ ] **4 — Ctrl+R history search overlay**: new `overlays/history_search.rs`; read `HistoryStore` entries; render fuzzy search popup with ↑/↓ selection; Enter loads selection into input
  - Verify: `cargo test` + manual Ctrl+R test
- [ ] **5 — Mouse interaction**: in event loop, handle `crossterm::event::Event::Mouse`; scroll wheel = `chat.scroll_up/down`; click = detect area from coordinate → `focus = area`
  - Verify: manual mouse scroll + click
- [ ] **6 — Focus visual effects**: in `app.rs` render, pass focus info to components; focused component gets `Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)` on border/title; unfocused gets dim `Color::DarkGray`
  - Verify: Tab cycles focus with visible border color change
- [ ] **7 — Pane resize**: add `Ctrl+Up/Down` key handling; adjust tool panel height constraint dynamically (min 0, max 50% area); store `tool_panel_height: u16` in App
  - Verify: `cargo build --features ui-tui` + manual test
