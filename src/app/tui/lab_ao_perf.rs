//! Lab: ApplicationOwned render cost on a restored / synthetic long session.
//!
//! Not a qa gate (`#[ignore]`). Run:
//! ```bash
//! eval "$(just cargo-wt-env)"
//! cargo test -p xylitol --lib lab_ao_session_perf_report -- --ignored --nocapture
//! cargo test -p xylitol --lib lab_ao_stream_delta_perf_report -- --ignored --nocapture
//! cargo test -p xylitol --lib lab_ao_memory_profile -- --ignored --nocapture
//! # optional: XYLITOL_LAB_SESSION=<uuid> (memory profile: profile one real session)
//! ```

use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use xylitol_tui::{InputEvent, Terminal};

use super::bridge::apply_xy_event;
use super::host::{HostEvent, HostSession};
use crate::app::core::driver::XyEvent;
use crate::infra::session::SessionManager;
use crate::protocol::ports::XySessionStore;
use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry, fixture_message_json};

struct LabTerminal {
    cols: u16,
    rows: u16,
    started: bool,
    stopped: bool,
    mouse_capture_active: bool,
    alternate_screen_active: bool,
}

impl LabTerminal {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            started: false,
            stopped: false,
            mouse_capture_active: false,
            alternate_screen_active: false,
        }
    }
}

impl Terminal for LabTerminal {
    fn write(&mut self, _data: &str) {}
    fn columns(&self) -> u16 {
        self.cols
    }
    fn rows(&self) -> u16 {
        self.rows
    }
    fn hide_cursor(&mut self) {}
    fn show_cursor(&mut self) {}
    fn clear_line(&mut self) {}
    fn clear_from_cursor(&mut self) {}
    fn clear_screen(&mut self) {}
    fn flush(&mut self) {}
    fn set_size_hint(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
    }
    fn start(&mut self) {
        self.started = true;
    }
    fn stop(&mut self) {
        self.stopped = true;
        self.mouse_capture_active = false;
        self.alternate_screen_active = false;
    }
    fn enter_alternate_screen(&mut self) {
        self.alternate_screen_active = true;
    }
    fn leave_alternate_screen(&mut self) {
        self.alternate_screen_active = false;
    }
    fn alternate_screen_active(&self) -> bool {
        self.alternate_screen_active
    }
    fn enable_mouse_capture(&mut self) {
        self.mouse_capture_active = true;
    }
    fn disable_mouse_capture(&mut self) {
        self.mouse_capture_active = false;
    }
    fn mouse_capture_active(&self) -> bool {
        self.mouse_capture_active
    }
}

fn default_sessions_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XYLITOL_SESSIONS_DIR") {
        return PathBuf::from(dir);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xylitol")
        .join("sessions")
}

fn mouse(kind: MouseEventKind, col: u16, row: u16) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Resident set size in kB from `/proc/self/status` (Linux lab only).
fn rss_kb() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse()
                .unwrap_or(0);
        }
    }
    0
}

#[tokio::test]
#[ignore = "lab: AO perf report against disk session or synthetic seed"]
async fn lab_ao_session_perf_report() {
    // Disk session is opt-in via XYLITOL_LAB_SESSION; default is the synthetic
    // seed so the probe runs on any machine (no hardcoded session UUIDs).
    let (sid, entries, source) = match std::env::var("XYLITOL_LAB_SESSION").ok() {
        Some(session_id) => {
            let sessions_dir = default_sessions_dir();
            let path = sessions_dir.join(format!("{session_id}.jsonl"));
            assert!(
                path.is_file(),
                "XYLITOL_LAB_SESSION={session_id} not found at {}",
                path.display()
            );
            let mgr = SessionManager::new(sessions_dir);
            let entries = mgr
                .load_entries(&session_id)
                .await
                .expect("load lab session");
            (
                session_id,
                entries,
                format!("disk:{path}", path = path.display()),
            )
        }
        None => {
            let mgr = SessionManager::in_memory();
            mgr.create("lab-ao-perf", Some("."), None)
                .await
                .expect("create");
            crate::app::debug_fixtures::seed_scene(&mgr, "lab-ao-perf", "ao-perf-scroll")
                .await
                .expect("seed ao-perf-scroll");
            let entries = mgr.load_entries("lab-ao-perf").await.expect("load seed");
            (
                "lab-ao-perf".into(),
                entries,
                "synthetic:ao-perf-scroll".into(),
            )
        }
    };

    let mut session = HostSession::new_product_ui(LabTerminal::new(120, 40));
    session.apply_cli_restored_session(&sid, entries);
    session.tui.clear_finalize_counters_for_test();

    let warm = Instant::now();
    session.tui.render_now().expect("warm paint");
    let warm_us = warm.elapsed().as_micros() as u64;
    let warm_perf = session.tui.last_render_perf();

    let mut idle_us = Vec::new();
    for _ in 0..30 {
        let t0 = Instant::now();
        session.step(HostEvent::Tick).expect("tick");
        // Force paint path like a dirty spinner would.
        session.tui.request_render(false);
        session.tui.render_now().expect("idle paint");
        idle_us.push(t0.elapsed().as_micros() as u64);
    }
    idle_us.sort_unstable();

    session.tui.clear_ao_reproject_frames_for_test();
    let mut wheel_us = Vec::new();
    let mut wheel_reprojected = 0u32;
    let mut wheel_vertical_shifted = 0u32;
    let wheel_notch = session.tui.application_owned_wheel_notch();
    for _ in 0..40 {
        let t0 = Instant::now();
        assert!(session.tui.application_owned_scroll_by(-wheel_notch));
        session.tui.render_now().expect("wheel paint");
        wheel_us.push(t0.elapsed().as_micros() as u64);
        if session.tui.last_render_perf().ao_reprojected {
            wheel_reprojected += 1;
        }
        if session.tui.last_render_perf().ao_vertical_shifted {
            wheel_vertical_shifted += 1;
        }
    }
    wheel_us.sort_unstable();

    let mut drag_us = Vec::new();
    session
        .step(HostEvent::Input(mouse(
            MouseEventKind::Down(MouseButton::Left),
            8,
            5,
        )))
        .expect("down");
    for row in 6..30 {
        let t0 = Instant::now();
        session
            .step(HostEvent::Input(mouse(
                MouseEventKind::Drag(MouseButton::Left),
                12,
                row,
            )))
            .expect("drag");
        session.tui.render_now().expect("drag paint");
        drag_us.push(t0.elapsed().as_micros() as u64);
    }
    session
        .step(HostEvent::Input(mouse(
            MouseEventKind::Up(MouseButton::Left),
            12,
            29,
        )))
        .expect("up");
    drag_us.sort_unstable();

    let last = session.tui.last_render_perf();
    eprintln!(
        "REPORT lab_ao_session_perf\n\
         source={source}\n\
         warm_us={warm_us} component_lines={} paint_lines={} finalize_checks={} reuses={}\n\
         idle_paint p50={}us p95={}us max={}us\n\
         wheel_paint p50={}us p95={}us max={}us reprojected={wheel_reprojected}/40 vertical_shifted={wheel_vertical_shifted}/40 ao_reproject_frames={}\n\
         drag_paint p50={}us p95={}us max={}us\n\
         last_frame component_lines={} paint_lines={} finalize_checks={} reuses={} do_render_us={} ao_reprojected={}",
        warm_perf.component_lines,
        warm_perf.paint_lines,
        warm_perf.finalize_width_checks,
        warm_perf.finalize_line_reuses,
        percentile(&idle_us, 0.50),
        percentile(&idle_us, 0.95),
        percentile(&idle_us, 1.00),
        percentile(&wheel_us, 0.50),
        percentile(&wheel_us, 0.95),
        percentile(&wheel_us, 1.00),
        session.tui.ao_reproject_frames_for_test(),
        percentile(&drag_us, 0.50),
        percentile(&drag_us, 0.95),
        percentile(&drag_us, 1.00),
        last.component_lines,
        last.paint_lines,
        last.finalize_width_checks,
        last.finalize_line_reuses,
        last.do_render_us,
        last.ao_reprojected,
    );

    assert!(
        wheel_reprojected >= 35,
        "most wheel frames should reproject-only, got {wheel_reprojected}/40"
    );

    assert!(
        warm_perf.paint_lines <= 40,
        "AO paint_lines must be ≤ term rows, got {}",
        warm_perf.paint_lines
    );
    assert!(
        warm_perf.component_lines > warm_perf.paint_lines,
        "long session should emit more component lines than paint surface"
    );
    assert!(
        warm_perf.finalize_width_checks <= warm_perf.paint_lines as u64 + 2,
        "finalize must not walk full transcript after AO project: checks={} paint={}",
        warm_perf.finalize_width_checks,
        warm_perf.paint_lines
    );
}

fn cpu_jiffies_self() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let parts: Vec<&str> = stat.split_whitespace().collect();
    // fields 14/15 are utime/stime (1-based → indices 13/14)
    let utime: u64 = parts.get(13)?.parse().ok()?;
    let stime: u64 = parts.get(14)?.parse().ok()?;
    Some(utime.saturating_add(stime))
}

fn cpu_pct(j0: u64, j1: u64, wall_secs: f64) -> f64 {
    let hz = 100.0; // Linux USER_HZ default; good enough for lab ratios
    if wall_secs <= 0.0 {
        return 0.0;
    }
    100.0 * (j1.saturating_sub(j0) as f64) / hz / wall_secs
}

async fn seed_long_session(session: &mut HostSession<LabTerminal>) {
    // Disk session is opt-in via XYLITOL_LAB_SESSION (see session perf report);
    // default runs on the synthetic seed so the probe is machine-portable.
    let (sid, entries) = match std::env::var("XYLITOL_LAB_SESSION").ok() {
        Some(session_id) => {
            let sessions_dir = default_sessions_dir();
            let path = sessions_dir.join(format!("{session_id}.jsonl"));
            assert!(
                path.is_file(),
                "XYLITOL_LAB_SESSION={session_id} not found at {}",
                path.display()
            );
            let mgr = SessionManager::new(sessions_dir);
            let entries = mgr
                .load_entries(&session_id)
                .await
                .expect("load lab session");
            (session_id, entries)
        }
        None => {
            let mgr = SessionManager::in_memory();
            mgr.create("lab-ao-stream", Some("."), None)
                .await
                .expect("create");
            crate::app::debug_fixtures::seed_scene(&mgr, "lab-ao-stream", "ao-perf-scroll")
                .await
                .expect("seed");
            let entries = mgr.load_entries("lab-ao-stream").await.expect("load");
            ("lab-ao-stream".into(), entries)
        }
    };
    session.apply_cli_restored_session(&sid, entries);
}

/// Compare per-delta forced paints vs tick-paced stream paints (production path).
///
/// Automated answer to "did stream coalesce / defer help?" — paint counts,
/// Σ do_render_us, and /proc self CPU% for each strategy.
#[tokio::test]
#[ignore = "lab: stream delta paint/CPU report"]
async fn lab_ao_stream_delta_perf_report() {
    const DELTAS: usize = 160;
    /// Simulate busy ticker: one paint every N deferred deltas (~60Hz vs token rate).
    const TICK_EVERY: usize = 8;

    let mut session = HostSession::new_product_ui(LabTerminal::new(120, 40));
    seed_long_session(&mut session).await;
    session.tui.render_now().expect("warm");

    session
        .step(HostEvent::Xy(Box::new(XyEvent::AgentStart {
            session_id: "lab".into(),
            model: "lab".into(),
        })))
        .expect("agent start");
    session
        .step(HostEvent::Xy(Box::new(XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        })))
        .expect("msg start");

    // --- A: legacy worst case (every token forces paint) ---
    let j0 = cpu_jiffies_self();
    let t0 = Instant::now();
    let mut uncoal_us = 0u64;
    let mut uncoal_paints = 0u32;
    let mut max_component_lines = 0usize;
    for i in 0..DELTAS {
        apply_xy_event(
            session.ui_model_mut(),
            &XyEvent::TextDelta(format!("tok{i} ")),
        );
        session.sync_ui_root_from_model();
        session.tui.render_now().expect("paint");
        let perf = session.tui.last_render_perf();
        uncoal_us = uncoal_us.saturating_add(perf.do_render_us);
        uncoal_paints += 1;
        max_component_lines = max_component_lines.max(perf.component_lines);
    }
    let uncoal_wall = t0.elapsed().as_secs_f64();
    let uncoal_cpu = match (j0, cpu_jiffies_self()) {
        (Some(a), Some(b)) => cpu_pct(a, b, uncoal_wall),
        _ => -1.0,
    };

    session.ui_model_mut().streaming_assistant.clear();
    session.sync_ui_root_from_model();
    session.tui.render_now().expect("reset paint");

    // --- B: production tick-paced (step defers TextDelta; paint ≤ once / N tokens) ---
    let j1 = cpu_jiffies_self();
    let t1 = Instant::now();
    let mut paced_us = 0u64;
    let mut paced_paints = 0u32;
    for i in 0..DELTAS {
        session
            .step(HostEvent::Xy(Box::new(XyEvent::TextDelta(format!(
                "tok{i} "
            )))))
            .expect("delta");
        assert!(
            session.tui.is_render_requested(),
            "TextDelta must arm render without painting immediately"
        );
        if (i + 1) % TICK_EVERY == 0 {
            session.tui.render_now().expect("paced paint");
            let perf = session.tui.last_render_perf();
            paced_us = paced_us.saturating_add(perf.do_render_us);
            paced_paints += 1;
        }
    }
    if session.tui.is_render_requested() {
        session.tui.render_now().expect("tail paint");
        let perf = session.tui.last_render_perf();
        paced_us = paced_us.saturating_add(perf.do_render_us);
        paced_paints += 1;
    }
    let paced_wall = t1.elapsed().as_secs_f64();
    let paced_cpu = match (j1, cpu_jiffies_self()) {
        (Some(a), Some(b)) => cpu_pct(a, b, paced_wall),
        _ => -1.0,
    };

    let us_ratio = if paced_us == 0 {
        0.0
    } else {
        uncoal_us as f64 / paced_us as f64
    };
    let paint_ratio = if paced_paints == 0 {
        0.0
    } else {
        f64::from(uncoal_paints) / f64::from(paced_paints)
    };

    eprintln!(
        "REPORT lab_ao_stream_delta_perf\n\
         deltas={DELTAS} tick_every={TICK_EVERY} max_component_lines={max_component_lines}\n\
         per_token_paint paints={uncoal_paints} paints_per_s={:.0} sum_do_render_us={uncoal_us} wall_s={uncoal_wall:.3} cpu%≈{uncoal_cpu:.1}\n\
         tick_paced      paints={paced_paints} paints_per_s={:.0} sum_do_render_us={paced_us} wall_s={paced_wall:.3} cpu%≈{paced_cpu:.1}\n\
         ratio paints={paint_ratio:.2}x  sum_us={us_ratio:.2}x (higher ⇒ tick-pace saves more)",
        uncoal_paints as f64 / uncoal_wall,
        paced_paints as f64 / paced_wall,
    );

    assert_eq!(uncoal_paints, DELTAS as u32);
    assert!(
        paced_paints <= DELTAS.div_ceil(TICK_EVERY) as u32 + 2,
        "tick-paced should paint about once per {TICK_EVERY} deltas, got {paced_paints}"
    );
    assert!(
        paced_us < uncoal_us,
        "tick-paced Σ do_render_us ({paced_us}) should beat per-token ({uncoal_us})"
    );
    assert!(
        paint_ratio >= 4.0,
        "expected fewer paints under tick pace, got {paint_ratio:.2}x"
    );
}

// ---- c2490: memory profile (transcript scale → resident memory) ----

fn msg_entry(role: &str, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: 0,
        },
        message: fixture_message_json(role, text),
    })
}

/// One turn = a short user line + a multi-line assistant block (CJK content,
/// wraps like real output at 120 cols).
async fn append_turn(store: &SessionManager, sid: &str, i: usize, block_lines: usize) {
    let user = format!("q{i}: brief ask");
    let block = (0..block_lines)
        .map(|l| format!("answer {i}.{l}: {}", "字".repeat(40)))
        .collect::<Vec<_>>()
        .join("\n");
    store
        .append_session_entry(sid, &msg_entry("user", &user))
        .await
        .expect("append user entry");
    store
        .append_session_entry(sid, &msg_entry("assistant", &block))
        .await
        .expect("append assistant entry");
}

/// Resident-memory profile: synthetic transcript ladder 1k → 1w → 10w display
/// lines in one process (models a session growing in place); RSS sampled from
/// `/proc/self/status` after a settled warm render per scale.
/// `XYLITOL_LAB_SESSION=<uuid>` profiles one real disk session instead.
#[tokio::test]
#[ignore = "lab: memory profile across transcript scales"]
async fn lab_ao_memory_profile() {
    const TERM_COLS: u16 = 120;
    const TERM_ROWS: u16 = 40;
    /// Display lines per assistant block (pre-wrap; actual lines measured live).
    const BLOCK_LINES: usize = 8;
    const SCALES: &[(&str, usize)] = &[("1k", 1_000), ("1w", 10_000), ("10w", 100_000)];

    eprintln!(
        "REPORT lab_ao_memory_profile baseline_rss_kb={} cols={TERM_COLS} rows={TERM_ROWS} block_lines={BLOCK_LINES}",
        rss_kb()
    );

    let mut session = HostSession::new_product_ui(LabTerminal::new(TERM_COLS, TERM_ROWS));

    if let Ok(session_uuid) = std::env::var("XYLITOL_LAB_SESSION") {
        let dir = default_sessions_dir();
        if dir.join(format!("{session_uuid}.jsonl")).is_file() {
            let mgr = SessionManager::new(dir);
            let entries = mgr
                .load_entries(&session_uuid)
                .await
                .expect("load lab session");
            let warm = Instant::now();
            session.apply_cli_restored_session(&session_uuid, entries);
            session.tui.render_now().expect("warm render");
            let perf = session.tui.last_render_perf();
            eprintln!(
                "REPORT lab_ao_memory_profile scale=disk entries_src={session_uuid} \
                 component_lines={} paint_lines={} warm_render_ms={} rss_kb={}",
                perf.component_lines,
                perf.paint_lines,
                warm.elapsed().as_millis(),
                rss_kb()
            );
            return;
        }
        eprintln!(
            "XYLITOL_LAB_SESSION={session_uuid} not found on disk; falling back to synthetic ladder"
        );
    }

    let mgr = SessionManager::in_memory();
    mgr.create("lab-ao-mem", Some("."), None)
        .await
        .expect("create session");
    let sid = "lab-ao-mem".to_string();

    let mut blocks_done = 0usize;
    let mut prev_rss = rss_kb();
    let mut prev_lines = 0u64;
    for (name, target_lines) in SCALES {
        let target_blocks = target_lines / BLOCK_LINES;
        let fill = Instant::now();
        while blocks_done < target_blocks {
            append_turn(&mgr, &sid, blocks_done, BLOCK_LINES).await;
            blocks_done += 1;
        }
        let entries = mgr.load_entries(&sid).await.expect("load entries");
        let entry_count = entries.len();
        let warm = Instant::now();
        session.apply_cli_restored_session(&sid, entries);
        session.tui.render_now().expect("warm render");
        let warm_ms = warm.elapsed().as_millis();
        session.tui.request_render(false);
        session.tui.render_now().expect("settle render");
        let perf = session.tui.last_render_perf();
        let rss = rss_kb();
        let d_rss = rss.saturating_sub(prev_rss);
        let d_lines = (perf.component_lines as u64).saturating_sub(prev_lines);
        let kb_per_1k_lines = if d_lines > 0 {
            d_rss as f64 * 1_000.0 / d_lines as f64
        } else {
            0.0
        };
        eprintln!(
            "REPORT lab_ao_memory_profile scale={name} store_fill_ms={} entries={} \
             component_lines={} paint_lines={} warm_render_ms={warm_ms} rss_kb={rss} \
             delta_rss_kb={d_rss} kb_per_1k_lines={:.1}",
            fill.elapsed().as_millis(),
            entry_count,
            perf.component_lines,
            perf.paint_lines,
            kb_per_1k_lines,
        );
        prev_rss = rss;
        prev_lines = perf.component_lines as u64;
    }
}
