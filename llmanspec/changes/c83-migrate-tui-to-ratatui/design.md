# c83-migrate-tui-to-ratatui — Design

## Context

- c82-fix-tui-core fixed P0 bugs in the pi-style ANSI diff engine, but the architecture itself is unsuitable for interactive TUI
- Ratatui, crossterm, and ratatui-textarea are already listed as dependencies in Cargo.toml
- The existing `state/` and `input/` layers are already designed as pure functions with zero ratatui dependency
- The `tui-interface` spec already assumes ratatui-based rendering in r1, r2, r6, r41, r45

## Architecture

```
┌─────────────────────────────────────────────┐
│  src/interface/tui/                          │
│                                              │
│  state/ (unchanged)         input/ (mostly)  │
│  ┌─────────────────┐      ┌──────────────┐  │
│  │ Transcript      │      │ decode.rs    │  │
│  │ Composer        │      │ keymap.rs    │  │
│  │ FocusCtx        │      │ action.rs    │  │
│  └──────┬──────────┘      └──────┬───────┘  │
│         │                        │          │
│         ▼                        ▼          │
│  ┌──────────────────────────────────────┐   │
│  │  mod.rs (event loop)                │   │
│  │  - ratatui::Terminal setup          │   │
│  │  - tokio::select! agent/input/timer │   │
│  │  - terminal.draw(|f| render(app,f)) │   │
│  └────────────────┬─────────────────────┘   │
│                   │                          │
│                   ▼                          │
│  ┌──────────────────────────────────────┐   │
│  │  engine/render.rs (NEW)              │   │
│  │  - render_transcript(app, f, area)   │   │
│  │  - render_composer(app, f, area)     │   │
│  │  - ratatui Layout + Widgets          │   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

## Phase 1: Rewrite engine/

### mod.rs → render.rs

Replace the old `compose_layout()` (Vec<String> output) with a ratatui-style render function:

```rust
pub(crate) fn render(app: &App, f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    render_transcript(app, f, chunks[0]);
    render_composer(app, f, chunks[1]);
}
```

### transcript_renderer.rs → Widget-based

Replace `render_transcript(&Transcript, u16, usize) -> Vec<String>` with:

```rust
pub(crate) fn render_transcript(app: &App, f: &mut Frame, area: Rect) {
    // Use ratatui List or Paragraph for transcript entries
    // Track scroll position via ScrollbarState
    // Render entries with appropriate styles (user/assistant/tool/error)
}
```

### composer_renderer.rs → Widget-based

Replace `render_composer(...) -> Vec<String>` with:

```rust
pub(crate) fn render_composer(app: &App, f: &mut Frame, area: Rect) {
    // Use Paragraph for input prompt + draft
    // Use Paragraph for status bar
}
```

### Deleted files

- `diff.rs` — no longer needed (ratatui Terminal handles frame diffs)
- `renderer.rs` — no longer needed (replaced by ratatui Terminal)

## Phase 2: Rewrite mod.rs event loop

### Startup sequence (before vs after)

Before (current):
```rust
let mut agent_stream = agent_loop.run(prompt, session_id).await; // starts immediately
loop { select! { agent_fut, input, timer } }
```

After:
```rust
// No agent_stream creation at startup
// agent_fut = pending() until user submits
let mut pending_prompt: Option<String> = None;

loop {
    if let Some(prompt) = pending_prompt.take() {
        agent_stream = agent_loop.run(&prompt, session_id).await;
    }

    let agent_fut = if agent_stream.is_some() {
        agent_stream.as_mut().unwrap().next().boxed()
    } else {
        pending().boxed()
    };

    select! {
        event = agent_fut => { ... }
        input = input_stream.next() => { ... }
        _ = frame_interval.tick() => {
            terminal.draw(|f| render(&app, f));
        }
    }
}
```

### Alternate screen setup

```rust
use ratatui::backend::CrosstermBackend;
use std::io::Stdout;

fn setup() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore() -> io::Result<()> {
    execute!(io::stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
```

## Migration strategy

The change should be done in a single commit — there is no benefit to partial migration since the old and new rendering paths cannot coexist. The state/ and input/ layers are already testable independently and will continue to pass all existing tests unchanged.

## Files to delete

| File | Reason |
|------|--------|
| `src/interface/tui/engine/diff.rs` | Replaced by ratatui diff |
| `src/interface/tui/engine/renderer.rs` | Replaced by ratatui Terminal |

## Files to rewrite

| File | Old | New |
|------|-----|-----|
| `src/interface/tui/mod.rs` | ANSI diff event loop | ratatui event loop |
| `src/interface/tui/engine/mod.rs` | compose_layout() | render() function |
| `src/interface/tui/engine/transcript_renderer.rs` | Vec<String> output | ratatui Widget |
| `src/interface/tui/engine/composer_renderer.rs` | Vec<String> output | ratatui Widget |

## Unchanged

| File | Reason |
|------|--------|
| `src/interface/tui/state/transcript.rs` | Pure state machine, no TUI dependency |
| `src/interface/tui/state/composer.rs` | Pure state, unchanged |
| `src/interface/tui/state/focus.rs` | Pure enum, unchanged |
| `src/interface/tui/input/decode.rs` | Pure function, unchanged |
| `src/interface/tui/input/keymap.rs` | Pure function, unchanged |
| `src/interface/tui/input/action.rs` | Mostly unchanged; ScrollUp/Down may become ratatui ScrollbarState |
| `src/interface/tui/engine/ansi.rs` | Keep markdown-rendering helpers, prune cursor/erase helpers |

## Test strategy

### Unit tests (must pass before and after)

```bash
cd /home/l8ng/Projects/__straydragon__/xylitol && \
cargo test --features ui-tui -- interface::tui::state -- && \
cargo test --features ui-tui -- interface::tui::input --
```

These test the pure state/input layers without any rendering. All 112 existing tests should pass with zero modifications.

### Integration tests (new)

```bash
cd /home/l8ng/Projects/__straydragon__/xylitol && \
cargo test --features ui-tui -- test
```

### Manual smoke test

```bash
cd /home/l8ng/Projects/__straydragon__/xylitol && \
cargo run --features ui-tui -- tui
```

Verify:
- Terminal enters alternate screen cleanly
- Composer appears at the bottom with prompt "▸"
- Typing characters appears in the composer
- Enter submits message → transcript shows user message with "▶" prefix
- Agent response appears in the transcript with streaming indicator "▸"
- PgUp/PgDn scrolls the transcript
- Ctrl+D quits and restores terminal
