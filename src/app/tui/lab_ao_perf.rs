//! Lab: ApplicationOwned render cost on a restored / synthetic long session.
//!
//! Not a qa gate (`#[ignore]`). Run:
//! ```bash
//! eval "$(just cargo-wt-env)"
//! cargo test -p xylitol --lib lab_ao_session_perf_report -- --ignored --nocapture
//! # optional: XYLITOL_LAB_SESSION=<uuid>
//! ```

use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use xylitol_tui::{InputEvent, Terminal};

use super::host::{HostEvent, HostSession};
use crate::infra::session::SessionManager;
use crate::protocol::ports::XySessionStore;

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

#[tokio::test]
#[ignore = "lab: AO perf report against disk session or synthetic seed"]
async fn lab_ao_session_perf_report() {
    let session_id = std::env::var("XYLITOL_LAB_SESSION")
        .unwrap_or_else(|_| "460ad16e-874f-418f-8ca0-dabc58f89320".into());
    let sessions_dir = default_sessions_dir();
    let path = sessions_dir.join(format!("{session_id}.jsonl"));

    let (sid, entries, source) = if path.is_file() {
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
    } else {
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
    for _ in 0..40 {
        let t0 = Instant::now();
        session
            .step(HostEvent::Input(mouse(MouseEventKind::ScrollUp, 20, 10)))
            .expect("wheel");
        session.tui.render_now().expect("wheel paint");
        wheel_us.push(t0.elapsed().as_micros() as u64);
        if session.tui.last_render_perf().ao_reprojected {
            wheel_reprojected += 1;
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
         idle_paint p50={}us p95={}us\n\
         wheel_paint p50={}us p95={}us reprojected={wheel_reprojected}/40 ao_reproject_frames={}\n\
         drag_paint p50={}us p95={}us\n\
         last_frame component_lines={} paint_lines={} finalize_checks={} reuses={} do_render_us={} ao_reprojected={}",
        warm_perf.component_lines,
        warm_perf.paint_lines,
        warm_perf.finalize_width_checks,
        warm_perf.finalize_line_reuses,
        percentile(&idle_us, 0.50),
        percentile(&idle_us, 0.95),
        percentile(&wheel_us, 0.50),
        percentile(&wheel_us, 0.95),
        session.tui.ao_reproject_frames_for_test(),
        percentile(&drag_us, 0.50),
        percentile(&drag_us, 0.95),
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
