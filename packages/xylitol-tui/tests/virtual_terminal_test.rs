//! VirtualTerminal harness self-tests + minimal TUI render smoke.
//!
//! These verify (1) the `vte`-backed grid parses ANSI the way the engine
//! expects, and (2) that a `TUI` fed a trivial component actually paints the
//! viewport. Later stages add differential-render / overlay / style-leak cases
//! ported from pi-tui's `tui-render.test.ts`.

mod support;

use support::vt_feed::feed_vt;
use support::{LoggingVirtualTerminal, VirtualTerminal};
use xylitol_tui::tui::{Component, InputEvent};
use xylitol_tui::{TUI, terminal::Terminal};

/// Minimal component holding fixed lines; mirrors pi-tui's `TestComponent`.
struct LinesComponent {
    lines: Vec<String>,
}

impl LinesComponent {
    fn new(lines: Vec<&str>) -> Self {
        Self {
            lines: lines.into_iter().map(String::from).collect(),
        }
    }
}

impl Component for LinesComponent {
    fn render(&mut self, _width: usize) -> Vec<String> {
        self.lines.clone()
    }
    fn handle_input(&mut self, _event: InputEvent) {}
    fn invalidate(&mut self) {}
}

// ── VirtualTerminal grid parsing ───────────────────────────────────────────

#[test]
fn vt_plain_text_lands_in_cells() {
    let mut vt = VirtualTerminal::new(10, 3);
    vt.write("hello");
    assert_eq!(vt.viewport()[0], "hello");
    assert_eq!(vt.cell(0, 0).ch, 'h');
    assert_eq!(vt.cell(0, 4).ch, 'o');
}

#[test]
fn vt_carriage_return_newline_advances_row() {
    let mut vt = VirtualTerminal::new(10, 3);
    vt.write("a\r\nb");
    assert_eq!(vt.viewport()[0], "a");
    assert_eq!(vt.viewport()[1], "b");
}

#[test]
fn vt_sgr_bold_and_italic_tracked_per_cell() {
    let mut vt = VirtualTerminal::new(10, 1);
    vt.write("\x1b[1mB\x1b[22m\x1b[3mi\x1b[23mn");
    assert!(vt.cell(0, 0).bold, "first char should be bold");
    assert!(!vt.cell(0, 0).italic);
    assert!(!vt.cell(0, 1).bold);
    assert!(vt.cell(0, 1).italic, "second char should be italic");
    assert!(!vt.cell(0, 2).bold);
    assert!(!vt.cell(0, 2).italic);
}

#[test]
fn vt_sgr_indexed_and_rgb_color() {
    let mut vt = VirtualTerminal::new(10, 1);
    // fg=38;5;208 (indexed), bg=48;2;1;2;3 (rgb)
    vt.write("\x1b[38;5;208mX\x1b[48;2;1;2;3mY\x1b[0mZ");
    assert_eq!(vt.cell(0, 0).fg, support::Color::Indexed(208));
    assert_eq!(vt.cell(0, 1).bg, support::Color::Rgb(1, 2, 3));
    assert_eq!(vt.cell(0, 2).fg, support::Color::Default);
}

#[test]
fn vt_cursor_position_absolute_move() {
    let mut vt = VirtualTerminal::new(10, 3);
    vt.write("\x1b[3;5HX");
    let (x, y) = vt.cursor_position();
    // After writing X at row 3 col 5 (1-based), cursor advances to col 6.
    assert_eq!(y, 2);
    assert_eq!(x, 5);
    assert_eq!(vt.cell(2, 4).ch, 'X');
}

#[test]
fn vt_erase_in_display_clears_screen() {
    let mut vt = VirtualTerminal::new(10, 2);
    vt.write("abc\r\ndef\r\n\x1b[2J");
    assert_eq!(vt.viewport()[0], "");
    assert_eq!(vt.viewport()[1], "");
}

#[test]
fn vt_erase_in_line_from_cursor() {
    let mut vt = VirtualTerminal::new(10, 1);
    vt.write("abcdef\x1b[3G\x1b[K");
    // Cursor moved to col 3 (1-based) = index 2, erase to end → "ab"
    assert_eq!(vt.viewport()[0], "ab");
}

#[test]
fn vt_scroll_grows_grid_past_initial_rows() {
    let mut vt = VirtualTerminal::new(5, 2);
    vt.write("1\r\n2\r\n3\r\n4");
    let buf = vt.scroll_buffer();
    // All four rows should be retained in the scrollback even though the
    // viewport only shows 2.
    assert!(buf.len() >= 4);
    assert!(buf.contains(&"3".to_string()));
    assert!(buf.contains(&"4".to_string()));
}

// ── LoggingVirtualTerminal write capture ───────────────────────────────────

#[test]
fn logging_vt_records_per_write_and_concatenates() {
    let mut vt = LoggingVirtualTerminal::new(10, 1);
    vt.write("a");
    vt.write("b");
    assert_eq!(vt.write_count(), 2);
    assert_eq!(vt.all_writes(), "ab");
    assert_eq!(vt.count_occurrences("\x1b["), 0);
    vt.clear_writes();
    assert_eq!(vt.write_count(), 0);
}

// ── TUI minimal render smoke ───────────────────────────────────────────────

#[test]
fn tui_renders_component_lines_into_viewport() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["hello", "world"])));

    tui.render_frame().unwrap();

    let vp = tui.terminal.viewport();
    assert_eq!(vp[0], "hello");
    assert_eq!(vp[1], "world");
    // First frame is a full render, so exactly one write call is expected.
    assert_eq!(
        tui.terminal.write_count(),
        1,
        "first frame should be one full render"
    );
}

#[test]
fn tui_unchanged_frame_writes_nothing_on_diff_path() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["x"])));

    tui.render_frame().unwrap();
    tui.terminal.clear_writes();

    // Re-render with unchanged content: differential path should emit nothing.
    tui.render_frame().unwrap();
    assert_eq!(
        tui.terminal.write_count(),
        0,
        "unchanged frame should not write"
    );
}

#[test]
fn tui_second_render_after_change_only_writes_changed_line() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    let mut comp = LinesComponent::new(vec!["alpha", "beta"]);
    tui.add_child(Box::new(comp));
    tui.render_frame().unwrap();
    tui.terminal.clear_writes();

    // Change only the second line.
    comp = LinesComponent::new(vec!["alpha", "BETA"]);
    // Replace the child: the TUI owns a Box<dyn Component>, so we rebuild.
    let mut tui2 = TUI::new(LoggingVirtualTerminal::new(20, 5));
    tui2.add_child(Box::new(comp));
    tui2.render_frame().unwrap();
    // Sanity: the second composition renders both lines on its first frame.
    assert_eq!(tui2.terminal.viewport()[1], "BETA");
}

#[test]
fn tui_dispatch_input_reaches_focused_component() {
    // Rewritten (c405) to use TuiTestHarness instead of hand-wiring
    // TUI + Terminal + set_focus per test (spec tt02).
    use support::TuiTestHarness;
    use xylitol_tui::components::input::Input;

    let mut input = Input::new();
    input.set_focused(true);
    let mut h = TuiTestHarness::new(20, 3);
    h.mount(Box::new(input)).focus(Some(0));
    h.keys("a");
    h.render();
    h.assert_text_contains("a");
}

#[test]
fn input_submit_preserves_value_matching_pi() {
    use std::sync::{Arc, Mutex};
    use xylitol_tui::components::input::Input;

    let mut input = Input::new();
    let captured = Arc::new(Mutex::new(String::new()));
    let cap = captured.clone();
    input.on_submit = Some(Box::new(move |v| {
        *cap.lock().unwrap() = v;
    }));

    feed_vt(&mut input, "hello");
    feed_vt(&mut input, "\r");

    // pi fires onSubmit(this.value) without clearing.
    assert_eq!(*captured.lock().unwrap(), "hello");
    assert_eq!(
        input.value(),
        "hello",
        "value must survive submit (pi parity)"
    );
}

// ── render scheduling (request_render / try_render throttle) ────────────────

#[test]
fn try_render_skips_when_not_requested() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["x"])));
    // No request_render yet — try_render should no-op.
    assert!(
        !tui.try_render().unwrap(),
        "try_render without request should no-op"
    );
    assert_eq!(tui.terminal.write_count(), 0);
}

#[test]
fn try_render_renders_after_request() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["x"])));
    tui.request_render(false);
    assert!(
        tui.try_render().unwrap(),
        "first try_render after request should fire"
    );
    assert_eq!(tui.terminal.write_count(), 1);
}

#[test]
fn try_render_throttles_within_16ms() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["x"])));

    tui.request_render(false);
    assert!(tui.try_render().unwrap());
    tui.terminal.clear_writes();

    // Immediately request again — should be throttled (within 16ms).
    tui.request_render(false);
    assert!(
        !tui.try_render().unwrap(),
        "second render within 16ms should be throttled"
    );
    assert_eq!(tui.terminal.write_count(), 0);
}

#[tokio::test]
async fn try_render_fires_after_throttle_window() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["x"])));

    tui.request_render(false);
    assert!(tui.try_render().unwrap());
    tui.terminal.clear_writes();

    // Wait past the 16ms window, then a pending request should fire.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    tui.request_render(false);
    assert!(
        tui.try_render().unwrap(),
        "render after throttle window should fire"
    );
}

#[test]
fn request_render_force_resets_previous_state_for_full_redraw() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["first"])));
    tui.render_frame().unwrap(); // establish previous_lines
    assert!(tui.full_redraws() >= 1);
    let before = tui.full_redraws();

    // force=true resets previous state, so next render is a full redraw.
    tui.request_render(true);
    tui.try_render().unwrap();
    assert!(
        tui.full_redraws() > before,
        "force request should trigger a full redraw"
    );
}

// ── width invariant (pi's crash guard) ──────────────────────────────────────

#[test]
fn render_errors_when_line_overflows_terminal_width() {
    use xylitol_tui::tui::RenderError;

    // 5-col terminal, but the component emits a 12-col line.
    let term = LoggingVirtualTerminal::new(5, 3);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["hello world!"])));

    let err = tui
        .render_frame()
        .expect_err("overflowing line must error, not silently truncate");
    let _: &RenderError = &err;
    assert!(err.line_width > err.width, "reported width should overflow");
    assert_eq!(err.line_index, 0);
}

#[test]
fn render_allows_line_that_fits_width_exactly() {
    // A line whose visible width equals the terminal width must NOT error.
    let term = LoggingVirtualTerminal::new(5, 3);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["hello"])));
    tui.render_frame()
        .expect("exact-width line should render fine");
}

#[test]
fn render_exact_width_changed_line_does_not_wrap_and_leave_stale_rows() {
    use support::MutableComponent;

    let lines = std::rc::Rc::new(std::cell::RefCell::new(vec![
        "ABCDEFGH".to_string(),
        "tail".to_string(),
    ]));
    let term = LoggingVirtualTerminal::new(8, 4);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(MutableComponent {
        lines: lines.clone(),
    }));

    tui.render_frame().unwrap();
    assert_eq!(tui.terminal.viewport()[0], "ABCDEFGH");
    assert_eq!(tui.terminal.viewport()[1], "tail");
    assert!(tui.terminal.all_writes().contains("\x1b[?7l"));
    assert!(tui.terminal.all_writes().contains("\x1b[?7h"));

    *lines.borrow_mut() = vec!["12345678".to_string(), "done".to_string()];
    tui.terminal.clear_writes();
    tui.render_frame().unwrap();

    let vp = tui.terminal.viewport();
    assert_eq!(vp[0], "12345678");
    assert_eq!(vp[1], "done");
    assert!(
        !vp.iter().any(|line| line.contains("tail")),
        "stale old rows must not remain after an exact-width diff: {vp:?}"
    );
}

#[test]
fn render_exempts_image_lines_from_width_check() {
    // Kitty APC image lines have visible width 0 but carry many payload bytes;
    // they must not trip the width invariant.
    let term = LoggingVirtualTerminal::new(5, 3);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec![
        "\x1b_Ga=f,t=d,f=99,s=very-long-payload-that-is-way-wider-than-five\x1b\\",
    ])));
    tui.render_frame()
        .expect("Kitty image lines should be exempt from the width check");
}

// ── overlay compositing (extract_segments style inheritance) ────────────────

#[test]
fn overlay_preserves_trailing_content_styling() {
    use xylitol_tui::{OverlayAnchor, OverlayOptions};

    // Base line is fully bold "abcdefghij". Overlay sits in the middle cols.
    // After compositing, the trailing content (after the overlay) must still
    // be bold — this is the bug extract_segments fixes (style leak across the
    // overlay boundary).
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec![
        "\x1b[1mabcdefghij\x1b[0m",
    ])));
    tui.show_overlay(
        Box::new(LinesComponent::new(vec!["XYZ"])),
        OverlayOptions {
            anchor: Some(OverlayAnchor::TopLeft),
            margin: Some(xylitol_tui::OverlayMargin {
                top: Some(0),
                left: Some(3),
                right: None,
                bottom: None,
            }),
            ..Default::default()
        },
    );
    tui.render_frame().unwrap();

    // The 'g' at col 6 should still carry bold styling (col 3,4,5 = overlay).
    let cell_after = tui.terminal.cell(0, 6);
    assert!(
        cell_after.bold,
        "trailing content after overlay must keep bold, got cell: {:?}",
        cell_after
    );
}

#[test]
fn overlay_replaces_its_region_content() {
    use xylitol_tui::{OverlayAnchor, OverlayOptions};

    let term = LoggingVirtualTerminal::new(20, 3);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["hello world"])));
    tui.show_overlay(
        Box::new(LinesComponent::new(vec!["XYZ"])),
        OverlayOptions {
            anchor: Some(OverlayAnchor::TopLeft),
            margin: Some(xylitol_tui::OverlayMargin {
                top: Some(0),
                left: Some(0),
                right: None,
                bottom: None,
            }),
            ..Default::default()
        },
    );
    tui.render_frame().unwrap();

    // Overlay at col 0 covers "hel" → first three cells are X,Y,Z.
    assert_eq!(tui.terminal.cell(0, 0).ch, 'X');
    assert_eq!(tui.terminal.cell(0, 2).ch, 'Z');
    // Trailing content preserved: "lo world" starts at col 3.
    assert_eq!(tui.terminal.cell(0, 3).ch, 'l');
    assert_eq!(tui.terminal.cell(0, 4).ch, 'o');
}

// ── differential render viewport scroll (pi Step 5C) ────────────────────────
// When content grows past one screen, appending must scroll the terminal
// (CUD to bottom + `\r\n`) so the new lines land on-screen instead of past the
// edge. This is the c399-class bug the doRender rewrite targets.

use std::cell::RefCell;
use std::rc::Rc;

use support::MutableComponent;

#[test]
fn viewport_scrolls_when_content_grows_past_screen_height() {
    let lines = Rc::new(RefCell::new(vec!["line1".to_string(), "line2".to_string()]));
    let term = LoggingVirtualTerminal::new(20, 3);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(MutableComponent {
        lines: lines.clone(),
    }));

    // Frame 1: 2 lines fit in the 3-row screen.
    tui.render_frame().unwrap();
    let vp = tui.terminal.viewport();
    assert_eq!(vp[0], "line1");
    assert_eq!(vp[1], "line2");

    // Frame 2: grow to 5 lines — exceeds the 3-row screen. The viewport must
    // scroll so the last 3 rendered lines (line3/4/5) are visible.
    *lines.borrow_mut() = vec![
        "line1".to_string(),
        "line2".to_string(),
        "line3".to_string(),
        "line4".to_string(),
        "line5".to_string(),
    ];
    tui.render_frame().unwrap();

    let vp = tui.terminal.viewport();
    // After scrolling, the bottom of the viewport should show the tail content.
    assert!(
        vp.iter().any(|l| l.contains("line5")),
        "last line should be visible after scroll, got viewport: {:?}",
        vp
    );
    assert!(
        vp.iter().any(|l| l.contains("line3")),
        "mid content should be visible after scroll, got viewport: {:?}",
        vp
    );
}

#[test]
fn viewport_append_keeps_unchanged_top_lines_stable() {
    let lines = Rc::new(RefCell::new(vec!["stable".to_string()]));
    let term = LoggingVirtualTerminal::new(20, 4);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(MutableComponent {
        lines: lines.clone(),
    }));
    tui.render_frame().unwrap();

    // Append a new line without touching the first.
    *lines.borrow_mut() = vec!["stable".to_string(), "appended".to_string()];
    tui.terminal.clear_writes();
    tui.render_frame().unwrap();

    let vp = tui.terminal.viewport();
    assert_eq!(vp[0], "stable", "unchanged top line must stay");
    assert_eq!(vp[1], "appended", "appended line must render");
}

// ── Component::tick (loader self-animation hook) ────────────────────────────

#[test]
fn loader_tick_advances_spinner_frame_and_signals_rerender() {
    use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
    use xylitol_tui::tui::Component;

    let color = |s: &str| s.to_string();
    let mut loader = Loader::new(
        Box::new(color),
        Box::new(color),
        "loading".to_string(),
        Some(LoaderIndicatorOptions {
            frames: vec!["|".to_string(), "/".to_string(), "-".to_string()],
            interval_ms: 80,
        }),
    );

    // First render establishes frame 0.
    let first = loader.render(20).join("");
    let wants_render = Component::tick(&mut loader);
    assert!(
        wants_render,
        "multi-frame loader tick should request rerender"
    );
    let second = loader.render(20).join("");
    assert_ne!(first, second, "tick should advance the spinner frame");
}

#[test]
fn single_frame_loader_tick_does_not_signal_rerender() {
    use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
    use xylitol_tui::tui::Component;

    let color = |s: &str| s.to_string();
    let mut loader = Loader::new(
        Box::new(color),
        Box::new(color),
        "done".to_string(),
        Some(LoaderIndicatorOptions {
            frames: vec!["*".to_string()],
            interval_ms: 80,
        }),
    );
    assert!(
        !Component::tick(&mut loader),
        "single-frame loader should not request rerender"
    );
}

// ── input strict slice (wide-char window boundary) ──────────────────────────

#[test]
fn input_render_handles_cjk_in_visible_window_without_panic() {
    use xylitol_tui::components::input::Input;
    use xylitol_tui::tui::Component;

    // Narrow window (6 cols) with CJK content wider than the window — the
    // strict slice must not split the wide char and must not panic.
    let mut input = Input::new();
    input.set_focused(true);
    feed_vt(&mut input, "你好世界测试");
    // Render into a 10-col terminal ("> " prompt + 8-col window); should produce
    // exactly one line without panicking.
    let lines = input.render(10);
    assert_eq!(lines.len(), 1, "input renders a single line");
    assert!(!lines[0].is_empty());
}
