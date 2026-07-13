//! Host harness tests — no real TTY (ath5).

use super::bridge::{UiModel, UiPhase, apply_xy_event};
use super::host::{HostEvent, HostSession, LayoutMode, TOO_SMALL_HINT, is_too_small};
use super::layout::build_root;
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
        "expected editor border or footer layout, got: {joined:?}"
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
        ("effects.rs", include_str!("effects.rs")),
        ("commands.rs", include_str!("commands.rs")),
        ("layout/root.rs", include_str!("layout/root.rs")),
        ("layout/slots.rs", include_str!("layout/slots.rs")),
        ("terminal_guard.rs", include_str!("terminal_guard.rs")),
        ("bridge/mod.rs", include_str!("bridge/mod.rs")),
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
    let root = include_str!("layout/root.rs");
    assert!(
        !root.contains("XyEvent"),
        "layout/root must stay XyEvent-free"
    );
    let slots = include_str!("layout/slots.rs");
    assert!(
        !slots.contains("XyEvent"),
        "layout/slots must stay XyEvent-free"
    );
    let bridge = include_str!("bridge/mod.rs");
    assert!(
        bridge.contains("apply_xy_event"),
        "bridge must own apply_xy_event"
    );
}

#[test]
fn shared_effect_pump_is_single_entry() {
    // ath6: production + harness share drain_pending; no duplicate PendingSlash match.
    let effects = include_str!("effects.rs");
    assert!(
        effects.contains("pub async fn drain_pending"),
        "effects must export drain_pending"
    );
    let mod_src = include_str!("mod.rs");
    assert!(
        mod_src.contains("drain_pending"),
        "production host loop must call drain_pending"
    );
    assert!(
        !mod_src.contains("PendingSlash::Exit"),
        "mod.rs must not duplicate PendingSlash match"
    );
    let harness = include_str!("harness.rs");
    assert!(
        harness.contains("drain_pending"),
        "harness pump must call drain_pending"
    );
    assert!(
        !harness.contains("PendingSlash::Exit"),
        "harness must not duplicate PendingSlash match"
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

fn alt_enter_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::ALT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[test]
fn harness_busy_enter_queues_steer() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    root.borrow_mut().set_editor_text("nudge");
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert_eq!(session.take_steer().as_deref(), Some("nudge"));
    assert!(root.borrow().editor_text().is_empty());
    assert_eq!(session.ui_model().pending_steer, vec!["nudge".to_string()]);
    assert!(
        !session.ui_model().entries.iter().any(
            |e| matches!(e, super::bridge::UiEntry::System { text } if text.contains("[steer]"))
        ),
        "steer must not be a scrollback system wall: {:?}",
        session.ui_model().entries
    );
    let frame = root.borrow_mut().render(80);
    assert!(
        frame.iter().any(|l| l.contains("Steering: nudge")),
        "missing Steering strip: {frame:?}"
    );
    assert!(
        frame
            .iter()
            .any(|l| l.contains("Alt+Up to edit all queued messages")),
        "missing dequeue hint: {frame:?}"
    );
    assert!(!root.borrow().tree_open());
}

#[test]
fn harness_busy_alt_enter_queues_follow_up() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    root.borrow_mut().set_editor_text("later");
    session.step(HostEvent::Input(alt_enter_event())).unwrap();
    assert_eq!(session.take_follow_up().as_deref(), Some("later"));
    assert!(root.borrow().editor_text().is_empty());
    assert_eq!(
        session.ui_model().pending_follow_up,
        vec!["later".to_string()]
    );
    let frame = root.borrow_mut().render(80);
    assert!(
        frame.iter().any(|l| l.contains("Follow-up: later")),
        "missing Follow-up strip: {frame:?}"
    );
}

fn alt_up_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Up,
        modifiers: KeyModifiers::ALT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[test]
fn harness_busy_alt_up_restores_queued_to_editor() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    root.borrow_mut().set_editor_text("nudge");
    session.step(HostEvent::Input(enter_event())).unwrap();
    let _ = session.take_steer();
    root.borrow_mut().set_editor_text("draft");
    session.step(HostEvent::Input(alt_up_event())).unwrap();
    assert!(session.take_dequeue());
    assert!(session.ui_model().pending_steer.is_empty());
    assert_eq!(root.borrow().editor_text(), "nudge\n\ndraft");
}

#[test]
fn harness_busy_esc_requests_abort_not_tree() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert!(session.take_abort());
    assert!(
        !root.borrow().tree_open(),
        "busy Esc must not open stub tree"
    );
}

#[test]
fn harness_idle_slash_exit_quits() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("/exit");
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert!(session.should_quit());
    assert!(session.take_submit().is_none());
}

#[test]
fn harness_idle_unknown_slash_stays_alive() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("/nope");
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert!(!session.should_quit());
    assert!(session.take_submit().is_none());
    assert!(
        session.ui_model().entries.iter().any(
            |e| matches!(e, super::bridge::UiEntry::System { text } if text.contains("unknown"))
        ),
        "unknown slash note missing: {:?}",
        session.ui_model().entries
    );
}

#[test]
fn harness_idle_slash_model_pending() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("/model");
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert_eq!(
        session.take_slash(),
        Some(super::commands::PendingSlash::CycleModel)
    );
}

#[tokio::test]
async fn harness_double_esc_opens_session_tree() {
    use super::harness::{ScriptedDriver, harness_sample_message_history_tree, pump_host_driver};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    driver.set_message_history_tree(harness_sample_message_history_tree());
    let mut stream = None;
    root.borrow_mut().set_editor_text("");
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert!(!root.borrow().tree_open(), "single Esc must not open tree");
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert!(
        !root.borrow().tree_open(),
        "tree opens after async Driver fetch"
    );
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();
    assert_eq!(driver.session_tree_calls(), 1);
    assert!(
        root.borrow().tree_open(),
        "double Esc on empty editor should open session tree"
    );
    session.render_now().unwrap();
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains("Session tree"),
        "expected tree layout; got: {joined}"
    );
    assert!(
        joined.contains("user:") && joined.contains("hello"),
        "live tree must render themed kind prefix + plain label; got: {joined}"
    );
}

#[test]
fn harness_esc_closes_session_tree() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut()
        .open_session_tree_for_test(super::layout::sample_tree_nodes_for_test(), Some("u2"));
    assert!(root.borrow().tree_open());
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert!(!root.borrow().tree_open());
}

#[tokio::test]
async fn harness_enter_travel_closes_tree() {
    use super::harness::{ScriptedDriver, harness_sample_session_messages, pump_host_driver};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    driver.set_session_messages(harness_sample_session_messages());
    let mut stream = None;
    root.borrow_mut()
        .open_session_tree_at_for_test(super::layout::sample_tree_nodes_for_test(), "u2");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();
    assert!(!root.borrow().tree_open());
    assert_eq!(driver.travel_calls(), vec!["u2".to_string()]);
    session.render_now().unwrap();
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains("history @ u2"),
        "expected travel transcript banner; got: {joined}"
    );
}

#[tokio::test]
async fn harness_enter_user_prefills_editor() {
    use super::harness::{ScriptedDriver, harness_sample_session_messages, pump_host_driver};
    use crate::domain::session_types::{SessionTreeKind, SessionTreeTravel};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    driver.set_session_messages(harness_sample_session_messages());
    driver.set_travel_override(
        "u1",
        SessionTreeTravel {
            kind: SessionTreeKind::MessageHistory,
            selected_id: "u1".into(),
            leaf_id: None,
            editor_text: Some("hello".into()),
        },
    );
    let mut stream = None;
    root.borrow_mut()
        .open_session_tree_at_for_test(super::layout::sample_tree_nodes_for_test(), "u1");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();
    assert!(!root.borrow().tree_open());
    assert_eq!(
        root.borrow().editor_text(),
        "hello",
        "user travel must prefill editor_text from Driver travel"
    );
    assert_eq!(driver.travel_calls(), vec!["u1".to_string()]);
}

#[tokio::test]
async fn harness_enter_assistant_does_not_prefill() {
    use super::harness::{ScriptedDriver, harness_sample_session_messages, pump_host_driver};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    driver.set_session_messages(harness_sample_session_messages());
    let mut stream = None;
    root.borrow_mut()
        .open_session_tree_at_for_test(super::layout::sample_tree_nodes_for_test(), "a1");
    assert!(
        root.borrow().editor_text().is_empty(),
        "tree open clears editor before travel"
    );
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();
    assert!(!root.borrow().tree_open());
    assert!(
        root.borrow().editor_text().is_empty(),
        "non-user travel must not prefill; got {:?}",
        root.borrow().editor_text()
    );
    assert_eq!(driver.travel_calls(), vec!["a1".to_string()]);
}

#[test]
fn harness_editor_slot_mutex_and_esc_closes() {
    use super::layout::EditorSlot;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();

    root.borrow_mut().open_slot_for_test(EditorSlot::Plate);
    assert_eq!(root.borrow().slot(), EditorSlot::Plate);
    let frame = root.borrow_mut().render(80);
    assert!(
        frame.iter().any(|l| l.contains("Command Plate")),
        "Plate must replace editor slot: {frame:?}"
    );
    assert!(
        !frame.iter().any(|l| l.contains("Session tree")),
        "slots are mutually exclusive"
    );

    // Opening Tree replaces Plate (mount directly in harness — open_slot queues Driver fetch).
    root.borrow_mut()
        .open_session_tree_for_test(super::layout::sample_tree_nodes_for_test(), Some("u2"));
    assert_eq!(root.borrow().slot(), EditorSlot::Tree);
    assert!(root.borrow().tree_open());

    session.step(HostEvent::Input(esc_event())).unwrap();
    assert_eq!(root.borrow().slot(), EditorSlot::Editor);

    root.borrow_mut().open_slot_for_test(EditorSlot::Settings);
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert_eq!(root.borrow().slot(), EditorSlot::Editor);

    root.borrow_mut().open_slot_for_test(EditorSlot::Choice);
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert_eq!(root.borrow().slot(), EditorSlot::Editor);
}

#[test]
fn harness_busy_esc_aborts_not_tree_slot() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    assert_eq!(root.borrow().slot(), super::layout::EditorSlot::Editor);
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert!(session.take_abort());
    assert_eq!(
        root.borrow().slot(),
        super::layout::EditorSlot::Editor,
        "busy Esc must abort and MUST NOT open Tree"
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

fn arrow_up_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Up,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[test]
fn harness_idle_up_recalls_submit_history() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("run me");
    session.step(HostEvent::Input(enter_event())).unwrap();
    let _ = session.take_submit();
    assert!(root.borrow().editor_text().is_empty());
    // Idle: dispatch Up into focused editor via host step → component tree.
    session.step(HostEvent::Input(arrow_up_event())).unwrap();
    assert_eq!(root.borrow().editor_text(), "run me");
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
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta("~/x", "m");
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
fn idle_editor_operation_zone_is_compact() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta("~/x", "m");
    root.apply_ui_model(&UiModel::new());
    let lines = root.render(80);
    // scrollback 0 + status 0 + editor (3) + footer 1
    assert!(
        lines.len() <= 5,
        "idle layout must stay compact (got {} lines): {lines:?}",
        lines.len()
    );
    let border_rows = lines.iter().filter(|l| l.contains('─')).count();
    assert!(
        border_rows >= 2,
        "operation-zone borders missing: {lines:?}"
    );

    root.set_editor_text("line1\nline2\nline3\nline4\nline5\nline6");
    let grown = root.render(80);
    assert!(
        grown.len() > lines.len(),
        "multi-line draft must grow editor slot: idle={} grown={}",
        lines.len(),
        grown.len()
    );
}

#[test]
fn layout_idle_status_occupies_zero_rows() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta("~/xylitol", "ornith");
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
fn layout_compacting_and_retry_stay_single_status_row() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta("~/xylitol", "ornith");

    let mut compacting = UiModel::new();
    compacting.begin_run("hi");
    compacting.status = Some("Compacting".into());
    root.apply_ui_model(&compacting);
    let compact_lines = root.render(80);
    let compact_status: Vec<_> = compact_lines
        .iter()
        .filter(|l| l.contains("Compacting"))
        .collect();
    assert_eq!(
        compact_status.len(),
        1,
        "Compacting must be a single status row: {compact_lines:?}"
    );
    let footer = compact_lines.last().expect("footer");
    assert!(
        !footer.contains("Compacting"),
        "Compacting must not live in footer: {footer}"
    );

    let mut retrying = UiModel::new();
    retrying.begin_run("hi");
    retrying.status = Some("Retry 1/3".into());
    root.apply_ui_model(&retrying);
    let retry_lines = root.render(80);
    let retry_status: Vec<_> = retry_lines.iter().filter(|l| l.contains("Retry")).collect();
    assert_eq!(
        retry_status.len(),
        1,
        "Retry must be a single status row: {retry_lines:?}"
    );
    let footer = retry_lines.last().expect("footer");
    assert!(
        !footer.contains("Retry"),
        "Retry must not live in footer: {footer}"
    );
}

#[test]
fn layout_busy_status_is_separate_from_footer() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta("~/xylitol", "ornith");
    let mut model = UiModel::new();
    model.begin_run("hello");
    root.apply_ui_model(&model);
    let lines = root.render(80);
    assert!(
        lines.iter().any(|l| l.contains("Working")),
        "busy status row missing: {lines:?}"
    );
    // Default Loader frames are braille spinners (⠋…).
    assert!(
        lines.iter().any(|l| {
            l.contains('⠋')
                || l.contains('⠙')
                || l.contains('⠹')
                || l.contains('⠸')
                || l.contains('⠼')
                || l.contains('⠴')
                || l.contains('⠦')
                || l.contains('⠧')
                || l.contains('⠇')
                || l.contains('⠏')
        }),
        "busy status must include spinner frame: {lines:?}"
    );
    let footer = lines.last().expect("footer");
    assert!(
        !footer.contains("Working"),
        "Working must not live in footer: {footer}"
    );
    assert!(
        !footer.contains('⠋') && !footer.contains('⠙'),
        "spinner must not live in footer: {footer}"
    );
    assert!(
        footer.contains("~/xylitol") && footer.contains("ornith"),
        "{footer}"
    );

    // Tick advances spinner without growing status into footer.
    let _ = root.tick();
    let after = root.render(80);
    assert!(
        after.iter().any(|l| l.contains("Working")),
        "tick must keep busy status: {after:?}"
    );
}

#[test]
fn scrollback_user_message_applies_background() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta(".", "m");
    let mut model = UiModel::new();
    model.entries.push(super::bridge::UiEntry::User {
        text: "hello bg".into(),
    });
    root.apply_ui_model(&model);
    let joined = root.render(80).join("\n");
    assert!(joined.contains("hello bg"), "user text missing: {joined}");
    // bg_rgb emits CSI 48;2;r;g;b
    assert!(
        joined.contains("\x1b[48;2;"),
        "user-message-bg ANSI missing: {joined:?}"
    );
}

#[test]
fn layout_ascii_user_glyph() {
    use super::layout::UiRoot;
    use super::widgets::GlyphSet;

    let mut root = UiRoot::new();
    root.set_glyphs(GlyphSet::Ascii);
    root.set_layout_meta(".", "m");
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
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta(".", "m");
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
    use super::layout::UiRoot;
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
    use super::layout::UiRoot;
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
