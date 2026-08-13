//! `/debug activity-fold-live` — inject the att33 live-window tape (no LLM).
//!
//! Reuses [`crate::app::tui::activity_fold::replay_live_window`]. Last frame
//! stays painted; a scroll notice lists PASS/FAIL per checkpoint.

use xylitol_tui::{Component, Terminal};

use crate::app::tui::activity_fold::{replay_live_window, strip_ansi_live_window};
use crate::app::tui::host::HostSession;

/// Run the live-window tape against the current product UI root.
pub(super) fn run_activity_fold_live<T: Terminal>(session: &mut HostSession<T>) {
    let mut model = session.ui_model().clone();
    let root = session.ui_root().cloned();
    let report = replay_live_window(&mut model, |m| {
        let Some(root) = root.as_ref() else {
            return String::new();
        };
        let mut r = root.borrow_mut();
        r.apply_ui_model(m);
        strip_ansi_live_window(&r.render(100).join("\n"))
    });
    *session.ui_model_mut() = model;
    session.sync_ui_root_from_model();

    let header = if report.ok {
        format!(
            "activity-fold-live: OK ({} checkpoints)",
            report.lines.len()
        )
    } else {
        "activity-fold-live: FAIL".into()
    };
    let body = std::iter::once(header)
        .chain(report.lines)
        .collect::<Vec<_>>()
        .join("\n");
    session.push_scroll_notice(body);
    let _ = session.render_now();
}
