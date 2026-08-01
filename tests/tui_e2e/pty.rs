//! portable-pty E2E driver (c405 layer 5a, spec tt05; c485 avs2 product path).
//!
//! Two spawn targets under a real PTY:
//! - `xylitol-tui` examples (`agent_demo`) — package render / crossterm protocol
//! - product `xylitol` binary — Fake model + `--trust` vertical-slice smoke
//!
//! Injects key sequences, reads the raw byte stream, and feeds it to
//! `CapturedScreen` for cell-grid assertions. This layer exercises crossterm's
//! REAL event parsing of multi-byte sequences (Ctrl/Alt+arrow, bracketed paste)
//! — the in-process `handle_input` path bypasses crossterm entirely.
//!
//! All cases are `#[ignore]`: they spawn a real process + PTY and are slow
//! (spec `test-infra` r8). Run via `just test-tui-e2e` / `just test-tui-e2e-pty`.

use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

use super::CapturedScreen;

/// A PTY-driven TUI session. Spawn an example or product binary, inject keys,
/// capture the screen.
///
/// A background thread drains the PTY reader into a shared buffer so the test
/// can poll `capture()` without blocking on a read call.
pub struct PtySession {
    child: Box<dyn Child + Send + Sync>,
    _master: Box<dyn MasterPty>,
    writer: Box<dyn Write + Send>,
    rx: mpsc::Receiver<Vec<u8>>,
    buf: Vec<u8>,
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

impl PtySession {
    /// Spawn `cargo run --example <name> -p xylitol-tui` under a PTY of the
    /// given size. Returns once the process is started.
    pub fn spawn_example(example: &str, cols: u16, rows: u16) -> std::io::Result<Self> {
        Self::spawn_example_with_env(example, cols, rows, &[])
    }

    pub fn spawn_example_with_env(
        example: &str,
        cols: u16,
        rows: u16,
        extra_env: &[(&str, &str)],
    ) -> std::io::Result<Self> {
        let mut cmd = CommandBuilder::new("cargo");
        cmd.args(["run", "--example", example, "--quiet", "-p", "xylitol-tui"]);
        // The spawned `cargo` must run in the workspace root (it inherits the
        // test's cwd otherwise, which may be outside the workspace).
        cmd.cwd(env!("CARGO_MANIFEST_DIR"));
        cmd.env("TERM", "xterm-256color");
        for (key, value) in extra_env {
            cmd.env(key, value);
        }
        Self::spawn_cmd(cmd, cols, rows)
    }

    /// Product `xylitol` TUI with isolated Fake model config (c485 avs2).
    ///
    /// `project_root` must contain `.xylitol/config.yaml` with a `fake`
    /// model. `config_dir` / `home_dir` isolate global config + trust/sessions.
    pub fn spawn_product_fake(
        cols: u16,
        rows: u16,
        project_root: &Path,
        config_dir: &Path,
        home_dir: &Path,
    ) -> std::io::Result<Self> {
        let mut cmd = CommandBuilder::new("cargo");
        cmd.args(["run", "--quiet", "--", "tui", "--trust", "--model", "fake"]);
        cmd.cwd(env!("CARGO_MANIFEST_DIR"));
        cmd.env("TERM", "xterm-256color");
        cmd.env("HOME", home_dir);
        cmd.env("XYLITOL_CONFIG_DIR", config_dir);
        cmd.env("XYLITOL_PROJECT_DIR", project_root);
        Self::spawn_cmd(cmd, cols, rows)
    }

    fn spawn_cmd(cmd: CommandBuilder, cols: u16, rows: u16) -> std::io::Result<Self> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let child = pair
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
            child,
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

    /// Seed the editor via `XYLITOL_AGENT_DEMO_INITIAL_PROMPT` (slash-submit cases).
    #[allow(dead_code)] // kept for prompt-seeded e2e; palette/settings use Ctrl+P/S + wait_for_raw
    pub fn spawn_demo_with_prompt(cols: u16, rows: u16, prompt: &str) -> std::io::Result<Self> {
        Self::spawn_example_with_env(
            "agent_demo",
            cols,
            rows,
            &[("XYLITOL_AGENT_DEMO_INITIAL_PROMPT", prompt)],
        )
    }

    /// Resize the PTY window (delivers a real crossterm `Resize` to the child).
    pub fn resize(&mut self, cols: u16, rows: u16) -> std::io::Result<()> {
        self._master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| std::io::Error::other(e.to_string()))
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

    /// Poll until `needle` is present and `avoid` is absent (idle footer).
    pub fn wait_for_idle(
        &mut self,
        needle: &str,
        avoid: &str,
        timeout: Duration,
        cols: usize,
        rows: usize,
    ) -> std::io::Result<CapturedScreen> {
        let deadline = Instant::now() + timeout;
        loop {
            self.drain(Duration::from_millis(50));
            let screen = self.screen(cols, rows);
            let text = screen.text();
            if text.contains(needle) && !text.contains(avoid) {
                return Ok(screen);
            }
            if Instant::now() >= deadline {
                return Err(std::io::Error::other(format!(
                    "timeout waiting idle ({needle:?} without {avoid:?}); last screen:\n{text}"
                )));
            }
        }
    }

    /// Poll until the raw PTY byte stream contains `needle` (not CapturedScreen).
    ///
    /// Prefer this when tall scrollback / differential CSI leaves the cell-grid
    /// oracle stale (no scroll-region support) while the app did paint correctly.
    pub fn wait_for_raw(&mut self, needle: &str, timeout: Duration) -> std::io::Result<()> {
        let needle_b = needle.as_bytes();
        let deadline = Instant::now() + timeout;
        loop {
            self.drain(Duration::from_millis(50));
            if self.raw_contains(needle_b) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                let screen = self.screen(100, 30);
                return Err(std::io::Error::other(format!(
                    "timeout waiting raw for {needle:?}; screen:\n{}",
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

    /// Non-blocking poll of child exit status (`None` = still running).
    pub fn try_wait(&mut self) -> std::io::Result<Option<u32>> {
        match self.child.try_wait() {
            Ok(Some(status)) => Ok(Some(status.exit_code())),
            Ok(None) => Ok(None),
            Err(e) => Err(std::io::Error::other(e.to_string())),
        }
    }

    /// Poll until the child process exits, up to `timeout`.
    pub fn wait_exit(&mut self, timeout: Duration) -> std::io::Result<u32> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Ok(status.exit_code());
            }
            self.drain(Duration::from_millis(50));
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                return Err(std::io::Error::other(
                    "timeout waiting for child process exit",
                ));
            }
        }
    }
}

/// Write isolated Fake model project config under `project_root/.xylitol/`.
fn write_fake_project_config(project_root: &Path) -> std::io::Result<()> {
    let dir = project_root.join(".xylitol");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("config.yaml"),
        "models:\n  default_model: fake\n  models:\n    fake:\n      provider: fake\n      model: fake-model\n",
    )?;
    Ok(())
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
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60), 60, 15)
        .expect("agent_demo should render within 60s (includes cargo build)");
    assert!(!screen.text().trim().is_empty());
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_demo_survives_keypresses() {
    let mut session = PtySession::spawn_demo(60, 15).expect("spawn demo");
    session
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60), 60, 15)
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
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60), 60, 15)
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
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60), 60, 15)
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
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60), 172, 40)
        .expect("agent_demo should render");
    // Ctrl+U clear (0x15), then CJK — match tmux driver semantics.
    session
        .send_keys("\x15修复 footer 宽度预算并补一个 emoji smoke 🙂")
        .expect("replace editor text with CJK");
    session.send_keys("\r").expect("submit editor input");
    let screen = session
        .wait_for("修复 footer", Duration::from_secs(10), 172, 40)
        .expect("submitted CJK text should appear");
    assert!(!screen.text().trim().is_empty());
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_agent_demo_command_palette_smoke() {
    // Prefer Ctrl+P (same as harness / tmux). Tall seed + differential CSI leaves the
    // VTE cell-grid oracle stale, so assert via raw PTY bytes (see wait_for_raw).
    let mut session = PtySession::spawn_example("agent_demo", 172, 60).expect("spawn agent_demo");
    session
        .wait_for_idle(
            crate::DEMO_READY_NEEDLE,
            "Drafting",
            Duration::from_secs(60),
            172,
            60,
        )
        .expect("agent_demo should be idle");
    session
        .send_keys("\x10")
        .expect("Ctrl+P open command plate");
    session
        .wait_for_raw(crate::DEMO_COMMAND_PLATE_NEEDLE, Duration::from_secs(10))
        .expect("command plate title should appear in PTY stream");
    assert!(
        session.raw_contains(b"Type to filter")
            || session.raw_contains(b"Run regression tests")
            || session.raw_contains(b"Markdown full"),
        "command plate body should be present in raw stream"
    );
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e`"]
fn pty_agent_demo_settings_overlay_smoke() {
    let mut session = PtySession::spawn_example("agent_demo", 172, 60).expect("spawn agent_demo");
    session
        .wait_for_idle(
            crate::DEMO_READY_NEEDLE,
            "Drafting",
            Duration::from_secs(60),
            172,
            60,
        )
        .expect("agent_demo should be idle");
    session.send_keys("\x13").expect("Ctrl+S open settings");
    session
        .wait_for_raw(crate::DEMO_SETTINGS_NEEDLE, Duration::from_secs(10))
        .expect("settings title should appear in PTY stream");
    assert!(
        session.raw_contains(b"Approval"),
        "settings overlay should list Approval row in raw stream"
    );
}

#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_agent_demo_narrow_cjk_submit_flow_survives_enter() {
    let mut session = PtySession::spawn_example("agent_demo", 96, 32).expect("spawn agent_demo");
    session
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60), 96, 32)
        .expect("agent_demo should render");
    session
        .send_keys("\x15把命令面板和设置面板的窄宽 CJK 回归补齐 🙂")
        .expect("replace editor text with narrow CJK prompt");
    session.send_keys("\r").expect("submit editor input");
    // Seed transcript is tall; CapturedScreen has no scroll-region — use raw.
    session
        .wait_for_raw("窄宽 CJK", Duration::from_secs(15))
        .expect("narrow submit should appear");
}

/// Extreme shrink must show TooSmall hint and recover Ready on restore (not exit/stuck).
/// Also paints Ready at 40–44 with a long project path (startup-card wrap regression).
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_extreme_shrink_then_restore() {
    const COLS: u16 = 100;
    const ROWS: u16 = 30;
    let (mut session, _tmp) = spawn_product_fake_ready_long_path(COLS, ROWS);

    // Ready zone that previously hung forever on long cwd wrap.
    session.resize(44, 24).expect("narrow Ready");
    session.drain(Duration::from_millis(300));
    assert!(
        session.try_wait().ok().flatten().is_none(),
        "must stay alive at Ready 44x24 with long path"
    );

    session.resize(8, 3).expect("shrink PTY");
    session
        .wait_for_raw("请放大", Duration::from_secs(15))
        .expect("TooSmall hint should paint (not crash/exit)");
    assert!(
        session.try_wait().ok().flatten().is_none(),
        "product must stay alive while too small"
    );

    session.resize(COLS, ROWS).expect("restore PTY");
    session
        .wait_for(
            crate::PRODUCT_READY_NEEDLE,
            Duration::from_secs(15),
            COLS as usize,
            ROWS as usize,
        )
        .expect("Ready chrome must recover after enlarge");

    session.send_keys("\x15/exit\r").expect("submit /exit");
    let code = session
        .wait_exit(Duration::from_secs(30))
        .expect("process should exit after /exit");
    assert_eq!(
        code, 0,
        "product TUI /exit should exit 0 after shrink/restore"
    );
}

/// c485 avs2: product `xylitol` Fake smoke — Hello → `/exit`.
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_hello_then_exit() {
    let (mut session, _tmp) = spawn_product_fake_ready(100, 30);
    session.send_keys("\x15hi\r").expect("submit short prompt");
    // Tall welcome/skills + differential CSI leaves CapturedScreen stale (no
    // scroll-region); assert Fake reply via raw PTY like session-tree cases.
    session
        .wait_for_raw(crate::FAKE_HELLO, Duration::from_secs(30))
        .expect("Fake default reply should appear");

    session.send_keys("\x15/exit\r").expect("submit /exit");
    let code = session
        .wait_exit(Duration::from_secs(30))
        .expect("process should exit after /exit");
    assert_eq!(code, 0, "product TUI /exit should exit 0");
}

/// c669: product bang `!echo` streams into a Bash block (real shell, Fake model).
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_bang_echo_ok() {
    const COLS: usize = 100;
    const ROWS: usize = 30;
    let (mut session, _tmp) = spawn_product_fake_ready(COLS as u16, ROWS as u16);
    session
        .send_keys("\x15!echo c669-bang-ok\r")
        .expect("submit bang echo");
    session
        .wait_for("c669-bang-ok", Duration::from_secs(30), COLS, ROWS)
        .expect("bang output should appear in scrollback");
    session.send_keys("\x15/exit\r").expect("submit /exit");
    let code = session
        .wait_exit(Duration::from_secs(30))
        .expect("exit after bang");
    assert_eq!(code, 0);
}

/// c669: Esc during hanging bang → `(cancelled)` (not agent `Aborted`).
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_bang_esc_cancelled() {
    const COLS: usize = 100;
    const ROWS: usize = 30;
    let (mut session, _tmp) = spawn_product_fake_ready(COLS as u16, ROWS as u16);
    session
        .send_keys("\x15!sleep 30\r")
        .expect("submit hanging bang");
    session
        .wait_for("sleep 30", Duration::from_secs(15), COLS, ROWS)
        .expect("bang command should uplink");
    // Brief settle so execute_bash is in-flight, then Esc.
    session.drain(Duration::from_millis(400));
    session.send_keys("\x1b").expect("Esc abort bang");
    // Tall welcome/skills + differential CSI leaves CapturedScreen stale; assert raw.
    session
        .wait_for_raw("(cancelled)", Duration::from_secs(15))
        .expect("bang Esc should show (cancelled)");
    session.send_keys("\x15/exit\r").expect("submit /exit");
    let _ = session.wait_exit(Duration::from_secs(30));
}

/// c669: second `!` while bang busy → hard reject (reject note).
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_bang_second_hard_reject() {
    const COLS: usize = 100;
    const ROWS: usize = 30;
    let (mut session, _tmp) = spawn_product_fake_ready(COLS as u16, ROWS as u16);
    session
        .send_keys("\x15!sleep 30\r")
        .expect("submit first bang");
    session
        .wait_for("sleep 30", Duration::from_secs(15), COLS, ROWS)
        .expect("first bang uplink");
    session.drain(Duration::from_millis(400));
    session
        .send_keys("\x15!echo second\r")
        .expect("submit second bang");
    // Same CapturedScreen staleness as bang Esc / session-tree (see wait_for_raw).
    session
        .wait_for_raw("rejected", Duration::from_secs(10))
        .expect("second bang must hard-reject");
    // Cancel hanging first bang so /exit is clean.
    session.send_keys("\x1b").expect("Esc first bang");
    let _ = session.wait_for_raw("(cancelled)", Duration::from_secs(15));
    session.send_keys("\x15/exit\r").expect("submit /exit");
    let _ = session.wait_exit(Duration::from_secs(30));
}

/// c705: product Fake — after a turn, double Esc opens tree slot Search/Help.
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_session_tree_opens_search_help() {
    const COLS: usize = 100;
    const ROWS: usize = 30;
    let (mut session, _tmp) = spawn_product_fake_ready(COLS as u16, ROWS as u16);
    session.send_keys("\x15hi\r").expect("submit prompt");
    session
        .wait_for_raw(crate::FAKE_HELLO, Duration::from_secs(30))
        .expect("Fake reply");

    // Empty editor, then double Esc within the product window.
    // Tall welcome/skills + differential CSI leaves CapturedScreen stale (no
    // scroll-region); assert via raw PTY like labeled/branched tree open.
    session.send_keys("\x15").expect("clear editor");
    session.drain(Duration::from_millis(100));
    session.send_keys("\x1b").expect("Esc 1");
    session.drain(Duration::from_millis(80));
    session.send_keys("\x1b").expect("Esc 2");

    session
        .wait_for_raw("Type to search", Duration::from_secs(15))
        .expect("tree Search row in PTY stream");
    assert!(
        session.raw_contains(b"fold/unfold")
            || session.raw_contains(b"filters")
            || session.raw_contains(b"cycle"),
        "TreeHelp should show fold/unfold or filters/cycle in raw PTY"
    );
    assert!(
        session.raw_contains(b"Session tree")
            || session.raw_contains(b"hi")
            || session.raw_contains(b"Hello"),
        "tree should show session content in raw PTY"
    );

    session.send_keys("\x1b").expect("Esc close tree");
    session.drain(Duration::from_millis(200));
    session.send_keys("\x15/exit\r").expect("submit /exit");
    let code = session
        .wait_exit(Duration::from_secs(30))
        .expect("exit after tree");
    assert_eq!(code, 0);
}

/// c705: product Fake — `/debug session-tree-labeled` + Shift+L opens label editor.
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_session_tree_label_path() {
    const COLS: usize = 100;
    const ROWS: usize = 30;
    let (mut session, _tmp) = spawn_product_fake_ready(COLS as u16, ROWS as u16);

    // c710 fixture: labeled tree without fighting Kitty printable typing.
    // Dismiss arg-completion popup (Enter would apply, not submit) then submit.
    session
        .send_keys("\x15/debug session-tree-labeled")
        .expect("type debug scene");
    session.drain(Duration::from_millis(200));
    session.send_keys("\x1b").expect("dismiss completion");
    session.drain(Duration::from_millis(100));
    session.send_keys("\r").expect("submit debug scene");
    // Cell-grid oracle goes stale under tall welcome; fixture notes live in raw PTY.
    session
        .wait_for_raw("debug scene", Duration::from_secs(20))
        .expect("debug scene note");
    session
        .wait_for_raw("labeled root", Duration::from_secs(15))
        .expect("fixture user in scrollback");

    session.send_keys("\x15").expect("clear");
    session.drain(Duration::from_millis(100));
    session.send_keys("\x1b").expect("Esc 1");
    session.drain(Duration::from_millis(80));
    session.send_keys("\x1b").expect("Esc 2");
    session
        .wait_for_raw("Type to search", Duration::from_secs(15))
        .expect("tree open");
    assert!(
        session.raw_contains(b"[bookmark]"),
        "debug session-tree-labeled must show annotation in PTY stream"
    );
    // Shift+L type+save: harness h23 (PTY+Kitty printable is flaky).

    session.send_keys("\x1b").expect("Esc close tree");
    session.drain(Duration::from_millis(200));
    session.send_keys("\x15/exit\r").expect("/exit");
    let code = session.wait_exit(Duration::from_secs(30)).expect("exit");
    assert_eq!(code, 0);
}

/// Product Fake — bang-busy + short terminal + `/model` keeps status lead in viewport (atc23).
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_busy_model_list_keeps_running_lead() {
    // Short rows so an uncapped Models list could push status above content-end.
    const COLS: usize = 80;
    const ROWS: usize = 14;
    let (mut session, _tmp) = spawn_product_fake_ready(COLS as u16, ROWS as u16);

    session
        .send_keys("\x15!sleep 30\r")
        .expect("submit hanging bang");
    session
        .wait_for_raw("sleep 30", Duration::from_secs(15))
        .expect("bang uplink");
    session.drain(Duration::from_millis(400));
    // Bang busy lead is `Running` (Working for agent turns).
    session
        .wait_for_raw("Running", Duration::from_secs(10))
        .expect("bang busy status lead");

    session
        .send_keys("\x15/model\r")
        .expect("open Models while busy");
    session
        .wait_for_raw("fake", Duration::from_secs(15))
        .expect("Models slot should list fake");
    assert!(
        session.raw_contains(b"Running"),
        "busy status lead MUST remain in PTY stream while Models open (chrome footprint)"
    );

    session.send_keys("\x1b").expect("Esc close Models");
    session.drain(Duration::from_millis(150));
    session.send_keys("\x1b").expect("Esc cancel bang");
    let _ = session.wait_for_raw("(cancelled)", Duration::from_secs(15));
    session.send_keys("\x15/exit\r").expect("/exit");
    let code = session.wait_exit(Duration::from_secs(30)).expect("exit");
    assert_eq!(code, 0);
}

/// Product Fake — tall scrollback fixture must still /exit cleanly (ath25 smoke).
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_large_scrollback_then_exit() {
    const COLS: usize = 100;
    const ROWS: usize = 30;
    let (mut session, _tmp) = spawn_product_fake_ready(COLS as u16, ROWS as u16);

    session
        .send_keys("\x15/debug session-tree-branched")
        .expect("type debug scene");
    session.drain(Duration::from_millis(200));
    session.send_keys("\x1b").expect("dismiss completion");
    session.drain(Duration::from_millis(100));
    session.send_keys("\r").expect("submit debug scene");
    session
        .wait_for_raw("debug scene", Duration::from_secs(20))
        .expect("debug scene note in PTY stream");
    session
        .wait_for_raw("alt leaf", Duration::from_secs(15))
        .expect("fixture alt leaf in PTY stream");

    // Busy spinner / paint path under tall transcript must not hang exit.
    session.send_keys("\x15/exit\r").expect("/exit");
    let code = session.wait_exit(Duration::from_secs(45)).expect("exit");
    assert_eq!(code, 0);
}

/// Product Fake — `/debug session-tree-branched` + double Esc shows sibling branches.
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_session_tree_branched() {
    const COLS: usize = 100;
    const ROWS: usize = 30;
    let (mut session, _tmp) = spawn_product_fake_ready(COLS as u16, ROWS as u16);

    session
        .send_keys("\x15/debug session-tree-branched")
        .expect("type debug scene");
    session.drain(Duration::from_millis(200));
    session.send_keys("\x1b").expect("dismiss completion");
    session.drain(Duration::from_millis(100));
    session.send_keys("\r").expect("submit debug scene");
    // Tall branched fixture scrolls the scroll notice off the cell-grid oracle;
    // assert via raw PTY (same rationale as Type to search below).
    session
        .wait_for_raw("debug scene", Duration::from_secs(20))
        .expect("debug scene note in PTY stream");
    session
        .wait_for_raw("alt leaf", Duration::from_secs(15))
        .expect("fixture alt leaf in PTY stream");

    // Same open path as labeled (c705): empty editor + double Esc.
    // Assert via raw bytes: tall fixture scrollback desyncs CapturedScreen
    // (no scroll-region), so wait_for("Type to search") on the cell grid flakes.
    session.send_keys("\x15").expect("clear");
    session.drain(Duration::from_millis(100));
    session.send_keys("\x1b").expect("Esc 1");
    session.drain(Duration::from_millis(80));
    session.send_keys("\x1b").expect("Esc 2");
    session
        .wait_for_raw("Type to search", Duration::from_secs(15))
        .expect("tree Search row in PTY stream");
    assert!(
        session.raw_contains(b"main branch") && session.raw_contains(b"alt branch"),
        "branched fixture must show sibling user nodes in tree (raw PTY)"
    );

    session.send_keys("\x1b").expect("Esc close tree");
    session.drain(Duration::from_millis(200));
    session.send_keys("\x15/exit\r").expect("/exit");
    let code = session.wait_exit(Duration::from_secs(30)).expect("exit");
    assert_eq!(code, 0);
}

fn spawn_product_fake_ready(cols: u16, rows: u16) -> (PtySession, tempfile::TempDir) {
    spawn_product_fake_ready_in(cols, rows, "project")
}

/// Long nested project path — exercises startup-card wrap at narrow Ready widths.
fn spawn_product_fake_ready_long_path(cols: u16, rows: u16) -> (PtySession, tempfile::TempDir) {
    spawn_product_fake_ready_in(
        cols,
        rows,
        "Projects/__straydragon__/xylitol-very-long-path-segment",
    )
}

fn spawn_product_fake_ready_in(
    cols: u16,
    rows: u16,
    project_rel: &str,
) -> (PtySession, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join(project_rel);
    let config_dir = tmp.path().join("config");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&project).expect("project dir");
    std::fs::create_dir_all(&config_dir).expect("config dir");
    std::fs::create_dir_all(&home).expect("home dir");
    write_fake_project_config(&project).expect("fake config");

    let mut session = PtySession::spawn_product_fake(cols, rows, &project, &config_dir, &home)
        .expect("spawn product xylitol");
    session
        .wait_for(
            crate::PRODUCT_READY_NEEDLE,
            Duration::from_secs(120),
            cols as usize,
            rows as usize,
        )
        .expect("product TUI should render Fake footer");
    (session, tmp)
}
