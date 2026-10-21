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

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
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
    serve: Option<std::process::Child>,
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(mut serve) = self.serve.take()
            && serve.try_wait().ok().flatten().is_none()
        {
            let _ = serve.kill();
            let _ = serve.wait();
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
        let (port, serve) = spawn_ephemeral_host(project_root, config_dir, home_dir)?;
        let mut cmd = CommandBuilder::new("cargo");
        cmd.args([
            "run",
            "--quiet",
            "--",
            "tui",
            "--trust",
            "--model",
            "fake",
            "--port",
            &port.to_string(),
        ]);
        cmd.cwd(env!("CARGO_MANIFEST_DIR"));
        cmd.env("TERM", "xterm-256color");
        cmd.env("HOME", home_dir);
        cmd.env("XYLITOL_CONFIG_DIR", config_dir);
        cmd.env("XYLITOL_PROJECT_DIR", project_root);
        let mut session = Self::spawn_cmd(cmd, cols, rows)?;
        session.serve = Some(serve);
        Ok(session)
    }

    /// Debug-only `xylitol tui designing` — attach Host, then open the scene slot.
    pub fn spawn_tui_designing(
        cols: u16,
        rows: u16,
        project_root: &Path,
        config_dir: &Path,
        home_dir: &Path,
    ) -> std::io::Result<Self> {
        let (port, serve) = spawn_ephemeral_host(project_root, config_dir, home_dir)?;
        let mut cmd = CommandBuilder::new("cargo");
        cmd.args([
            "run",
            "--quiet",
            "--",
            "tui",
            "--trust",
            "--model",
            "fake",
            "--port",
            &port.to_string(),
            "designing",
        ]);
        cmd.cwd(env!("CARGO_MANIFEST_DIR"));
        cmd.env("TERM", "xterm-256color");
        cmd.env("HOME", home_dir);
        cmd.env("XYLITOL_CONFIG_DIR", config_dir);
        cmd.env("XYLITOL_PROJECT_DIR", project_root);
        let mut session = Self::spawn_cmd(cmd, cols, rows)?;
        session.serve = Some(serve);
        Ok(session)
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
            serve: None,
        })
    }

    /// Spawn ApplicationOwned (`agent_demo_alt`) under a PTY — c2070 entry.
    pub fn spawn_demo_alt(cols: u16, rows: u16) -> std::io::Result<Self> {
        Self::spawn_example("agent_demo_alt", cols, rows)
    }

    /// Spawn `cargo run --example agent_demo -p xylitol-tui` under a PTY of the
    /// given size. Returns once the process is started.
    pub fn spawn_demo(cols: u16, rows: u16) -> std::io::Result<Self> {
        Self::spawn_example("agent_demo", cols, rows)
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

    /// Bytes accumulated in the raw PTY output stream so far（时序采样用）。
    pub fn raw_len(&self) -> usize {
        self.buf.len()
    }

    /// `buf[offset..]` 是否含 `needle`（帧批标记判定）。
    pub fn buf_tail_contains(&self, offset: usize, needle: &[u8]) -> bool {
        self.buf.len() > offset && windows_two(&self.buf[offset..], needle)
    }

    /// `buf[offset..]` 中 `needle` 出现次数（帧计数）。
    pub fn count_marks_since(&self, offset: usize, needle: &[u8]) -> usize {
        if self.buf.len() <= offset {
            return 0;
        }
        let hay = &self.buf[offset..];
        if needle.is_empty() {
            return 0;
        }
        hay.windows(needle.len()).filter(|w| *w == needle).count()
    }

    /// True if the raw byte stream received so far contains `needle` (a byte
    /// substring, e.g. a CSI escape sequence emitted at startup). Used to
    /// assert protocol-negotiation sequences were sent (c410 tp01).
    pub fn raw_contains(&self, needle: &[u8]) -> bool {
        windows_two(self.buf.as_slice(), needle)
    }

    /// Poll until `needle` appears in the raw stream **after** the last
    /// occurrence of `after` (e.g. second `?1049h` after `?1049l` on resume).
    pub fn wait_for_raw_after(
        &mut self,
        needle: &str,
        after: &str,
        timeout: Duration,
    ) -> std::io::Result<()> {
        let needle_b = needle.as_bytes();
        let after_b = after.as_bytes();
        let deadline = Instant::now() + timeout;
        loop {
            self.drain(Duration::from_millis(50));
            if let Some(after_at) = find_last_subslice(&self.buf, after_b) {
                let tail = &self.buf[after_at + after_b.len()..];
                if windows_two(tail, needle_b) {
                    return Ok(());
                }
            }
            if Instant::now() >= deadline {
                return Err(std::io::Error::other(format!(
                    "timeout waiting raw for {needle:?} after last {after:?}"
                )));
            }
        }
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

fn spawn_ephemeral_host(
    project_root: &Path,
    config_dir: &Path,
    home_dir: &Path,
) -> std::io::Result<(u16, std::process::Child)> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "--",
        "serve",
        "--host",
        "127.0.0.1",
        "--port",
        "0",
    ]);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    cmd.env("TERM", "xterm-256color");
    cmd.env("HOME", home_dir);
    cmd.env("XYLITOL_CONFIG_DIR", config_dir);
    cmd.env("XYLITOL_PROJECT_DIR", project_root);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::other("serve stderr missing"))?;
    let (tx, rx) = mpsc::channel::<u16>();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if let Some(port) = parse_host_listening_port(&line) {
                        let _ = tx.send(port);
                        break;
                    }
                }
            }
        }
    });
    match rx.recv_timeout(Duration::from_secs(120)) {
        Ok(port) => Ok((port, child)),
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(std::io::Error::other(
                "timed out waiting for Host listening on …",
            ))
        }
    }
}

fn parse_host_listening_port(line: &str) -> Option<u16> {
    let marker = "Host listening on ";
    let rest = line.trim().strip_prefix(marker)?;
    rest.rsplit(':').next()?.parse().ok()
}

/// Write isolated Fake model project config under `project_root/.xylitol/`.
fn write_fake_project_config(project_root: &Path) -> std::io::Result<()> {
    let dir = project_root.join(".xylitol");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("config.yaml"),
        "models:\n  default_model: fake\n  models:\n    fake:\n      provider: fake\n      model: fake-model\n      api_key: fake-e2e\n",
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

fn find_last_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
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
        .wait_for_idle(
            crate::DEMO_READY_NEEDLE,
            "Drafting",
            Duration::from_secs(60),
            172,
            40,
        )
        .expect("agent_demo should render");
    // Ctrl+U clear (0x15), then CJK — match tmux driver semantics.
    session
        .send_keys("\x15修复 footer 宽度预算并补一个 emoji smoke 🙂")
        .expect("replace editor text with CJK");
    session.send_keys("\r").expect("submit editor input");
    // Differential CSI can leave CapturedScreen stale; assert via raw PTY bytes.
    session
        .wait_for_raw("修复 footer", Duration::from_secs(10))
        .expect("submitted CJK text should appear in PTY stream");
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
        .expect("Ready fixed zone must recover after enlarge");

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

/// c2020 ath29 / S6: lab opt-in mouse via `XYLITOL_TUI_MOUSE=1` on **agent_demo**
/// (not the product TUI) enables capture CSI; clean exit must Disable on teardown.
/// Product `TerminalGuard` ignores this env — mouse UX waits ApplicationOwned / c2070.
#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_agent_demo_mouse_opt_in_enable_then_exit() {
    let mut session =
        PtySession::spawn_example_with_env("agent_demo", 100, 30, &[("XYLITOL_TUI_MOUSE", "1")])
            .expect("spawn agent_demo with mouse opt-in");
    session
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60), 100, 30)
        .expect("agent_demo should render");
    // crossterm EnableMouseCapture emits CSI ?1000h / ?1002h / ?1003h / …
    assert!(
        session.raw_contains(b"\x1b[?1000h") || session.raw_contains(b"\x1b[?1003h"),
        "opt-in mouse must emit EnableMouseCapture CSI in PTY stream"
    );
    // Demo quits on Ctrl+C only when the editor is empty (default prompt is not).
    session.send_keys("\x15").expect("clear editor (Ctrl+U)");
    session.send_keys("\x03").expect("Ctrl+C quit demo");
    let code = session
        .wait_exit(Duration::from_secs(30))
        .expect("demo should exit after quit");
    assert_eq!(code, 0, "mouse opt-in demo must exit cleanly");
    // Teardown should DisableMouseCapture (crossterm emits CSI ?1000l / related).
    assert!(
        session.raw_contains(b"\x1b[?1000l") || session.raw_contains(b"\x1b[?1003l"),
        "exit must emit DisableMouseCapture CSI so mouse mode does not linger"
    );
}

/// c2070 ApplicationOwned minimal PTY gate: `agent_demo_alt` enters alt-buffer + mouse
/// without `XYLITOL_TUI_MOUSE` / `XYLITOL_AGENT_DEMO_MODE`; exit leaves alt,
/// disables mouse, and dumps transcript marker onto the main buffer stream.
#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_agent_demo_alt_alt_mouse_and_exit_dump() {
    const COLS: u16 = 100;
    const ROWS: u16 = 30;
    const DUMP_MARK: &str = "c2070-mode-b-pty-dump-marker";
    let mut session = PtySession::spawn_example_with_env(
        "agent_demo_alt",
        COLS,
        ROWS,
        &[("XYLITOL_AGENT_DEMO_INITIAL_PROMPT", DUMP_MARK)],
    )
    .expect("spawn agent_demo_alt");
    session
        .wait_for(
            crate::DEMO_READY_NEEDLE,
            Duration::from_secs(90),
            COLS as usize,
            ROWS as usize,
        )
        .expect("agent_demo_alt should render");

    // EnterAlternateScreen (CSI ?1049h) — ApplicationOwned begin.
    assert!(
        session.raw_contains(b"\x1b[?1049h"),
        "ApplicationOwned must enter alt-buffer (CSI ?1049h)"
    );
    // Mouse from begin_application_owned_session — not XYLITOL_TUI_MOUSE.
    assert!(
        session.raw_contains(b"\x1b[?1000h") || session.raw_contains(b"\x1b[?1003h"),
        "ApplicationOwned must EnableMouseCapture without XYLITOL_TUI_MOUSE"
    );

    session.send_keys("\x15").expect("clear editor");
    session.send_keys("\x03").expect("Ctrl+C quit");
    let code = session
        .wait_exit(Duration::from_secs(30))
        .expect("agent_demo_alt should exit");
    assert_eq!(code, 0, "ApplicationOwned demo must exit cleanly");

    assert!(
        session.raw_contains(b"\x1b[?1049l"),
        "ApplicationOwned teardown must leave alt-buffer (CSI ?1049l)"
    );
    assert!(
        session.raw_contains(b"\x1b[?1000l") || session.raw_contains(b"\x1b[?1003l"),
        "ApplicationOwned teardown must DisableMouseCapture"
    );
    // Exit dump writes transcript (incl. seeded editor/prompt path) to main buffer.
    assert!(
        session.raw_contains(DUMP_MARK.as_bytes()),
        "exit dump should emit seeded transcript marker into PTY stream after leave-alt"
    );
}

/// c2070 ApplicationOwned: SGR drag on transcript should emit OSC52 copy-on-release.
#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_agent_demo_alt_drag_select_osc52() {
    const COLS: u16 = 100;
    const ROWS: u16 = 30;
    let mut session = PtySession::spawn_demo_alt(COLS, ROWS).expect("spawn agent_demo_alt");
    session
        .wait_for(
            crate::DEMO_READY_NEEDLE,
            Duration::from_secs(90),
            COLS as usize,
            ROWS as usize,
        )
        .expect("agent_demo_alt should render");
    assert!(
        session.raw_contains(b"\x1b[?1049h"),
        "precondition: alt-buffer active"
    );

    // Vertical drag across mid-transcript (avoid fold headers / dock).
    session.send_keys("\x1b[<0;12;6M").expect("mouse down");
    session.send_keys("\x1b[<32;12;14M").expect("mouse drag");
    session.send_keys("\x1b[<0;12;14m").expect("mouse up");
    session.drain(Duration::from_millis(500));
    session
        .wait_for_raw("\x1b]52;", Duration::from_secs(8))
        .expect("drag-select release must emit OSC52");

    session.send_keys("\x15").expect("clear editor");
    session.send_keys("\x03").expect("quit");
    let code = session.wait_exit(Duration::from_secs(30)).expect("exit");
    assert_eq!(code, 0);
}

/// c2070 ApplicationOwned: wheel over transcript must not crash; viewport stays app-owned
/// (smoke — sticky persistence is covered by package unit tests).
#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_agent_demo_alt_wheel_smoke() {
    const COLS: u16 = 100;
    const ROWS: u16 = 30;
    let mut session = PtySession::spawn_demo_alt(COLS, ROWS).expect("spawn agent_demo_alt");
    session
        .wait_for(
            crate::DEMO_READY_NEEDLE,
            Duration::from_secs(90),
            COLS as usize,
            ROWS as usize,
        )
        .expect("agent_demo_alt should render");

    // SGR wheel up (button 64) then wheel down (65) over transcript.
    for _ in 0..6 {
        session.send_keys("\x1b[<64;20;5M").expect("wheel up");
    }
    for _ in 0..3 {
        session.send_keys("\x1b[<65;20;5M").expect("wheel down");
    }
    session.drain(Duration::from_millis(300));
    session
        .wait_for(
            crate::DEMO_READY_NEEDLE,
            Duration::from_secs(10),
            COLS as usize,
            ROWS as usize,
        )
        .expect("ApplicationOwned must stay alive after wheel");

    session.send_keys("\x15").expect("clear");
    session.send_keys("\x03").expect("quit");
    assert_eq!(session.wait_exit(Duration::from_secs(30)).expect("exit"), 0);
}

/// c2070 ApplicationOwned H4: drag from transcript into dock still copy-on-release (clamp).
#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_agent_demo_alt_dock_clamp_copy() {
    const COLS: u16 = 100;
    const ROWS: u16 = 30;
    let mut session = PtySession::spawn_demo_alt(COLS, ROWS).expect("spawn agent_demo_alt");
    session
        .wait_for(
            crate::DEMO_READY_NEEDLE,
            Duration::from_secs(90),
            COLS as usize,
            ROWS as usize,
        )
        .expect("agent_demo_alt should render");

    // Down in transcript, drag into lower dock band, release in dock.
    session.send_keys("\x1b[<0;8;4M").expect("down transcript");
    session.send_keys("\x1b[<32;8;28M").expect("drag into dock");
    session.send_keys("\x1b[<0;8;28m").expect("up in dock");
    session.drain(Duration::from_millis(400));
    session
        .wait_for_raw("\x1b]52;", Duration::from_secs(5))
        .expect("dock-clamp release must still OSC52");

    session.send_keys("\x15").expect("clear");
    session.send_keys("\x03").expect("quit");
    assert_eq!(session.wait_exit(Duration::from_secs(30)).expect("exit"), 0);
}

/// c2070 ptim11: Ctrl+G with instant `$EDITOR` leave/re-enter alt + mouse.
#[test]
#[ignore = "E2E: spawns a real PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_agent_demo_alt_suspend_resume_restores_alt() {
    const COLS: u16 = 100;
    const ROWS: u16 = 30;
    let mut session = PtySession::spawn_example_with_env(
        "agent_demo_alt",
        COLS,
        ROWS,
        &[
            ("XYLITOL_AGENT_DEMO_REAL_EDITOR", "1"),
            ("EDITOR", "true"),
            ("VISUAL", "true"),
        ],
    )
    .expect("spawn agent_demo_alt with instant editor");
    session
        .wait_for(
            crate::DEMO_READY_NEEDLE,
            Duration::from_secs(90),
            COLS as usize,
            ROWS as usize,
        )
        .expect("agent_demo_alt should render");
    assert!(
        session.raw_contains(b"\x1b[?1049h"),
        "precondition: alt entered"
    );

    // Ctrl+G → suspend → `true` exits 0 → resume ApplicationOwned.
    session.send_keys("\x07").expect("Ctrl+G");
    session
        .wait_for_raw("\x1b[?1049l", Duration::from_secs(10))
        .expect("suspend must leave alt-buffer");
    // Re-enter after editor exits (must be a *new* ?1049h after leave).
    session
        .wait_for_raw_after("\x1b[?1049h", "\x1b[?1049l", Duration::from_secs(10))
        .expect("resume must re-enter alt-buffer");
    session
        .wait_for(
            crate::DEMO_READY_NEEDLE,
            Duration::from_secs(15),
            COLS as usize,
            ROWS as usize,
        )
        .expect("UI must recover after suspend");

    session.send_keys("\x15").expect("clear");
    session.send_keys("\x03").expect("quit");
    assert_eq!(session.wait_exit(Duration::from_secs(30)).expect("exit"), 0);
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
    // Tall welcome/skills + differential CSI → CapturedScreen stale; use raw.
    session
        .wait_for_raw("c669-bang-ok", Duration::from_secs(30))
        .expect("bang output should appear in PTY stream");
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
        .wait_for_raw("sleep 30", Duration::from_secs(15))
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
        .wait_for_raw("sleep 30", Duration::from_secs(15))
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

/// c705: designing preview — session-tree-labeled + tree shows annotation.
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_session_tree_label_path() {
    const COLS: usize = 100;
    const ROWS: usize = 30;
    let (mut session, _tmp) = spawn_tui_designing_ready(COLS as u16, ROWS as u16);

    session
        .wait_for_raw("session-tree-labeled", Duration::from_secs(20))
        .expect("scene slot lists labeled");
    session.send_keys("\x1b[B").expect("down to labeled");
    session.drain(Duration::from_millis(80));
    session.send_keys("\r").expect("select labeled scene");
    session
        .wait_for_raw("designing preview", Duration::from_secs(20))
        .expect("designing preview note");
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
///
/// The interactive bang loop pumps input (Esc abort) but not slash effects, so
/// the `/model` arm applies when the abort resumes the loop; cache-first mount
/// then shows the picker immediately. This pins that the picker (a) mounts,
/// (b) never displaces the busy status lead from the PTY stream (fixed zone
/// footprint), (c) stays Esc-closable despite the post-abort stale-Esc
/// suppress, and (d) exits cleanly.
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

    // c2790 / atc23 original contract: /model is Inline (atm18) — busy Enter
    // submits it, the bang-loop tick drains it, and the picker mounts WHILE
    // the bang is still running (no abort needed to resume the queue). The
    // `→ * fake` row is the mounted SelectList cursor + focused mark rendered
    // only by the Models slot.
    session
        .send_keys("/model\r")
        .expect("submit /model while bang busy");
    session
        .wait_for_raw("→ * fake", Duration::from_secs(15))
        .expect("Models picker must mount while the bang is STILL RUNNING");
    assert!(
        session.raw_contains(b"Running"),
        "busy status lead MUST remain in PTY stream while Models open (fixed-zone footprint / atc23)"
    );

    session.send_keys("\x1b").expect("Esc close Models");
    session.drain(Duration::from_millis(150));
    session
        .send_keys("\x1b")
        .expect("Esc cancel the still-running bang");
    session
        .wait_for_raw("(cancelled)", Duration::from_secs(15))
        .expect("bang cancelled");
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
    let (mut session, _tmp) = spawn_tui_designing_ready(COLS as u16, ROWS as u16);
    session
        .wait_for_raw("session-tree-branched", Duration::from_secs(20))
        .expect("scene slot lists branched");
    session.send_keys("\x1b[B\x1b[B").expect("down to branched");
    session.drain(Duration::from_millis(80));
    session.send_keys("\r").expect("select branched scene");
    session
        .wait_for_raw("designing preview", Duration::from_secs(20))
        .expect("designing preview note in PTY stream");
    session
        .wait_for_raw("alt leaf", Duration::from_secs(15))
        .expect("fixture alt leaf in PTY stream");

    // Busy spinner / paint path under tall transcript must not hang exit.
    session.send_keys("\x15/exit\r").expect("/exit");
    let code = session.wait_exit(Duration::from_secs(45)).expect("exit");
    assert_eq!(code, 0);
}

/// Product Fake — real-turn `/session-fork` + double Esc shows sibling user nodes.
///
/// c2740 made `/debug` scenes process-local: a seeded fixture never reaches the
/// host-side session store the tree reads, so the branch structure is grown
/// through the supported fork flow instead (trunk turn → fork → alt turn).
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_fake_session_tree_branched() {
    const COLS: usize = 100;
    const ROWS: usize = 30;
    let (mut session, _tmp) = spawn_product_fake_ready(COLS as u16, ROWS as u16);

    // Trunk turn: a real user node in the host session.
    session
        .send_keys("\x15main branch\r")
        .expect("submit trunk prompt");
    session
        .wait_for_raw(crate::FAKE_HELLO, Duration::from_secs(30))
        .expect("trunk Fake reply");

    // Fork at leaf (dismiss arg-completion popup first, same as /debug typing).
    session
        .send_keys("\x15/session-fork")
        .expect("type fork slash");
    session.drain(Duration::from_millis(200));
    session.send_keys("\x1b").expect("dismiss completion");
    session.drain(Duration::from_millis(100));
    session.send_keys("\r").expect("submit fork");
    session
        .wait_for_raw("forked → session", Duration::from_secs(20))
        .expect("fork notice");
    session
        .wait_for_idle("fake", "Working", Duration::from_secs(15), COLS, ROWS)
        .expect("idle after fork before alt prompt");

    // Alt branch turn under the fork. The second Fake reply is byte-identical
    // to the first (already in the raw stream), so wait for Hello *after* the
    // fork notice. A fixed drain can return while the alt turn is still
    // Working; double Esc then latches abort instead of opening the tree.
    session
        .send_keys("\x15alt branch\r")
        .expect("submit alt prompt");
    session
        .wait_for_raw_after(
            crate::FAKE_HELLO,
            "forked → session",
            Duration::from_secs(30),
        )
        .expect("alt Fake reply after fork");
    // CapturedScreen idle-oracle is stale under tall scrollback (no
    // scroll-region); the second Hello in the raw stream is the settle signal.
    session.drain(Duration::from_millis(200));

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
    // "user: " rows are tree vocabulary — the transcript renders prompts with
    // a "❯ " marker instead, so these needles prove the tree drew both nodes.
    // Cell-grid oracle: the raw stream splits each row across style spans
    // (dim "user: " prefix + label), so contiguous byte matching flakes.
    session
        .wait_for("user: main branch", Duration::from_secs(15), COLS, ROWS)
        .expect("tree shows trunk user node");
    session
        .wait_for("user: alt branch", Duration::from_secs(15), COLS, ROWS)
        .expect("tree shows alt branch user node");

    session.send_keys("\x1b").expect("Esc close tree");
    session.drain(Duration::from_millis(200));
    session.send_keys("\x15/exit\r").expect("/exit");
    let code = session.wait_exit(Duration::from_secs(30)).expect("exit");
    assert_eq!(code, 0);
}

fn spawn_product_fake_ready(cols: u16, rows: u16) -> (PtySession, tempfile::TempDir) {
    spawn_product_fake_ready_in(cols, rows, "project")
}

fn spawn_tui_designing_ready(cols: u16, rows: u16) -> (PtySession, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp.path().join("project");
    let config_dir = tmp.path().join("config");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&project).expect("project dir");
    std::fs::create_dir_all(&config_dir).expect("config dir");
    std::fs::create_dir_all(&home).expect("home dir");
    write_fake_project_config(&project).expect("fake config");
    let mut session = PtySession::spawn_tui_designing(cols, rows, &project, &config_dir, &home)
        .expect("spawn tui designing");
    if session
        .wait_for_raw("Trust project", Duration::from_secs(8))
        .is_ok()
    {
        session.send_keys("\r").expect("confirm trust");
    }
    session
        .wait_for_raw("scenes", Duration::from_secs(120))
        .expect("designing TUI should open the scene slot");
    (session, tmp)
}

/// Long nested project path — exercises startup-card wrap at narrow Ready widths.
///
/// Nest under the tempdir using this checkout's path relative to `$HOME`
/// (fallback: crate dir name), so each clone uses its own path depth/length.
fn spawn_product_fake_ready_long_path(cols: u16, rows: u16) -> (PtySession, tempfile::TempDir) {
    spawn_product_fake_ready_in(cols, rows, &checkout_rel_under_home())
}

fn checkout_rel_under_home() -> String {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if let Some(home) = home {
        let home = std::path::PathBuf::from(home);
        if let Ok(rel) = root.strip_prefix(&home) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }
    root.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "xylitol".into())
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

/// c2891-followup：滚轮响应签名（真 PTY=crossterm EventStream 全链）。
/// 单发 SGR wheel → 首个 paint 批(`?2026h`)延迟；连发 → 排空时长与帧数。
/// 「卡」的客观基线：签名劣化即回归；数值供调参对照（非精密时序断言）。
#[test]
#[ignore = "E2E: product PTY + cargo build; run via `just test-tui-e2e-pty`"]
fn pty_product_wheel_latency_signature() {
    let (mut session, _tmp) = spawn_product_fake_ready(100, 30);
    // seed：8 轮 fake 对话，确保 transcript 远高于视口（30 行窗）可滚。
    for i in 0..8 {
        session.send_keys("\x15hi\r").expect("seed submit");
        let needle = format!("{} (#{i})", crate::FAKE_HELLO);
        let _ = session.wait_for_raw(&needle, Duration::from_secs(20));
    }
    // 先 down 消 follow/bottom 对齐，up 才有位移可言。
    session
        .send_keys("\x1b[<65;10;5M")
        .expect("wheel down pre-align");
    std::thread::sleep(Duration::from_millis(200));
    session.drain(Duration::from_millis(200));
    session.drain(Duration::from_millis(400));
    let wheel = "\x1b[<64;10;5M"; // SGR wheel-up @ transcript row5 col10
    let frame_mark = "\x1b[?2026h";

    // ── 单发首响 ×10 ──
    let mut single_lat_ms: Vec<u128> = Vec::new();
    for _ in 0..10 {
        // 先 down 一格确保下方有位移空间（防顶溢 no-op 假 MAX）。
        session.send_keys("\x1b[<65;10;5M").expect("pre-align down");
        std::thread::sleep(Duration::from_millis(60));
        session.drain(Duration::from_millis(60));
        let base = session.raw_len();
        let t0 = std::time::Instant::now();
        session.send_keys(wheel).expect("wheel");
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            session.drain(Duration::from_millis(2));
            if session.raw_len() > base && session.buf_tail_contains(base, frame_mark.as_bytes()) {
                single_lat_ms.push(t0.elapsed().as_millis());
                break;
            }
            if std::time::Instant::now() > deadline {
                single_lat_ms.push(u128::MAX); // no-paint sentinel
                break;
            }
        }
        // 等价真实慢滚手间隙
        std::thread::sleep(Duration::from_millis(120));
    }
    single_lat_ms.sort_unstable();
    let mut ok: Vec<u128> = single_lat_ms
        .iter()
        .filter(|v| **v != u128::MAX)
        .copied()
        .collect();
    ok.sort_unstable();
    let med = ok.get(ok.len() / 2).copied().unwrap_or(u128::MAX);
    println!("wheel-single latency ms: median={med} ok={ok:?} all={single_lat_ms:?}");

    // ── 连发 20@25ms down（自上部向下恒有效）：排空时长 + 帧数 ──
    session.drain(Duration::from_millis(200));
    let base = session.raw_len();
    let t0 = std::time::Instant::now();
    let wheel_dn = "\x1b[<65;10;5M";
    for _ in 0..20 {
        session.send_keys(wheel_dn).expect("burst wheel");
        std::thread::sleep(Duration::from_millis(25));
    }
    let injected_done = std::time::Instant::now();
    let quiet_deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut last_len;
    loop {
        let before = session.raw_len();
        std::thread::sleep(Duration::from_millis(60));
        session.drain(Duration::from_millis(30));
        last_len = session.raw_len();
        if last_len == before
            && std::time::Instant::now().duration_since(injected_done)
                > std::time::Duration::from_millis(150)
        {
            break;
        }
        if std::time::Instant::now() > quiet_deadline {
            break;
        }
    }
    let drain_total = t0.elapsed().as_millis();
    let frames = session.count_marks_since(base, frame_mark.as_bytes());
    println!("wheel-burst: total_drain={drain_total}ms frames={frames}");
    assert!(
        med < 300,
        "single wheel median {med}ms >= 300 —— 首响签名劣化"
    );
    assert!(frames >= 1, "burst frames {frames} < 1 —— 合帧/响应缺失");
    assert!(
        drain_total < 4500,
        "burst drain {drain_total}ms —— 排空迟滞"
    );

    session.send_keys("\x15/exit\r").expect("submit /exit");
    let code = session
        .wait_exit(Duration::from_secs(30))
        .expect("exit after wheel signature");
    assert_eq!(code, 0, "clean exit");
}
