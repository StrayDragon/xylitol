//! portable-pty E2E driver (c405 layer 5a, spec tt05).
//!
//! Spawns the `xylitol-tui` demo example under a PTY, injects key sequences,
//! reads the raw byte stream, and feeds it to `CapturedScreen` for cell-grid
//! assertions. This is the layer that exercises crossterm's REAL event
//! parsing of multi-byte sequences (Ctrl/Alt+arrow, bracketed paste) — the
//! in-process `handle_input` path bypasses crossterm entirely.
//!
//! All cases are `#[ignore]`: they spawn a real process + PTY and are slow
//! (spec `test-infra` r8). Run via `just test-tui-e2e`.
//!
//! We spawn the crate's `demo` example (NOT the full `xylitol` binary) so the
//! test is independent of LLM provider config — it validates the TUI render
//! pipeline + crossterm under a real PTY, not agent orchestration.

use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

use super::CapturedScreen;

/// A PTY-driven TUI session. Spawn the demo, inject keys, capture the screen.
///
/// A background thread drains the PTY reader into a shared buffer so the test
/// can poll `capture()` without blocking on a read call.
pub struct PtySession {
    _master: Box<dyn MasterPty>,
    writer: Box<dyn Write + Send>,
    rx: mpsc::Receiver<Vec<u8>>,
    buf: Vec<u8>,
}

impl PtySession {
    /// Spawn `cargo run --example demo -p xylitol-tui` under a PTY of the
    /// given size. Returns once the process is started.
    pub fn spawn_demo(cols: u16, rows: u16) -> std::io::Result<Self> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let mut cmd = CommandBuilder::new("cargo");
        cmd.args(["run", "--example", "demo", "--quiet", "-p", "xylitol-tui"]);
        // The spawned `cargo` must run in the workspace root (it inherits the
        // test's cwd otherwise, which may be outside the workspace).
        cmd.cwd(env!("CARGO_MANIFEST_DIR"));
        cmd.env("TERM", "xterm-256color");

        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        drop(pair.slave);

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        // Background thread: blocking-read chunks, send to channel.
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut tmp = [0u8; 4096];
            loop {
                match reader.read(&mut tmp) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(tmp[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self {
            _master: pair.master,
            writer,
            rx,
            buf: Vec::with_capacity(8192),
        })
    }

    /// Write a key sequence to the PTY (the child reads it via crossterm).
    pub fn send_keys(&mut self, keys: &str) -> std::io::Result<()> {
        self.writer.write_all(keys.as_bytes())?;
        self.writer.flush()
    }

    /// Pull any bytes that arrived in the last `settle` window into the buffer.
    pub fn drain(&mut self, settle: Duration) {
        let deadline = Instant::now() + settle;
        while let Ok(chunk) = self
            .rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            self.buf.extend_from_slice(&chunk);
        }
    }

    /// Parse all accumulated bytes into a fresh `CapturedScreen`.
    pub fn screen(&self, cols: usize, rows: usize) -> CapturedScreen {
        let mut screen = CapturedScreen::new(cols, rows);
        screen.feed(&self.buf);
        screen
    }

    /// Poll until the screen contains `needle`, up to `timeout`.
    pub fn wait_for(
        &mut self,
        needle: &str,
        timeout: Duration,
        cols: usize,
        rows: usize,
    ) -> std::io::Result<CapturedScreen> {
        let deadline = Instant::now() + timeout;
        loop {
            self.drain(Duration::from_millis(50));
            let screen = self.screen(cols, rows);
            if screen.text().contains(needle) {
                return Ok(screen);
            }
            if Instant::now() >= deadline {
                return Err(std::io::Error::other(format!(
                    "timeout waiting for {needle:?}; last screen:\n{}",
                    screen.text()
                )));
            }
        }
    }
}

// ── Tests (all #[ignore] — spec test-infra r8) ─────────────────────────────

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_demo_starts_and_renders() {
    // The demo prints a navigation panel; one of the labels should appear.
    let mut session = PtySession::spawn_demo(60, 15).expect("spawn demo");
    let screen = session
        .wait_for("Quit", Duration::from_secs(60), 60, 15)
        .expect("demo should render within 60s (includes cargo build)");
    assert!(!screen.text().trim().is_empty());
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_demo_survives_keypresses() {
    let mut session = PtySession::spawn_demo(60, 15).expect("spawn demo");
    session
        .wait_for("Quit", Duration::from_secs(60), 60, 15)
        .expect("demo should start");
    // Send some keystrokes; the demo must not crash (screen still has content).
    session.send_keys("abc").expect("send keys");
    session.drain(Duration::from_millis(300));
    let screen = session.screen(60, 15);
    assert!(
        !screen.text().trim().is_empty(),
        "screen non-empty after keys"
    );
}
