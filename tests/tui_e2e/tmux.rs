//! tmux E2E driver (c405 layer 5b, spec tt06).
//!
//! A hand-written thin wrapper around the `tmux` CLI (no binding crate). It
//! drives a REAL terminal emulator: spawn a detached session, inject keys via
//! `send-keys`, capture the screen via `capture-pane -e -p` (preserving SGR
//! escape sequences). This is the "closest to the human eye" layer — it
//! catches real-terminal compatibility and color-regression bugs that the
//! in-process vte harness cannot.
//!
//! All cases are `#[ignore]` (spec `test-infra` r8) and skip unless `tmux` is
//! installed. Run via `just test-tui-e2e`.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Skip the test (return early) if `tmux` is not on PATH. Call at the top of
/// each tmux test so the suite degrades gracefully on tmux-less hosts.
macro_rules! require_tmux {
    () => {
        if std::process::Command::new("tmux")
            .arg("-V")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_err()
        {
            eprintln!("skip: tmux not installed");
            return;
        }
    };
}

/// A tmux session driving a real terminal emulator. RAII: `Drop` kills the
/// session so panicking tests never leak `xyl_e2e_*` sessions (spec r8).
pub struct TmuxSession {
    name: String,
}

impl TmuxSession {
    /// Create a detached session running the `xylitol-tui` demo example at the
    /// given size. Sets `TERM=xterm-256color` so SGR color sequences are
    /// generated. The session name is PID-suffixed for parallel safety.
    pub fn spawn_demo(cols: u16, rows: u16) -> std::io::Result<Self> {
        let name = format!("xyl_e2e_{}", std::process::id());
        // The session command runs in the workspace root (tmux starts sessions
        // in the current cwd, but the test process cwd may be outside it).
        let workspace = env!("CARGO_MANIFEST_DIR");
        let window_cmd =
            format!("cd {workspace} && cargo run --example demo --quiet -p xylitol-tui");
        // new-session -d (detached) -s <name> -x <cols> -y <rows> <command>
        let status = Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-s",
                &name,
                "-x",
                &cols.to_string(),
                "-y",
                &rows.to_string(),
                &window_cmd,
            ])
            .env("TERM", "xterm-256color")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !status.success() {
            return Err(std::io::Error::other(format!(
                "tmux new-session failed (status {status})"
            )));
        }
        Ok(Self { name })
    }

    /// Inject keys via `tmux send-keys`. Special keys (Enter, Escape, C-Left,
    /// M-Right, Up, etc.) are passed as separate args so tmux interprets them;
    /// use `send_text` for literal strings.
    #[allow(dead_code)] // harness API; used by later editor-port E2E tests
    pub fn send(&self, keys: &[&str]) -> std::io::Result<()> {
        let mut cmd = Command::new("tmux");
        cmd.args(["send-keys", "-t", &self.name]);
        for k in keys {
            cmd.arg(k);
        }
        cmd.status()?;
        Ok(())
    }

    /// Send a literal string (no key-name interpretation) via `send-keys -l`.
    #[allow(dead_code)] // harness API; used by later editor-port E2E tests
    pub fn send_text(&self, text: &str) -> std::io::Result<()> {
        Command::new("tmux")
            .args(["send-keys", "-t", &self.name, "-l", text])
            .status()?;
        Ok(())
    }

    /// Capture the pane as text. With `with_sgr=true`, preserves ANSI escape
    /// sequences (so color assertions work); otherwise strips them.
    pub fn capture(&self, with_sgr: bool) -> std::io::Result<String> {
        let mut cmd = Command::new("tmux");
        cmd.args(["capture-pane", "-t", &self.name, "-p", "-J"]);
        if with_sgr {
            cmd.arg("-e");
        }
        let out = cmd.output()?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// Poll until the captured screen contains `needle`, up to `timeout`.
    /// Polling (not fixed sleep) is mandatory for tmux — it has no render-
    /// ready signal (see design.md risk table).
    pub fn wait_for(&self, needle: &str, timeout: Duration) -> std::io::Result<String> {
        let deadline = Instant::now() + timeout;
        loop {
            let screen = self.capture(false)?;
            if screen.contains(needle) {
                return Ok(screen);
            }
            if Instant::now() >= deadline {
                return Err(std::io::Error::other(format!(
                    "timeout waiting for {needle:?}; last screen:\n{screen}"
                )));
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
}

impl Drop for TmuxSession {
    fn drop(&mut self) {
        // Kill the session even on panic (spec r8: no orphaned sessions).
        let _ = Command::new("tmux")
            .args(["kill-session", "-t", &self.name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

// ── Tests (all #[ignore] — spec test-infra r8) ─────────────────────────────

#[test]
#[ignore = "E2E: needs tmux; run via `just test-tui-e2e`"]
fn tmux_demo_starts_and_shows_content() {
    require_tmux!();
    let session = TmuxSession::spawn_demo(60, 15).expect("spawn tmux session");
    // The demo prints a nav panel; wait for a known label.
    let screen = session
        .wait_for("Quit", Duration::from_secs(60))
        .expect("demo should render within 60s (includes cargo build)");
    assert!(!screen.trim().is_empty());
}

#[test]
#[ignore = "E2E: needs tmux; run via `just test-tui-e2e`"]
fn tmux_captures_styled_output() {
    // Covers scenario tt06: capture-pane -e preserves SGR sequences, so a
    // colored region surfaces as an escape sequence in the capture.
    require_tmux!();
    let session = TmuxSession::spawn_demo(60, 15).expect("spawn tmux session");
    session
        .wait_for("Quit", Duration::from_secs(60))
        .expect("demo should render");
    let styled = session.capture(true).expect("capture -e");
    // The demo uses SGR colors (cyan/yellow/dim helpers); at least one escape
    // sequence should be present in a styled render.
    assert!(
        styled.contains("\x1b["),
        "capture-pane -e should contain SGR escapes; got:\n{styled}"
    );
}
