# c90-refactor-tui-core Tasks

## Phase A — Foundation

- [ ] **A1 — Cargo.toml**: add `pulldown-cmark 0.12`, `tui-textarea 0.7` behind `ui-tui` feature; remove `termimad` from `ui-tui`
  - Verify: `cargo build --features ui-tui` compiles
- [ ] **A2 — Create `component.rs`**: define `Component` trait (render, is_dirty, mark_clean, handle_event) + `OverlayStack` with push/pop/input-routing/z-order render
  - Verify: `cargo check --features ui-tui`
- [ ] **A3 — Create `event.rs`**: define `TuiEvent` (Agent/Key/Tick/Shutdown) and `AppAction` (RunPrompt/Interrupt/Clear/SetDiff/QueuePrompt) enums
  - Verify: `cargo check --features ui-tui`
- [ ] **A4 — Create `mod.rs`**: module root, re-export `run_tui()`
  - Verify: `cargo check --features ui-tui`

## Phase B — Core Components

- [ ] **B1 — Create `markdown.rs`**: `MarkdownRenderer` wrapping `pulldown_cmark::Parser`, mapping events to ratatui Spans/Lines. Support headings, bold/italic, inline code, fenced code blocks (syntect), lists, links, blockquotes, horizontal rules
  - Verify: `cargo check --features ui-tui`
- [ ] **B2 — Create `input.rs`**: `InputComponent` wrapping `tui_textarea::TextArea`. Enter = submit, Shift+Enter = newline. Readline bindings (Ctrl+K/U/W/A/E/Left/Right). Slash command detection
  - Verify: `cargo check --features ui-tui`
- [ ] **B3 — Create `chat.rs`**: `ChatComponent` with scrollable `Vec<Message>`, streaming delta append, markdown rendering, role labels, scroll with j/k/↑/↓/g/G, scroll indicator, auto-scroll to bottom
  - Verify: `cargo check --features ui-tui`
- [ ] **B4 — Create `status_bar.rs`**: `StatusBar` showing running/ready state, model name, token count (input/output), session name
  - Verify: `cargo check --features ui-tui`
- [ ] **B5 — Create `overlays/` module**: `mod.rs` (OverlayStack integration), `help.rs` (keyboard reference), `selector.rs` (generic list with up/down/enter/esc)
  - Verify: `cargo check --features ui-tui`

## Phase C — Event Loop & Integration

- [ ] **C1 — Create `app.rs`**: `App` struct owning all components + `OverlayStack`. Elm-like `update(event) -> Option<AppAction>` method. `render(frame)` with dirty-flag gating + synchronized output
  - Verify: `cargo check --features ui-tui`
- [ ] **C2 — Implement `run_tui()` in `app.rs`**: terminal setup (alternate screen, raw mode, synchronized output), spawn blocking key reader, tokio::select! on agent_rx/key_rx/tick, wire AppAction → agent_loop.run()/abort()/queue
  - Verify: `cargo check --features ui-tui`
- [ ] **C3 — Create `history.rs`**: `HistoryStore` reading/writing `~/.xylitol/history` (newline-delimited), max N entries, `add()`, `iter()`, `search()`
  - Verify: `cargo test`
- [ ] **C4 — Create `slash.rs`**: `SlashCommand` parser + `Completer` with prefix-tree matching for /clear/help/quit
  - Verify: `cargo test`

## Phase D — Verify & Polish

- [ ] **D1 — Delete old files**: remove `src/interface/tui/` entirely (all 12 legacy files), keep `src/interface/diff_review/` intact
  - Verify: `cargo build --features ui-tui` compiles
- [ ] **D2 — Wire up**: confirm `App::new()` receives `ToolRegistry`, `AppConfig`, `ResolvedProfile`, `SessionService` — same signature as the old `run_tui()`
  - Verify: `cargo run` starts TUI, `cargo test` passes, `just qa` passes
