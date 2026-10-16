//! Overlay focus-restore (c575 / D08) — host-driven, no real TTY.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::{Arc, Mutex};
use xylitol_tui::{Component, FocusTarget, InputEvent, OverlayOptions, OverlayUnfocusOptions, TUI};

mod support;
use support::VirtualTerminal;

struct Recorder {
    label: String,
    log: Arc<Mutex<Vec<char>>>,
}

impl Recorder {
    fn new(label: &str) -> (Self, Arc<Mutex<Vec<char>>>) {
        let log = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                label: label.into(),
                log: Arc::clone(&log),
            },
            log,
        )
    }
}

impl Component for Recorder {
    fn render(&mut self, _width: usize) -> Vec<String> {
        vec![self.label.clone()]
    }

    fn handle_input(&mut self, event: InputEvent) {
        if let InputEvent::Key(k) = event
            && let KeyCode::Char(c) = k.code
        {
            self.log.lock().expect("log").push(c);
        }
    }

    fn invalidate(&mut self) {}
}

fn key(c: char) -> InputEvent {
    InputEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
}

fn tui_with_editor() -> (TUI<VirtualTerminal>, Arc<Mutex<Vec<char>>>) {
    let term = VirtualTerminal::new(40, 10);
    let mut tui = TUI::new(term);
    let (editor, log) = Recorder::new("EDITOR");
    tui.add_child(Box::new(editor));
    tui.set_focus(Some(0));
    (tui, log)
}

#[test]
fn f1_capturing_hide_restores_pre_focus() {
    let (mut tui, editor_log) = tui_with_editor();
    let (overlay, overlay_log) = Recorder::new("OVL");
    let handle = tui.show_overlay(Box::new(overlay), OverlayOptions::default());
    assert!(handle.is_focused(&tui));
    tui.dispatch_event(key('x'));
    assert_eq!(*overlay_log.lock().unwrap(), vec!['x']);
    handle.hide(&mut tui);
    assert!(!handle.is_focused(&tui));
    tui.dispatch_event(key('e'));
    assert_eq!(*editor_log.lock().unwrap(), vec!['e']);
}

#[test]
fn f2_set_focus_steal_reclaims_on_dispatch() {
    let (mut tui, editor_log) = tui_with_editor();
    let (overlay, overlay_log) = Recorder::new("OVL");
    let handle = tui.show_overlay(Box::new(overlay), OverlayOptions::default());
    assert!(handle.is_focused(&tui));
    // Temporary steal to root — next input must reclaim overlay.
    tui.set_focus(Some(0));
    assert!(!handle.is_focused(&tui));
    tui.dispatch_event(key('r'));
    assert!(handle.is_focused(&tui));
    assert_eq!(*overlay_log.lock().unwrap(), vec!['r']);
    assert!(editor_log.lock().unwrap().is_empty());
}

#[test]
fn f3_unfocus_does_not_reclaim() {
    let (mut tui, editor_log) = tui_with_editor();
    let (overlay, overlay_log) = Recorder::new("OVL");
    let handle = tui.show_overlay(Box::new(overlay), OverlayOptions::default());
    handle.unfocus(&mut tui, None);
    assert!(!handle.is_focused(&tui));
    tui.dispatch_event(key('e'));
    assert_eq!(*editor_log.lock().unwrap(), vec!['e']);
    assert!(overlay_log.lock().unwrap().is_empty());
    assert!(!handle.is_focused(&tui));
}

#[test]
fn f4_set_focus_none_clears_restore() {
    let (mut tui, _editor_log) = tui_with_editor();
    let (overlay, overlay_log) = Recorder::new("OVL");
    let handle = tui.show_overlay(Box::new(overlay), OverlayOptions::default());
    tui.set_focus(None);
    assert!(!handle.is_focused(&tui));
    tui.dispatch_event(key('z'));
    assert!(overlay_log.lock().unwrap().is_empty());
    assert!(!handle.is_focused(&tui));
}

#[test]
fn f5_non_capturing_show_and_explicit_focus() {
    let (mut tui, editor_log) = tui_with_editor();
    let (overlay, overlay_log) = Recorder::new("NC");
    let handle = tui.show_overlay(
        Box::new(overlay),
        OverlayOptions {
            non_capturing: true,
            ..Default::default()
        },
    );
    assert!(!handle.is_focused(&tui));
    tui.dispatch_event(key('e'));
    assert_eq!(*editor_log.lock().unwrap(), vec!['e']);

    handle.focus(&mut tui);
    assert!(handle.is_focused(&tui));
    tui.dispatch_event(key('n'));
    assert_eq!(*overlay_log.lock().unwrap(), vec!['n']);

    handle.set_hidden(&mut tui, true);
    handle.set_hidden(&mut tui, false);
    assert!(
        !handle.is_focused(&tui),
        "set_hidden(false) must not auto-focus NC"
    );
}

#[test]
fn f6_nested_hide_restores_parent() {
    let (mut tui, _editor_log) = tui_with_editor();
    let (parent, parent_log) = Recorder::new("PARENT");
    let parent_h = tui.show_overlay(Box::new(parent), OverlayOptions::default());
    let (child, _child_log) = Recorder::new("CHILD");
    let child_h = tui.show_overlay(Box::new(child), OverlayOptions::default());
    assert!(child_h.is_focused(&tui));
    child_h.hide(&mut tui);
    assert!(parent_h.is_focused(&tui));
    tui.dispatch_event(key('p'));
    assert_eq!(*parent_log.lock().unwrap(), vec!['p']);
}

#[test]
fn f7_unfocus_target_while_blocked() {
    let (mut tui, editor_log) = tui_with_editor();
    let (replacement, repl_log) = Recorder::new("REPL");
    tui.add_child(Box::new(replacement));
    let (overlay, overlay_log) = Recorder::new("OVL");
    let handle = tui.show_overlay(Box::new(overlay), OverlayOptions::default());
    // Steal to replacement → blocked.
    tui.set_focus(Some(1));
    assert!(!handle.is_focused(&tui));
    handle.unfocus(
        &mut tui,
        Some(OverlayUnfocusOptions {
            target: Some(FocusTarget::Root(0)),
        }),
    );
    // Replacement still focused until it yields.
    assert!(!handle.is_focused(&tui));
    tui.dispatch_event(key('1'));
    assert_eq!(*repl_log.lock().unwrap(), vec!['1']);
    // Yield: set_focus(None) while blocked-by replacement → resume to target (editor).
    tui.set_focus(None);
    tui.dispatch_event(key('e'));
    assert_eq!(*editor_log.lock().unwrap(), vec!['e']);
    assert!(overlay_log.lock().unwrap().is_empty());
}
