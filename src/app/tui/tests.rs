//! Host harness tests — no real TTY (ath5).

use super::bridge::{UiModel, UiPhase, apply_xy_event};
use super::host::{HostEvent, HostSession, LayoutMode, TOO_SMALL_HINT, is_too_small};
use super::ui_root::build_root;
use crate::app::core::driver::XyEvent;
use xylitol_tui::{Component, InputEvent, Terminal};

/// Minimal in-memory terminal for host tests.
struct TestTerminal {
    cols: u16,
    rows: u16,
    pub frames: Vec<String>,
    started: bool,
    stopped: bool,
}

impl TestTerminal {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            frames: Vec::new(),
            started: false,
            stopped: false,
        }
    }
}

impl Terminal for TestTerminal {
    fn write(&mut self, data: &str) {
        self.frames.push(data.to_string());
    }
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
    }
}

#[test]
fn too_small_predicate() {
    assert!(is_too_small(39, 24));
    assert!(is_too_small(80, 5));
    assert!(!is_too_small(40, 6));
}

#[test]
fn harness_min_size_shows_hint() {
    let mut session = HostSession::new(TestTerminal::new(20, 3), build_root);
    assert_eq!(session.mode(), LayoutMode::TooSmall);
    session.render_now().expect("render");
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains(TOO_SMALL_HINT),
        "expected hint in writes, got: {joined:?}"
    );
}

#[test]
fn harness_resize_to_ready() {
    let mut session = HostSession::new(TestTerminal::new(20, 3), build_root);
    assert_eq!(session.mode(), LayoutMode::TooSmall);
    session
        .step(HostEvent::Resize { cols: 80, rows: 24 })
        .unwrap();
    session.render_now().unwrap();
    assert_eq!(session.mode(), LayoutMode::Ready);
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains('─') || joined.contains("ornith") || joined.contains("~/"),
        "expected editor border or footer chrome, got: {joined:?}"
    );
}

#[test]
fn harness_never_calls_terminal_start() {
    let mut session = HostSession::new(TestTerminal::new(80, 24), build_root);
    session.render_now().unwrap();
    session
        .step(HostEvent::Input(InputEvent::Paste("hi".into())))
        .unwrap();
    assert!(
        !session.tui.terminal.started,
        "HostSession must not call Terminal::start"
    );
}

#[test]
fn product_tui_source_has_no_tui_start_call() {
    let sources = [
        ("mod.rs", include_str!("mod.rs")),
        ("host.rs", include_str!("host.rs")),
        ("ui_root.rs", include_str!("ui_root.rs")),
        ("terminal_guard.rs", include_str!("terminal_guard.rs")),
        ("bridge.rs", include_str!("bridge.rs")),
    ];
    for (name, src) in sources {
        for line in src.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with('*') {
                continue;
            }
            assert!(
                !trimmed.contains("TUI::start")
                    && !trimmed.contains(".start_with_flag")
                    && !trimmed.contains("tui.start("),
                "{name}: product must not call TUI::start — line: {trimmed}"
            );
        }
    }
}

#[test]
fn render_modules_do_not_match_xy_event() {
    // atb1: render layer must not match XyEvent — only bridge does.
    let ui_root = include_str!("ui_root.rs");
    assert!(
        !ui_root.contains("XyEvent"),
        "ui_root must stay XyEvent-free"
    );
    let bridge = include_str!("bridge.rs");
    assert!(
        bridge.contains("apply_xy_event"),
        "bridge must own apply_xy_event"
    );
}

#[test]
fn quit_event_stops_session() {
    let mut session = HostSession::new(TestTerminal::new(80, 24), build_root);
    session.step(HostEvent::Quit).unwrap();
    assert!(session.should_quit());
}

#[test]
fn build_root_ready_is_component() {
    let mut kids = build_root(LayoutMode::Ready);
    assert_eq!(kids.len(), 1);
    let lines = kids[0].render(80);
    assert!(!lines.is_empty());
}

fn ctrl_c_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[test]
fn harness_ctrl_c_clears_editor_then_quits() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("keep me");

    session.step(HostEvent::Input(ctrl_c_event())).unwrap();
    assert!(
        !session.should_quit(),
        "non-empty editor must clear, not quit"
    );
    assert!(
        root.borrow().editor_text().is_empty(),
        "Ctrl+C should clear editor"
    );

    session.step(HostEvent::Input(ctrl_c_event())).unwrap();
    assert!(
        session.should_quit(),
        "second Ctrl+C on empty editor should quit"
    );
}

#[test]
fn harness_ctrl_c_consumed_before_editor_insert() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("x");
    session.step(HostEvent::Input(ctrl_c_event())).unwrap();
    assert_eq!(root.borrow().editor_text(), "");
    assert!(!session.should_quit());
}

fn esc_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn enter_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[test]
fn harness_double_esc_opens_session_tree() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("");
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert!(!root.borrow().tree_open(), "single Esc must not open tree");
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert!(
        root.borrow().tree_open(),
        "double Esc on empty editor should open session tree"
    );
    session.render_now().unwrap();
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains("Session tree"),
        "expected tree chrome; got: {joined}"
    );
}

#[test]
fn harness_esc_closes_session_tree() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().open_session_tree_for_test();
    assert!(root.borrow().tree_open());
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert!(!root.borrow().tree_open());
}

#[test]
fn harness_enter_travel_stub_closes_tree() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().open_session_tree_for_test();
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert!(!root.borrow().tree_open());
    session.render_now().unwrap();
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains("travel →"),
        "expected travel stub in transcript; got: {joined}"
    );
}

#[test]
fn harness_xy_events_update_ui_model_and_transcript() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.on_run_started("hello");
    session
        .step(HostEvent::Xy(Box::new(XyEvent::TextDelta("Hi".into()))))
        .unwrap();
    session
        .step(HostEvent::Xy(Box::new(XyEvent::QueueUpdate {
            steer_count: 1,
            follow_up_count: 0,
        })))
        .unwrap();
    assert_eq!(session.ui_model().phase, UiPhase::Busy);
    assert_eq!(session.ui_model().queue.steer_count, 1);

    session
        .step(HostEvent::Xy(Box::new(XyEvent::AgentEnd {
            messages: vec![],
        })))
        .unwrap();
    assert_eq!(session.ui_model().phase, UiPhase::Idle);
    assert!(!session.run_active());

    session.render_now().unwrap();
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains("hello") && joined.contains("Hi"),
        "expected bridge scrollback; got: {joined}"
    );
    assert!(
        joined.contains('❯') || joined.contains("> hello"),
        "expected user glyph prefix; got: {joined}"
    );
}

#[test]
fn harness_middle_turn_end_keeps_busy() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.on_run_started("x");
    session
        .step(HostEvent::Xy(Box::new(XyEvent::TurnEnd { turn_index: 0 })))
        .unwrap();
    assert_eq!(session.ui_model().phase, UiPhase::Busy);
    assert!(session.run_active());
}

#[test]
fn harness_idle_enter_queues_submit() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("run me");
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert_eq!(session.take_submit().as_deref(), Some("run me"));
    assert!(root.borrow().editor_text().is_empty());
}

#[test]
fn apply_xy_event_sequence_snapshot() {
    let mut model = UiModel::new();
    model.begin_run("prompt");
    apply_xy_event(
        &mut model,
        &XyEvent::AgentStart {
            session_id: "s".into(),
            model: "m".into(),
        },
    );
    apply_xy_event(&mut model, &XyEvent::TurnStart { turn_index: 0 });
    apply_xy_event(&mut model, &XyEvent::TextDelta("A".into()));
    apply_xy_event(&mut model, &XyEvent::TurnEnd { turn_index: 0 });
    apply_xy_event(&mut model, &XyEvent::TurnStart { turn_index: 1 });
    apply_xy_event(
        &mut model,
        &XyEvent::MessageEnd {
            role: "assistant".into(),
            message: None,
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::QueueUpdate {
            steer_count: 0,
            follow_up_count: 0,
        },
    );
    apply_xy_event(&mut model, &XyEvent::AgentEnd { messages: vec![] });

    let lines = model.scrollback_lines();
    assert!(lines.iter().any(|l| l.contains("user: prompt")));
    assert!(lines.iter().any(|l| l.contains("assistant: A")));
    assert_eq!(model.phase, UiPhase::Idle);
    assert!(model.status.is_none());
}

#[test]
fn idle_scrollback_is_silent_no_placeholder_wall() {
    use super::ui_root::UiRoot;

    let mut root = UiRoot::new();
    root.set_chrome_meta("~/x", "m");
    root.apply_ui_model(&UiModel::new());
    let joined = root.render(80).join("\n");
    assert!(
        !joined.contains("empty — submit"),
        "idle must not shout placeholder: {joined}"
    );
    assert!(joined.contains('─'), "editor operation-zone border missing");
    assert!(joined.contains("~/x"), "footer missing");
}

#[test]
fn chrome_idle_status_occupies_zero_rows() {
    use super::ui_root::UiRoot;

    let mut root = UiRoot::new();
    root.set_chrome_meta("~/xylitol", "ornith");
    root.apply_ui_model(&UiModel::new());
    let lines = root.render(80);
    assert!(
        !lines.iter().any(|l| l.contains("Working")),
        "idle must not show busy status: {lines:?}"
    );
    let footer = lines.last().expect("footer");
    assert!(footer.contains("~/xylitol"), "{footer}");
    assert!(footer.contains("ornith"), "{footer}");
    assert!(
        !footer.contains("enter submit"),
        "footer must not be a key-chord wall: {footer}"
    );
}

#[test]
fn chrome_busy_status_is_separate_from_footer() {
    use super::ui_root::UiRoot;

    let mut root = UiRoot::new();
    root.set_chrome_meta("~/xylitol", "ornith");
    let mut model = UiModel::new();
    model.begin_run("hello");
    root.apply_ui_model(&model);
    let lines = root.render(80);
    assert!(
        lines.iter().any(|l| l.contains("Working")),
        "busy status row missing: {lines:?}"
    );
    let footer = lines.last().expect("footer");
    assert!(
        !footer.contains("Working"),
        "Working must not live in footer: {footer}"
    );
    assert!(
        footer.contains("~/xylitol") && footer.contains("ornith"),
        "{footer}"
    );
}

#[test]
fn chrome_ascii_user_glyph() {
    use super::glyphs::GlyphSet;
    use super::ui_root::UiRoot;

    let mut root = UiRoot::new();
    root.set_glyphs(GlyphSet::Ascii);
    root.set_chrome_meta(".", "m");
    let mut model = UiModel::new();
    model.begin_run("hi");
    root.apply_ui_model(&model);
    let lines = root.render(80);
    let joined = lines.join("\n");
    assert!(
        joined.contains("> hi") || joined.contains(">\x1b") || joined.contains("> "),
        "ascii user glyph missing: {joined}"
    );
    // Unicode ❯ must not appear when ascii is forced.
    assert!(!joined.contains('❯'), "{joined}");
}

#[test]
fn scrollback_assistant_uses_markdown_not_plain_prefix() {
    use super::ui_root::UiRoot;

    let mut root = UiRoot::new();
    root.set_chrome_meta(".", "m");
    let mut model = UiModel::new();
    model.entries.push(super::bridge::UiEntry::Assistant {
        text: "**bold** hi".into(),
    });
    root.apply_ui_model(&model);
    let joined = root.render(80).join("\n");
    assert!(
        !joined.contains("assistant:"),
        "must not use plain assistant: prefix: {joined}"
    );
    assert!(joined.contains("hi"), "{joined}");
}

#[test]
fn scrollback_thinking_fold_shows_ctrl_t_hint() {
    use super::ui_root::UiRoot;
    use xylitol_tui::InputEvent;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.entries.push(super::bridge::UiEntry::Thinking {
        text: "secret plan".into(),
    });
    root.apply_ui_model(&model);
    let idle = root.render(80).join("\n");
    assert!(idle.contains("(Ctrl+T)"), "{idle}");
    assert!(
        !idle.contains("secret plan"),
        "collapsed must hide body: {idle}"
    );

    // Toggle expand via Ctrl+T (synthetic key through handle_input).
    // Use HostSession path would need key event; call fold via listener keys.
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Char('t'),
        KeyModifiers::CONTROL,
    )));
    let open = root.render(80).join("\n");
    assert!(open.contains("secret plan"), "{open}");
    assert!(root.fold().thinking_expanded);
}

#[test]
fn scrollback_diff_header_tint_and_body() {
    use super::ui_root::UiRoot;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.entries.push(super::bridge::UiEntry::Diff {
        summary: "edit foo.rs".into(),
        display_diff: "+ hello\n- world\n".into(),
    });
    root.apply_ui_model(&model);
    let collapsed = root.render(80).join("\n");
    assert!(collapsed.contains("(Alt+E)"), "{collapsed}");
    assert!(collapsed.contains("edit foo.rs"), "{collapsed}");

    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Char('e'),
        KeyModifiers::ALT,
    )));
    let open = root.render(80).join("\n");
    assert!(
        open.contains("hello") || open.contains('+'),
        "expanded diff body missing: {open}"
    );
}

#[test]
fn preflight_error_messages_are_cli_friendly() {
    let msg = super::TuiPreflightError::NoModelSelected.to_string();
    assert!(msg.contains("selected model"), "{msg}");
    assert!(msg.contains("--model") || msg.contains("config"), "{msg}");

    let msg = super::TuiPreflightError::StdinNotTty.to_string();
    assert!(msg.contains("TTY"), "{msg}");
}
