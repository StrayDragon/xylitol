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
fn mouse_moved_does_not_request_render_by_default() {
    use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    // Product default is ApplicationOwned: Moved stays idle; Left Down starts
    // app selection and may schedule a frame (expected AO delta vs Inline).
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
        session.tui.is_render_requested(),
        "ApplicationOwned Left Down begins selection and must schedule a frame"
    );
    session.render_now().unwrap();
    assert!(!session.tui.is_render_requested());

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
fn application_owned_host_coalesced_wheel_paints_once_without_tick_tail() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 12));
    for i in 0..40 {
        session.push_scroll_notice(format!("wheel-history-{i:02}"));
    }
    session.render_now().expect("seed long AO viewport");
    session.tui.terminal.frames.clear();
    let frames_before = session.tui.frame_count();
    let notch = session.tui.application_owned_wheel_notch();

    session
        .apply_ao_wheel_delta(-(notch * 2))
        .expect("coalesced wheel delta");

    assert_eq!(session.tui.frame_count(), frames_before + 1);
    assert!(
        session.tui.last_render_perf().ao_reprojected,
        "host coalesce should take the cached AO reproject path"
    );
    assert!(!session.tui.is_render_requested());
    assert!(
        !session.wants_busy_tick(),
        "normal wheel motion must not leave a residual busy-tick tail"
    );
    assert!(
        !session.tui.terminal.frames.is_empty(),
        "the coalesced viewport delta should paint immediately"
    );

    let frames_after_first = session.tui.frame_count();
    session
        .apply_ao_wheel_delta(-notch)
        .expect("continuous wheel delta");
    assert_eq!(
        session.tui.frame_count(),
        frames_after_first,
        "continuous wheel paint should respect the independent 60fps cap"
    );
    assert!(session.tui.is_render_requested());
    let deadline = session
        .tui
        .application_owned_wheel_render_deadline()
        .expect("capped wheel paint must expose its exact deadline");
    assert!(
        deadline <= std::time::Instant::now() + std::time::Duration::from_millis(16),
        "wheel deadline must be based on the previous paint, not a fresh host tick"
    );
    session
        .apply_ao_wheel_delta(-notch)
        .expect("additional wheel delta before deadline");
    assert_eq!(
        session.tui.application_owned_wheel_render_deadline(),
        Some(deadline),
        "new wheel input must not restart the existing paint deadline"
    );
    assert!(
        !session.wants_busy_tick(),
        "wheel deadline owns this wake; the generic busy ticker must not add delay"
    );
    session.tui.render_now().expect("flush capped wheel paint");
    assert!(
        session
            .tui
            .application_owned_wheel_render_deadline()
            .is_none()
    );
    assert!(!session.wants_busy_tick());
}

#[test]
fn inline_unhandled_mouse_down_does_not_request_render() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui_with_meta_mode(
        TestTerminal::new(80, 24),
        "/tmp".into(),
        "model".into(),
        xylitol_tui::InteractionMode::Inline,
    );
    session.render_now().unwrap();
    assert!(!session.tui.is_render_requested());

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
        "Inline unhandled mouse Down must not schedule a frame"
    );
}

#[test]
fn product_tui_source_has_no_tui_start_call() {
    let sources = [
        ("mod.rs", include_str!("mod.rs")),
        ("host/mod.rs", include_str!("host/mod.rs")),
        ("effects/mod.rs", include_str!("effects/mod.rs")),
        ("effects/slash/mod.rs", include_str!("effects/slash/mod.rs")),
        (
            "effects/slash/chrome.rs",
            include_str!("effects/slash/chrome.rs"),
        ),
        (
            "effects/slash/compact.rs",
            include_str!("effects/slash/compact.rs"),
        ),
        (
            "effects/slash/debug.rs",
            include_str!("effects/slash/debug.rs"),
        ),
        (
            "effects/slash/model.rs",
            include_str!("effects/slash/model.rs"),
        ),
        (
            "effects/slash/session.rs",
            include_str!("effects/slash/session.rs"),
        ),
        (
            "effects/pending_ui/mod.rs",
            include_str!("effects/pending_ui/mod.rs"),
        ),
        (
            "effects/pending_ui/clipboard.rs",
            include_str!("effects/pending_ui/clipboard.rs"),
        ),
        (
            "effects/pending_ui/import.rs",
            include_str!("effects/pending_ui/import.rs"),
        ),
        (
            "effects/pending_ui/models.rs",
            include_str!("effects/pending_ui/models.rs"),
        ),
        (
            "effects/pending_ui/resume.rs",
            include_str!("effects/pending_ui/resume.rs"),
        ),
        (
            "effects/pending_ui/theme.rs",
            include_str!("effects/pending_ui/theme.rs"),
        ),
        (
            "effects/pending_ui/tree.rs",
            include_str!("effects/pending_ui/tree.rs"),
        ),
        ("effects/bang.rs", include_str!("effects/bang.rs")),
        ("commands.rs", include_str!("commands.rs")),
        ("layout/root/mod.rs", include_str!("layout/root/mod.rs")),
        (
            "layout/root/slot_input.rs",
            include_str!("layout/root/slot_input.rs"),
        ),
        ("layout/slots/mod.rs", include_str!("layout/slots/mod.rs")),
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
    let slots = include_str!("layout/slots/mod.rs");
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
fn product_slash_catalog_matches_agent_ssot() {
    // c1175 / sc3 / atm7: TUI catalog names == product SSOT (same crate build).
    use crate::app::product_commands::product_slash_commands;
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
    let follow_idx = frame
        .iter()
        .position(|l| l.contains("Follow-up: later"))
        .unwrap_or_else(|| panic!("missing Follow-up strip: {frame:?}"));
    let status_idx = frame
        .iter()
        .position(|l| l.contains("Assembling") || l.contains("Working"))
        .unwrap_or_else(|| panic!("missing busy status: {frame:?}"));
    assert!(
        follow_idx < status_idx,
        "Follow-up must sit above status/spinner; follow={follow_idx} status={status_idx}; {frame:?}"
    );
    assert!(
        status_idx - follow_idx <= 4,
        "Follow-up must be docked next to status (not under startup card); gap={}; {frame:?}",
        status_idx - follow_idx
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
    use super::layout::{EditorSlot, EditorSlotKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();

    root.borrow_mut().open_slot_for_test(EditorSlot::Plate);
    assert_eq!(root.borrow().slot(), EditorSlotKind::Plate);
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
    assert_eq!(root.borrow().slot(), EditorSlotKind::Tree);
    assert!(root.borrow().tree_open());

    session.step(HostEvent::Input(esc_event())).unwrap();
    assert_eq!(root.borrow().slot(), EditorSlotKind::Editor);

    root.borrow_mut().open_slot_for_test(EditorSlot::Settings);
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert_eq!(root.borrow().slot(), EditorSlotKind::Editor);

    root.borrow_mut()
        .open_slot_for_test(EditorSlot::choice_shell());
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert_eq!(root.borrow().slot(), EditorSlotKind::Editor);
}

#[test]
fn harness_busy_esc_aborts_not_tree_slot() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    session.on_run_started("hello");
    assert_eq!(root.borrow().slot(), super::layout::EditorSlotKind::Editor);
    session.step(HostEvent::Input(esc_event())).unwrap();
    assert!(session.take_abort());
    assert_eq!(
        root.borrow().slot(),
        super::layout::EditorSlotKind::Editor,
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
    for i in 0..40 {
        session.push_scroll_notice(format!("submit-scroll-{i:02}"));
    }
    session.render_now().expect("seed long transcript");
    let notch = session.tui.application_owned_wheel_notch();
    assert!(session.tui.application_owned_scroll_by(-(notch * 2)));

    root.borrow_mut().set_editor_text("run me");
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert_eq!(session.take_submit().as_deref(), Some("run me"));
    assert!(root.borrow().editor_text().is_empty());
    assert!(
        !session.tui.application_owned_scroll_by(notch),
        "successful Enter submit must already be following the transcript bottom"
    );
}

#[test]
fn harness_empty_enter_follows_bottom_without_submit() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    for i in 0..40 {
        session.push_scroll_notice(format!("empty-enter-scroll-{i:02}"));
    }
    session.render_now().expect("seed long transcript");
    let notch = session.tui.application_owned_wheel_notch();
    assert!(session.tui.application_owned_scroll_by(-(notch * 2)));

    session.step(HostEvent::Input(enter_event())).unwrap();

    assert!(session.take_submit().is_none());
    assert!(
        !session.tui.application_owned_scroll_by(notch),
        "empty Enter must follow the transcript bottom without submitting"
    );

    assert!(session.tui.application_owned_scroll_by(-(notch * 2)));
    root.borrow_mut()
        .open_slot_for_test(super::layout::EditorSlot::Plate);
    session.step(HostEvent::Input(enter_event())).unwrap();
    assert!(
        session.tui.application_owned_scroll_by(notch),
        "Enter in a non-Editor slot must not force the transcript to the bottom"
    );
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
            timestamp: 0,
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
                timestamp: 0,
            },
            message: fixture_message_json("user", "only me"),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a1".into(),
                parent_id: None,
                timestamp: 0,
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
fn harness_resume_snapshot_rebuild_renders_full_history_once_idle() {
    use super::bridge::UiEntry;
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry, fixture_message_json};

    let entries = vec![
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "r-u1".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: fixture_message_json("user", "first question"),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "r-a1".into(),
                parent_id: Some("r-u1".into()),
                timestamp: 0,
            },
            message: fixture_message_json("assistant", "full answer one"),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "r-u2".into(),
                parent_id: Some("r-a1".into()),
                timestamp: 0,
            },
            message: fixture_message_json("user", "follow-up"),
        }),
    ];
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.apply_resume_session("sid-resume", entries);
    let model = session.ui_model();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::User { text } if text.contains("first question")
        )) && model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Assistant { text, .. } if text.contains("full answer one")
        )) && model.entries.iter().any(|e| matches!(
            e,
            UiEntry::User { text } if text.contains("follow-up")
        )),
        "resume rebuild MUST project full history in one shot: {:?}",
        model.entries
    );
    assert!(
        !session.is_busy(),
        "snapshot projection MUST leave the session Idle (no fake spinner)"
    );
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
                timestamp: 0,
            },
            message: fixture_message_json("user", "hi"),
        }),
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a1".into(),
                parent_id: Some("u1".into()),
                timestamp: 0,
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
        id: super::bridge::allocate_thinking_id(&[], "secret plan"),
        text: "secret plan".into(),
        elapsed_secs: None,
    });
    root.apply_ui_model(&model);
    let idle = root.render(80).join("\n");
    assert!(idle.contains("(Ctrl+T)"), "{idle}");
    assert!(
        idle.contains("Thought"),
        "flushed thinking L1 is Thought: {idle}"
    );
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
fn tools_per_block_override_and_alt_e_clears() {
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use xylitol_tui::InputEvent;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "t1".into(),
        name: "read".into(),
        args_preview: "a.rs".into(),
        tool_path: None,
        write_content: None,
        display_diff: None,
        output: "body-one".into(),
        is_error: false,
        done: true,
    });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "t2".into(),
        name: "read".into(),
        args_preview: "b.rs".into(),
        tool_path: None,
        write_content: None,
        display_diff: None,
        output: "body-two".into(),
        is_error: false,
        done: true,
    });
    root.apply_ui_model(&model);
    let _ = root.render(80);
    assert!(root.fold().tools_effective("t1"));
    assert!(root.fold().tools_effective("t2"));

    root.toggle_fold_target(FoldTarget::Tool("t1".into()));
    assert!(!root.fold().tools_effective("t1"));
    assert!(root.fold().tools_effective("t2"));
    assert_eq!(root.fold().tools_overrides.len(), 1);

    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Char('e'),
        KeyModifiers::ALT,
    )));
    assert!(root.fold().tools_overrides.is_empty());
    // Default flipped from true → false; both follow default.
    assert!(!root.fold().tools_effective("t1"));
    assert!(!root.fold().tools_effective("t2"));
}

#[test]
fn thinking_per_id_override_and_ctrl_t_clears() {
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use xylitol_tui::InputEvent;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    let t1 = "alpha thought".to_string();
    let t2 = "beta thought".to_string();
    let id1 = super::bridge::allocate_thinking_id(&[], &t1);
    model.entries.push(super::bridge::UiEntry::Thinking {
        id: id1.clone(),
        text: t1,
        elapsed_secs: None,
    });
    let id2 = super::bridge::allocate_thinking_id(&model.entries, &t2);
    model.entries.push(super::bridge::UiEntry::Thinking {
        id: id2.clone(),
        text: t2,
        elapsed_secs: None,
    });
    root.apply_ui_model(&model);
    let _ = root.render(80);
    assert!(!root.fold().thinking_effective(&id1));
    assert!(!root.fold().thinking_effective(&id2));

    root.toggle_fold_target(FoldTarget::Thinking(id1.clone()));
    assert!(root.fold().thinking_effective(&id1));
    assert!(!root.fold().thinking_effective(&id2));

    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Char('t'),
        KeyModifiers::CONTROL,
    )));
    assert!(root.fold().thinking_overrides.is_empty());
    assert!(root.fold().thinking_effective(&id1));
    assert!(root.fold().thinking_effective(&id2));
}

#[test]
fn single_tool_toggle_does_not_miss_unrelated_assistant() {
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    for i in 0..8 {
        model.entries.push(super::bridge::UiEntry::Assistant {
            text: format!("history-{i}\n\nparagraph"),
        });
    }
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "only".into(),
        name: "bash".into(),
        args_preview: "ls".into(),
        tool_path: None,
        write_content: None,
        display_diff: None,
        output: "out".into(),
        is_error: false,
        done: true,
    });
    root.apply_ui_model(&model);
    let _ = root.render(80);
    root.clear_scrollback_entry_misses_for_test();

    root.toggle_fold_target(FoldTarget::Tool("only".into()));
    let _ = root.render(80);
    let misses = root.scrollback_entry_misses_for_test();
    assert!(
        misses <= 1,
        "single-block toggle must not re-Markdown assistants; misses={misses}"
    );
}

#[test]
fn harness_mouse_triangle_toggles_tool_fold() {
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    {
        let mut model = session.ui_model().clone();
        model.entries.push(super::bridge::UiEntry::Tool {
            timeout_secs: None,
            id: "click-me".into(),
            name: "read".into(),
            args_preview: "x.rs".into(),
            tool_path: None,
            write_content: None,
            display_diff: None,
            output: "tool-body-visible".into(),
            is_error: false,
            done: true,
        });
        *session.ui_model_mut() = model;
        session.sync_ui_root_from_model();
    }
    session.step(HostEvent::Tick).unwrap();
    // Force a paint so fold_hits populate.
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    let hit = {
        let r = root.borrow();
        r.fold_hits()
            .regions
            .iter()
            .find(|reg| matches!(&reg.target, FoldTarget::Tool(id) if id == "click-me"))
            .cloned()
    };
    let Some(region) = hit else {
        panic!(
            "expected tool triangle hit region; regions={:?}",
            root.borrow().fold_hits().regions
        );
    };
    assert!(root.borrow().fold().tools_effective("click-me"));

    let screen_row = region
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    let screen_col = region.col_start as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: screen_col,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        !root.borrow().fold().tools_effective("click-me"),
        "triangle click must toggle tools override"
    );
    session.tui.request_render(true);
    session.step_paint_only().unwrap();
    let after = root.borrow_mut().render(80).join("\n");
    assert!(
        !after.contains("tool-body-visible"),
        "AO paint must collapse tool body after triangle toggle: {after}"
    );

    // Click body column (not triangle) must not toggle back.
    let before = root.borrow().fold().tools_effective("click-me");
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: (region.col_end + 4) as u16,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert_eq!(
        root.borrow().fold().tools_effective("click-me"),
        before,
        "non-triangle column must not toggle"
    );
}

#[test]
fn harness_mouse_drag_across_triangle_does_not_toggle_fold() {
    // att22: while transcript drag-select is active, crossing the fold triangle
    // must not toggle (engine only consults hit_priority on Left Down).
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    {
        let mut model = session.ui_model().clone();
        model.entries.push(super::bridge::UiEntry::Tool {
            timeout_secs: None,
            id: "drag-me".into(),
            name: "read".into(),
            args_preview: "y.rs".into(),
            tool_path: None,
            write_content: None,
            display_diff: None,
            output: "drag-body".into(),
            is_error: false,
            done: true,
        });
        *session.ui_model_mut() = model;
        session.sync_ui_root_from_model();
    }
    session.step(HostEvent::Tick).unwrap();
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    let region = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(&reg.target, FoldTarget::Tool(id) if id == "drag-me"))
        .cloned()
        .expect("tool triangle hit region");
    let screen_row = region
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    let body_col = (region.col_end + 4) as u16;
    let tri_col = region.col_start as u16;
    assert!(root.borrow().fold().tools_effective("drag-me"));

    // Start selection on header body (miss fold hit).
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: body_col,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        root.borrow().fold().tools_effective("drag-me"),
        "body Down must not toggle"
    );

    // Drag across the triangle column — must not flip fold.
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: tri_col,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        root.borrow().fold().tools_effective("drag-me"),
        "Drag over triangle must not toggle fold"
    );

    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: tri_col,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        root.borrow().fold().tools_effective("drag-me"),
        "release after drag across triangle must not toggle fold"
    );
}

#[test]
fn harness_mouse_triangle_toggles_thinking_fold() {
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    let thinking_text = "think-body-visible".to_string();
    let thinking_id = {
        let mut model = session.ui_model().clone();
        let id = super::bridge::allocate_thinking_id(&model.entries, &thinking_text);
        model.entries.push(super::bridge::UiEntry::Thinking {
            id: id.clone(),
            text: thinking_text.clone(),
            elapsed_secs: None,
        });
        *session.ui_model_mut() = model;
        session.sync_ui_root_from_model();
        id
    };
    session.step(HostEvent::Tick).unwrap();
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    let region = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(&reg.target, FoldTarget::Thinking(id) if id == &thinking_id))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected thinking triangle hit; regions={:?}",
                root.borrow().fold_hits().regions
            )
        });
    assert!(!root.borrow().fold().thinking_effective(&thinking_id));

    let screen_row = region
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    let screen_col = region.col_start as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: screen_col,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        root.borrow().fold().thinking_effective(&thinking_id),
        "thinking triangle click must toggle per-id override"
    );
    session.tui.request_render(true);
    session.step_paint_only().unwrap();
    let after = root.borrow_mut().render(80).join("\n");
    assert!(
        after.contains(&thinking_text),
        "AO paint must show thinking body after triangle expand: {after}"
    );
}

#[test]
fn harness_mouse_triangle_toggles_diff_fold() {
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    {
        let mut model = session.ui_model().clone();
        model.entries.push(super::bridge::UiEntry::Diff {
            summary: "edit unique-diff.rs".into(),
            display_diff: "+ unique-diff-body-line\n- old\n".into(),
        });
        *session.ui_model_mut() = model;
        session.sync_ui_root_from_model();
    }
    session.step(HostEvent::Tick).unwrap();
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    let (region, key) = {
        let r = root.borrow();
        let region = r
            .fold_hits()
            .regions
            .iter()
            .find(|reg| matches!(&reg.target, FoldTarget::Diff(_)))
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "expected Diff triangle hit; regions={:?}",
                    r.fold_hits().regions
                )
            });
        let FoldTarget::Diff(key) = region.target.clone() else {
            unreachable!("matched Diff above");
        };
        (region, key)
    };
    assert!(root.borrow().fold().tools_effective(&key));

    let screen_row = region
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: region.col_start as u16,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        !root.borrow().fold().tools_effective(&key),
        "Diff triangle click must toggle tools override"
    );
    session.tui.request_render(true);
    session.step_paint_only().unwrap();
    let after = root.borrow_mut().render(80).join("\n");
    assert!(
        !after.contains("unique-diff-body-line"),
        "AO paint must hide Diff body after triangle collapse: {after}"
    );
}

#[test]
fn harness_mouse_triangle_toggles_ask_fold() {
    use super::bridge::AskPhase;
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    {
        let mut model = session.ui_model().clone();
        model.entries.push(super::bridge::UiEntry::Ask {
            id: "ask-click".into(),
            summary: "Ask · choose".into(),
            detail_lines: vec!["ask-detail-visible".into()],
            phase: AskPhase::Answered,
            expanded: true,
        });
        *session.ui_model_mut() = model;
        session.sync_ui_root_from_model();
    }
    session.step(HostEvent::Tick).unwrap();
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    let region = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(&reg.target, FoldTarget::Ask(id) if id == "ask-click"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected Ask triangle hit; regions={:?}",
                root.borrow().fold_hits().regions
            )
        });
    assert!(root.borrow().fold().tools_effective("ask-click"));

    let screen_row = region
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: region.col_start as u16,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        !root.borrow().fold().tools_effective("ask-click"),
        "Ask triangle click must toggle tools override"
    );
    session.tui.request_render(true);
    session.step_paint_only().unwrap();
    let after = root.borrow_mut().render(80).join("\n");
    assert!(
        !after.contains("ask-detail-visible"),
        "AO paint must hide Ask detail after triangle collapse: {after}"
    );
}

#[test]
fn harness_mouse_triangle_toggles_compaction_fold() {
    // att29: Compaction triangle flips compaction_expanded; body click does not;
    // tools overrides stay intact.
    use super::bridge::CompactionBlockStatus;
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    {
        let mut model = session.ui_model().clone();
        model.entries.push(super::bridge::UiEntry::Tool {
            timeout_secs: None,
            id: "keep-override".into(),
            name: "read".into(),
            args_preview: "z.rs".into(),
            tool_path: None,
            write_content: None,
            display_diff: None,
            output: "tool-keep".into(),
            is_error: false,
            done: true,
        });
        model.entries.push(super::bridge::UiEntry::Compaction {
            status: CompactionBlockStatus::Complete,
            summary: "compaction-summary-body".into(),
            tokens_before: 12_345,
            detail: None,
        });
        *session.ui_model_mut() = model;
        session.sync_ui_root_from_model();
    }
    session.step(HostEvent::Tick).unwrap();
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    root.borrow_mut()
        .toggle_fold_target(FoldTarget::Tool("keep-override".into()));
    assert!(!root.borrow().fold().tools_effective("keep-override"));
    let overrides_before = root.borrow().fold().tools_overrides.len();
    assert!(!root.borrow().fold().compaction_expanded);
    // Re-paint so Compaction hit rows match post-override layout.
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    let region = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(reg.target, FoldTarget::Compaction))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected Compaction triangle hit; regions={:?}",
                root.borrow().fold_hits().regions
            )
        });

    let screen_row = region
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: region.col_start as u16,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        root.borrow().fold().compaction_expanded,
        "Compaction triangle must flip compaction_expanded"
    );
    assert_eq!(
        root.borrow().fold().tools_overrides.len(),
        overrides_before,
        "Compaction click MUST NOT clear tools overrides"
    );
    assert!(
        !root.borrow().fold().tools_effective("keep-override"),
        "tools override must survive Compaction toggle"
    );
    session.tui.request_render(true);
    session.step_paint_only().unwrap();
    let after = root.borrow_mut().render(80).join("\n");
    assert!(
        after.contains("compaction-summary-body"),
        "AO paint must show Compaction summary after expand: {after}"
    );

    // Re-hit after expand: triangle stays on the Compacted header (same line as
    // [compaction]); click it to collapse again.
    session.tui.request_render(true);
    session.step_paint_only().unwrap();
    let region_expanded = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(reg.target, FoldTarget::Compaction))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected Compaction triangle after expand; regions={:?}",
                root.borrow().fold_hits().regions
            )
        });
    let screen_row_expanded = region_expanded
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: region_expanded.col_start as u16,
            row: screen_row_expanded,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        !root.borrow().fold().compaction_expanded,
        "Compaction triangle must collapse after a second click"
    );
    session.tui.request_render(true);
    session.step_paint_only().unwrap();
    let collapsed_again = root.borrow_mut().render(80).join("\n");
    assert!(
        !collapsed_again.contains("compaction-summary-body"),
        "AO paint must hide Compaction summary after triangle collapse: {collapsed_again}"
    );

    // Re-expand so the non-triangle body click check below still has a visible header.
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: region_expanded.col_start as u16,
            row: screen_row_expanded,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(root.borrow().fold().compaction_expanded);
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    let before = root.borrow().fold().compaction_expanded;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: (region_expanded.col_end + 4) as u16,
            row: screen_row_expanded,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert_eq!(
        root.borrow().fold().compaction_expanded,
        before,
        "non-triangle Compaction body must not toggle"
    );
}

#[test]
fn harness_mouse_hint_toggles_output_viewport() {
    // att30: Ctrl+O hint band flips tools_output_expanded; isomorphic with Ctrl+O.
    use super::widgets::FoldTarget;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    {
        let mut model = session.ui_model().clone();
        let mut output = String::new();
        for i in 0..20 {
            output.push_str(&format!("viewport-line-{i}\n"));
        }
        model.entries.push(super::bridge::UiEntry::Bash {
            command: "big".into(),
            status: super::bridge::BashBlockStatus::Success,
            output,
            exclude_from_context: false,
        });
        *session.ui_model_mut() = model;
        session.sync_ui_root_from_model();
    }
    session.step(HostEvent::Tick).unwrap();
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    assert!(
        !root
            .borrow()
            .fold_hits()
            .regions
            .iter()
            .any(|r| matches!(r.target, FoldTarget::Tool(_))),
        "Bash MUST NOT register L1 Tool triangle"
    );
    let region = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(reg.target, FoldTarget::OutputViewport))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected OutputViewport hint hit; regions={:?}",
                root.borrow().fold_hits().regions
            )
        });
    assert!(!root.borrow().fold().tools_output_expanded);

    let screen_row = region
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: region.col_start as u16,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        root.borrow().fold().tools_output_expanded,
        "hint click must flip tools_output_expanded"
    );
    session.tui.request_render(true);
    session.step_paint_only().unwrap();
    let expanded = root.borrow_mut().render(80).join("\n");
    assert!(
        expanded.contains("viewport-line-0"),
        "expanded viewport must show early lines: {expanded}"
    );

    // Collapse via Ctrl+O (same bool), then expand again via key to confirm isomorphism.
    root.borrow_mut()
        .handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Char('o'),
            KeyModifiers::CONTROL,
        )));
    assert!(
        !root.borrow().fold().tools_output_expanded,
        "Ctrl+O must share tools_output_expanded with hint click"
    );
    root.borrow_mut()
        .handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Char('o'),
            KeyModifiers::CONTROL,
        )));
    assert!(root.borrow().fold().tools_output_expanded);
}

#[test]
fn harness_mouse_segment_marker_toggles_one_step() {
    // att31: L2 summary fold-marker click = that segment one-step expand;
    // body click does not; coexisting L1 Tool triangle still works (att32).
    use super::activity_fold::SegmentLevel;
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(100, 32));
    let root = session.ui_root().expect("product ui").clone();
    {
        let mut model = session.ui_model().clone();
        model
            .entries
            .extend(activity_turn("u0", "tool-l2", "old.rs", "a0"));
        model
            .entries
            .extend(activity_turn("u1", "tool-l1", "new.rs", "a1"));
        *session.ui_model_mut() = model;
        session.sync_ui_root_from_model();
    }
    {
        let mut r = root.borrow_mut();
        r.activity_mut().force_level("seg-0", SegmentLevel::L3);
        // Newest turn: open its cluster so a coexisting L1 tool hit is on screen.
        let _ = r.activity_mut().toggle_cluster("seg-3:c0");
        r.touch_activity();
    }
    session.step(HostEvent::Tick).unwrap();
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    assert_eq!(root.borrow().activity().level_of("seg-0"), SegmentLevel::L3);
    assert!(
        !root
            .borrow()
            .fold_hits()
            .regions
            .iter()
            .any(|r| matches!(&r.target, FoldTarget::Tool(id) if id == "tool-l2")),
        "L3 envelope MUST NOT register L1 hits for collapsed middles"
    );
    let seg_hit = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(&reg.target, FoldTarget::Segment(id) if id == "seg-0"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected Segment(seg-0) hit; regions={:?}",
                root.borrow().fold_hits().regions
            )
        });
    let tool_hit = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(&reg.target, FoldTarget::Tool(id) if id == "tool-l1"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "expected L1 Tool(tool-l1) hit on same screen; regions={:?}",
                root.borrow().fold_hits().regions
            )
        });

    let seg_row = seg_hit
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: seg_hit.col_start as u16,
            row: seg_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert_eq!(
        root.borrow().activity().level_of("seg-0"),
        SegmentLevel::L2,
        "Segment marker click MUST expand one step (L3→L2)"
    );

    // Re-collapse and confirm body column does not toggle.
    {
        let mut r = root.borrow_mut();
        r.activity_mut().force_level("seg-0", SegmentLevel::L3);
        r.touch_activity();
    }
    session.tui.request_render(true);
    session.step_paint_only().unwrap();
    let seg_hit = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(&reg.target, FoldTarget::Segment(id) if id == "seg-0"))
        .cloned()
        .expect("Segment hit after re-collapse");
    let seg_row = seg_hit
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: (seg_hit.col_end + 4) as u16,
            row: seg_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert_eq!(
        root.borrow().activity().level_of("seg-0"),
        SegmentLevel::L3,
        "summary body click MUST NOT toggle segment"
    );

    // L1 tool triangle still toggles its own override (same paint).
    let tool_row = tool_hit
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    let before = root.borrow().fold().tools_effective("tool-l1");
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: tool_hit.col_start as u16,
            row: tool_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert_ne!(
        root.borrow().fold().tools_effective("tool-l1"),
        before,
        "coexisting L1 Tool triangle must still toggle"
    );
    assert_eq!(
        root.borrow().activity().level_of("seg-0"),
        SegmentLevel::L3,
        "L1 tool click MUST NOT change L3 envelope level"
    );
}

#[test]
fn harness_l2_ignores_l1_override_after_segment_present() {
    // att31 / att25 deep-dive A: while seg is L2, flipping tools override must
    // not change L2 summary paint (middles not rendered).
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .extend(activity_turn("u0", "hidden-tool", "x.rs", "a0"));
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let before = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        before.contains("Explored") || before.contains("file"),
        "L2 summary expected: {before}"
    );
    root.toggle_fold_target(FoldTarget::Tool("hidden-tool".into()));
    let after = strip_ansi_activity(&root.render(100).join("\n"));
    assert_eq!(
        before, after,
        "L1 override MUST NOT change L2 segment appearance"
    );
    assert_eq!(root.activity().level_of("seg-0"), SegmentLevel::L2);
}

#[test]
fn segment_toggle_one_step_and_local_paint_misses() {
    // att31 + ath25: Segment toggle is one ladder step; not nearest; local misses.
    // Warm cache at L0 first (same pattern as activity_fold_att25_level_switch…).
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    for i in 0..6 {
        model.entries.push(super::bridge::UiEntry::Assistant {
            text: format!("history-{i}\n\nparagraph"),
        });
    }
    model
        .entries
        .extend(activity_turn("u", "only", "x.rs", "tail"));
    root.apply_ui_model(&model);
    let _ = root.render(80);
    root.activity_mut().force_level("seg-6", SegmentLevel::L3);
    root.touch_activity();
    let _ = root.render(80);
    root.clear_scrollback_entry_misses_for_test();

    root.toggle_fold_target(FoldTarget::Segment("seg-6".into()));
    assert_eq!(root.activity().level_of("seg-6"), SegmentLevel::L2);
    let _ = root.render(80);
    let misses = root.scrollback_entry_misses_for_test();
    assert!(
        misses <= 3,
        "Segment one-step toggle must stay local; misses={misses}"
    );

    // Pointed toggle must not move a different collapsed segment.
    let mut model2 = UiModel::new();
    model2
        .entries
        .extend(activity_turn("u0", "t0", "a.rs", "a0"));
    model2
        .entries
        .extend(activity_turn("u1", "t1", "b.rs", "a1"));
    let mut root2 = UiRoot::new();
    root2.apply_ui_model(&model2);
    root2.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root2.activity_mut().force_level("seg-3", SegmentLevel::L2);
    root2.touch_activity();
    root2.toggle_fold_target(FoldTarget::Segment("seg-0".into()));
    assert_eq!(root2.activity().level_of("seg-0"), SegmentLevel::L3);
    assert_eq!(
        root2.activity().level_of("seg-3"),
        SegmentLevel::L2,
        "must not use expandNearest"
    );
}

#[test]
fn compaction_and_viewport_toggle_miss_bound() {
    // ath25 / att29–att30: global Compaction / OutputViewport toggle must not
    // re-Markdown unrelated Assistant history.
    use super::bridge::CompactionBlockStatus;
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    for i in 0..8 {
        model.entries.push(super::bridge::UiEntry::Assistant {
            text: format!("history-{i}\n\nparagraph"),
        });
    }
    model.entries.push(super::bridge::UiEntry::Compaction {
        status: CompactionBlockStatus::Complete,
        summary: "sum".into(),
        tokens_before: 100,
        detail: None,
    });
    let mut output = String::new();
    for i in 0..20 {
        output.push_str(&format!("line-{i}\n"));
    }
    model.entries.push(super::bridge::UiEntry::Bash {
        command: "x".into(),
        status: super::bridge::BashBlockStatus::Success,
        output,
        exclude_from_context: false,
    });
    root.apply_ui_model(&model);
    let _ = root.render(80);
    root.clear_scrollback_entry_misses_for_test();

    root.toggle_fold_target(FoldTarget::Compaction);
    let _ = root.render(80);
    let misses_compaction = root.scrollback_entry_misses_for_test();
    assert!(
        misses_compaction <= 2,
        "Compaction toggle must not re-Markdown assistants; misses={misses_compaction}"
    );

    root.clear_scrollback_entry_misses_for_test();
    root.toggle_fold_target(FoldTarget::OutputViewport);
    let _ = root.render(80);
    let misses_viewport = root.scrollback_entry_misses_for_test();
    assert!(
        misses_viewport <= 2,
        "OutputViewport toggle must not re-Markdown assistants; misses={misses_viewport}"
    );
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
        FoldHitTable, GlyphSet, ScrollbackFold, ScrollbackPaintCache,
        find_stable_markdown_prefix_end, render_scrollback,
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
    let _ = render_scrollback(
        &growing,
        glyphs,
        theme,
        &fold,
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        80,
        &mut warm,
        &mut FoldHitTable::default(),
    );
    for i in 0..40 {
        growing.streaming_assistant.push_str(&format!("tok{i} "));
        if i % 10 == 9 {
            growing.streaming_assistant.push_str("\n\n");
        }
        let _ = render_scrollback(
            &growing,
            glyphs,
            theme,
            &fold,
            &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
            80,
            &mut warm,
            &mut FoldHitTable::default(),
        );
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
    let got = render_scrollback(
        &growing,
        glyphs,
        theme,
        &fold,
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        80,
        &mut warm,
        &mut FoldHitTable::default(),
    );
    let mut cold = ScrollbackPaintCache::default();
    let expected = render_scrollback(
        &growing,
        glyphs,
        theme,
        &fold,
        &mut crate::app::tui::activity_fold::ActivityFoldState::default(),
        80,
        &mut cold,
        &mut FoldHitTable::default(),
    );
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
fn interaction_mode_defaults_to_application_owned() {
    let session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    assert_eq!(
        session.tui.interaction_mode(),
        xylitol_tui::InteractionMode::ApplicationOwned
    );
    assert!(
        session.tui.application_session_active(),
        "product default MUST begin ApplicationOwned session"
    );
    assert!(session.tui.mouse_capture_enabled());
    assert_eq!(
        crate::app::tui::TuiRunOptions::default().interaction_mode,
        xylitol_tui::InteractionMode::ApplicationOwned
    );
}

#[test]
fn lab_inline_env_selects_inline_at_construction() {
    assert_eq!(
        crate::app::tui::lab_interaction_mode(Some("1")),
        xylitol_tui::InteractionMode::Inline
    );
    assert_eq!(
        crate::app::tui::lab_interaction_mode(Some(" true ")),
        xylitol_tui::InteractionMode::Inline
    );
    assert_eq!(
        crate::app::tui::lab_interaction_mode(Some("YES")),
        xylitol_tui::InteractionMode::Inline
    );
    assert_eq!(
        crate::app::tui::lab_interaction_mode(Some("0")),
        xylitol_tui::InteractionMode::ApplicationOwned
    );
    assert_eq!(
        crate::app::tui::lab_interaction_mode(None),
        xylitol_tui::InteractionMode::ApplicationOwned
    );
}

#[test]
fn interaction_application_owned_at_construction_registers_dock() {
    let mut session = HostSession::new_product_ui_with_meta_mode(
        TestTerminal::new(80, 24),
        "/tmp".into(),
        "model".into(),
        xylitol_tui::InteractionMode::ApplicationOwned,
    );
    assert!(session.tui.application_session_active());
    assert!(session.tui.mouse_capture_enabled());
    assert!(session.tui.dock_rows() >= 4);
    session.render_now().unwrap();
    // Measured dock after paint should stay above the input band floor.
    assert!(session.tui.dock_rows() >= 4);
}

#[test]
fn application_owned_copy_notice_chrome_ath31() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui_with_meta_mode(
        TestTerminal::new(80, 24),
        "/tmp".into(),
        "model".into(),
        xylitol_tui::InteractionMode::ApplicationOwned,
    );
    session.push_scroll_notice("hello world for application-owned copy");
    session.render_now().unwrap();

    let root = session.ui_root().expect("product ui root").clone();
    let (origin_row, origin_col) = root.borrow().editor_screen_origin_for_test();
    assert_eq!(origin_col, 0);
    let term_rows = session.tui.terminal.rows() as usize;
    let dock = session.tui.dock_rows();
    assert!(
        origin_row as usize >= term_rows.saturating_sub(dock),
        "editor origin must sit in ApplicationOwned dock band: origin={origin_row} dock_top={}",
        term_rows.saturating_sub(dock)
    );

    // Drag-select transcript → copy-on-release → host take_copy_notice → «Copied».
    let mouse = |kind, col, row| {
        InputEvent::Mouse(MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        })
    };
    session
        .step(HostEvent::Input(mouse(
            MouseEventKind::Down(MouseButton::Left),
            0,
            0,
        )))
        .unwrap();
    session
        .step(HostEvent::Input(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            5,
            0,
        )))
        .unwrap();
    session
        .step(HostEvent::Input(mouse(
            MouseEventKind::Up(MouseButton::Left),
            5,
            0,
        )))
        .unwrap();
    session.render_now().unwrap();

    assert_eq!(root.borrow().copy_notice_body_for_test(), Some("Copied"));
    assert!(
        root.borrow().chrome_toast_body().is_none(),
        "ath31 MUST NOT use Error: chrome-toast for copy success"
    );
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains("Copied"),
        "frame MUST show Copied cue, got: {joined:?}"
    );
    assert!(
        !joined.contains("Error: Copied") && !joined.contains("Error:Copied"),
        "Copied MUST NOT be Error: toast morph, got: {joined:?}"
    );
}

#[test]
fn application_owned_copy_notice_arms_copied_cue_not_error_toast() {
    let mut session = HostSession::new_product_ui_with_meta_mode(
        TestTerminal::new(80, 24),
        "/tmp".into(),
        "model".into(),
        xylitol_tui::InteractionMode::ApplicationOwned,
    );
    session.render_now().unwrap();
    let root = session.ui_root().expect("ui root");
    root.borrow_mut().arm_copy_notice();
    assert_eq!(root.borrow().copy_notice_body_for_test(), Some("Copied"));
    assert!(root.borrow().chrome_toast_body().is_none());
    let frame = root.borrow_mut().render(80).join("\n");
    assert!(
        frame.contains("Copied"),
        "ApplicationOwned copy cue must paint in chrome: {frame}"
    );
    assert!(
        !frame.contains("Error: Copied"),
        "must not use Error: toast shape: {frame}"
    );
}

// ── c1760 activity-fold (att23–att28) ─────────────────────────────────

fn activity_turn(user: &str, tool_id: &str, path: &str, asst: &str) -> Vec<super::bridge::UiEntry> {
    vec![
        super::bridge::UiEntry::User { text: user.into() },
        super::bridge::UiEntry::Tool {
            timeout_secs: None,
            id: tool_id.into(),
            name: "read".into(),
            args_preview: path.into(),
            tool_path: Some(path.into()),
            write_content: None,
            display_diff: None,
            output: "ok".into(),
            is_error: false,
            done: true,
        },
        super::bridge::UiEntry::Assistant { text: asst.into() },
    ]
}

fn strip_ansi_activity(s: &str) -> String {
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

#[test]
fn activity_fold_att23_l2_shows_summary_keeps_user_asst_scrollnotice() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    for i in 0..3 {
        model.entries.extend(activity_turn(
            &format!("u{i}"),
            &format!("t{i}"),
            &format!("f{i}.rs"),
            &format!("a{i}"),
        ));
    }
    model.entries.push(super::bridge::UiEntry::ScrollNotice {
        text: "nav-note".into(),
    });
    root.apply_ui_model(&model);
    // keep_recent=2 → oldest seg-0 auto-crush via force (simulate rebuild crush)
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(plain.contains("u0"), "user must stay: {plain}");
    assert!(plain.contains("a0"), "final assistant must stay: {plain}");
    assert!(
        plain.contains("Explored") || plain.contains("file"),
        "L2 summary expected: {plain}"
    );
    assert!(
        !plain.contains("body-hidden-marker") && !plain.contains("f0.rs\nok"),
        "collapsed middle detail should not fully paint; got {plain}"
    );
    // tool args_preview may still appear in summary counts path — ensure tool body "ok" after f0 not as rail block:
    // stronger: ScrollNotice never absorbed
    assert!(
        plain.contains("nav-note"),
        "ScrollNotice must stay: {plain}"
    );
}

#[test]
fn activity_fold_att24_worked_for_and_no_fake_duration_or_pm() {
    use super::activity_fold::{SegmentClock, SegmentLevel};
    use super::layout::UiRoot;
    use time::macros::datetime;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .extend(activity_turn("u", "t1", "a.rs", "done"));
    // second turn with reliable diff stats
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u2".into() });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "edit1".into(),
        name: "edit".into(),
        args_preview: "b.rs".into(),
        tool_path: Some("b.rs".into()),
        write_content: None,
        display_diff: Some("+new\n-old\n".into()),
        output: String::new(),
        is_error: false,
        done: true,
    });
    model
        .entries
        .push(super::bridge::UiEntry::Assistant { text: "a2".into() });
    root.apply_ui_model(&model);

    root.activity_mut().force_level("seg-0", SegmentLevel::L3);
    root.touch_activity();
    // no clock → must not invent a duration number
    let plain_no = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain_no.contains("Worked for"),
        "L3 row without stamps still paints Worked for: {plain_no}"
    );
    assert!(
        !plain_no.contains("Worked for 0")
            && !plain_no.contains("Worked for 1")
            && !plain_no.contains("Worked for 2"),
        "must not fake duration: {plain_no}"
    );

    root.activity_mut().set_clock(
        "seg-0",
        SegmentClock {
            start: Some(datetime!(2026-01-01 0:00 UTC)),
            end: Some(datetime!(2026-01-01 0:02:03 UTC)),
        },
    );
    root.touch_activity();
    let plain_yes = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain_yes.contains("Worked for 2m 3s"),
        "reliable stamps → duration: {plain_yes}"
    );

    root.activity_mut().force_level("seg-3", SegmentLevel::L2);
    root.touch_activity();
    let plain_l2 = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain_l2.contains("+1 -1") || plain_l2.contains("+1") && plain_l2.contains("-1"),
        "reliable diff pm: {plain_l2}"
    );
}

#[test]
fn activity_fold_att25_l2_ignores_alt_e_then_l0_restores_l1() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use xylitol_tui::InputEvent;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .extend(activity_turn("u", "only", "x.rs", "asst"));
    // force tools default open so body visible at L0
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let before = strip_ansi_activity(&root.render(100).join("\n"));
    root.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Char('e'),
        KeyModifiers::ALT,
    )));
    let after_alt = strip_ansi_activity(&root.render(100).join("\n"));
    assert_eq!(
        before
            .lines()
            .filter(|l| l.contains("Explored") || l.contains("Worked"))
            .collect::<Vec<_>>(),
        after_alt
            .lines()
            .filter(|l| l.contains("Explored") || l.contains("Worked"))
            .collect::<Vec<_>>(),
        "Alt+E must not change L2 appearance"
    );

    root.activity_mut().toggle_cluster("seg-0:c0");
    root.touch_activity();
    // Alt+E may have flipped tools default closed — re-open for L1 visibility check.
    if !root.fold().tools_effective("only") {
        root.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Char('e'),
            KeyModifiers::ALT,
        )));
    }
    let opened = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        opened.contains("x.rs") || opened.contains("read"),
        "opening the cluster must show the tool block: {opened}"
    );
    use super::widgets::FoldTarget;
    root.toggle_fold_target(FoldTarget::Tool("only".into()));
    assert!(
        !root.fold().tools_effective("only"),
        "after cluster expand, per-block L1 override must work again"
    );
}

#[test]
fn activity_fold_att26_auto_degrade_keeps_recent_and_streaming() {
    use super::activity_fold::{AutoTrigger, SegmentLevel};
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    // 4 activity turns; rebuild crushes all ended turns to envelope L3.
    for i in 0..4 {
        model.entries.extend(activity_turn(
            &format!("u{i}"),
            &format!("t{i}"),
            &format!("f{i}.rs"),
            &format!("a{i}"),
        ));
    }
    root.apply_ui_model(&model);
    let entries = root.activity().settings.enabled;
    assert!(entries);
    assert!(
        root.activity_mut()
            .auto_degrade(&model.entries, AutoTrigger::Rebuild, false)
    );
    assert_eq!(root.activity().level_of("seg-0"), SegmentLevel::L3);
    assert_eq!(root.activity().level_of("seg-3"), SegmentLevel::L3);
    assert_eq!(root.activity().level_of("seg-6"), SegmentLevel::L3);
    assert_eq!(root.activity().level_of("seg-9"), SegmentLevel::L3);

    // Streaming protect: newest stays L0 even if rebuild would crush all ended turns.
    let mut root2 = UiRoot::new();
    root2.apply_ui_model(&model);
    root2
        .activity_mut()
        .auto_degrade(&model.entries, AutoTrigger::TurnEnd, true);
    assert_eq!(
        root2.activity().level_of("seg-9"),
        SegmentLevel::L0,
        "streaming current turn must not auto-crush"
    );
}

#[test]
fn activity_fold_att27_markers_and_full_chord_hints() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;
    use super::widgets::GlyphSet;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.entries.extend(activity_turn("u", "t", "a.rs", "a"));
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    let fold_mark = GlyphSet::from_env().fold();
    let unfold_mark = GlyphSet::from_env().unfold();
    assert!(
        plain.contains("Worked for"),
        "L2 envelope header must stay: {plain}"
    );
    assert!(
        plain.contains(unfold_mark),
        "expanded envelope must use unfold marker: {plain}"
    );
    assert!(
        plain.contains(fold_mark),
        "L2 cluster heads must use fold marker: {plain}"
    );
    assert!(
        plain.contains("(Alt+Shift+E)"),
        "expand hint must be full chord: {plain}"
    );
    assert!(
        !plain.contains("(Alt+E)"),
        "must not confuse with L1 Alt+E: {plain}"
    );

    root.activity_mut().force_level("seg-0", SegmentLevel::L3);
    root.touch_activity();
    let plain3 = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(plain3.contains("(Alt+Shift+E)"), "L3 expand hint: {plain3}");
    assert!(!plain3.contains("(Alt+E)"), "no Alt+E on L3: {plain3}");
}

#[test]
fn activity_fold_att28_expand_collapse_nearest_and_silent() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use xylitol_tui::InputEvent;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    for i in 0..3 {
        model.entries.extend(activity_turn(
            &format!("u{i}"),
            &format!("t{i}"),
            &format!("f{i}.rs"),
            &format!("a{i}"),
        ));
    }
    root.apply_ui_model(&model);
    // rebuild crushes every ended turn; expandNearest opens the newest envelope
    root.activity_mut().auto_degrade(
        &model.entries,
        super::activity_fold::AutoTrigger::Rebuild,
        false,
    );
    assert_eq!(root.activity().level_of("seg-0"), SegmentLevel::L3);
    assert_eq!(root.activity().level_of("seg-6"), SegmentLevel::L3);

    assert!(root.expand_nearest_activity());
    assert_eq!(root.activity().level_of("seg-6"), SegmentLevel::L2);
    assert_eq!(root.activity().level_of("seg-0"), SegmentLevel::L3);

    // Collapse nearest eligible: entered L2 → floor L3
    assert!(root.collapse_nearest_activity());
    assert_eq!(root.activity().level_of("seg-6"), SegmentLevel::L3);

    // Silent expand when no L3 envelope and every cluster already open — not the
    // keep-window default (heads-only). expandNearest opens the nearest cluster.
    let mut quiet_model = UiModel::new();
    for i in 0..2 {
        quiet_model.entries.extend(activity_turn(
            &format!("q{i}"),
            &format!("qt{i}"),
            &format!("q{i}.rs"),
            &format!("qa{i}"),
        ));
    }
    let mut quiet = UiRoot::new();
    quiet.apply_ui_model(&quiet_model);
    assert!(
        quiet.expand_nearest_activity(),
        "keep-window heads-only → expandNearest opens a cluster"
    );
    assert_eq!(quiet.activity().level_of("seg-0"), SegmentLevel::L0);
    assert_eq!(quiet.activity().level_of("seg-3"), SegmentLevel::L0);
    assert!(
        quiet.activity().cluster_is_expanded("seg-3", "seg-3:c0"),
        "expandNearest on keep-window opens a cluster without inventing an envelope"
    );

    let mut untouched = UiRoot::new();
    untouched.apply_ui_model(&quiet_model);
    untouched.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Char('e'),
        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
    )));
    assert_eq!(
        untouched.activity().level_of("seg-0"),
        SegmentLevel::L0,
        "virgin recent window → collapseNearest silent"
    );
    assert_eq!(untouched.activity().level_of("seg-3"), SegmentLevel::L0);
}

#[test]
fn activity_fold_att25_level_switch_local_paint_misses() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    for i in 0..6 {
        model.entries.push(super::bridge::UiEntry::Assistant {
            text: format!("history-{i}\n\nparagraph"),
        });
    }
    model
        .entries
        .extend(activity_turn("u", "only", "x.rs", "tail"));
    root.apply_ui_model(&model);
    let _ = root.render(80);
    root.clear_scrollback_entry_misses_for_test();

    root.activity_mut().force_level("seg-6", SegmentLevel::L2);
    root.touch_activity();
    let _ = root.render(80);
    let misses = root.scrollback_entry_misses_for_test();
    assert!(
        misses <= 3,
        "segment level switch must not re-Markdown all assistants; misses={misses}"
    );
}

#[test]
fn activity_fold_att31_expand_header_then_collapse() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .extend(activity_turn("u", "only", "x.rs", "asst"));
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let _ = root.render(100);
    root.toggle_fold_target(FoldTarget::Cluster("seg-0:c0".into()));
    let expanded = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        expanded.contains("x.rs") || expanded.contains("read"),
        "cluster expand must reveal L1: {expanded}"
    );
    root.toggle_fold_target(FoldTarget::Cluster("seg-0:c0".into()));
    let collapsed = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        collapsed.contains("Explored") || collapsed.contains("file"),
        "header remains after collapse: {collapsed}"
    );
    assert!(
        !collapsed.contains("x.rs\nok"),
        "collapsed cluster must hide tool body: {collapsed}"
    );
}

#[test]
fn activity_fold_envelope_header_stays_when_expanded() {
    // att23 / att31: expand envelope → ▾ Worked for stays; click again folds back.
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;
    use super::widgets::{FoldTarget, GlyphSet};

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .extend(activity_turn("u", "only", "x.rs", "asst"));
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L3);
    root.touch_activity();
    let folded = strip_ansi_activity(&root.render(100).join("\n"));
    let fold_mark = GlyphSet::from_env().fold();
    let unfold_mark = GlyphSet::from_env().unfold();
    assert!(folded.contains("Worked for"), "L3 envelope: {folded}");
    assert!(folded.contains(fold_mark), "L3 fold marker: {folded}");
    assert!(
        !folded.contains("Explored") && !folded.contains("x.rs"),
        "L3 must hide cluster heads: {folded}"
    );

    root.toggle_fold_target(FoldTarget::Segment("seg-0".into()));
    assert_eq!(root.activity().level_of("seg-0"), SegmentLevel::L2);
    let opened = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        opened.contains("Worked for"),
        "expanded envelope MUST keep Worked for header: {opened}"
    );
    assert!(
        opened.contains(unfold_mark),
        "expanded envelope MUST use ▾: {opened}"
    );
    assert!(
        opened.contains("Explored") || opened.contains("file"),
        "L2 must show cluster heads: {opened}"
    );
    assert!(
        root.fold_hits()
            .regions
            .iter()
            .any(|r| matches!(&r.target, FoldTarget::Segment(id) if id == "seg-0")),
        "expanded envelope MUST keep Segment hit"
    );

    root.toggle_fold_target(FoldTarget::Segment("seg-0".into()));
    assert_eq!(root.activity().level_of("seg-0"), SegmentLevel::L3);
    let refolded = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(refolded.contains("Worked for"), "re-fold: {refolded}");
    assert!(
        !refolded.contains("Explored"),
        "re-folded envelope hides cluster heads: {refolded}"
    );
}

#[test]
fn activity_fold_nested_mouse_envelope_vs_cluster() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "c0".into(),
        name: "read".into(),
        args_preview: "a.rs".into(),
        tool_path: Some("a.rs".into()),
        write_content: None,
        display_diff: None,
        output: "ok0".into(),
        is_error: false,
        done: true,
    });
    model
        .entries
        .push(super::bridge::UiEntry::Assistant { text: "mid".into() });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "c1".into(),
        name: "read".into(),
        args_preview: "b.rs".into(),
        tool_path: Some("b.rs".into()),
        write_content: None,
        display_diff: None,
        output: "ok1".into(),
        is_error: false,
        done: true,
    });
    model.entries.push(super::bridge::UiEntry::Assistant {
        text: "last".into(),
    });
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let _ = root.render(100);
    assert!(
        root.fold_hits()
            .regions
            .iter()
            .any(|reg| matches!(&reg.target, FoldTarget::Cluster(id) if id == "seg-0:c0"))
    );
    assert!(
        root.fold_hits()
            .regions
            .iter()
            .any(|reg| matches!(&reg.target, FoldTarget::Cluster(id) if id == "seg-0:c1"))
    );
    root.toggle_fold_target(FoldTarget::Cluster("seg-0:c0".into()));
    let _ = root.render(100);
    assert_eq!(root.activity().level_of("seg-0"), SegmentLevel::L2);
    assert!(
        root.fold_hits()
            .regions
            .iter()
            .any(|r| matches!(&r.target, FoldTarget::Tool(id) if id == "c0")),
        "opening cluster 0 must reveal its L1"
    );
    assert!(
        !root
            .fold_hits()
            .regions
            .iter()
            .any(|r| matches!(&r.target, FoldTarget::Tool(id) if id == "c1")),
        "cluster 1 must stay collapsed"
    );
}

#[test]
fn activity_fold_att26_rebuild_paints_worked_for() {
    use super::activity_fold::{AutoTrigger, SegmentLevel};
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    for i in 0..4 {
        model.entries.extend(activity_turn(
            &format!("u{i}"),
            &format!("t{i}"),
            &format!("f{i}.rs"),
            &format!("a{i}"),
        ));
    }
    root.apply_ui_model(&model);
    root.activity_mut()
        .auto_degrade(&model.entries, AutoTrigger::Rebuild, false);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain.contains("Worked for"),
        "ended turns must paint envelope: {plain}"
    );
    assert_eq!(root.activity().level_of("seg-0"), SegmentLevel::L3);
    assert_eq!(root.activity().level_of("seg-9"), SegmentLevel::L3);
}

#[test]
fn activity_fold_att33_live_planning_and_open_cluster_updates() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.phase = UiPhase::Busy;
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    root.apply_ui_model(&model);
    let empty = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        !empty.contains("Planning next moves"),
        "busy chrome is status, not Planning: {empty}"
    );
    assert!(
        !empty.contains("Worked for"),
        "live window must not wrap current turn: {empty}"
    );

    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "sealed".into(),
        name: "read".into(),
        args_preview: "old.rs".into(),
        tool_path: Some("old.rs".into()),
        write_content: None,
        display_diff: None,
        output: "ok".into(),
        is_error: false,
        done: true,
    });
    model
        .entries
        .push(super::bridge::UiEntry::Assistant { text: "mid".into() });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "open1".into(),
        name: "edit".into(),
        args_preview: "a.rs".into(),
        tool_path: Some("a.rs".into()),
        write_content: None,
        display_diff: Some("+a\n-b\n".into()),
        output: String::new(),
        is_error: false,
        done: true,
    });
    root.apply_ui_model(&model);
    let before = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        !before.contains("Planning next moves"),
        "open cluster still unsealed → no Planning placeholder: {before}"
    );
    assert!(
        before.contains("Explored") || before.contains("Editing"),
        "cluster headers present: {before}"
    );
    let frozen = before
        .lines()
        .find(|l| l.contains("Explored"))
        .unwrap_or("")
        .to_string();

    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "open2".into(),
        name: "edit".into(),
        args_preview: "b.rs".into(),
        tool_path: Some("b.rs".into()),
        write_content: None,
        display_diff: Some("+x\n".into()),
        output: String::new(),
        is_error: false,
        done: true,
    });
    root.apply_ui_model(&model);
    let after = strip_ansi_activity(&root.render(100).join("\n"));
    let frozen_after = after
        .lines()
        .find(|l| l.contains("Explored"))
        .unwrap_or("")
        .to_string();
    assert_eq!(
        frozen, frozen_after,
        "sealed -3 header must not change when open cluster updates"
    );
    assert!(
        after.contains("Editing 2 files") || after.contains("+2") || after.contains("+1"),
        "open cluster -2 must update counts: {after}"
    );
}

#[test]
fn activity_fold_att33_ask_waiting_stays_clickable() {
    use super::bridge::AskPhase;
    use super::widgets::FoldTarget;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    let mut session = HostSession::new_product_ui(TestTerminal::new(100, 32));
    let root = session.ui_root().expect("product ui").clone();
    {
        let mut model = session.ui_model().clone();
        model.phase = UiPhase::Busy;
        model
            .entries
            .push(super::bridge::UiEntry::User { text: "u".into() });
        model.entries.push(super::bridge::UiEntry::Ask {
            id: "ask-live".into(),
            summary: "Ask · choose".into(),
            detail_lines: vec!["ask-live-detail".into()],
            phase: AskPhase::Waiting,
            expanded: true,
        });
        *session.ui_model_mut() = model;
        session.sync_ui_root_from_model();
    }
    session.step(HostEvent::Tick).unwrap();
    session.tui.request_render(true);
    session.step_paint_only().unwrap();

    let plain = strip_ansi_activity(&root.borrow_mut().render(100).join("\n"));
    assert!(
        plain.contains("Asking questions"),
        "Ask waiting must label the live tail: {plain}"
    );
    let region = root
        .borrow()
        .fold_hits()
        .regions
        .iter()
        .find(|reg| matches!(&reg.target, FoldTarget::Ask(id) if id == "ask-live"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "Ask Waiting must stay hittable; regions={:?}",
                root.borrow().fold_hits().regions
            )
        });
    let screen_row = region
        .content_row
        .saturating_sub(root.borrow().fold_hits().scroll_top) as u16;
    session
        .step(HostEvent::Input(InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: region.col_start as u16,
            row: screen_row,
            modifiers: KeyModifiers::NONE,
        })))
        .unwrap();
    assert!(
        !root.borrow().fold().tools_effective("ask-live"),
        "Ask Waiting triangle must remain interactive"
    );
}

#[test]
fn activity_fold_att33_cluster_click_expands_open_cluster() {
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.phase = UiPhase::Busy;
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "sealed-tool".into(),
        name: "read".into(),
        args_preview: "z.rs".into(),
        tool_path: Some("z.rs".into()),
        write_content: None,
        display_diff: None,
        output: "ok".into(),
        is_error: false,
        done: true,
    });
    root.apply_ui_model(&model);
    let _ = root.render(100);
    assert!(
        !root
            .fold_hits()
            .regions
            .iter()
            .any(|r| matches!(&r.target, FoldTarget::Tool(id) if id == "sealed-tool")),
        "live cluster kids hidden until cluster triangle click"
    );
    assert!(
        root.fold_hits()
            .regions
            .iter()
            .any(|reg| matches!(reg.target, FoldTarget::Cluster(_))),
        "Exploring cluster header must register a fold triangle"
    );
    root.toggle_fold_target(FoldTarget::Cluster("seg-0:c0".into()));
    let _ = root.render(100);
    assert!(
        root.fold_hits()
            .regions
            .iter()
            .any(|r| matches!(&r.target, FoldTarget::Tool(id) if id == "sealed-tool")),
        "cluster triangle must reveal sealed kids"
    );
}

#[test]
fn activity_fold_att33_live_window_tape_reproduces_stream() {
    use super::activity_fold::{replay_live_window, strip_ansi_live_window};
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    let report = replay_live_window(&mut model, |m| {
        root.apply_ui_model(m);
        strip_ansi_live_window(&root.render(100).join("\n"))
    });
    assert!(
        report.ok,
        "live-window tape failed:\n{}",
        report.lines.join("\n")
    );
}

#[test]
fn activity_fold_default_hides_cluster_kids_and_uses_edited() {
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "e1".into(),
        name: "edit".into(),
        args_preview: "a.rs".into(),
        tool_path: Some("a.rs".into()),
        write_content: None,
        display_diff: None,
        output: "ok-a".into(),
        is_error: false,
        done: true,
    });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "e2".into(),
        name: "edit".into(),
        args_preview: "b.rs".into(),
        tool_path: Some("b.rs".into()),
        write_content: None,
        display_diff: None,
        output: "ok-b".into(),
        is_error: false,
        done: true,
    });
    model.entries.push(super::bridge::UiEntry::Assistant {
        text: "done".into(),
    });
    root.apply_ui_model(&model);
    let collapsed = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        collapsed.contains("Edited 2 files"),
        "sealed edit cluster must say Edited: {collapsed}"
    );
    assert!(
        !collapsed.contains("Worked for"),
        "default L0 keep-window must not paint envelope: {collapsed}"
    );
    assert!(
        !collapsed.contains("Explored 2 files"),
        "must not label an edit cluster Explored: {collapsed}"
    );
    assert!(
        !collapsed.contains("ok-a") && !collapsed.contains("a.rs"),
        "default cluster kids stay hidden: {collapsed}"
    );

    root.toggle_fold_target(FoldTarget::Cluster("seg-0:c0".into()));
    let expanded = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        expanded.contains("a.rs") && expanded.contains("b.rs"),
        "opening the cluster reveals same-column tool rows: {expanded}"
    );
    assert!(
        !expanded.contains("Worked for"),
        "opening a keep-window cluster must not invent Worked for: {expanded}"
    );
    assert_eq!(
        root.activity().level_of("seg-0"),
        super::activity_fold::SegmentLevel::L0
    );
}

#[test]
fn activity_fold_thinking_only_is_thought_not_explored() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "hi".into() });
    model.entries.push(super::bridge::UiEntry::Thinking {
        id: "th".into(),
        text: "consider".into(),
        elapsed_secs: None,
    });
    model.entries.push(super::bridge::UiEntry::Assistant {
        text: "hello".into(),
    });
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(plain.contains("Thought"), "thinking-only header: {plain}");
    assert!(
        !plain.contains("Explored"),
        "must not invent Explored: {plain}"
    );
    assert!(
        !plain.contains("consider"),
        "Thought cluster kids stay folded by default: {plain}"
    );
    assert!(
        !plain
            .lines()
            .any(|l| l.contains("thinking") && l.contains("Ctrl+T")),
        "must not paint a second thinking L1 header: {plain}"
    );

    root.toggle_fold_target(super::widgets::FoldTarget::Cluster("seg-0:c0".into()));
    let opened = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        opened.contains("consider"),
        "opening Thought reveals the body: {opened}"
    );
    assert!(
        !opened
            .lines()
            .any(|l| l.contains("thinking") && l.contains("Ctrl+T")),
        "expanded Thought still has no thinking L1 header: {opened}"
    );
}

#[test]
fn activity_fold_thought_only_cluster_shows_frozen_duration() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "hi".into() });
    model.entries.push(super::bridge::UiEntry::Thinking {
        id: "th".into(),
        text: "consider".into(),
        elapsed_secs: Some(17),
    });
    model.entries.push(super::bridge::UiEntry::Assistant {
        text: "hello".into(),
    });
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain.contains("Thought 17s"),
        "thought-only cluster MUST show frozen duration: {plain}"
    );
    assert!(
        !plain
            .lines()
            .any(|l| l.contains("Thought 17s") && l.contains("Ctrl+T")),
        "thought-only cluster MUST NOT paint L1 Ctrl+T: {plain}"
    );
}

#[test]
fn activity_fold_sealed_thought_stays_when_later_thinking_streams() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.phase = UiPhase::Busy;
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "hi".into() });
    model.entries.push(super::bridge::UiEntry::Thinking {
        id: "th-old".into(),
        text: "first burst".into(),
        elapsed_secs: Some(17),
    });
    model
        .entries
        .push(super::bridge::UiEntry::Assistant { text: "mid".into() });
    apply_xy_event(&mut model, &XyEvent::ThinkingDelta("second burst".into()));
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain.contains("Thought 17s"),
        "sealed Thought MUST NOT flip back to Thinking: {plain}"
    );
    assert!(
        plain.contains("Thinking"),
        "live burst MUST still be Thinking: {plain}"
    );
}

#[test]
fn activity_fold_thinking_text_without_id_does_not_flip_headers() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.phase = UiPhase::Busy;
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "hi".into() });
    model.entries.push(super::bridge::UiEntry::Thinking {
        id: "th-old".into(),
        text: "first burst".into(),
        elapsed_secs: Some(17),
    });
    model
        .entries
        .push(super::bridge::UiEntry::Assistant { text: "mid".into() });
    model.streaming_thinking = "orphan buffer".into();
    assert!(model.streaming_think_id.is_none());
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain.contains("Thought 17s"),
        "buffer without live id MUST NOT flip sealed Thought: {plain}"
    );
    let thinking_headers = plain
        .lines()
        .filter(|l| l.contains("Thinking") && !l.contains("Thought"))
        .count();
    assert_eq!(
        thinking_headers, 0,
        "orphan streaming_thinking MUST NOT paint a Thinking header: {plain}"
    );
}

#[test]
fn activity_fold_todo_tools_are_used_not_thought_cluster() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "t-list".into(),
        name: "todo_list".into(),
        args_preview: String::new(),
        tool_path: None,
        write_content: None,
        display_diff: None,
        output: r#"{"items":[]}"#.into(),
        is_error: false,
        done: true,
    });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "t-up".into(),
        name: "todo_update".into(),
        args_preview: String::new(),
        tool_path: None,
        write_content: None,
        display_diff: None,
        output: r#"{"items":[]}"#.into(),
        is_error: false,
        done: true,
    });
    model.entries.push(super::bridge::UiEntry::Thinking {
        id: "th".into(),
        text: "Good progress. Now I need to use ask".into(),
        elapsed_secs: Some(4),
    });
    model.entries.push(super::bridge::UiEntry::Assistant {
        text: "继续，用 `ask` 问用户一个选择".into(),
    });
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain.contains("Used 2 tools"),
        "two distinct todo_* calls MUST count as Used 2 tools: {plain}"
    );
    assert!(
        !plain
            .lines()
            .any(|l| l.contains("Thought") && l.contains("Alt+Shift+E")),
        "MUST NOT use Thought as the aggregate cluster header: {plain}"
    );
    root.toggle_fold_target(super::widgets::FoldTarget::Cluster("seg-0:c0".into()));
    let opened = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        opened.contains("Thought 4s") && opened.contains("(Ctrl+T)"),
        "L1 thinking kid MUST keep frozen duration: {opened}"
    );
}

#[test]
fn activity_fold_repeated_unknown_tools_count_calls() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    for i in 0..4 {
        model.entries.push(super::bridge::UiEntry::Tool {
            timeout_secs: None,
            id: format!("t-{i}"),
            name: "todo_update".into(),
            args_preview: String::new(),
            tool_path: None,
            write_content: None,
            display_diff: None,
            output: "{}".into(),
            is_error: false,
            done: true,
        });
    }
    model.entries.push(super::bridge::UiEntry::Todo {
        summary: "Todo · 10/10".into(),
        detail_lines: vec![],
    });
    model.entries.push(super::bridge::UiEntry::Thinking {
        id: "th".into(),
        text: "done".into(),
        elapsed_secs: Some(10),
    });
    model
        .entries
        .push(super::bridge::UiEntry::Assistant { text: "ok".into() });
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain.contains("Used 4 tools"),
        "four todo_update calls MUST be Used 4 tools, not unique-name 1 or checklist+name 2: {plain}"
    );
    assert!(
        !plain.contains("Used 2 tools"),
        "checklist row MUST NOT inflate Used N: {plain}"
    );
}

#[test]
fn activity_fold_compaction_only_has_no_explored_header() {
    use super::activity_fold::SegmentLevel;
    use super::bridge::CompactionBlockStatus;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.entries.push(super::bridge::UiEntry::User {
        text: "cool 总结下".into(),
    });
    model.entries.push(super::bridge::UiEntry::Compaction {
        status: CompactionBlockStatus::Complete,
        summary: "sum".into(),
        tokens_before: 101_494,
        detail: None,
    });
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        !plain.contains("Explored"),
        "compaction-only must not invent Explored: {plain}"
    );
    assert!(
        plain.contains("101") || plain.contains("compaction") || plain.contains("Compacted"),
        "compaction block must still paint: {plain}"
    );
}

#[test]
fn activity_fold_live_write_placeholder_is_editing_not_dots() {
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.phase = UiPhase::Busy;
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "w1".into(),
        name: "write".into(),
        args_preview: "...".into(),
        tool_path: None,
        write_content: Some("fn demo() {}".into()),
        display_diff: None,
        output: String::new(),
        is_error: false,
        done: false,
    });
    root.apply_ui_model(&model);
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain.contains("Editing"),
        "inflight write must still say Editing: {plain}"
    );
    assert!(
        !plain.contains("Editing ...") && !plain.contains("Editing…"),
        "must not treat path placeholder as a filename: {plain}"
    );
    assert!(
        !plain.contains("fn demo"),
        "streaming write body stays folded by default: {plain}"
    );
    assert!(
        !plain.contains("Planning next moves"),
        "MUST NOT paint Planning placeholder: {plain}"
    );
    assert!(
        root.fold_hits()
            .regions
            .iter()
            .any(|r| matches!(r.target, FoldTarget::Cluster(_))),
        "Editing cluster header must register a fold triangle: {:?}",
        root.fold_hits().regions
    );

    root.toggle_fold_target(FoldTarget::Cluster("seg-0:c0".into()));
    let opened = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        opened.contains("fn demo") && opened.contains("Write"),
        "opening the cluster reveals the streaming write: {opened}"
    );

    if let super::bridge::UiEntry::Tool { done, .. } = &mut model.entries[1] {
        *done = true;
    }
    root.apply_ui_model(&model);
    let after_end = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        after_end.contains("fn demo") && after_end.contains("Write"),
        "ToolEnd on an opened cluster must not auto-collapse kids: {after_end}"
    );
}

#[test]
fn activity_fold_live_thinking_stream_merges_into_thought() {
    use super::layout::UiRoot;
    use super::widgets::FoldTarget;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.phase = UiPhase::Busy;
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    apply_xy_event(
        &mut model,
        &XyEvent::ThinkingDelta("consider next edit".into()),
    );
    model
        .thought_clock
        .pin_start_at(std::time::Instant::now() - std::time::Duration::from_secs(17));
    root.apply_ui_model(&model);
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        plain.contains("Thinking"),
        "stream must be a Thinking bar: {plain}"
    );
    assert!(
        !plain.contains("Thought"),
        "MUST NOT freeze Thought duration while still streaming: {plain}"
    );
    assert!(
        !plain.contains("consider next edit"),
        "Thinking body stays folded by default: {plain}"
    );
    assert!(
        !plain
            .lines()
            .any(|l| l.contains("thinking") && l.contains("Ctrl+T")),
        "must not show a second thinking L1 header: {plain}"
    );
    assert!(
        !plain.contains("Planning next moves"),
        "MUST NOT paint Planning placeholder: {plain}"
    );

    root.toggle_fold_target(FoldTarget::Cluster("seg-0:c0".into()));
    let opened = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        opened.contains("consider next edit"),
        "opening Thinking reveals the stream: {opened}"
    );
    assert!(
        opened.contains("Thinking") && !opened.contains("Thought"),
        "expanded stream is still Thinking: {opened}"
    );

    let expect_dur = model
        .thought_clock
        .started_at()
        .expect("stream must stamp start")
        .elapsed()
        .as_secs();
    apply_xy_event(&mut model, &XyEvent::AgentEnd { messages: vec![] });
    root.apply_ui_model(&model);
    let flushed = strip_ansi_activity(&root.render(100).join("\n"));
    let expect = format!("Thought {expect_dur}s");
    assert!(
        flushed.contains(&expect),
        "after stream end the bar MUST become {expect}: {flushed}"
    );
    assert!(
        !flushed.contains("Thinking"),
        "flushed thought-only cluster MUST NOT keep Thinking: {flushed}"
    );
    assert!(
        flushed.contains("consider next edit"),
        "expand survives flush: {flushed}"
    );
}

#[test]
fn activity_fold_text_delta_seals_thought_without_waiting_for_body() {
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model.phase = UiPhase::Busy;
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    apply_xy_event(
        &mut model,
        &XyEvent::ThinkingDelta("consider next edit".into()),
    );
    let start = std::time::Instant::now() - std::time::Duration::from_secs(2);
    model.thought_clock.pin_start_at(start);
    model
        .thought_clock
        .stamp_end_at(start + std::time::Duration::from_secs(2));
    apply_xy_event(&mut model, &XyEvent::TextDelta("answer".into()));
    root.apply_ui_model(&model);
    let mid = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        mid.contains("Thought 2s"),
        "first TextDelta MUST freeze Thought at thinking-channel end: {mid}"
    );
    assert!(
        !mid.contains("Thinking"),
        "body stream MUST NOT keep the Thinking label: {mid}"
    );

    apply_xy_event(&mut model, &XyEvent::TextDelta(" continues".into()));
    apply_xy_event(&mut model, &XyEvent::AgentEnd { messages: vec![] });
    root.apply_ui_model(&model);
    let done = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(
        done.contains("Thought 2s"),
        "body / AgentEnd MUST NOT inflate thought wall-clock: {done}"
    );
}

#[test]
fn activity_fold_mcp_cluster_is_used() {
    use super::activity_fold::SegmentLevel;
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    model
        .entries
        .push(super::bridge::UiEntry::User { text: "u".into() });
    model.entries.push(super::bridge::UiEntry::Tool {
        timeout_secs: None,
        id: "mcp1".into(),
        name: "mcp__lspz__get_symbols".into(),
        args_preview: String::new(),
        tool_path: None,
        write_content: None,
        display_diff: None,
        output: "ok".into(),
        is_error: false,
        done: true,
    });
    model
        .entries
        .push(super::bridge::UiEntry::Assistant { text: "a".into() });
    root.apply_ui_model(&model);
    root.activity_mut().force_level("seg-0", SegmentLevel::L2);
    root.touch_activity();
    let plain = strip_ansi_activity(&root.render(100).join("\n"));
    assert!(plain.contains("Used get_symbols"), "MCP header: {plain}");
    assert!(
        !plain.contains("Explored"),
        "MCP must not be Explored: {plain}"
    );
}

#[test]
fn activity_fold_live_ask_close_idles_and_drops_asking() {
    use super::activity_fold::{
        LIVE_ASK_CLOSE_TEXT, live_ask_close_events, replay_live_window, strip_ansi_live_window,
    };
    use super::layout::UiRoot;

    let mut root = UiRoot::new();
    let mut model = UiModel::new();
    let report = replay_live_window(&mut model, |m| {
        root.apply_ui_model(m);
        strip_ansi_live_window(&root.render(100).join("\n"))
    });
    assert!(report.ok, "tape must pass before close-out");
    for ev in live_ask_close_events(
        r#"{"status":"answered","answers":[{"id":"next","values":["continue"],"labels":["Continue"],"was_custom":false}]}"#,
    ) {
        apply_xy_event(&mut model, &ev);
    }
    root.apply_ui_model(&model);
    root.apply_activity_after_turn_end();
    let plain = strip_ansi_live_window(&root.render(100).join("\n"));
    assert_eq!(model.phase, UiPhase::Idle);
    assert!(
        !plain.contains("Asking questions"),
        "live tail must leave Ask waiting: {plain}"
    );
    assert!(
        plain.contains(LIVE_ASK_CLOSE_TEXT),
        "closing assistant body missing: {plain}"
    );
    assert!(
        plain.contains("Edited 2 files") || plain.contains("Explored old.rs"),
        "ended turn keeps cluster heads: {plain}"
    );
    assert!(
        !plain.contains("等待回答"),
        "Ask must not stay waiting: {plain}"
    );
    assert!(
        !plain.contains("Worked for"),
        "just-ended live turn must stay live-window (no envelope): {plain}"
    );

    root.toggle_fold_target(super::widgets::FoldTarget::Cluster("seg-0:c1".into()));
    let opened = strip_ansi_live_window(&root.render(100).join("\n"));
    assert!(
        opened.contains("a.rs") || opened.contains("Edit"),
        "cluster must actually open: {opened}"
    );
    assert!(
        !opened.contains("Worked for"),
        "opening a cluster on the live turn must not invent Worked for: {opened}"
    );
}

// ── c2200 scene slice: semantic dump over product paint (lessons 1–3) ──

/// Lesson 1: thinking + todo_* → L2 cluster head is `Used 2 tools` (by call
/// count), not `Thought`. Asserted on the product frame via the semantic dump.
#[test]
fn scene_dump_thinking_plus_todo_is_used_not_thought() {
    use super::activity_fold::scene::SceneBuilder;
    use super::activity_fold::{count_cluster, partition_segments};

    let mut b = SceneBuilder::begin();
    b.thinking("plan todos");
    b.tool_start("td1", "todo_update", "");
    b.todo_result(
        "td1",
        "todo_update",
        r#"{"items":[{"id":"1","content":"a","status":"completed"}]}"#,
    );
    b.tool_start("td2", "todo_list", "");
    b.todo_result(
        "td2",
        "todo_list",
        r#"{"items":[{"id":"1","content":"a","status":"completed"}]}"#,
    );
    let entries = b.entries().to_vec();

    let (plain, dump) = b.render(100);
    let text = dump.to_text();
    let l2_used: Vec<_> = dump
        .rows_with_chord("L2 cluster")
        .filter(|r| r.cluster_head.contains("Used"))
        .collect();
    assert!(
        l2_used.iter().any(|r| r.cluster_head.contains("2 tools")),
        "L2 cluster head must be Used 2 tools (lesson 1); dump:\n{text}\nframe:\n{plain}"
    );
    assert!(
        !dump
            .rows_with_chord("L2 cluster")
            .any(|r| r.cluster_head.contains("Thought")),
        "thinking + todo_* must not be Thought (lesson 1); dump:\n{text}\nframe:\n{plain}"
    );
    let segs = partition_segments(&entries);
    let counts = count_cluster(&entries, &segs[0].clusters[0]);
    assert_eq!(counts.used_calls, 2, "two todo_* invocations");
    assert!(
        !counts.is_thought_only(),
        "thinking is a kid, not the header"
    );
}

/// Lesson 2: four same-name unknown tools → `Used 4 tools` counts
/// invocations, not unique names.
#[test]
fn scene_dump_four_same_unknown_tools_count_invocations() {
    use super::activity_fold::scene::SceneBuilder;
    use super::activity_fold::{count_cluster, partition_segments};

    let mut b = SceneBuilder::begin();
    b.thinking("plan calls");
    for i in 0..4 {
        b.tool_start(&format!("u{i}"), "todo_update", "");
        b.tool_end(&format!("u{i}"), "todo_update");
    }
    let entries = b.entries().to_vec();

    let (plain, dump) = b.render(100);
    let text = dump.to_text();
    let l2_used: Vec<_> = dump
        .rows_with_chord("L2 cluster")
        .filter(|r| r.cluster_head.contains("Used"))
        .collect();
    assert!(
        l2_used.iter().any(|r| r.cluster_head.contains("4 tools")),
        "L2 cluster head must be Used 4 tools (lesson 2); dump:\n{text}\nframe:\n{plain}"
    );
    assert!(
        !l2_used.iter().any(|r| r.cluster_head.contains("1 tool")),
        "must not collapse by unique name; dump:\n{text}"
    );
    let segs = partition_segments(&entries);
    let counts = count_cluster(&entries, &segs[0].clusters[0]);
    assert_eq!(counts.used_calls, 4, "invocations, not unique names");
    assert_eq!(counts.used_names.as_slice(), ["todo_update"]);
}

/// Flushed thinking goes through product `ThinkingDelta` + `flush_streaming`,
/// not `entries.push(Thinking)`.
#[test]
fn scene_thinking_flushed_uses_product_flush() {
    use super::activity_fold::scene::SceneBuilder;
    use super::bridge::UiEntry;

    let mut b = SceneBuilder::begin();
    b.thinking_flushed("first burst", 1);
    assert!(
        b.live_think_idle(),
        "flush must clear live think id and buffer"
    );
    match b.entries().last() {
        Some(UiEntry::Thinking {
            text,
            elapsed_secs: Some(1),
            ..
        }) if text == "first burst" => {}
        other => panic!("expected flushed Thinking 1s, got {other:?}"),
    }
}

#[test]
fn scene_live_xy_steps_host_session() {
    use super::activity_fold::scene::SceneBuilder;
    use super::activity_fold::strip_ansi_live_window;

    let mut b = SceneBuilder::begin();
    b.thinking("plan");
    b.tool_start("r1", "read", "a.rs");
    b.tool_end("r1", "read");

    let mut session = HostSession::new_product_ui(TestTerminal::new(100, 32));
    session.on_run_started("scene");
    for ev in b.live_events() {
        session.step(HostEvent::Xy(Box::new(ev.clone()))).unwrap();
    }
    session.step(HostEvent::Tick).unwrap();
    session.render_now().unwrap();
    let joined = strip_ansi_live_window(&session.tui.terminal.frames.concat());
    assert!(
        joined.contains("Explor") || joined.contains("a.rs") || joined.contains("read"),
        "LiveXy HostSession::step must paint explore/read; got:\n{joined}"
    );
}

#[test]
fn chrome_op_toast_is_not_scroll_notice() {
    use super::bridge::UiEntry;
    use crate::app::debug_fixtures::ChromeOp;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.apply_chrome_op(ChromeOp::Toast);
    let body = session
        .ui_root()
        .expect("product ui root")
        .borrow()
        .chrome_toast_body()
        .unwrap_or("")
        .to_string();
    assert!(
        body.contains("toast"),
        "expected chrome toast body; got {body:?}"
    );
    assert!(
        !session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::ScrollNotice { .. })),
        "chrome toast must not be a ScrollNotice"
    );
}

#[test]
fn chrome_op_slot_models_mounts_picker() {
    use super::layout::EditorSlotKind;
    use crate::app::debug_fixtures::ChromeOp;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.apply_chrome_op(ChromeOp::SlotModels);
    assert_eq!(
        session.ui_root().expect("product ui root").borrow().slot(),
        EditorSlotKind::Models
    );
}

#[test]
fn chrome_op_next_turn_cue_is_not_scroll_notice() {
    use super::bridge::UiEntry;
    use crate::app::debug_fixtures::ChromeOp;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.apply_chrome_op(ChromeOp::NextTurnCue);
    let cue = session
        .ui_root()
        .expect("product ui root")
        .borrow()
        .status_next_turn_cue_for_test();
    assert_eq!(cue.as_deref(), Some("Next turn: preview"));
    assert!(
        !session
            .ui_model()
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::ScrollNotice { .. })),
        "next-turn cue must not be a ScrollNotice"
    );
}

#[test]
fn chrome_op_slot_choice_and_tree_mount() {
    use super::layout::EditorSlotKind;
    use crate::app::debug_fixtures::ChromeOp;

    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    session.apply_chrome_op(ChromeOp::SlotChoice);
    assert_eq!(
        session.ui_root().expect("product ui root").borrow().slot(),
        EditorSlotKind::Choice
    );
    session.apply_chrome_op(ChromeOp::SlotTree);
    assert_eq!(
        session.ui_root().expect("product ui root").borrow().slot(),
        EditorSlotKind::Tree
    );
}

/// Lesson 3: a sealed Thought cluster stays `Thought` when a new Thinking
/// stream starts; the new stream paints its own live head. Asserted on the
/// product frame via the semantic dump (both rows must be present).
#[test]
fn scene_dump_sealed_thought_stays_thought_under_new_stream() {
    use super::activity_fold::scene::SceneBuilder;
    use super::activity_fold::{count_cluster, partition_segments};

    let mut b = SceneBuilder::begin();
    b.thinking_flushed("first burst", 1);
    b.assistant("mid");
    b.message_end();
    let entries = b.entries().to_vec();

    let segs = partition_segments(&entries);
    let sealed = &segs[0].clusters[0];
    assert!(
        sealed.seal_assistant_idx.is_some(),
        "cluster must be sealed by the assistant body"
    );
    let counts = count_cluster(&entries, sealed);
    assert!(counts.is_thought_only(), "sealed cluster is thought-only");

    b.live_thinking("second burst");
    let (plain, dump) = b.render(100);
    let text = dump.to_text();
    assert!(
        dump.rows_with_chord("L2 cluster")
            .any(|r| r.cluster_head.contains("Thought 1s")),
        "sealed cluster head must stay Thought 1s (lesson 3); dump:\n{text}\nframe:\n{plain}"
    );
    assert!(
        dump.rows_with_chord("L2 cluster")
            .any(|r| r.cluster_head == "Thinking"),
        "new stream must paint its own Thinking head; dump:\n{text}\nframe:\n{plain}"
    );
    assert!(
        plain.contains("Thought 1s") && plain.contains("Thinking"),
        "frame band: sealed Thought row and live Thinking row coexist; frame:\n{plain}"
    );
}

/// The semantic dump must cover the same product paint path
/// (`render_scrollback` under UiRoot), not a test-only paint.
#[test]
fn scene_dump_covers_product_render_scrollback() {
    use super::activity_fold::scene::SceneBuilder;
    use super::activity_fold::{ActivityFoldState, SemanticDump, strip_ansi_live_window};
    use super::layout::LayoutTheme;
    use super::widgets::{
        FoldHitTable, GlyphSet, ScrollbackFold, ScrollbackPaintCache, render_scrollback,
    };

    let mut b = SceneBuilder::begin();
    b.thinking("plan");
    b.tool_start("r1", "read", "a.rs");
    b.tool_end("r1", "read");
    b.assistant("done");
    b.message_end();
    let entries = b.entries().to_vec();

    let mut model = UiModel::new();
    model.entries = entries.clone();
    let mut activity = ActivityFoldState::default();
    let frame = render_scrollback(
        &model,
        GlyphSet::from_env(),
        LayoutTheme::product_dark(),
        &ScrollbackFold::default(),
        &mut activity,
        100,
        &mut ScrollbackPaintCache::default(),
        &mut FoldHitTable::default(),
    );
    let plain = strip_ansi_live_window(&frame.join("\n"));
    assert!(plain.contains("Explored a.rs"), "frame:\n{plain}");

    let dump = SemanticDump::from_product_frame(&plain, &entries);
    assert!(
        dump.rows_with_chord("L2 cluster")
            .any(|r| r.cluster_head.contains("Explored a.rs")),
        "dump must anchor the sealed read cluster; dump:\n{}",
        dump.to_text()
    );
}
