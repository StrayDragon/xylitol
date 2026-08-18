//! Scripted live-window tape (c1761 / att33).
//!
//! Same frames drive:
//! - `cargo test` / UiRoot harness (`replay_live_window`)
//! - `/debug activity-fold-live` (inject + Choice slot; not a real ReAct run)
//!
//! Ended-session hand-test is `/debug activity-fold-resume` (seeded JSONL).
//!
//! Reproduce: last painted frame stays on the transcript so a FAIL names the
//! step (busy → Thinking → inflight tool → sealed -3 → open -2 → Ask).
//! Answering Choice then applies [`live_ask_close_events`] (not a live agent).

use serde_json::json;

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::{UiModel, apply_xy_event};

/// Scripted-tape assertion failure (crate-private; not `Xy*`).
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct LiveTapeMismatch(pub String);

impl From<String> for LiveTapeMismatch {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// One observable checkpoint after applying `events`.
pub struct LiveWindowFrame {
    pub name: &'static str,
    pub events: Vec<XyEvent>,
    pub must: &'static [&'static str],
    pub must_not: &'static [&'static str],
}

pub struct LiveWindowTapeReport {
    pub ok: bool,
    pub lines: Vec<String>,
}

fn tool_start(id: &str, name: &str, path: &str) -> XyEvent {
    XyEvent::ToolExecutionStart {
        id: id.into(),
        name: name.into(),
        args: json!({ "path": path }),
    }
}

fn tool_end(id: &str, name: &str) -> XyEvent {
    XyEvent::ToolExecutionEnd {
        id: id.into(),
        name: name.into(),
        result: "ok".into(),
        is_error: false,
    }
}

/// Tool id for the tape's Ask Waiting frame and the Choice close-out.
pub const LIVE_ASK_TOOL_ID: &str = "ask1";

/// Distinct closing assistant body after the debug Choice is answered / skipped.
pub const LIVE_ASK_CLOSE_TEXT: &str = "Thanks — continuing from your answer.";

/// Scripted UX after `/debug activity-fold-live` Choice finishes (no ReAct).
pub fn live_ask_close_events(result: &str) -> Vec<XyEvent> {
    vec![
        XyEvent::ToolExecutionEnd {
            id: LIVE_ASK_TOOL_ID.into(),
            name: "ask".into(),
            result: result.into(),
            is_error: false,
        },
        XyEvent::TextDelta(LIVE_ASK_CLOSE_TEXT.into()),
        XyEvent::AgentEnd { messages: vec![] },
    ]
}

/// Ordered live-window checkpoints (after `UiModel::begin_run`).
pub fn live_window_frames() -> Vec<LiveWindowFrame> {
    vec![
        LiveWindowFrame {
            name: "1-busy-before-tokens",
            events: vec![],
            must: &["debug: activity-fold live window"],
            must_not: &["Planning next moves", "Worked for"],
        },
        LiveWindowFrame {
            name: "2-thinking-stream-is-inflight",
            events: vec![XyEvent::ThinkingDelta("consider next edit".into())],
            must: &["Thinking"],
            must_not: &["Planning next moves", "Worked for", "Ctrl+T", "Thought"],
        },
        LiveWindowFrame {
            name: "3-inflight-read-short-line",
            events: vec![tool_start("r1", "read", "old.rs")],
            must: &["Exploring old.rs"],
            must_not: &["Worked for", "Editing", "Planning next moves"],
        },
        LiveWindowFrame {
            name: "4-toolend-opens-cluster-header",
            events: vec![tool_end("r1", "read")],
            must: &["Exploring old.rs"],
            must_not: &["Worked for", "Planning next moves"],
        },
        LiveWindowFrame {
            name: "5-assistant-body-seals-no-planning",
            events: vec![XyEvent::TextDelta("mid-body".into())],
            must: &["mid-body"],
            must_not: &["Planning next moves"],
        },
        LiveWindowFrame {
            name: "6-new-cluster-inflight-edit",
            events: vec![tool_start("e1", "edit", "a.rs")],
            must: &["Explored old.rs", "Editing a.rs"],
            must_not: &["Worked for", "Planning next moves"],
        },
        LiveWindowFrame {
            name: "7-open-cluster-after-first-edit",
            events: vec![tool_end("e1", "edit")],
            must: &["Explored old.rs", "Editing a.rs"],
            must_not: &["Worked for", "Planning next moves"],
        },
        LiveWindowFrame {
            name: "8-toolend-updates-minus2-not-minus3",
            events: vec![tool_start("e2", "edit", "b.rs"), tool_end("e2", "edit")],
            must: &["Explored old.rs", "Editing 2 files"],
            must_not: &["Worked for", "Planning next moves"],
        },
        LiveWindowFrame {
            name: "9-ask-waiting",
            events: vec![XyEvent::ToolExecutionStart {
                id: LIVE_ASK_TOOL_ID.into(),
                name: "ask".into(),
                args: json!({}),
            }],
            must: &["Asking questions", "Ask ·"],
            must_not: &["Planning next moves", "Worked for"],
        },
    ]
}

/// Apply the tape to `model`, painting after each frame. Does not idle the run.
pub fn replay_live_window(
    model: &mut UiModel,
    mut paint: impl FnMut(&UiModel) -> String,
) -> LiveWindowTapeReport {
    model.begin_run("debug: activity-fold live window");
    let mut lines = Vec::new();
    let mut ok = true;
    for frame in live_window_frames() {
        for event in &frame.events {
            apply_xy_event(model, event);
        }
        let plain = paint(model);
        match check_plain(&plain, &frame) {
            Ok(()) => lines.push(format!("PASS {}", frame.name)),
            Err(why) => {
                ok = false;
                lines.push(format!("FAIL {}: {why}", frame.name));
            }
        }
    }
    LiveWindowTapeReport { ok, lines }
}

pub fn check_plain(plain: &str, frame: &LiveWindowFrame) -> Result<(), LiveTapeMismatch> {
    for needle in frame.must {
        if !plain.contains(needle) {
            return Err(format!("missing `{needle}` in: {plain}").into());
        }
    }
    for needle in frame.must_not {
        if plain.contains(needle) {
            return Err(format!("unexpected `{needle}` in: {plain}").into());
        }
    }
    Ok(())
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}
