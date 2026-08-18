use super::*;
use crate::tui::{Component, InputEvent};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use std::cell::RefCell;
use std::rc::Rc;

fn t() -> EditorTheme {
    EditorTheme {
        border_color: Box::new(|s| s.to_string()),
        select_list_theme: SelectListTheme::default(),
    }
}

/// Default test clock: each `now()` is ≥20ms after the previous, so rapid
/// `insert_ch` in unit tests looks like human typing (no false paste coalesce).
/// Paste-burst tests pass an explicit frozen/`advance(1ms)` [`MockClock`] instead.
#[derive(Clone)]
struct TypingClock {
    t: std::cell::Cell<std::time::Instant>,
}
impl TypingClock {
    fn new() -> Self {
        Self {
            t: std::cell::Cell::new(std::time::Instant::now()),
        }
    }
}
impl Clock for TypingClock {
    fn now(&self) -> std::time::Instant {
        let n = self.t.get();
        self.t.set(
            n + std::time::Duration::from_millis(
                crate::paste_burst::PASTE_BURST_CHAR_INTERVAL_MS + 12,
            ),
        );
        n
    }
}

fn clk() -> Box<dyn Clock> {
    Box::new(TypingClock::new())
}

#[test]
fn empty() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    assert!(e.render(30).iter().any(|l| l.contains('─')));
}

#[test]
fn empty_draft_renders_one_content_row() {
    let mut e = Editor::new(
        t(),
        EditorOptions {
            padding_x: 1,
            terminal_rows: 8,
        },
        clk(),
    );
    e.set_focused(true);
    let lines = e.render(40);
    // top border + 1 content + bottom border
    assert_eq!(lines.len(), 3, "empty draft must stay compact: {lines:?}");
    e.set_text("a\nb\nc\nd\ne\nf".into());
    let grown = e.render(40);
    assert!(grown.len() > 3, "multi-line draft must grow: {grown:?}");
}

#[test]
fn insert_get() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.insert_ch("h");
    e.insert_ch("i");
    assert_eq!(e.get_text(), "hi");
}

#[test]
fn backspace_del() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.insert_ch("x");
    e.backspace();
    assert_eq!(e.get_text(), "");
}

#[test]
fn newline_split() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.insert_ch("a");
    e.newline();
    e.insert_ch("b");
    assert_eq!(e.state.lines.len(), 2);
    assert_eq!(e.get_text(), "a\nb");
}

#[test]
fn undo_test() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.insert_ch("a");
    assert_eq!(e.get_text(), "a");
    e.undo();
    assert_eq!(e.get_text(), "");
}

#[test]
fn word_lr() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.insert_ch("a");
    e.insert_ch("b");
    e.insert_ch(" ");
    e.insert_ch("c");
    e.word_left();
    assert_eq!(e.state.cursor_col, 3);
    e.word_right();
    assert_eq!(e.state.cursor_col, 4);
}

#[test]
fn submit_cb() {
    let s = Rc::new(RefCell::new(String::new()));
    let sc = s.clone();
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.on_submit = Some(Box::new(move |v| *sc.borrow_mut() = v));
    e.insert_ch("test");
    e.submit();
    assert_eq!(*s.borrow(), "test");
    assert!(e.get_text().is_empty());
}

// c425 new tests

#[test]
fn visual_line_map_wraps_long_lines() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.insert_inner("a".repeat(100).as_str());
    let vls = e.build_visual_line_map(30);
    assert!(
        vls.len() >= 3,
        "100-char line in 30-col should wrap to >=3 VLs, got {}",
        vls.len()
    );
}

#[test]
fn visual_line_map_empty_line_still_one_vl() {
    let e = Editor::new(t(), EditorOptions::default(), clk());
    let vls = e.build_visual_line_map(80);
    assert_eq!(vls.len(), 1);
    assert_eq!(vls[0].logical_line, 0);
}

#[test]
fn move_cursor_up_down_preserves_visual_col() {
    let mut e = Editor::new(
        t(),
        EditorOptions {
            padding_x: 0,
            terminal_rows: 40,
        },
        clk(),
    );
    e.set_text("aaaa\nbbbb\ncccc".to_string());
    e.last_width = 80;
    e.state.cursor_line = 2;
    e.state.cursor_col = 3;
    e.move_cursor(-1, 0);
    assert_eq!(e.state.cursor_line, 1);
    // On a non-wrapped line, visual col = cursor col = 3
    assert_eq!(e.state.cursor_col, 3, "up-arrow should stay at same column");
}

#[test]
fn preferred_visual_col_cleared_on_horizontal() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text("aaaa\nbbbb\ncccc".to_string());
    e.last_width = 80;
    e.state.cursor_line = 2;
    e.state.cursor_col = 3;
    // Move up — this sets preferred_visual_col via compute_vertical_move_column
    e.move_cursor(-1, 0);
    // Now move left — preferred should be cleared by set_cursor_col
    e.move_cursor(0, -1);
    assert!(
        e.preferred_visual_col.is_none(),
        "horizontal move should clear preferred_visual_col"
    );
}

#[test]
fn page_scroll_moves_by_page_size() {
    let mut e = Editor::new(
        t(),
        EditorOptions {
            padding_x: 0,
            terminal_rows: 40,
        },
        clk(),
    );
    // 50 single-char lines — page size = 12
    let lines: Vec<String> = (0..50).map(|i| format!("line {}", i)).collect();
    e.set_text(lines.join("\n"));
    e.state.cursor_line = 5;
    e.state.cursor_col = 0;
    e.last_width = 80;
    e.page_scroll(1);
    // Should have moved ~12 visual lines (all single-line lines → 12 logical lines)
    assert!(
        e.state.cursor_line >= 15,
        "page_down from line 5 should go to ~17 (5+12), got {}",
        e.state.cursor_line
    );
}

#[test]
fn long_paste_collapses_and_expands_fully() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    let pasted = (0..15)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    e.handle_input(InputEvent::Paste(pasted.clone()));
    let display = e.get_text();
    assert!(
        display.contains("[paste #1 +15 lines]"),
        "expected collapse marker, got: {display}"
    );
    assert_eq!(e.get_expanded_text(), pasted);
    assert!(
        !e.get_expanded_text().contains("[paste #"),
        "expanded must not leave marker fragments"
    );
}

#[test]
fn short_paste_inserts_inline() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.handle_input(InputEvent::Paste("hello\nworld".into()));
    assert_eq!(e.get_text(), "hello\nworld");
    assert_eq!(e.get_expanded_text(), "hello\nworld");
}

#[test]
fn char_paste_collapses_and_expands() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    let pasted = "x".repeat(1001);
    e.handle_input(InputEvent::Paste(pasted.clone()));
    let display = e.get_text();
    assert!(
        display.contains("[paste #1 1001 chars]"),
        "expected char marker, got: {display}"
    );
    assert_eq!(e.get_expanded_text(), pasted);
}

#[test]
fn paste_burst_enter_inserts_newline_not_submit() {
    use crate::clock::MockClock;
    use std::time::Duration;
    let theme = t();
    let mut clock = MockClock::new();
    // Type 8 fast chars → burst detected
    let mut e = Editor::new(theme, EditorOptions::default(), Box::new(clock.clone()));
    for _ in 0..8 {
        e.insert_ch("x");
        clock.advance(Duration::from_millis(1));
    }
    // Now Enter should be suppressed
    let now = clock.now();
    assert!(
        e.paste_burst.should_insert_newline_instead_of_submit(now),
        "8 fast chars should trigger paste burst enter suppression"
    );
}

#[test]
fn paste_burst_reset_on_nonprintable() {
    use crate::clock::MockClock;
    use std::time::Duration;
    let theme = t();
    let mut clock = MockClock::new();
    let mut e = Editor::new(theme, EditorOptions::default(), Box::new(clock.clone()));
    for _ in 0..8 {
        e.insert_ch("x");
        clock.advance(Duration::from_millis(1));
    }
    assert!(
        e.paste_burst
            .should_insert_newline_instead_of_submit(clock.now())
    );
    // Simulate a non-printable key (CursorLeft)
    e.paste_burst.reset();
    assert!(
        !e.paste_burst
            .should_insert_newline_instead_of_submit(clock.now()),
        "reset should clear burst state"
    );
}

#[test]
fn history_draft_restored_on_back_past_first() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.add_to_history("line 1".to_string());
    e.add_to_history("line 2".to_string());
    e.set_text("current".to_string());
    // Navigate through history
    e.navigate_history(-1); // → line 2
    e.navigate_history(-1); // → line 1
    // Now go back past first
    e.navigate_history(1); // → line 2
    assert_eq!(e.get_text(), "line 2");
}

#[test]
fn exit_history_on_edit() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.add_to_history("old".to_string());
    e.navigate_history(-1);
    assert_eq!(e.history_index, 0);
    // Typing a character should exit history browsing
    e.insert_ch("x");
    assert_eq!(e.history_index, -1, "edit should exit history browsing");
}

#[test]
fn visual_line_map_multiple_logical_lines() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text("short\nloooooooooooooooooooooooooooooong\nshort".to_string());
    let vls = e.build_visual_line_map(20);
    // Line 0: 1 VL, Line 1: wraps to ~2 VLs, Line 2: 1 VL → total ≥ 4
    assert!(vls.len() >= 4, "expected >=4 VLs, got {}", vls.len());
    // All VLs should have correct logical_line mapping
    assert_eq!(vls[0].logical_line, 0);
    assert_eq!(vls[vls.len() - 1].logical_line, 2);
}

#[test]
fn find_current_visual_line_at_end_of_wrapped_line() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    // Long-ish line in 30-col editor → wraps to multiple VLs
    e.set_text("abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123".to_string());
    let vls = e.build_visual_line_map(30);
    assert!(
        vls.len() >= 2,
        "long line at width 30 should wrap to ≥2 VLs"
    );
    // Cursor at end of line should be on the last VL
    e.state.cursor_col = e.state.lines[0].len();
    let ci = e.find_current_visual_line(&vls);
    assert_eq!(
        ci,
        vls.len() - 1,
        "cursor at end should be on last VL ({}), got {}",
        vls.len() - 1,
        ci
    );
}

#[test]
fn set_text_internal_start_placement() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text_internal("hello\nworld", CursorPlacement::Start);
    assert_eq!(e.state.cursor_line, 0);
    assert_eq!(e.state.cursor_col, 0);
}

#[test]
fn set_text_internal_end_placement() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text_internal("hello\nworld", CursorPlacement::End);
    assert_eq!(e.state.cursor_line, 1);
    assert_eq!(e.state.cursor_col, 5); // "world".len()
}

#[test]
fn set_text_places_cursor_at_end_like_pi() {
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text("hello\nworld".into());
    assert_eq!(e.state.cursor_line, 1);
    assert_eq!(e.state.cursor_col, 5);
}

fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: col,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
}

#[test]
fn editor_multiline_drag_select_and_copy() {
    use crate::selection::RecordingClipboardSink;
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text("alpha\nbeta\ngamma".into());
    let _ = e.render(40);
    // Local coords: row 0 = top border; content starts at row 1.
    let mut sink = RecordingClipboardSink::default();
    assert!(e.handle_mouse_local(
        &mouse(MouseEventKind::Down(MouseButton::Left), 0, 1),
        &mut sink
    ));
    assert!(e.is_selection_dragging());
    assert!(e.handle_mouse_local(
        &mouse(MouseEventKind::Drag(MouseButton::Left), 4, 3),
        &mut sink
    ));
    assert!(e.handle_mouse_local(
        &mouse(MouseEventKind::Up(MouseButton::Left), 4, 3),
        &mut sink
    ));
    assert!(!e.is_selection_dragging());
    assert_eq!(e.selected_text().as_deref(), Some("alpha\nbeta\ngamm"));
    assert_eq!(sink.copies, vec!["alpha\nbeta\ngamm".to_string()]);
    assert!(
        !e.take_pending_clipboard().is_empty(),
        "copy-on-release must queue OSC52"
    );
    let painted = e.render(40);
    let joined = painted.join("\n");
    assert!(
        joined.contains("\x1b[7m"),
        "visible editor lines must show selection highlight"
    );
}

#[test]
fn editor_selection_independent_of_empty_click() {
    use crate::selection::RecordingClipboardSink;
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text("only".into());
    let _ = e.render(40);
    let mut sink = RecordingClipboardSink::default();
    e.handle_mouse_local(
        &mouse(MouseEventKind::Down(MouseButton::Left), 1, 1),
        &mut sink,
    );
    e.handle_mouse_local(
        &mouse(MouseEventKind::Up(MouseButton::Left), 1, 1),
        &mut sink,
    );
    assert!(sink.copies.is_empty());
    assert!(!e.has_selection());
}

#[test]
fn editor_double_click_selects_whole_word_on_release() {
    use crate::selection::RecordingClipboardSink;
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text("say apple pie".into());
    let _ = e.render(40);
    let mut sink = RecordingClipboardSink::default();
    // Content row 1; 'l' of apple is display column 7.
    let col = 7u16;
    let row = 1u16;
    e.handle_mouse_local(
        &mouse(MouseEventKind::Down(MouseButton::Left), col, row),
        &mut sink,
    );
    e.handle_mouse_local(
        &mouse(MouseEventKind::Up(MouseButton::Left), col, row),
        &mut sink,
    );
    assert!(sink.copies.is_empty(), "first click must not copy");
    e.handle_mouse_local(
        &mouse(MouseEventKind::Down(MouseButton::Left), col, row),
        &mut sink,
    );
    assert_eq!(e.selected_text().as_deref(), Some("apple"));
    e.handle_mouse_local(
        &mouse(MouseEventKind::Up(MouseButton::Left), col, row),
        &mut sink,
    );
    assert_eq!(sink.copies, vec!["apple".to_string()]);
    assert_eq!(e.selected_text().as_deref(), Some("apple"));
}

#[test]
fn editor_triple_click_selects_whole_line_on_release() {
    use crate::selection::RecordingClipboardSink;
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text("say apple pie".into());
    let _ = e.render(40);
    let mut sink = RecordingClipboardSink::default();
    let col = 7u16;
    let row = 1u16;
    for _ in 0..3 {
        e.handle_mouse_local(
            &mouse(MouseEventKind::Down(MouseButton::Left), col, row),
            &mut sink,
        );
        e.handle_mouse_local(
            &mouse(MouseEventKind::Up(MouseButton::Left), col, row),
            &mut sink,
        );
    }
    assert_eq!(e.selected_text().as_deref(), Some("say apple pie"));
    assert_eq!(
        sink.copies.last().map(String::as_str),
        Some("say apple pie")
    );
}

#[test]
fn editor_selection_clears_on_key() {
    use crate::selection::RecordingClipboardSink;
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.set_text("ab\ncd".into());
    let _ = e.render(40);
    let mut sink = RecordingClipboardSink::default();
    e.handle_mouse_local(
        &mouse(MouseEventKind::Down(MouseButton::Left), 0, 1),
        &mut sink,
    );
    e.handle_mouse_local(
        &mouse(MouseEventKind::Drag(MouseButton::Left), 1, 2),
        &mut sink,
    );
    assert!(e.has_selection() || e.is_selection_dragging());
    e.handle_input(InputEvent::Key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Right,
        crossterm::event::KeyModifiers::NONE,
    )));
    assert!(!e.has_selection());
    assert!(!e.is_selection_dragging());
}

#[test]
fn editor_click_on_scrolled_top_row_does_not_ghost_select() {
    // Regression: former edge-zone scroll on Down/tick remapped focus to
    // earlier buffer cells, so a plain click looked like "select all before".
    use crate::selection::RecordingClipboardSink;
    let mut e = Editor::new(
        t(),
        EditorOptions {
            padding_x: 0,
            terminal_rows: 8, // max_vis ≈ 5
        },
        clk(),
    );
    e.set_text(
        (0..12)
            .map(|i| format!("L{i:02}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let _ = e.render(40);
    let offset_before = e.scroll_offset;
    assert!(
        offset_before > 0,
        "expected scrolled editor, offset={offset_before}"
    );
    let mut sink = RecordingClipboardSink::default();
    // Click mid first visible content row (edge zone under old logic).
    e.handle_mouse_local(
        &mouse(MouseEventKind::Down(MouseButton::Left), 2, 1),
        &mut sink,
    );
    // Simulate terminal Moved flood + idle tick while button held.
    let _ = e.handle_mouse_local(&mouse(MouseEventKind::Moved, 2, 1), &mut sink);
    assert!(!e.tick(), "click hold must not edge-scroll the viewport");
    e.handle_mouse_local(
        &mouse(MouseEventKind::Up(MouseButton::Left), 2, 1),
        &mut sink,
    );
    assert_eq!(
        e.scroll_offset, offset_before,
        "plain click must not change scroll"
    );
    assert!(!e.has_selection(), "plain click must not leave a selection");
    assert!(sink.copies.is_empty(), "plain click must not copy");
}

#[test]
fn editor_drag_onto_more_border_does_not_autoscroll() {
    // Product cut: no editor viewport edge auto-scroll while selecting.
    use crate::selection::RecordingClipboardSink;
    let mut e = Editor::new(
        t(),
        EditorOptions {
            padding_x: 0,
            terminal_rows: 8,
        },
        clk(),
    );
    e.set_text(
        (0..12)
            .map(|i| format!("L{i:02}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let _ = e.render(40);
    let offset_before = e.scroll_offset;
    assert!(offset_before > 0);
    let mut sink = RecordingClipboardSink::default();
    e.handle_mouse_local(
        &mouse(MouseEventKind::Down(MouseButton::Left), 0, 2),
        &mut sink,
    );
    e.handle_mouse_local(
        &mouse(MouseEventKind::Drag(MouseButton::Left), 0, 0),
        &mut sink,
    );
    assert_eq!(
        e.scroll_offset, offset_before,
        "drag onto ↑ more must not auto-scroll editor viewport"
    );
}

#[test]
fn editor_more_border_survives_narrow_width() {
    let mut e = Editor::new(
        t(),
        EditorOptions {
            padding_x: 0,
            terminal_rows: 8,
        },
        clk(),
    );
    e.set_text(
        (0..12)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let _ = e.render(40);
    assert!(e.scroll_offset > 0);
    let narrow = e.render(12);
    let top = &narrow[0];
    assert!(
        top.contains('↑') || top.contains("more"),
        "narrow width must keep more cue: {top:?}"
    );
}

#[test]
fn shift_enter_inserts_newline() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    e.insert_ch("a");
    e.handle_input(InputEvent::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::SHIFT,
    )));
    assert_eq!(e.get_text(), "a\n");
}

#[test]
fn ghostty_shift_enter_char_lf_inserts_newline() {
    // Ghostty (kitty-active): Shift+Enter → text `\n` → crossterm Char('\n')
    // without SHIFT (pi keys.ts). Must not submit / must not be printable.
    use crate::keys::with_kitty_protocol_active;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    with_kitty_protocol_active(true, || {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.insert_ch("a");
        e.handle_input(InputEvent::Key(KeyEvent::new(
            KeyCode::Char('\n'),
            KeyModifiers::NONE,
        )));
        assert_eq!(e.get_text(), "a\n");
    });
}

#[test]
fn external_editor_path_expands_paste_markers() {
    // Ctrl+G / $EDITOR must see real paste bodies, not `[paste #N …]`.
    let mut e = Editor::new(t(), EditorOptions::default(), clk());
    let pasted = (0..26)
        .map(|i| format!("body-line-{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    e.handle_input(InputEvent::Paste(pasted.clone()));
    assert!(
        e.get_text().contains("[paste #1 +26 lines]"),
        "display collapses: {}",
        e.get_text()
    );
    let for_editor = e.get_expanded_text();
    assert_eq!(for_editor, pasted);
    assert!(
        !for_editor.contains("[paste #"),
        "external editor must not receive collapse markers"
    );
}

#[test]
fn paste_burst_suppresses_mid_burst_rerender() {
    use crate::clock::MockClock;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    // Frozen MockClock: all insert_ch share one Instant → consecutive burst.
    let mut e = Editor::new(t(), EditorOptions::default(), Box::new(MockClock::new()));
    for ch in ["a", "b", "c", "d", "e", "f", "g"] {
        e.insert_ch(ch);
        assert!(
            e.input_wants_rerender(&InputEvent::Key(KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::NONE,
            ))),
            "below burst threshold must still want paint"
        );
    }
    e.insert_ch("h"); // 8th → paste burst
    assert_eq!(e.get_text(), "abcdefgh", "model must keep all chars");
    assert!(
        !e.input_wants_rerender(&InputEvent::Key(KeyEvent::new(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
        ))),
        "mid-burst keys must suppress typewriter paints"
    );
    e.paste_burst.reset();
    e.paste_burst_needs_paint = true;
    assert!(e.tick(), "idle tick must catch up one paint after burst");
    assert!(!e.paste_burst_needs_paint);
}

#[test]
fn editor_mouse_down_wants_rerender_after_handle() {
    use crate::selection::RecordingClipboardSink;
    use crate::tui::Component;
    let mut e = Editor::new(
        t(),
        EditorOptions {
            padding_x: 0,
            ..Default::default()
        },
        clk(),
    );
    e.set_text("primary flow".into());
    let _ = e.render(40);
    let ev = InputEvent::Mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 1));
    // Before handle: no drag yet → default mouse policy is quiet.
    assert!(!Component::input_wants_rerender(&e, &ev));
    Component::handle_input(&mut e, ev.clone());
    assert!(
        Component::input_wants_rerender(&e, &ev),
        "after Down, dragging must request a paint so click highlight is visible"
    );
    assert!(e.is_selection_dragging());
    let mut sink = RecordingClipboardSink::default();
    e.handle_mouse_local(
        &mouse(MouseEventKind::Up(MouseButton::Left), 0, 1),
        &mut sink,
    );
    assert!(sink.copies.is_empty(), "empty click must not copy");
    assert!(!e.has_selection(), "empty click must clear selection");
}
