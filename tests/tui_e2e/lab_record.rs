#![allow(clippy::all)]
//! ~keep 终端录制 lab：把产品 TUI 的屏幕按帧打时间戳写进 JSONL，供事后回溯行为。
//! ~keep 跑法（人跑，不进 `just qa`）：
//! ~keep ```text
//! ~keep cargo nextest run --test tui_e2e --run-ignored all -E "test(lab_record_product_tui)" --no-capture
//! ~keep ```
//! ~keep 输出 `/tmp/xylitol_tui_recording/<timestamp>.jsonl`：每行一帧，含相对毫秒、屏幕文本与关键状态
//! ~keep （是否有 spinner、队列行、用户条目数）。`XYLITOL_LAB_KEYS` 可覆盖按键脚本。

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::pty::PtySession;
use crate::PRODUCT_READY_NEEDLE;

const FRAME_INTERVAL: Duration = Duration::from_millis(50);
const RECORD_FOR: Duration = Duration::from_secs(14);

fn scratch_dirs(root: &PathBuf) -> std::io::Result<(PathBuf, PathBuf, PathBuf)> {
    let project = root.join("project");
    let config = root.join("config");
    let home = root.join("home");
    std::fs::create_dir_all(&project)?;
    std::fs::create_dir_all(&config)?;
    std::fs::create_dir_all(&home)?;
    let dir = project.join(".xylitol");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("config.yaml"),
        "models:\n  default_model: fake\n  entries:\n    fake:\n      provider: fake\n      model: fake-model\n      api_key: fake-lab\n",
    )?;
    Ok((project, config, home))
}

#[test]
#[ignore = "人跑终端录制 lab，不进 just qa"]
fn lab_record_product_tui() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let out_dir = std::path::PathBuf::from("/tmp/xylitol_tui_recording");
    std::fs::create_dir_all(&out_dir).expect("recording dir");
    let out_path = out_dir.join(format!("{stamp}.jsonl"));
    let mut out = std::fs::File::create(&out_path).expect("recording file");

    let root = std::env::temp_dir().join(format!("xylitol_lab_rec_{stamp}"));
    let (project, config, home) = scratch_dirs(&root).expect("scratch dirs");
    let mut session = PtySession::spawn_product_fake_with_env(
        120,
        40,
        &project,
        &config,
        &home,
        &[("XYLITOL_FAKE_SLOW_STREAM", "10,300")],
    )
    .expect("spawn product tui");

    session
        .wait_for(PRODUCT_READY_NEEDLE, Duration::from_secs(30), 120, 40)
        .expect("product ready");

    let keys = std::env::var("XYLITOL_LAB_KEYS").unwrap_or_else(|_| {
        [
            ("t1", "\r"),
            ("t2", "\r"),
            ("t3", "\x1b\r"),
            ("t4", "\x1b\r"),
        ]
        .into_iter()
        .map(|(text, enter)| format!("{text}{enter}"))
        .collect::<Vec<_>>()
        .join("")
    });

    let start = Instant::now();
    let mut next_frame = Duration::ZERO;
    let mut typed_at: Option<Duration> = None;
    while start.elapsed() < RECORD_FOR {
        session.drain(Duration::from_millis(20));
        let elapsed = start.elapsed();
        if typed_at.is_none() && elapsed >= Duration::from_millis(800) {
            session.send_keys(&keys).expect("send keys");
            typed_at = Some(elapsed);
        }
        if elapsed >= next_frame {
            next_frame = elapsed + FRAME_INTERVAL;
            let screen = session.screen(120, 40);
            let text = screen.text();
            let frame = serde_json::json!({
                "ms": elapsed.as_millis() as u64,
                "spinner": text.contains('⬢'),
                "queue_rows": text.matches("Follow-up").count() + text.matches("Steering").count(),
                "user_rows": text.matches('❯').count(),
                "text": text,
            });
            writeln!(out, "{frame}").expect("write frame");
        }
    }
    println!("recording written to {}", out_path.display());
}
