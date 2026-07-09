//! portable-pty E2E driver (c405 layer 5a, spec tt05).
//!
//! Spawns the `xylitol-tui` example surface under a PTY, injects key sequences,
//! reads the raw byte stream, and feeds it to `CapturedScreen` for cell-grid
//! assertions. This is the layer that exercises crossterm's REAL event
//! parsing of multi-byte sequences (Ctrl/Alt+arrow, bracketed paste) — the
//! in-process `handle_input` path bypasses crossterm entirely.
//!
//! All cases are `#[ignore]`: they spawn a real process + PTY and are slow
//! (spec `test-infra` r8). Run via `just test-tui-e2e`.
//!
//! We spawn the crate's fake coding-agent example (NOT the full `xylitol`
//! binary) so the test is independent of LLM provider config — it validates
//! the TUI render pipeline + crossterm under a real PTY, not agent orchestration.

use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

use super::CapturedScreen;

/// A PTY-driven TUI session. Spawn an example, inject keys, capture the screen.
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
    /// Spawn `cargo run --example <name> -p xylitol-tui` under a PTY of the
    /// given size. Returns once the process is started.
    pub fn spawn_example(example: &str, cols: u16, rows: u16) -> std::io::Result<Self> {
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
        cmd.args(["run", "--example", example, "--quiet", "-p", "xylitol-tui"]);
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

    /// Spawn `cargo run --example agent_demo -p xylitol-tui` under a PTY of the
    /// given size. Returns once the process is started.
    pub fn spawn_demo(cols: u16, rows: u16) -> std::io::Result<Self> {
        Self::spawn_example("agent_demo", cols, rows)
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

    /// True if the raw byte stream received so far contains `needle` (a byte
    /// substring, e.g. a CSI escape sequence emitted at startup). Used to
    /// assert protocol-negotiation sequences were sent (c410 tp01).
    pub fn raw_contains(&self, needle: &[u8]) -> bool {
        windows_two(self.buf.as_slice(), needle)
    }
}

/// Naive substring search (the `buf` is small in tests; no need for memchr).
fn windows_two(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

// ── Tests (all #[ignore] — spec test-infra r8) ─────────────────────────────

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_demo_starts_and_renders() {
    // The fake coding-agent example prints a title row; wait for it.
    let mut session = PtySession::spawn_demo(60, 15).expect("spawn demo");
    let screen = session
        .wait_for("fake coding agent demo", Duration::from_secs(60), 60, 15)
        .expect("agent_demo should render within 60s (includes cargo build)");
    assert!(!screen.text().trim().is_empty());
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_demo_survives_keypresses() {
    let mut session = PtySession::spawn_demo(60, 15).expect("spawn demo");
    session
        .wait_for("fake coding agent demo", Duration::from_secs(60), 60, 15)
        .expect("agent_demo should start");
    // Send some keystrokes; the demo must not crash (screen still has content).
    session.send_keys("abc").expect("send keys");
    session.drain(Duration::from_millis(300));
    let screen = session.screen(60, 15);
    assert!(
        !screen.text().trim().is_empty(),
        "screen non-empty after keys"
    );
}

/// c410 tp01: the demo emits the Kitty keyboard protocol query (CSI >7u) at
/// start. We verify by scanning the raw PTY byte stream for the push sequence.
#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_kitty_query_emitted_at_start() {
    let mut session = PtySession::spawn_demo(60, 15).expect("spawn demo");
    session
        .wait_for("fake coding agent demo", Duration::from_secs(60), 60, 15)
        .expect("demo should render (so start() has run)");
    // CSI >7u = \x1b[>7u — the Kitty enhancement push pi/xy emit at start.
    assert!(
        session.raw_contains(b"\x1b[>7u"),
        "Kitty query sequence CSI >7u should be in the startup byte stream"
    );
}

/// c410 tp04: bracketed paste is enabled at start (CSI ?2004h).
#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_bracketed_paste_enabled_at_start() {
    let mut session = PtySession::spawn_demo(60, 15).expect("spawn demo");
    session
        .wait_for("fake coding agent demo", Duration::from_secs(60), 60, 15)
        .expect("demo should render");
    assert!(
        session.raw_contains(b"\x1b[?2004h"),
        "bracketed paste enable (CSI ?2004h) should be in the startup stream"
    );
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_agent_demo_submit_flow_survives_enter() {
    let mut session = PtySession::spawn_example("agent_demo", 172, 40).expect("spawn agent_demo");
    session
        .wait_for("fake coding agent demo", Duration::from_secs(60), 172, 40)
        .expect("agent_demo should render");
    session
        .send_keys("\x15修复 footer 宽度预算并补一个 emoji smoke 🙂")
        .expect("replace editor text with CJK");
    session.send_keys("\r").expect("submit editor input");
    session.drain(Duration::from_millis(500));
    let screen = session.screen(172, 40);
    let text = screen.text();
    assert!(
        !text.trim().is_empty(),
        "screen must remain populated after submit flow"
    );
    assert!(
        text.contains("修复 footer") || text.contains("Running rg and cargo test"),
        "submit flow should remain visible after Enter; got:\n{text}"
    );
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_agent_demo_command_palette_smoke() {
    let mut session = PtySession::spawn_demo(172, 40).expect("spawn agent_demo");
    session
        .wait_for("fake coding agent demo", Duration::from_secs(60), 172, 40)
        .expect("agent_demo should render");
    session.send_keys("\x10").expect("open command palette");
    let screen = session
        .wait_for("Command Palette", Duration::from_secs(10), 172, 40)
        .expect("command palette should appear");
    let text = screen.text();
    assert!(
        text.contains("Run regression tests"),
        "command palette content should be visible; got:\n{text}"
    );
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_agent_demo_settings_overlay_smoke() {
    let mut session = PtySession::spawn_demo(172, 40).expect("spawn agent_demo");
    session
        .wait_for("fake coding agent demo", Duration::from_secs(60), 172, 40)
        .expect("agent_demo should render");
    session.send_keys("\x13").expect("open settings overlay");
    let screen = session
        .wait_for("Session Settings", Duration::from_secs(10), 172, 40)
        .expect("settings overlay should appear");
    let text = screen.text();
    assert!(
        text.contains("Approval"),
        "settings overlay content should be visible; got:\n{text}"
    );
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_agent_demo_narrow_cjk_submit_flow_survives_enter() {
    let mut session = PtySession::spawn_example("agent_demo", 96, 32).expect("spawn agent_demo");
    session
        .wait_for("fake coding agent demo", Duration::from_secs(60), 96, 32)
        .expect("agent_demo should render");
    session
        .send_keys("\x15把命令面板和设置面板的窄宽 CJK 回归补齐 🙂")
        .expect("replace editor text with narrow CJK prompt");
    session.send_keys("\r").expect("submit editor input");
    session.drain(Duration::from_millis(500));
    let screen = session.screen(96, 32);
    let text = screen.text();
    assert!(
        !text.trim().is_empty(),
        "screen must remain populated after narrow submit flow"
    );
    assert!(
        text.contains("窄宽 CJK") || text.contains("Running rg and cargo test"),
        "narrow submit flow should remain visible after Enter; got:\n{text}"
    );
}
