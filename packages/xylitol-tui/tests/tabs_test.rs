use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use xylitol_tui::keybindings::{
    KeybindingsConfig, KeybindingsManager, KeybindingsScope, create_default_definitions,
};
use xylitol_tui::utils::strip_ansi_codes;
use xylitol_tui::{Component, InputEvent, Tabs};

fn key(code: KeyCode) -> InputEvent {
    InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn left_down(column: u16, row: u16) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn wheel(up: bool, column: u16, row: u16) -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: if up {
            MouseEventKind::ScrollUp
        } else {
            MouseEventKind::ScrollDown
        },
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn bind_tabs_arrows() -> KeybindingsScope {
    let mut user = KeybindingsConfig::new();
    user.insert("tui.tabs.prev".into(), vec!["left".into()]);
    user.insert("tui.tabs.next".into(), vec!["right".into()]);
    let kb = Rc::new(RefCell::new(KeybindingsManager::new(
        create_default_definitions(),
        user,
    )));
    KeybindingsScope::enter(kb)
}

fn three() -> Tabs {
    let mut tabs = Tabs::new(vec!["alpha".into(), "beta".into(), "gamma".into()]);
    tabs.set_selected(1);
    tabs
}

fn long_titles() -> Vec<String> {
    vec![
        "AlphaAAA".into(),
        "BravoBBB".into(),
        "CharlieC".into(),
        "DeltaDDD".into(),
        "EchoEEEE".into(),
    ]
}

#[test]
fn bound_keys_cycle_selected() {
    let _scope = bind_tabs_arrows();
    let mut tabs = three();
    tabs.handle_input(key(KeyCode::Right));
    assert_eq!(tabs.selected(), 2);
    tabs.handle_input(key(KeyCode::Left));
    assert_eq!(tabs.selected(), 1);
}

#[test]
fn unbound_keys_do_not_change_selected() {
    let kb = Rc::new(RefCell::new(KeybindingsManager::new(
        create_default_definitions(),
        HashMap::new(),
    )));
    let _scope = KeybindingsScope::enter(kb);
    let mut tabs = three();
    tabs.handle_input(key(KeyCode::Left));
    tabs.handle_input(key(KeyCode::Right));
    tabs.handle_input(key(KeyCode::Char('1')));
    assert_eq!(tabs.selected(), 1);
}

#[test]
fn empty_titles_ignore_bound_keys() {
    let _scope = bind_tabs_arrows();
    let mut tabs = Tabs::new(Vec::new());
    tabs.handle_input(key(KeyCode::Right));
    assert_eq!(tabs.selected(), 0);
}

#[test]
fn render_is_one_strip_line() {
    let mut tabs = Tabs::new(vec!["one".into(), "two".into()]);
    let lines = tabs.render(40);
    assert_eq!(lines.len(), 1);
    let vis = strip_ansi_codes(&lines[0]);
    assert!(vis.contains("one"));
    assert!(vis.contains("two"));
    assert!(!vis.contains('\n'));
}

#[test]
fn title_ranges_cover_visible_source_indices() {
    let mut tabs = Tabs::new(vec!["one".into(), "two".into()]);
    let _ = tabs.render(40);
    let ranges = tabs.title_col_ranges();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].0, 0);
    assert_eq!(ranges[1].0, 1);
    assert!(ranges[0].1 < ranges[0].2);
    assert!(ranges[1].1 < ranges[1].2);
    assert!(ranges[0].2 <= ranges[1].1);
    assert!(ranges[1].2 <= 40);
    assert!(tabs.overflow_col_ranges().is_empty());
}

#[test]
fn overflow_keeps_selected_and_markers() {
    let mut tabs = Tabs::new(long_titles());
    tabs.set_selected(2);
    let lines = tabs.render(20);
    let vis = strip_ansi_codes(&lines[0]);
    assert!(vis.contains("CharlieC"), "selected must stay full: {vis:?}");
    assert!(!vis.contains("AlphaAAA"));
    assert!(!vis.contains("EchoEEEE"));
    assert!(vis.contains("+"), "hidden counts: {vis:?}");
    assert!(vis.contains('←') || vis.contains('→'), "{vis:?}");
}

#[test]
fn overflow_ranges_keep_source_index() {
    let mut tabs = Tabs::new(long_titles());
    tabs.set_selected(4);
    let _ = tabs.render(20);
    let titles = tabs.title_col_ranges();
    assert!(titles.iter().all(|(i, _, _)| *i <= 4));
    assert!(titles.iter().any(|(i, _, _)| *i == 4));
    assert!(!titles.iter().any(|(i, _, _)| *i == 0));
}

#[test]
fn overflow_marker_points_first_hidden() {
    let mut tabs = Tabs::new(long_titles());
    tabs.set_selected(2);
    let _ = tabs.render(20);
    let titles: Vec<usize> = tabs.title_col_ranges().iter().map(|r| r.0).collect();
    let markers = tabs.overflow_col_ranges();
    assert!(!markers.is_empty());
    for &(idx, start, end, count) in &markers {
        assert!(start < end);
        assert!(count > 0);
        assert!(!titles.contains(&idx), "hidden index {idx} in titles");
    }
    let left = markers.iter().find(|r| r.0 < 2);
    let right = markers.iter().find(|r| r.0 > 2);
    if let Some(left) = left {
        assert_eq!(
            left.0,
            titles.iter().copied().min().unwrap().saturating_sub(1)
        );
    }
    if let Some(right) = right {
        assert_eq!(right.0, titles.iter().copied().max().unwrap() + 1);
    }
}

#[test]
fn click_title_selects() {
    let mut tabs = Tabs::new(vec!["alpha".into(), "beta".into(), "gamma".into()]);
    let _ = tabs.render(40);
    let (_, start, end) = tabs.title_col_ranges()[1];
    let col = ((start + end) / 2) as u16;
    tabs.handle_input(left_down(col, 0));
    assert_eq!(tabs.selected(), 1);
}

#[test]
fn click_overflow_jumps_hidden() {
    let mut tabs = Tabs::new(long_titles());
    tabs.set_selected(2);
    let _ = tabs.render(20);
    let right = tabs
        .overflow_col_ranges()
        .into_iter()
        .find(|r| r.0 > 2)
        .expect("right marker");
    let col = ((right.1 + right.2) / 2) as u16;
    let expected = right.0;
    tabs.handle_input(left_down(col, 0));
    assert_eq!(tabs.selected(), expected);
}

#[test]
fn wheel_on_strip_cycles() {
    let mut tabs = three();
    let _ = tabs.render(40);
    assert!(tabs.offer_pointer(&wheel(true, 0, 0)));
    assert_eq!(tabs.selected(), 2);
    assert!(tabs.offer_pointer(&wheel(false, 0, 0)));
    assert_eq!(tabs.selected(), 1);
}

#[test]
fn pointer_off_strip_ignored() {
    let mut tabs = three();
    let _ = tabs.render(40);
    let before = tabs.selected();
    assert!(!tabs.offer_pointer(&left_down(0, 2)));
    assert_eq!(tabs.selected(), before);
}

#[test]
fn pointer_on_strip_consumes() {
    let mut tabs = Tabs::new(vec!["alpha".into(), "beta".into(), "gamma".into()]);
    let _ = tabs.render(40);
    let (_, start, end) = tabs.title_col_ranges()[1];
    let col = ((start + end) / 2) as u16;
    assert!(tabs.offer_pointer(&left_down(col, 0)));
    assert_eq!(tabs.selected(), 1);
}

#[test]
fn origin_offsets_hit_columns() {
    let mut tabs = Tabs::new(vec!["alpha".into(), "beta".into(), "gamma".into()]);
    tabs.set_screen_origin(10, 3);
    let _ = tabs.render(40);
    let (_, start, end) = tabs.title_col_ranges()[1];
    let col = 10 + ((start + end) / 2) as u16;
    assert!(!tabs.offer_pointer(&left_down(col, 0)));
    assert!(tabs.offer_pointer(&left_down(col, 3)));
    assert_eq!(tabs.selected(), 1);
}
