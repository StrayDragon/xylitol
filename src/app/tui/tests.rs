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
    mouse_capture_active: bool,
    alternate_screen_active: bool,
}

impl TestTerminal {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            frames: Vec::new(),
            started: false,
            stopped: false,
            mouse_capture_active: false,
            alternate_screen_active: false,
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
        self.mouse_capture_active = false;
        self.alternate_screen_active = false;
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
    fn enter_alternate_screen(&mut self) {
        self.alternate_screen_active = true;
    }
    fn leave_alternate_screen(&mut self) {
        self.alternate_screen_active = false;
    }
    fn alternate_screen_active(&self) -> bool {
        self.alternate_screen_active
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
        joined.contains('─') || joined.contains("model-name") || joined.contains("~/"),
        "expected editor border or footer layout, got: {joined:?}"
    );
}

#[test]
fn harness_too_small_hint_safe_at_cjk_underfull_width() {
    // "请放大终端" is 10 cols; chars().count()==5 used to overflow and exit.
    for cols in [1u16, 5, 9, 10, 12] {
        let mut session = HostSession::new(TestTerminal::new(cols, 3), build_root);
        assert_eq!(session.mode(), LayoutMode::TooSmall);
        session
            .render_now()
            .unwrap_or_else(|e| panic!("TooSmallHint must render at width {cols}: {e}"));
    }
}

#[test]
fn harness_extreme_shrink_then_restore_recovers_ready() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.render_now().unwrap();
    assert_eq!(session.mode(), LayoutMode::Ready);

    session
        .step(HostEvent::Resize { cols: 8, rows: 3 })
        .expect("tiny resize must not exit");
    assert_eq!(session.mode(), LayoutMode::TooSmall);
    session.render_now().expect("hint at tiny size");

    session
        .step(HostEvent::Resize { cols: 80, rows: 24 })
        .expect("restore must not exit");
    session.render_now().unwrap();
    assert_eq!(session.mode(), LayoutMode::Ready);
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
fn mouse_input_does_not_request_render_by_default() {
    use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.render_now().unwrap();
    assert!(!session.tui.is_render_requested());

    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        !session.tui.is_render_requested(),
        "Moved must not schedule a frame"
    );

    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 2,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        !session.tui.is_render_requested(),
        "unhandled mouse Down must not schedule a frame"
    );

    session
        .step(HostEvent::Input(InputEvent::Key(
            crossterm::event::KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
        )))
        .unwrap();
    assert!(
        session.tui.is_render_requested(),
        "Key path must still request render"
    );
}

#[test]
fn product_tui_source_has_no_tui_start_call() {
    let sources = [
        ("mod.rs", include_str!("mod.rs")),
        ("host/mod.rs", include_str!("host/mod.rs")),
        ("effects/mod.rs", include_str!("effects/mod.rs")),
        ("effects/slash.rs", include_str!("effects/slash.rs")),
        (
            "effects/pending_ui.rs",
            include_str!("effects/pending_ui.rs"),
        ),
        ("effects/bang.rs", include_str!("effects/bang.rs")),
        ("commands.rs", include_str!("commands.rs")),
        ("layout/root/mod.rs", include_str!("layout/root/mod.rs")),
        (
            "layout/root/slot_input.rs",
            include_str!("layout/root/slot_input.rs"),
        ),
        ("layout/slots.rs", include_str!("layout/slots.rs")),
        ("terminal_guard.rs", include_str!("terminal_guard.rs")),
        ("bridge/mod.rs", include_str!("bridge/mod.rs")),
        ("bridge/model.rs", include_str!("bridge/model.rs")),
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
    let root = include_str!("layout/root/mod.rs");
    assert!(
        !root.contains("XyEvent"),
        "layout/root must stay XyEvent-free"
    );
    let slot_input = include_str!("layout/root/slot_input.rs");
    assert!(
        !slot_input.contains("XyEvent"),
        "layout/root/slot_input must stay XyEvent-free"
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
    let effects = include_str!("effects/mod.rs");
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
fn ath12_entry_files_under_hard_smell_loc() {
    // c1840 / ath12: physical LOC is SHOULD ~800; approaching ~1200 is hard smell.
    // Function complexity HARD gate is scripts/check_complexity.py (just qa).
    const HARD_SMELL: usize = 1200;
    let files = [
        ("host/mod.rs", include_str!("host/mod.rs")),
        ("layout/root/mod.rs", include_str!("layout/root/mod.rs")),
        ("effects/mod.rs", include_str!("effects/mod.rs")),
        ("bridge/mod.rs", include_str!("bridge/mod.rs")),
    ];
    for (name, src) in files {
        let lines = src.lines().count();
        assert!(
            lines < HARD_SMELL,
            "{name} has {lines} lines (hard smell {HARD_SMELL}); split further per ath12"
        );
    }
}

#[test]
fn product_slash_catalog_matches_agent_ssot() {
    // c1175 / sc3 / atm7: TUI catalog names == product SSOT (same crate build).
    use crate::app::product_commands::{LEGACY_SHORT_NAMES, product_slash_commands};
    use crate::app::tui::layout::product_slash_commands_for_editor;

    let ssot: Vec<&str> = product_slash_commands().iter().map(|c| c.name).collect();
    let catalog: Vec<String> = product_slash_commands_for_editor()
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(
        catalog,
        ssot.iter().map(|s| (*s).to_string()).collect::<Vec<_>>()
    );
    for legacy in LEGACY_SHORT_NAMES {
        assert!(!catalog.iter().any(|n| n == legacy));
    }
}

#[test]
fn legacy_short_slash_names_are_not_parsed() {
    // A03 / c1175: /tree etc. stay unknown at the parse layer.
    use super::commands::parse_slash_command;
    for legacy in [
        "/tree", "/fork", "/export", "/import", "/compact", "/resume", "/new", "/clone", "/name",
    ] {
        assert!(
            parse_slash_command(legacy).is_none(),
            "{legacy} must not parse as PendingSlash"
        );
    }
    assert!(parse_slash_command("/session-tree").is_some());
    assert!(parse_slash_command("/exit").is_some());
    assert!(parse_slash_command("/quit").is_some());
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
            |e| matches!(e, super::bridge::UiEntry::ScrollNotice { text } if text.contains("[steer]"))
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
fn harness_busy_steer_expands_paste_marker() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    let pasted = (0..12)
        .map(|i| format!("steer{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    session
        .step(HostEvent::Input(InputEvent::Paste(pasted.clone())))
        .unwrap();
    assert!(
        root.borrow()
            .editor_display_text()
            .contains("[paste #1 +12 lines]")
    );
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert_eq!(session.take_steer().as_deref(), Some(pasted.as_str()));
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
fn harness_busy_ctrl_c_aborts_not_quit() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    // Empty editor: pre-c1570 Ctrl+C would quit; must abort instead.
    root.borrow_mut().set_editor_text(String::new());
    session.on_run_started("hello");
    session.step(HostEvent::Input(ctrl_c_event())).unwrap();
    assert!(session.take_abort(), "busy Ctrl+C must latch abort");
    assert!(
        !session.should_quit(),
        "busy Ctrl+C must not quit even with empty editor"
    );
    assert!(!root.borrow().tree_open());
}

#[test]
fn harness_busy_ctrl_c_with_draft_still_aborts() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("steer draft");
    session.on_run_started("hello");
    session.step(HostEvent::Input(ctrl_c_event())).unwrap();
    assert!(session.take_abort());
    assert!(!session.should_quit());
    // Same as Esc: do not clear draft as a side effect of abort latch.
    assert_eq!(root.borrow().editor_text(), "steer draft");
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
            |e| matches!(e, super::bridge::UiEntry::ScrollNotice { text } if text.contains("unknown"))
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
        Some(super::commands::PendingSlash::OpenModels)
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
        "tree opens after async XyDriver fetch"
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
    let entries = &session.ui_model().entries;
    assert!(
        !matches!(
            entries.first(),
            Some(super::bridge::UiEntry::ScrollNotice { text }) if text.contains("history @")
        ),
        "history @ MUST NOT be prepended as entries[0]; got: {entries:?}"
    );
    assert!(
        matches!(
            entries.last(),
            Some(super::bridge::UiEntry::ScrollNotice { text }) if text.contains("history @ u2")
        ),
        "history @ MUST trail (above input); got: {entries:?}"
    );
}

#[tokio::test]
async fn harness_enter_user_prefills_editor() {
    use super::harness::{ScriptedDriver, harness_sample_session_messages, pump_host_driver};
    use crate::protocol::session::{SessionTreeKind, SessionTreeTravel};

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
        "user travel must prefill editor_text from XyDriver travel"
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

    // Opening Tree replaces Plate (mount directly in harness — open_slot queues XyDriver fetch).
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

#[test]
fn harness_app_thinking_toggle_via_binding_id() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    assert!(!root.borrow().fold().thinking_expanded);
    session
        .step(HostEvent::Input(InputEvent::Key(KeyEvent {
            code: KeyCode::Char('t'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })))
        .unwrap();
    assert!(root.borrow().fold().thinking_expanded);
}

#[tokio::test]
async fn harness_idle_slash_reload_keeps_history_and_calls_runtime() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.push_scroll_notice("seed history");
    let before = session.ui_model().entries.len();

    let mut driver = ScriptedDriver::new();
    driver.set_dollar_skill_catalog_for_driver(vec![("demo".into(), "demo skill".into())]);
    let mut stream = None;

    root.borrow_mut().set_editor_text("/reload");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert_eq!(driver.reload_runtime_calls(), 1);
    assert_eq!(
        session.ui_model().entries.len(),
        before + 1,
        "reload must append one scroll notice, not clear history"
    );
    assert!(
        session.ui_model().entries.iter().any(
            |e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("Reload:") && text.contains("skills:"))
        ),
        "expected reload system report; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_history_copy_last_copies_assistant() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("prompt");
    session
        .step(HostEvent::Xy(Box::new(XyEvent::TextDelta(
            "copy-me-please".into(),
        ))))
        .unwrap();
    session
        .step(HostEvent::Xy(Box::new(XyEvent::AgentEnd {
            messages: Vec::new(),
        })))
        .unwrap();
    session.push_scroll_notice("trailing scroll notice");

    let mut driver = ScriptedDriver::new();
    let mut stream = None;
    root.borrow_mut().set_editor_text("/history-copy-last");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert_eq!(driver.copy_text_calls(), vec!["copy-me-please".to_string()]);
    assert!(
        session.ui_model().entries.iter().any(
            |e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("Copied") && text.contains("chars"))
        ),
        "expected copy ok note; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_history_copy_last_empty_prompts() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    let mut stream = None;
    root.borrow_mut().set_editor_text("/history-copy-last");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert!(driver.copy_text_calls().is_empty());
    assert!(
        session.ui_model().entries.iter().any(
            |e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("no assistant message"))
        ),
        "expected empty note; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_busy_history_copy_last_still_copies() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("first");
    session
        .step(HostEvent::Xy(Box::new(XyEvent::TextDelta("A".into()))))
        .unwrap();
    session
        .step(HostEvent::Xy(Box::new(XyEvent::AgentEnd {
            messages: Vec::new(),
        })))
        .unwrap();
    session.on_run_started("busy again");
    assert!(session.is_busy());

    let mut driver = ScriptedDriver::new();
    let mut stream = None;
    root.borrow_mut().set_editor_text("/history-copy-last");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert_eq!(driver.copy_text_calls(), vec!["A".to_string()]);
    assert!(
        session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("Copied"))),
        "busy must still copy; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_history_copy_emits_pending_osc52_on_terminal() {
    use super::harness::{ScriptedDriver, pump_host_driver};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("prompt");
    session
        .step(HostEvent::Xy(Box::new(XyEvent::TextDelta(
            "osc52-payload".into(),
        ))))
        .unwrap();
    session
        .step(HostEvent::Xy(Box::new(XyEvent::AgentEnd {
            messages: Vec::new(),
        })))
        .unwrap();

    let mut driver = ScriptedDriver::new();
    let osc = "\x1b]52;c;dGVzdA==\x07";
    driver.set_next_copy_pending_osc52(Some(osc.into()));
    let mut stream = None;
    root.borrow_mut().set_editor_text("/history-copy-last");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert!(
        session.tui.terminal.frames.iter().any(|f| f.contains(osc)),
        "host must write deferred OSC 52 via Terminal before render; frames={:?}",
        session.tui.terminal.frames
    );
}

#[tokio::test]
async fn harness_idle_slash_trust_persists_without_reload() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::core::driver::ProjectTrustMode;
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    let mut stream = None;

    root.borrow_mut().set_editor_text("/trust");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert_eq!(
        driver.persist_project_trust_calls(),
        vec![ProjectTrustMode::TrustCwd]
    );
    assert_eq!(driver.reload_runtime_calls(), 0);
    assert!(
        session.ui_model().entries.iter().any(|e| matches!(
            e,
            UiEntry::ScrollNotice { text }
                if text.contains("trusted")
                    && (text.contains("/reload") || text.contains("restart"))
        )),
        "expected trust+reload hint; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_busy_slash_trust_refused() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    assert!(session.is_busy());

    let mut driver = ScriptedDriver::new();
    let mut stream = None;
    root.borrow_mut().set_editor_text("/trust");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert!(driver.persist_project_trust_calls().is_empty());
    assert_eq!(driver.reload_runtime_calls(), 0);
    assert!(
        session.ui_model().entries.iter().any(
            |e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("agent busy") && text.contains("/trust"))
        ),
        "expected busy refuse; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_busy_slash_reload_refused() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    assert!(session.is_busy());

    let mut driver = ScriptedDriver::new();
    let mut stream = None;
    root.borrow_mut().set_editor_text("/reload");
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert_eq!(driver.reload_runtime_calls(), 0);
    assert!(
        session.ui_model().entries.iter().any(
            |e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("agent busy") && text.contains("/reload"))
        ),
        "expected busy refuse note; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_busy_slash_session_name_allows_not_steer() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    assert!(session.is_busy());

    let mut driver = ScriptedDriver::new();
    let mut stream = None;
    root.borrow_mut().set_editor_text("/session-name my-run");
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert!(
        session.take_steer().is_none(),
        "Allow slash must not enqueue steer"
    );
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert_eq!(driver.set_session_name_calls(), vec!["my-run".to_string()]);
    assert!(
        session.ui_model().entries.iter().any(|e| matches!(
            e,
            UiEntry::ScrollNotice { text } if text.contains("Session name set")
        )),
        "expected name set note; got {:?}",
        session.ui_model().entries
    );
}

#[test]
fn harness_busy_unknown_slash_not_steered() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    root.borrow_mut().set_editor_text("/totally-unknown");
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert!(session.take_steer().is_none());
    assert!(
        session.ui_model().entries.iter().any(|e| matches!(
            e,
            super::bridge::UiEntry::ScrollNotice { text }
                if text.contains("unknown command not steered")
        )),
        "got {:?}",
        session.ui_model().entries
    );
}

#[test]
fn harness_keybindings_reload_keeps_old_on_bad_json() {
    use crate::app::tui::keybindings::{ReloadOutcome, matches_binding};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keybindings.json");
    std::fs::write(&path, r#"{ "app.interrupt": ["ctrl+x"] }"#).unwrap();
    assert!(matches!(
        session.reload_keybindings(dir.path()),
        ReloadOutcome::Applied { .. }
    ));
    assert!(matches_binding(
        &KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
        "app.interrupt"
    ));
    std::fs::write(&path, "{ broken").unwrap();
    assert!(matches!(
        session.reload_keybindings(dir.path()),
        ReloadOutcome::Failed { .. }
    ));
    assert!(matches_binding(
        &KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
        "app.interrupt"
    ));
}

#[test]
fn harness_long_paste_collapses_display_and_submit_expands() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let pasted = (0..15)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    session
        .step(HostEvent::Input(InputEvent::Paste(pasted.clone())))
        .unwrap();
    let display = root.borrow().editor_display_text();
    assert!(
        display.contains("[paste #1 +15 lines]"),
        "display should collapse: {display}"
    );
    assert_eq!(root.borrow().editor_text(), pasted);
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert_eq!(session.take_submit().as_deref(), Some(pasted.as_str()));
}

fn ctrl_v_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Char('v'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[tokio::test]
async fn harness_paste_image_inserts_abs_path() {
    use super::harness::{ScriptedDriver, pump_host_driver};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    // Minimal PNG header bytes (ScriptedDriver only writes; no decode on stage).
    driver.set_clipboard_image(b"\x89PNG\r\n\x1a\nfake".to_vec(), "image/png");
    let mut stream = None;

    session.step(HostEvent::Input(ctrl_v_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    let text = root.borrow().editor_text();
    assert!(
        text.contains("xylitol-paste-") && text.ends_with(".png"),
        "expected tempfile path in editor, got {text:?}"
    );
    assert!(
        !text.contains("base64") && text.len() < 500,
        "must not embed base64: {text:?}"
    );
    assert_eq!(driver.staged_paste_paths().len(), 1);
}

#[tokio::test]
async fn harness_paste_image_submit_stays_text() {
    use super::harness::{ScriptedDriver, pump_host_driver};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    driver.set_clipboard_image(b"\x89PNG\r\n\x1a\nfake".to_vec(), "image/png");
    let mut stream = None;

    session.step(HostEvent::Input(ctrl_v_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();
    let path_text = root.borrow().editor_text();
    session.step(HostEvent::Input(enter_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert_eq!(driver.runs.len(), 1);
    assert_eq!(driver.runs[0], path_text);
    assert!(
        driver.runs[0].contains("xylitol-paste-"),
        "run must be path text: {:?}",
        driver.runs[0]
    );
}

#[tokio::test]
async fn harness_paste_image_miss_error() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let mut driver = ScriptedDriver::new();
    // No clipboard image set → Ok(None).
    let mut stream = None;

    session.step(HostEvent::Input(ctrl_v_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert!(
        session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::Error { text } if text.contains("no image or text"))),
        "expected empty-clipboard error; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_paste_text_fallback_inserts() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    driver.set_clipboard_text("hello from clipboard");
    let mut stream = None;

    session.step(HostEvent::Input(ctrl_v_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert_eq!(root.borrow().editor_text(), "hello from clipboard");
    assert!(
        !session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::Error { .. })),
        "text fallback must not error; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_paste_image_driver_err_falls_back_then_errors() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let mut driver = ScriptedDriver::new();
    driver.set_clipboard_image_error("wl-paste exploded");
    let mut stream = None;

    session.step(HostEvent::Input(ctrl_v_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert!(
        session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::Error { text } if text.contains("no image or text"))),
        "image Err + no text → empty error; got {:?}",
        session.ui_model().entries
    );
}

#[tokio::test]
async fn harness_paste_image_err_with_text_fallback() {
    use super::harness::{ScriptedDriver, pump_host_driver};
    use crate::app::tui::bridge::UiEntry;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let mut driver = ScriptedDriver::new();
    driver.set_clipboard_image_error("wl-paste exploded");
    driver.set_clipboard_text("recovered text");
    let mut stream = None;

    session.step(HostEvent::Input(ctrl_v_event())).unwrap();
    pump_host_driver(&mut session, &mut driver, &mut stream)
        .await
        .unwrap();

    assert_eq!(root.borrow().editor_text(), "recovered text");
    assert!(
        !session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::Error { .. })),
        "text after image err must not error; got {:?}",
        session.ui_model().entries
    );
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

#[tokio::test]
async fn harness_new_session_seeds_prior_user_prompt() {
    use crate::app::core::driver::SessionListEntry;
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry, fixture_message_json};

    let cwd = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut driver = super::harness::ScriptedDriver::new();
    driver.set_session_list(vec![SessionListEntry {
        id: "prior".into(),
        name: None,
        first_message: Some("prior prompt".into()),
        message_count: 1,
        modified_unix: Some(100),
        parent_session_id: None,
        tree_prefix: String::new(),
        cwd: Some(cwd),
        path: None,
    }]);
    driver.set_session_messages(vec![SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "u1".into(),
            parent_id: None,
            timestamp: String::new(),
        },
        message: fixture_message_json("user", "prior prompt"),
    })]);

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.set_editor_history_seed_sessions(1);
    session.seed_editor_history_for_new_session(&driver).await;
    let root = session.ui_root().expect("product ui").clone();
    assert!(root.borrow().editor_text().is_empty());
    session.step(HostEvent::Input(arrow_up_event())).unwrap();
    assert_eq!(root.borrow().editor_text(), "prior prompt");
}

#[test]
fn harness_resume_seeds_only_entry_users() {
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry, fixture_message_json};

    let entries = vec![
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: String::new(),
            },
            message: fixture_message_json("user", "only me"),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a1".into(),
                parent_id: None,
                timestamp: String::new(),
            },
            message: fixture_message_json("assistant", "nope"),
        }),
    ];
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.seed_editor_history_from_entries(&entries);
    let root = session.ui_root().expect("product ui").clone();
    session.step(HostEvent::Input(arrow_up_event())).unwrap();
    assert_eq!(root.borrow().editor_text(), "only me");
}

#[test]
fn harness_cli_restored_session_rebuilds_transcript() {
    use super::bridge::UiEntry;
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry, fixture_message_json};

    let entries = vec![
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: String::new(),
            },
            message: fixture_message_json("user", "hi"),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a1".into(),
                parent_id: Some("u1".into()),
                timestamp: String::new(),
            },
            message: fixture_message_json("assistant", "hello there"),
        }),
    ];
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.apply_cli_restored_session("sid-restored", entries);
    let model = session.ui_model();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::User { text } if text.contains("hi")
        )),
        "CLI restore must show user turn: {:?}",
        model.entries
    );
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Assistant { text, .. } if text.contains("hello there")
        )),
        "CLI restore must show assistant turn: {:?}",
        model.entries
    );
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::ScrollNotice { text } if text.contains("restored → session sid-restored")
        )),
        "expected restored note: {:?}",
        model.entries
    );
    let root = session.ui_root().expect("product ui").clone();
    session.step(HostEvent::Input(arrow_up_event())).unwrap();
    assert_eq!(root.borrow().editor_text(), "hi");
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
    // c1135: loaded-resources card sits above scrollback; compact check is
    // relative growth of the editor slot, not absolute line count.
    assert!(
        lines.iter().any(|l| l.contains("xylitol")),
        "idle must show startup resources card: {lines:?}"
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
fn layout_idle_status_is_one_blank_above_editor() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta("~/xylitol", "model-name");
    root.apply_ui_model(&UiModel::new());
    let lines = root.render(80);
    assert!(
        !lines.iter().any(|l| l.contains("Working")),
        "idle must not show busy status: {lines:?}"
    );
    // Idle status slot is one blank row immediately above the editor zone
    // (after resources card + scrollback + queue).
    let editor_border = lines
        .iter()
        .position(|l| l.contains('─') && !l.contains('╭') && !l.contains('╰'))
        .expect("editor top border");
    assert!(
        editor_border > 0 && lines[editor_border - 1].is_empty(),
        "idle must keep one blank above editor: {lines:?}"
    );
    let footer = lines.last().expect("footer");
    assert!(footer.contains("~/xylitol"), "{footer}");
    assert!(footer.contains("model-name"), "{footer}");
    assert!(
        !footer.contains("enter submit"),
        "footer must not be a key-chord wall: {footer}"
    );
}

#[test]
fn layout_busy_status_keeps_leading_blank() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta("~/xylitol", "model-name");
    let mut model = UiModel::new();
    model.begin_run("hello");
    root.apply_ui_model(&model);
    let lines = root.render(80);
    let working_idx = lines
        .iter()
        .position(|l| l.contains("Working"))
        .expect("busy Working row");
    assert!(
        working_idx > 0 && lines[working_idx - 1].is_empty(),
        "busy spinner must keep leading blank: {lines:?}"
    );
}

#[test]
fn layout_compacting_and_retry_stay_single_status_row() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta("~/xylitol", "model-name");

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
    root.set_layout_meta("~/xylitol", "model-name");
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
        footer.contains("~/xylitol") && footer.contains("model-name"),
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
fn scrollback_user_message_no_wash_bg() {
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
    let user_line = joined
        .lines()
        .find(|l| strip_ansi(l).contains("hello bg"))
        .expect("user line");
    assert!(
        !user_line.contains("\x1b[48;2;"),
        "user row MUST NOT apply user-message-bg wash: {user_line:?}"
    );
}

#[test]
fn scrollback_bash_block_tint_and_gap() {
    use super::bridge::BashBlockStatus;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta(".", "m");
    let mut model = UiModel::new();
    model.entries.push(super::bridge::UiEntry::User {
        text: "before".into(),
    });
    model.entries.push(super::bridge::UiEntry::Bash {
        command: "echo hi".into(),
        status: BashBlockStatus::Success,
        output: "hi".into(),
        exclude_from_context: false,
    });
    root.apply_ui_model(&model);
    let lines = root.render(80);
    let is_inter_spacer = |l: &str| l.contains("\x1b[49m") && !l.contains("\x1b[48;2");
    let before = lines
        .iter()
        .position(|l| strip_ansi(l).contains("before"))
        .expect("user");
    let bash = lines
        .iter()
        .position(|l| strip_ansi(l).contains("$ echo hi"))
        .expect("bash");
    assert!(before < bash);
    let spacer_run = lines[before + 1..bash]
        .iter()
        .filter(|l| is_inter_spacer(l))
        .count();
    assert!(
        spacer_run >= 1,
        "att10 untinted spacer between blocks: {spacer_run}"
    );
    let joined = lines.join("\n");
    assert!(
        joined.contains("\x1b[48;2;"),
        "bash success status rail missing: {joined:?}"
    );
    let bash_line = lines
        .iter()
        .find(|l| strip_ansi(l).contains("$ echo hi"))
        .expect("$ echo");
    let wash = {
        let p = xylitol_tui::Palette::dark();
        format!(
            "\x1b[48;2;{};{};{}m",
            p.tool_success_bg.r, p.tool_success_bg.g, p.tool_success_bg.b
        )
    };
    assert!(
        !bash_line.contains(&wash),
        "bash MUST NOT use full tool-success-bg wash: {bash_line:?}"
    );
}

#[test]
fn scrollback_bash_ctrl_o_viewport_keeps_rail() {
    use super::bridge::BashBlockStatus;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta(".", "m");
    let mut model = UiModel::new();
    let long_out: String = (0..20).map(|i| format!("line-{i}\n")).collect();
    model.entries.push(super::bridge::UiEntry::Bash {
        command: "seq".into(),
        status: BashBlockStatus::Success,
        output: long_out,
        exclude_from_context: false,
    });
    root.apply_ui_model(&model);

    let collapsed = root.render(160);
    let joined = collapsed.join("\n");
    assert!(
        strip_ansi(&joined).contains("ctrl+o to expand"),
        "collapsed viewport hint missing"
    );
    let bash_line = collapsed
        .iter()
        .find(|l| strip_ansi(l).contains("$ seq"))
        .expect("$ seq");
    assert!(
        bash_line.contains("\x1b[48;2;"),
        "bash header must have status rail"
    );
    assert!(
        xylitol_tui::visible_width(bash_line) >= 160,
        "railed row must span terminal width, got {}",
        xylitol_tui::visible_width(bash_line)
    );

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use xylitol_tui::InputEvent;
    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Char('o'),
        KeyModifiers::CONTROL,
    )));
    assert!(root.fold().tools_output_expanded);
    let expanded = root.render(160).join("\n");
    assert!(
        !strip_ansi(&expanded).contains("ctrl+o to expand"),
        "expanded viewport should not show collapse hint"
    );
    assert!(strip_ansi(&expanded).contains("line-0"));
    assert!(strip_ansi(&expanded).contains("line-19"));
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for x in chars.by_ref() {
                    if x.is_ascii_alphabetic() {
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

#[test]
fn harness_ready_narrow_with_long_cwd_does_not_hang() {
    // Regression: startup-card wrap_plain spun forever on long path tokens at
    // Ready widths 40–44 (felt like "shrink then dead").
    // Use this checkout's path so each clone exercises its own cwd length.
    let long_cwd = super::host::display_path(env!("CARGO_MANIFEST_DIR"));
    let mut session = HostSession::new_product_ui_with_meta(
        TestTerminal::new(80, 24),
        long_cwd,
        "fake-model-with-a-very-long-name".into(),
    );
    session.render_now().expect("warm");
    for cols in [50u16, 45, 44, 42, 40] {
        session
            .step(HostEvent::Resize { cols, rows: 24 })
            .unwrap_or_else(|e| panic!("step {cols}: {e}"));
        session
            .render_now()
            .unwrap_or_else(|e| panic!("render_now {cols}: {e}"));
        assert_eq!(session.mode(), LayoutMode::Ready, "cols={cols}");
    }
    session
        .step(HostEvent::Resize { cols: 8, rows: 3 })
        .expect("too small");
    session.render_now().expect("hint");
    assert_eq!(session.mode(), LayoutMode::TooSmall);
    session
        .step(HostEvent::Resize { cols: 80, rows: 24 })
        .expect("restore");
    session.render_now().expect("ready again");
    assert_eq!(session.mode(), LayoutMode::Ready);
}

#[test]
fn idle_tick_skips_paint_when_clean() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.render_now().expect("warm");
    let frames = session.tui.frame_count();
    session.step(HostEvent::Tick).expect("tick");
    assert_eq!(
        session.tui.frame_count(),
        frames,
        "idle Tick must not paint when nothing is dirty"
    );
}

#[test]
fn bang_chunk_marks_dirty_and_paints_on_tick() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.begin_bash_exec("echo hi", false);
    session.render_now().expect("warm");
    let frames = session.tui.frame_count();
    session.append_bash_chunk(b"hi from bang\n");
    assert_eq!(
        session.tui.frame_count(),
        frames,
        "chunk must not force immediate paint"
    );
    // Honor TUI 16ms throttle after render_now.
    std::thread::sleep(std::time::Duration::from_millis(20));
    session.step(HostEvent::Tick).expect("tick");
    assert!(
        session.tui.frame_count() > frames,
        "Tick must paint after paint_dirty bang chunk"
    );
    let text = session.tui.terminal.frames.join("\n");
    // Frames hold raw ANSI writes; plain "hi from bang" may be split — also check model.
    assert!(
        session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, super::bridge::UiEntry::Bash { output, .. } if output.contains("hi from bang")))
            || text.contains("hi from bang"),
        "bash chunk must land in model/viewport"
    );
}

#[test]
fn upper_cache_reused_across_spinner_ticks() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    root.set_layout_meta("~/xylitol", "model-name");
    let mut model = UiModel::new();
    model.begin_run("hello");
    for i in 0..40 {
        model.entries.push(super::bridge::UiEntry::Assistant {
            text: format!("line-{i}"),
        });
    }
    root.apply_ui_model(&model);
    let _ = root.render(80);
    let rebuilt = root.upper_rebuild_count_for_test();
    assert!(rebuilt >= 1);
    // Status-only ticks should not rebuild upper.
    for _ in 0..5 {
        let _ = root.tick();
        let _ = root.render(80);
    }
    assert_eq!(
        root.upper_rebuild_count_for_test(),
        rebuilt,
        "spinner ticks must reuse upper cache"
    );
    // Model change must rebuild.
    model.entries.push(super::bridge::UiEntry::Assistant {
        text: "extra".into(),
    });
    root.apply_ui_model(&model);
    let _ = root.render(80);
    assert!(
        root.upper_rebuild_count_for_test() > rebuilt,
        "apply_ui_model must invalidate upper cache"
    );
}

#[test]
fn set_loaded_resources_skips_identical_snap_upper_bump() {
    use crate::app::core::driver::LoadedResourcesSnapshot;
    use xylitol_tui::Component;

    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let snap = LoadedResourcesSnapshot {
        mcp_configured: 2,
        mcp_connecting_label: Some("connecting 0/2".into()),
        ..LoadedResourcesSnapshot::default()
    };
    root.set_loaded_resources(snap.clone());
    let _ = root.render(80);
    let rebuilt = root.upper_rebuild_count_for_test();
    root.set_loaded_resources(snap);
    let _ = root.render(80);
    assert_eq!(
        root.upper_rebuild_count_for_test(),
        rebuilt,
        "identical loaded-resources snap MUST NOT invalidate upper"
    );
    root.set_loaded_resources(LoadedResourcesSnapshot {
        mcp_configured: 2,
        mcp_connecting_label: Some("connecting 1/2".into()),
        ..LoadedResourcesSnapshot::default()
    });
    let _ = root.render(80);
    assert!(
        root.upper_rebuild_count_for_test() > rebuilt,
        "label change MUST invalidate upper"
    );
}

#[test]
fn scrollback_entry_cache_limits_misses_under_streaming() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.begin_run("hello");
    for i in 0..30 {
        model.entries.push(super::bridge::UiEntry::Assistant {
            text: format!("history-{i}\n\nparagraph"),
        });
    }
    root.apply_ui_model(&model);
    let _ = root.render(80);
    root.clear_scrollback_entry_misses_for_test();

    // Streaming deltas change only the tail — committed entries must be cache hits.
    for i in 0..20 {
        model.streaming_assistant.push_str(&format!("x{i}"));
        root.apply_ui_model(&model);
        let _ = root.render(80);
    }
    let misses = root.scrollback_entry_misses_for_test();
    assert!(
        misses <= 2,
        "TextDelta must not re-Markdown all history; misses={misses}"
    );
}

#[test]
fn streaming_assistant_reuses_stable_prefix_under_deltas() {
    use super::layout::{LayoutTheme, UiRoot};
    use super::widgets::{
        GlyphSet, ScrollbackFold, ScrollbackPaintCache, find_stable_markdown_prefix_end,
        render_scrollback,
    };

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.begin_run("hello");
    // Seed enough complete paragraphs so stable prefix is non-empty.
    model.streaming_assistant = "alpha para\n\nbeta para\n\n".into();
    root.apply_ui_model(&model);
    let _ = root.render(80);
    root.clear_streaming_assistant_parse_counts_for_test();

    for i in 0..40 {
        model.streaming_assistant.push_str(&format!("tok{i} "));
        if i % 10 == 9 {
            model.streaming_assistant.push_str("\n\n");
        }
        root.apply_ui_model(&model);
        let _ = root.render(80);
    }

    let full = root.streaming_assistant_full_parses_for_test();
    assert!(
        full <= 8,
        "stable prefix must limit full Markdown parses; full_parses={full}"
    );
    assert!(
        find_stable_markdown_prefix_end(&model.streaming_assistant) > 0,
        "fixture must keep a stable prefix"
    );

    // Warm incremental cache across growth, then compare to a cold full paint.
    let theme = LayoutTheme::product_dark();
    let glyphs = GlyphSet::from_env();
    let fold = ScrollbackFold::default();
    let mut warm = ScrollbackPaintCache::default();
    let mut growing = UiModel::new();
    growing.streaming_assistant = "alpha para\n\nbeta para\n\n".into();
    let _ = render_scrollback(&growing, glyphs, theme, fold, 80, &mut warm);
    for i in 0..40 {
        growing.streaming_assistant.push_str(&format!("tok{i} "));
        if i % 10 == 9 {
            growing.streaming_assistant.push_str("\n\n");
        }
        let _ = render_scrollback(&growing, glyphs, theme, fold, 80, &mut warm);
    }
    assert_eq!(
        growing.streaming_assistant, model.streaming_assistant,
        "warm growth must match root fixture text"
    );
    assert!(
        warm.streaming_assistant.full_parses <= 8,
        "warm cache full_parses={}",
        warm.streaming_assistant.full_parses
    );
    let got = render_scrollback(&growing, glyphs, theme, fold, 80, &mut warm);
    let mut cold = ScrollbackPaintCache::default();
    let expected = render_scrollback(&growing, glyphs, theme, fold, 80, &mut cold);
    assert_eq!(
        got, expected,
        "warm incremental paint must match cold full Markdown(text+…)"
    );
}

#[test]
fn streaming_paint_does_not_break_bash_ctrl_o_viewport() {
    use super::bridge::BashBlockStatus;
    use super::layout::UiRoot;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use xylitol_tui::InputEvent;

    let mut root = UiRoot::new();
    root.set_layout_meta(".", "m");
    let mut model = UiModel::new();
    model.begin_run("hi");
    model.streaming_assistant = "streaming…\n\nmore ".into();
    let long_out: String = (0..20).map(|i| format!("line-{i}\n")).collect();
    model.entries.push(super::bridge::UiEntry::Bash {
        command: "seq".into(),
        status: BashBlockStatus::Success,
        output: long_out,
        exclude_from_context: false,
    });
    root.apply_ui_model(&model);
    let _ = root.render(100);
    assert!(
        !root.fold().tools_output_expanded,
        "default viewport collapsed"
    );
    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Char('o'),
        KeyModifiers::CONTROL,
    )));
    assert!(
        root.fold().tools_output_expanded,
        "Ctrl+O must still expand tool/bash viewport while assistant streams"
    );
    let expanded = root.render(100);
    assert!(
        !strip_ansi(&expanded.join("\n")).contains("ctrl+o to expand"),
        "expanded viewport must drop collapse hint"
    );
}

#[test]
fn models_picker_left_right_cycle_thinking_levels() {
    use super::layout::{ModelPickerRow, UiRoot};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use xylitol_tui::{Component, InputEvent};

    let levels = vec![
        "off".into(),
        "minimal".into(),
        "low".into(),
        "medium".into(),
        "high".into(),
    ];
    let row = ModelPickerRow {
        id: "qwen".into(),
        label: "qwen".into(),
        levels: levels.clone(),
        provisional: "medium".into(),
    };
    let mut root = UiRoot::new();
    let _ = root.render(120);
    root.mount_models_picker(vec![row]);

    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Left,
        KeyModifiers::NONE,
    )));
    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    )));
    let left = root.take_pending_model_select().expect("left confirm");
    assert_eq!(left.model_id, "qwen");
    assert_eq!(left.thinking, "low");

    let row = ModelPickerRow {
        id: "qwen".into(),
        label: "qwen".into(),
        levels,
        provisional: "medium".into(),
    };
    root.mount_models_picker(vec![row]);
    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Right,
        KeyModifiers::NONE,
    )));
    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    )));
    let right = root.take_pending_model_select().expect("right confirm");
    assert_eq!(right.thinking, "high");
}

#[test]
fn interaction_mode_defaults_to_mode_a() {
    let session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    assert_eq!(
        session.tui.interaction_mode(),
        xylitol_tui::InteractionMode::Inline
    );
    assert!(!session.tui.application_session_active());
    assert_eq!(
        crate::app::tui::TuiRunOptions::default().interaction_mode,
        xylitol_tui::InteractionMode::Inline
    );
}

#[test]
fn interaction_mode_b_restacks_and_registers_dock() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.render_now().unwrap();
    session.apply_interaction_mode(xylitol_tui::InteractionMode::ApplicationOwned);
    assert!(session.tui.application_session_active());
    assert!(session.tui.mouse_capture_enabled());
    assert!(session.tui.mode_b_dock_rows() >= 4);
    session.render_now().unwrap();
    // Measured dock after paint should stay above the input band floor.
    assert!(session.tui.mode_b_dock_rows() >= 4);

    session.apply_interaction_mode(xylitol_tui::InteractionMode::Inline);
    assert!(!session.tui.application_session_active());
    assert!(!session.tui.mouse_capture_enabled());
}
