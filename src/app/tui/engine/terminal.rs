//! Terminal abstraction — the engine's only I/O boundary (c399).
//!
//! Modeled on pi-tui's `ProcessTerminal` (skill `rendering-engine.md` Step 1),
//! but Rust + crossterm collapses pi-tui's hardest parts: crossterm's
//! `event::read()` yields one `Event` at a time (stdin splitting done),
//! `KeyEvent { code, modifiers }` is protocol-agnostic (Kitty/modifyOtherKeys/
//! legacy all normalized — no `keys.ts` / `stdin-buffer.ts`), and
//! `Event::Paste` covers bracketed paste. So this layer is a thin wrapper.
//!
//! The engine never touches `io::stdout`/`stdin` directly. Tests use a
//! `VirtualTerminal` (capture writes, feed synthetic events); production uses
//! `ProcessTerminal` (raw mode + real stdin/stdout). ANSI sequences are emitted
//! as raw bytes via `write` — the engine builds one buffer per frame, not
//! per-Command `queue`, so it fully controls the byte stream (required for
//! diff-correctness and for the `VirtualTerminal` to replay the same bytes).

use std::io::{self, Write};

use crossterm::event::{self, Event};
use crossterm::terminal::{self as cterm};

/// The engine's terminal surface. The engine calls only these methods; a test
/// implementation captures `write` bytes and feeds synthetic `poll_event`s.
pub trait Terminal {
    /// Current terminal width in columns (re-read each frame; resize changes it).
    fn width(&self) -> u16;
    /// Current terminal height in rows.
    fn height(&self) -> u16;

    /// Write a raw ANSI byte buffer to stdout. The engine builds one buffer per
    /// frame and calls this once; callers MUST NOT split a frame into multiple
    /// writes (loses the synchronized-output atomicity and desyncs the diff).
    fn write(&mut self, data: &str);

    /// Poll for one terminal event with a timeout. Returns `None` on timeout.
    /// Production blocks up to `timeout`; tests return queued synthetic events.
    fn poll_event(&mut self, timeout: std::time::Duration) -> Option<Event>;
}

/// Production terminal backed by crossterm over real stdin/stdout.
///
/// `start()` enters raw mode + enables bracketed paste; `stop()` restores.
/// The engine drives the lifecycle; `ProcessTerminal` itself does no rendering
/// beyond `write` (no implicit cursor moves or clears).
pub struct ProcessTerminal {
    stdout: io::Stdout,
    cols: u16,
    rows: u16,
}

impl ProcessTerminal {
    /// Enter raw mode and capture the current terminal size. The caller is
    /// responsible for having installed the panic-restore hook BEFORE this
    /// (so a panic between raw-mode-on and the engine's `stop()` still restores).
    pub fn start() -> io::Result<Self> {
        cterm::enable_raw_mode()?;
        // Enable bracketed paste (skill Step 1); disable in stop().
        let mut stdout = io::stdout();
        let _ = write!(stdout, "\x1b[?2004h");
        let _ = stdout.flush();
        let (cols, rows) = cterm::size().unwrap_or((80, 24));
        Ok(Self { stdout, cols, rows })
    }

    /// Restore the terminal: disable bracketed paste + raw mode, show cursor.
    /// Best-effort — never panic (called from Drop and signal handlers).
    pub fn stop(&mut self) {
        let _ = write!(self.stdout, "\x1b[?2004l"); // bracketed paste off
        let _ = crossterm::execute!(self.stdout, crossterm::cursor::Show);
        let _ = cterm::disable_raw_mode();
        let _ = self.stdout.flush();
    }
}

impl Terminal for ProcessTerminal {
    fn width(&self) -> u16 {
        self.cols
    }
    fn height(&self) -> u16 {
        self.rows
    }

    fn write(&mut self, data: &str) {
        let _ = self.stdout.write_all(data.as_bytes());
        let _ = self.stdout.flush();
    }

    fn poll_event(&mut self, timeout: std::time::Duration) -> Option<Event> {
        // crossterm's blocking poll/read. A Resize event updates our cached
        // size so the next frame's width()/height() reflect it.
        if event::poll(timeout).ok()? {
            let ev = event::read().ok()?;
            if let Event::Resize(c, r) = ev {
                self.cols = c;
                self.rows = r;
            }
            Some(ev)
        } else {
            None
        }
    }
}

impl Drop for ProcessTerminal {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Test terminal: captures `write` bytes (for assertions) and feeds queued
/// synthetic events on `poll_event`. Used by the engine's diff invariant tests
/// (`virtual_terminal.rs`) — the same engine code runs against this as against
/// `ProcessTerminal`, so behavior is verifiable without a real terminal.
#[cfg(test)]
pub(crate) struct CapturingTerminal {
    pub cols: u16,
    pub rows: u16,
    pub written: String,
    pub events: std::collections::VecDeque<Event>,
}

#[cfg(test)]
impl CapturingTerminal {
    pub(crate) fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            written: String::new(),
            events: std::collections::VecDeque::new(),
        }
    }
    pub(crate) fn queue_event(&mut self, ev: Event) {
        self.events.push_back(ev);
    }
    pub(crate) fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
    }
}

#[cfg(test)]
impl Terminal for CapturingTerminal {
    fn width(&self) -> u16 {
        self.cols
    }
    fn height(&self) -> u16 {
        self.rows
    }
    fn write(&mut self, data: &str) {
        self.written.push_str(data);
    }
    fn poll_event(&mut self, _timeout: std::time::Duration) -> Option<Event> {
        self.events.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capturing_terminal_records_writes() {
        let mut t = CapturingTerminal::new(80, 24);
        t.write("hello");
        t.write("\x1b[31mred");
        assert_eq!(t.written, "hello\x1b[31mred");
        assert_eq!(t.width(), 80);
        assert_eq!(t.height(), 24);
    }

    #[test]
    fn capturing_terminal_feeds_queued_events() {
        let mut t = CapturingTerminal::new(80, 24);
        t.queue_event(Event::Resize(100, 30));
        let ev = t.poll_event(std::time::Duration::ZERO);
        assert!(matches!(ev, Some(Event::Resize(100, 30))));
        // Next poll is empty (no more queued).
        assert!(t.poll_event(std::time::Duration::ZERO).is_none());
    }

    #[test]
    fn capturing_terminal_resize_updates_dims() {
        let mut t = CapturingTerminal::new(80, 24);
        t.resize(120, 40);
        assert_eq!(t.width(), 120);
        assert_eq!(t.height(), 40);
    }
}
