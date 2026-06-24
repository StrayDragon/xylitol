---
depends_on: [c82-fix-tui-core]
---

# c83-migrate-tui-to-ratatui

## Why

c82-fix-tui-core fixed three P0 blocking bugs in the TUI, but the underlying rendering architecture remains fundamentally unsuitable for interactive use.

The current TUI uses a **self-built ANSI differential renderer** (pi-style engine) that was designed for streaming agent output, **not** for interactive terminal applications. The problems are systemic:

### 1. Layout computation is unreliable

`compose_layout()` does simple line-count-based clipping of transcript output and appends composer lines. When transcript lines exceed or fall short of the available viewport, composer content overlaps or leaves gaps. There is no proper constraint-based layout.

### 2. Cursor management is ad-hoc

The cursor position is tracked via `CURSOR_MARKER` string sentinel, which is searched for and replaced in rendered output. This breaks under scrolling, multi-line input, and diff-based redraw.

### 3. ANSI diff rendering is not suitable for interactive use

`TuiRenderer` maintains a `prev_lines: Vec<String>` and writes ANSI escape sequences line-by-line. This works for one-directional output but fails when:
- The user types characters and the cursor position must update precisely
- The transcript scrolls and old content must reappear
- Focus changes and visual indicators must update instantly

### 4. No proper terminal state management

No alternate screen, no scrolling regions, no synchronized update wrappers. Terminal history is polluted with TUI output. Scrollback is unreliable.

### 5. Premature agent startup

`run_tui_engine()` calls `agent_loop.run(prompt, session_id).await` with `prompt = ""` on startup, triggering a model API call before the user has typed anything.

### 6. Ratatui dependencies already present

`Cargo.toml` already lists `ratatui`, `crossterm`, and `ratatui-textarea` as dependencies. The spec (`tui-interface/spec.toon`) already describes a ratatui-based architecture in requirements r1, r2, r6, r41, r45, etc. The state layer (`state/`) is already designed as a pure ratatui-independent domain layer.

## What Changes

### Phase 1: Replace engine/ rendering with ratatui widgets

| File | Action | Description |
|------|--------|-------------|
| `src/interface/tui/engine/mod.rs` | Rewrite | Replace `compose_layout()` with ratatui `render()` function operating on `Frame` |
| `src/interface/tui/engine/transcript_renderer.rs` | Rewrite | Render `Transcript` entries as ratatui `Paragraph`/`List` widgets, scrollable via `ScrollbarState` |
| `src/interface/tui/engine/composer_renderer.rs` | Rewrite | Render `Composer` into ratatui `Paragraph` + status bar as ratatui layout |
| `src/interface/tui/engine/ansi.rs` | Keep (or prune) | Keep ANSI helpers for markdown rendering; remove cursor/erase helpers no longer needed |
| `src/interface/tui/engine/diff.rs` | Delete | No longer needed — ratatui handles frame diffs natively |
| `src/interface/tui/engine/renderer.rs` | Delete | No longer needed — ratatui `Terminal` handles diff rendering |

### Phase 2: Rewrite event loop in mod.rs

| Action | Description |
|--------|-------------|
| Use `ratatui::Terminal` with `crossterm::tty::TtyStream` | Standard ratatui init pattern |
| Replace custom diff render loop | `terminal.draw(\|f\| render(app, f))` per frame |
| Use alternate screen | `enable_raw_mode` + `EnterAlternateScreen` on start, reverse on exit |
| Remove premature agent startup | Only create agent stream when user submits first message |
| Use `synchronized_update` for flicker-free rendering | Wrap frame writes in `BeginSynchronizedUpdate`/`EndSynchronizedUpdate` |
| Keep `tokio::select!` with 3 branches | agent, input, frame_timer — same structure, ratatui draw instead of custom render |

### Phase 3: input/ and state/ remain unchanged

- `input/keymap.rs` — pure function, unchanged
- `input/action.rs` — minor: actions like `ScrollUp`/`ScrollDown` may be replaced by ratatui `ScrollbarState`
- `input/decode.rs` — pure function, unchanged
- `state/transcript.rs` — unchanged, fully ratatui-independent
- `state/composer.rs` — unchanged, but `TextArea` from `ratatui-textarea` will be wired in the render layer
- `state/focus.rs` — unchanged

### Phase 4: Fix premature agent startup

- `mod.rs`: remove `let mut agent_stream = agent_loop.run(prompt, ...).await` from initialization
- Only start agent stream when `pending_prompt.take()` yields a user-submitted message
- `cli/mod.rs`: `run_tui_engine` signature simplified (no prompt parameter needed)

## Capabilities

- `tui-interface`

## Impact

- **Modified**: `src/interface/tui/mod.rs`, `src/interface/tui/engine/*.rs`
- **Deleted**: `src/interface/tui/engine/diff.rs`, `src/interface/tui/engine/renderer.rs`
- **Unchanged**: `src/interface/tui/state/*`, `src/interface/tui/input/*`
- **Modified**: `src/interface/cli/mod.rs` (remove prompt arg from TUI call)
- **Zero impact**: agent loop, session, config, tools, BDD tests
- **All 112 existing TUI unit tests** for state/ and input/ must continue to pass unchanged
