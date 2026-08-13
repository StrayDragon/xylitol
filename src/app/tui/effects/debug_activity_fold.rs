//! `/debug activity-fold-live` — inject the att33 live-window tape (no LLM).
//!
//! Reuses [`crate::app::tui::activity_fold::replay_live_window`]. Last frame
//! stays painted; a scroll notice lists PASS/FAIL per checkpoint. Then the
//! real Ask Choice slot is mounted so answering can run
//! [`crate::app::tui::activity_fold::live_ask_close_events`].

use xylitol_tui::{ChoiceMode, ChoiceOption, ChoiceQuestion, Component, Terminal};

use crate::app::tui::activity_fold::{replay_live_window, strip_ansi_live_window};
use crate::app::tui::host::HostSession;

/// Fixture questions for the live-window Ask slot (not a real `ask` tool).
pub(super) fn live_ask_questions() -> Vec<ChoiceQuestion> {
    vec![ChoiceQuestion {
        id: "next".into(),
        label: "Next".into(),
        prompt: "activity-fold-live: next step?".into(),
        mode: ChoiceMode::Single,
        options: vec![
            ChoiceOption::new("continue", "Continue").recommended(),
            ChoiceOption::new("stop", "Stop here"),
        ],
        allow_other: false,
    }]
}

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

    session.arm_debug_live_ask_close();
    let (tx, _rx) = tokio::sync::oneshot::channel();
    session.mount_ask_choice(live_ask_questions(), tx);
    let _ = session.render_now();
}
