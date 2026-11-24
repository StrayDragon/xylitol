#![allow(clippy::all)]
//! ~keep 终端录制 lab：把产品 TUI 的屏幕按帧打时间戳写进 JSONL，供事后回溯行为。
//! ~keep 跑法（人跑，不进 `just qa`）：
//! ~keep ```text
//! ~keep cargo nextest run --test tui_e2e --run-ignored all -E "test(lab_record_product_tui)" --no-capture
//! ~keep ```
//! ~keep 输出 `/tmp/xylitol_tui_recording/<timestamp>.jsonl`：每行一帧，含相对毫秒、屏幕文本与关键状态
//! ~keep （是否有 spinner、队列行、用户条目数）。同目录 `<timestamp>.timeline.txt` 只保留状态变化与
//! ~keep 屏幕差异，供人读和检索。`XYLITOL_LAB_KEYS` 可覆盖按键脚本。
//! ~keep `XYLITOL_LAB_POKE` 若设置，则在 4.5s 再送这段按键，用来确认主循环当时仍收键。

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
    let timeline_path = out_dir.join(format!("{stamp}.timeline.txt"));
    let mut out = std::fs::File::create(&out_path).expect("recording file");
    let mut timeline = std::fs::File::create(&timeline_path).expect("timeline file");
    writeln!(
        timeline,
        "# beats keep only frames whose spinner, queue, user rows, or screen text changed"
    )
    .expect("timeline header");

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
    let poke = std::env::var("XYLITOL_LAB_POKE").unwrap_or_default();
    let mut poked = poke.is_empty();
    let mut prev_text = String::new();
    let mut prev_sig: Option<(bool, usize, usize)> = None;
    let mut spinner_on: Option<(u64, bool)> = None;
    let mut seen_spinner = false;
    let mut spinner_gaps: Vec<(u64, u64)> = Vec::new();
    while start.elapsed() < RECORD_FOR {
        session.drain(Duration::from_millis(20));
        let elapsed = start.elapsed();
        if typed_at.is_none() && elapsed >= Duration::from_millis(800) {
            session.send_keys(&keys).expect("send keys");
            typed_at = Some(elapsed);
        }
        if !poked && elapsed >= Duration::from_millis(4500) {
            session.send_keys(&poke).expect("poke key");
            poked = true;
        }
        if elapsed >= next_frame {
            next_frame = elapsed + FRAME_INTERVAL;
            let screen = session.screen(120, 40);
            let text = screen.text();
            let spinner = text.contains('⬢');
            let queue_rows = text.matches("Follow-up").count() + text.matches("Steering").count();
            let user_rows = text.matches('❯').count();
            let ms = elapsed.as_millis() as u64;
            let frame = serde_json::json!({
                "ms": ms,
                "spinner": spinner,
                "queue_rows": queue_rows,
                "user_rows": user_rows,
                "text": text,
            });
            writeln!(out, "{frame}").expect("write frame");
            if let Some((at, was_on)) = spinner_on {
                if was_on && !spinner {
                    spinner_on = Some((ms, false));
                } else if !was_on && spinner {
                    if seen_spinner {
                        spinner_gaps.push((at, ms.saturating_sub(at)));
                    }
                    seen_spinner = true;
                    spinner_on = Some((ms, true));
                }
            } else {
                seen_spinner = spinner;
                spinner_on = Some((ms, spinner));
            }
            let sig = (spinner, queue_rows, user_rows);
            if prev_sig != Some(sig) || text != prev_text {
                writeln!(
                    timeline,
                    "{ms}\tspinner={spinner}\tqueue={queue_rows}\tusers={user_rows}"
                )
                .expect("timeline beat");
                write_screen_diff(&mut timeline, &prev_text, &text);
                prev_sig = Some(sig);
                prev_text = text;
            }
        }
    }
    writeln!(timeline, "# spinner gaps (off starts at ms, lasts ms)").expect("gap header");
    for (at, held) in spinner_gaps {
        writeln!(timeline, "gap\t{at}\t{held}").expect("gap row");
    }
    println!("recording written to {}", out_path.display());
    println!("timeline written to {}", timeline_path.display());
}

fn write_screen_diff(out: &mut std::fs::File, before: &str, after: &str) {
    let before: Vec<&str> = before.lines().filter(|line| !line.is_empty()).collect();
    let after: Vec<&str> = after.lines().filter(|line| !line.is_empty()).collect();
    let mut shown = 0usize;
    for line in &after {
        if shown >= 8 {
            break;
        }
        if !before.contains(line) {
            let clip: String = line.chars().take(100).collect();
            writeln!(out, "  + {clip}").expect("diff add");
            shown += 1;
        }
    }
    for line in &before {
        if shown >= 12 {
            break;
        }
        if !after.contains(line) {
            let clip: String = line.chars().take(100).collect();
            writeln!(out, "  - {clip}").expect("diff del");
            shown += 1;
        }
    }
}
