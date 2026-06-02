---
name: tui-cli-primitives
description: >-
  Reusable TUI/CLI primitives extracted from Zellij's architecture for building
  terminal applications like coding agents, TUI tools, and CLI dashboards.
  Covers terminal input handling, text selection/copy, pane layout patterns,
  debounced rendering, IPC messaging, and session persistence. Use when building
  a TUI app, coding agent CLI, terminal multiplexer, or any interactive terminal
  tool that needs selection, copy, keyboard handling, or pane management.
---

# TUI/CLI Reusable Primitives from Zellij

Patterns and code references extracted from Zellij's codebase, applicable to
any Rust terminal application (coding agent TUI, dashboard, REPL, etc.).

## Primitive Index

| Primitive | Zellij Source | Standalone Crate Alternative | Difficulty |
|-----------|---------------|------------------------------|------------|
| [Keyboard input parsing](#1-keyboard-input) | vendored termwiz + KittyParser | `crossterm`, `termwiz` | Easy |
| [Text selection & copy](#2-text-selection--copy) | `Grid.selection` + OSC 52 | Custom (see pattern below) | Medium |
| [Terminal raw mode](#3-terminal-raw-mode) | `ClientOsApi` | `crossterm::terminal` | Easy |
| [Debounced rendering](#4-debounced-rendering) | `background_jobs` 10ms timer | Custom (see pattern below) | Easy |
| [Actor thread messaging](#5-actor-thread-messaging) | `ThreadSenders` + `Bus<T>` | `crossbeam`, `tokio::mpsc` | Medium |
| [Bounded backpressure](#6-bounded-backpressure) | PTY→Screen bounded(50) | `crossbeam::bounded` | Easy |
| [Pane tiling layout](#7-pane-tiling-layout) | `TiledPaneGrid` | `ratatui`, `taffy` | Medium |
| [VT terminal emulation](#8-vt-terminal-emulation) | `Grid` + `vte::Perform` | `vte` + custom | Hard |
| [Session persistence](#9-session-persistence) | `SessionLayoutMetadata` | Custom serialization | Medium |
| [IPC client-server](#10-ipc-client-server) | Protobuf + Unix socket | `prost` + `interprocess` | Medium |
| [Plugin sandboxing](#11-plugin-sandboxing) | Wasmi + WASI + permissions | `wasmi`, `wasmtime` | Hard |

---

## 1. Keyboard Input

**Zellij approach**: Three-layer pipeline — raw bytes → Kitty protocol parser → termwiz fallback.

**For your project** (recommended):

```rust
// Using crossterm (simplest)
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

crossterm::terminal::enable_raw_mode()?;
if event::poll(std::time::Duration::from_millis(100))? {
    if let Event::Key(key) = event::read()? {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => break,
            (_, KeyCode::Char(c)) => handle_char(c),
            _ => {}
        }
    }
}
```

**For Kitty keyboard protocol** (advanced, like Zellij):

Reference: `zellij-client/src/keyboard_parser.rs` (private, study pattern only)

```rust
// Kitty protocol state machine pattern
enum KittyState { Ground, Escape, CSI, ParsingNumber, ParsingModifiers, Done }

// CSI u sequences: \x1b[<keycode>;<modifiers>u
// Example: \x1b[97;5u = Ctrl+a
fn parse_kitty_sequence(bytes: &[u8]) -> Option<KeyWithModifier> {
    // Parse number before ';' = keycode
    // Parse number after ';' = modifier flags
    // Terminator: 'u' (unicode), '~' (legacy), or letter
}
```

**Zellij source locations**:
- Kitty parser: `zellij-client/src/keyboard_parser.rs`
- termwiz vendored: `zellij-utils/src/vendored/`
- Key normalization: `zellij-utils/src/input/mod.rs` (`parse_keys`, `cast_termwiz_key`)

---

## 2. Text Selection & Copy

**Zellij approach**: Grid tracks `Selection` (start/end positions), renders highlight, copies via external command or OSC 52.

**Reusable pattern for coding agent TUI**:

```rust
struct Selection {
    start: Position,
    end: Position,
    active: bool,
}

struct Position { line: usize, col: usize }

impl Selection {
    fn contains(&self, line: usize, col: usize) -> bool {
        let (s, e) = self.ordered();
        if line < s.line || line > e.line { return false; }
        if line == s.line && col < s.col { return false; }
        if line == e.line && col > e.col { return false; }
        true
    }

    fn ordered(&self) -> (&Position, &Position) {
        if (self.start.line, self.start.col) <= (self.end.line, self.end.col) {
            (&self.start, &self.end)
        } else {
            (&self.end, &self.start)
        }
    }

    fn extract_text(&self, lines: &[String]) -> String {
        let (s, e) = self.ordered();
        let mut result = String::new();
        for line_idx in s.line..=e.line {
            if line_idx >= lines.len() { break; }
            let line = &lines[line_idx];
            let start_col = if line_idx == s.line { s.col } else { 0 };
            let end_col = if line_idx == e.line { e.col } else { line.len() };
            result.push_str(&line[start_col..end_col.min(line.len())]);
            if line_idx < e.line { result.push('\n'); }
        }
        result
    }
}
```

**Clipboard integration** (two methods from Zellij):

```rust
// Method 1: External command (xclip, pbcopy, wl-copy)
fn copy_to_clipboard_cmd(text: &str, cmd: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(cmd)
        .stdin(Stdio::piped())
        .spawn()?;
    child.stdin.take().unwrap().write_all(text.as_bytes())?;
    child.wait()?;
    Ok(())
}

// Method 2: OSC 52 (works over SSH, no external tool needed)
fn copy_to_clipboard_osc52(text: &str) {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(text);
    print!("\x1b]52;c;{}\x07", encoded);
}
```

**Zellij source locations**:
- Selection model: `zellij-server/src/panes/selection.rs`
- Grid selection: `zellij-server/src/panes/grid.rs` (`selection` field)
- Clipboard: `zellij-server/src/tab/clipboard.rs`
- Copy command: `zellij-server/src/tab/copy_command.rs`
- OSC 52: search `osc_52` or `\x1b]52` in grid.rs

---

## 3. Terminal Raw Mode

```rust
// crossterm pattern (Zellij uses similar on client side)
fn setup_terminal() -> std::io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
        crossterm::cursor::Hide,
    )?;
    Ok(())
}

fn restore_terminal() -> std::io::Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
        crossterm::cursor::Show,
    )?;
    Ok(())
}
```

**Zellij source**: `zellij-client/src/os_input_output.rs` (`ClientOsApi` trait)

---

## 4. Debounced Rendering

**Zellij pattern**: Data changes queue a render request; background thread coalesces within 10ms.

```rust
use std::sync::mpsc;
use std::time::{Duration, Instant};

enum RenderMsg { RequestRender, DoRender, Exit }

fn spawn_render_debouncer(
    render_tx: mpsc::Sender<RenderMsg>,
) -> mpsc::Sender<RenderMsg> {
    let (req_tx, req_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let debounce = Duration::from_millis(10);
        let mut pending = false;
        let mut last_request = Instant::now();
        loop {
            let timeout = if pending {
                debounce.saturating_sub(last_request.elapsed())
            } else {
                Duration::from_secs(3600)
            };
            match req_rx.recv_timeout(timeout) {
                Ok(RenderMsg::RequestRender) => {
                    pending = true;
                    last_request = Instant::now();
                },
                Ok(RenderMsg::Exit) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if pending {
                        let _ = render_tx.send(RenderMsg::DoRender);
                        pending = false;
                    }
                },
                _ => {}
            }
        }
    });
    req_tx
}
```

**Zellij source**: `zellij-server/src/background_jobs.rs` (`BackgroundJob::RenderToClients`)

---

## 5. Actor Thread Messaging

**Zellij pattern**: Each subsystem = thread + typed enum channel.

```rust
use crossbeam_channel::{bounded, unbounded, Sender, Receiver, select};

enum ScreenMsg { UpdateContent(String), Resize(u16, u16), Render, Exit }
enum InputMsg { Key(char), Mouse(u16, u16), Exit }

struct ThreadSenders {
    to_screen: Sender<ScreenMsg>,
    to_input: Sender<InputMsg>,
}

fn screen_thread(rx: Receiver<ScreenMsg>, senders: ThreadSenders) {
    loop {
        match rx.recv() {
            Ok(ScreenMsg::UpdateContent(s)) => { /* update state */ },
            Ok(ScreenMsg::Render) => { /* render to stdout */ },
            Ok(ScreenMsg::Exit) => break,
            _ => {}
        }
    }
}
```

**Zellij source**: `zellij-server/src/thread_bus.rs`

---

## 6. Bounded Backpressure

Prevent fast producers from overwhelming consumers:

```rust
// Zellij: PTY output → Screen uses bounded(50)
let (tx, rx) = crossbeam_channel::bounded::<Vec<u8>>(50);

// Producer (PTY reader): blocks when channel full
// This naturally throttles shell output
tx.send(bytes)?;

// Consumer (Screen): processes at its own pace
// Use select! for multiple sources with fair scheduling
select! {
    recv(bounded_rx) -> msg => { /* PTY output (backpressured) */ },
    recv(unbounded_rx) -> msg => { /* user actions (never blocked) */ },
}
```

**Zellij source**: Screen's `Bus<ScreenInstruction>` with dual receivers

---

## 7. Pane Tiling Layout

**For coding agent TUI**, prefer `ratatui` over Zellij internals:

```rust
use ratatui::layout::{Constraint, Direction, Layout};

let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(30),  // file tree
        Constraint::Percentage(70),  // main content
    ])
    .split(frame.area());

let right_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Min(3),          // editor
        Constraint::Length(10),       // agent output
    ])
    .split(chunks[1]);
```

**Zellij's layout concepts to borrow** (implement yourself):
- Swap layouts: pre-defined arrangements auto-selected by pane count
- Stacked panes: collapsed tabs within a pane area
- Floating overlay: z-indexed panes on top of tiled grid

**Zellij source**: `zellij-server/src/panes/tiled_panes.rs`, `floating_panes/`

---

## 8. VT Terminal Emulation

If embedding a terminal in your TUI (e.g. agent running shell commands):

```rust
use vte::{Parser, Perform};

struct TermGrid {
    lines: Vec<Vec<char>>,
    cursor: (usize, usize),
    width: usize,
    height: usize,
}

impl Perform for TermGrid {
    fn print(&mut self, c: char) {
        let (row, col) = self.cursor;
        if col < self.width {
            self.lines[row][col] = c;
            self.cursor.1 += 1;
        }
    }
    fn execute(&mut self, byte: u8) {
        match byte {
            0x0A => { /* LF: new line */ },
            0x0D => { self.cursor.1 = 0; /* CR */ },
            0x08 => { if self.cursor.1 > 0 { self.cursor.1 -= 1; } /* BS */ },
            _ => {}
        }
    }
    fn csi_dispatch(&mut self, params: &vte::Params, _inter: &[u8], _ignore: bool, action: char) {
        match action {
            'H' => { /* cursor position */ },
            'J' => { /* erase display */ },
            'K' => { /* erase line */ },
            'm' => { /* SGR: text styling */ },
            _ => {}
        }
    }
    // ... hook, put, unhook for DCS; osc_dispatch for OSC
}

let mut parser = Parser::new();
parser.advance(&mut grid, &pty_output_bytes);
```

**Zellij source**: `zellij-server/src/panes/grid.rs` (5000+ line full implementation)

---

## 9. Session Persistence

**Zellij pattern**: Periodically serialize layout → KDL file, restore on resurrection.

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SessionState {
    tabs: Vec<TabState>,
    focused_tab: usize,
    timestamp: u64,
}

#[derive(Serialize, Deserialize)]
struct TabState {
    name: String,
    panes: Vec<PaneState>,
}

// Dirty detection: only serialize when state diverges from initial layout
fn is_dirty(current: &SessionState, initial: &SessionState) -> bool {
    current.tabs.len() != initial.tabs.len()
    || current.tabs.iter().zip(&initial.tabs).any(|(c, i)| c.panes.len() != i.panes.len())
}

// Background serialization with interval guard
fn background_serialize(state: &SessionState, path: &Path, interval: Duration) {
    // Check last_write_time; skip if too recent
    // serde_json::to_writer(file, state)
}
```

**Zellij source**: `zellij-server/src/session_layout_metadata.rs`

---

## 10. IPC Client-Server

Length-prefixed protobuf over Unix sockets:

```rust
use prost::Message;

fn send<T: Message>(writer: &mut impl Write, msg: &T) -> std::io::Result<()> {
    let bytes = msg.encode_to_vec();
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

fn recv<T: Message + Default>(reader: &mut impl Read) -> std::io::Result<T> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    T::decode(&buf[..]).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
```

**Zellij source**: `zellij-utils/src/ipc.rs`

---

## 11. Plugin Sandboxing

If your coding agent needs a plugin system:

```rust
// Wasmi (Zellij's choice): pure Rust, no JIT, slow but safe
use wasmi::{Engine, Module, Store, Linker};

let engine = Engine::default();
let module = Module::new(&engine, wasm_bytes)?;
let mut store = Store::new(&engine, host_state);
let mut linker = Linker::new(&engine);

// Export host functions to WASM
linker.func_wrap("env", "host_command", |caller, cmd_id: i32| {
    // Check permissions before executing
})?;
```

**Zellij source**: `zellij-server/src/plugins/plugin_loader.rs`

---

## Coding Agent TUI Quick Recipe

For a typical coding agent TUI, combine these primitives:

```
┌─ Input Thread ─────────────┐
│ crossterm events            │
│ → KeyEvent / MouseEvent     │──→ mpsc::channel
└─────────────────────────────┘     │
                                    ▼
┌─ Main Thread ──────────────────────────────────┐
│ select! {                                       │
│   input_rx => dispatch_action(key)              │
│   agent_rx => update_agent_output(msg)          │
│   render_rx => render_all_panes()               │
│ }                                               │
└─────────────────┬───────────────────────────────┘
                  │
    ┌─────────────┴─────────────┐
    ▼                           ▼
┌─ Agent Thread ─┐    ┌─ Render Debouncer ─┐
│ LLM API calls  │    │ 10ms coalesce      │
│ tool execution │    │ → render_rx        │
│ → agent_rx     │    └────────────────────┘
└────────────────┘
```

Recommended crate stack:
- `ratatui` + `crossterm` — TUI framework + terminal I/O
- `crossbeam-channel` — typed message passing between threads
- `vte` — if embedding terminal output
- `prost` + `interprocess` — if client-server IPC needed
- `wasmi` — if plugin sandboxing needed
