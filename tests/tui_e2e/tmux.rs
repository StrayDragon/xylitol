use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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

pub struct TmuxSession {
    name: String,
}

impl TmuxSession {
    pub fn spawn_demo(cols: u16, rows: u16) -> std::io::Result<Self> {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let name = format!("xyl_e2e_{}_{}", std::process::id(), n);
        let workspace = env!("CARGO_MANIFEST_DIR");
        let path = std::env::var("PATH").unwrap_or_default();
        let window_cmd = format!(
            "cd {workspace} && PATH='{path}' cargo run --example agent_demo --quiet -p xylitol-tui"
        );
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

    pub fn send(&self, keys: &[&str]) -> std::io::Result<()> {
        let mut cmd = Command::new("tmux");
        cmd.args(["send-keys", "-t", &self.name]);
        for k in keys {
            cmd.arg(k);
        }
        cmd.status()?;
        Ok(())
    }

    pub fn send_text(&self, text: &str) -> std::io::Result<()> {
        Command::new("tmux")
            .args(["send-keys", "-t", &self.name, "-l", text])
            .status()?;
        Ok(())
    }

    pub fn capture(&self, with_sgr: bool) -> std::io::Result<String> {
        let mut cmd = Command::new("tmux");
        cmd.args(["capture-pane", "-t", &self.name, "-p", "-J"]);
        if with_sgr {
            cmd.arg("-e");
        }
        let out = cmd.output()?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

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
        let _ = Command::new("tmux")
            .args(["kill-session", "-t", &self.name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

#[test]
#[ignore = "E2E: needs tmux; run via `just test-tui-e2e`"]
fn tmux_demo_starts_and_shows_content() {
    require_tmux!();
    let session = TmuxSession::spawn_demo(60, 15).expect("spawn tmux session");
    let screen = session
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60))
        .expect("agent_demo should render within 60s (includes cargo build)");
    assert!(!screen.trim().is_empty());
}

#[test]
#[ignore = "E2E: needs tmux; run via `just test-tui-e2e`"]
fn tmux_captures_styled_output() {
    require_tmux!();
    let session = TmuxSession::spawn_demo(60, 15).expect("spawn tmux session");
    session
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60))
        .expect("agent_demo should render");
    let styled = session.capture(true).expect("capture -e");
    assert!(
        styled.contains("\x1b["),
        "capture-pane -e should contain SGR escapes; got:\n{styled}"
    );
}

#[test]
#[ignore = "E2E: needs tmux; run via `just test-tui-e2e`"]
fn tmux_agent_demo_cjk_submit_flow_survives_enter() {
    require_tmux!();
    let session = TmuxSession::spawn_demo(172, 40).expect("spawn tmux session");
    session
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60))
        .expect("agent_demo should render");
    session.send(&["C-u"]).expect("clear editor line");
    session
        .send_text("修复 footer 宽度预算并补一个 emoji smoke 🙂")
        .expect("send CJK text");
    session.send(&["Enter"]).expect("submit");
    let screen = session
        .wait_for("修复 footer", Duration::from_secs(10))
        .expect("submitted CJK text should appear in captured pane");
    assert!(!screen.trim().is_empty());
}

#[test]
#[ignore = "E2E: needs tmux; run via `just test-tui-e2e`"]
fn tmux_agent_demo_command_palette_smoke() {
    require_tmux!();
    let session = TmuxSession::spawn_demo(172, 40).expect("spawn tmux session");
    session
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60))
        .expect("agent_demo should render");
    session.send(&["C-p"]).expect("open command plate");
    let screen = session
        .wait_for(crate::DEMO_COMMAND_PLATE_NEEDLE, Duration::from_secs(10))
        .expect("command plate should appear");
    assert!(
        screen.contains("Run regression tests") || screen.contains("Markdown full"),
        "command plate content should be visible; got:\n{screen}"
    );
}

#[test]
#[ignore = "E2E: needs tmux; run via `just test-tui-e2e`"]
fn tmux_agent_demo_settings_overlay_smoke() {
    require_tmux!();
    let session = TmuxSession::spawn_demo(172, 40).expect("spawn tmux session");
    session
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60))
        .expect("agent_demo should render");
    session.send(&["C-s"]).expect("open settings overlay");
    let screen = session
        .wait_for(crate::DEMO_SETTINGS_NEEDLE, Duration::from_secs(10))
        .expect("settings overlay should appear");
    assert!(
        screen.contains("Approval"),
        "settings overlay content should be visible; got:\n{screen}"
    );
}

#[test]
#[ignore = "E2E: needs tmux; run via `just test-tui-e2e`"]
fn tmux_agent_demo_narrow_cjk_submit_flow_survives_enter() {
    require_tmux!();
    let session = TmuxSession::spawn_demo(96, 32).expect("spawn tmux session");
    session
        .wait_for(crate::DEMO_READY_NEEDLE, Duration::from_secs(60))
        .expect("agent_demo should render");
    session.send(&["C-u"]).expect("clear editor line");
    session
        .send_text("把命令面板和设置面板的窄宽 CJK 回归补齐 🙂")
        .expect("send narrow CJK text");
    session.send(&["Enter"]).expect("submit");
    let screen = session
        .wait_for("窄宽 CJK", Duration::from_secs(10))
        .expect("submitted narrow CJK text should appear in captured pane");
    assert!(!screen.trim().is_empty());
}
