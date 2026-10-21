mod support;

use support::vt_feed::feed_vt;
use support::{LoggingVirtualTerminal, VirtualTerminal};
use xylitol_tui::tui::{Component, InputEvent};
use xylitol_tui::{TUI, terminal::Terminal};

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
    assert_eq!(vt.viewport()[0], "ab");
}

#[test]
fn vt_scroll_grows_grid_past_initial_rows() {
    let mut vt = VirtualTerminal::new(5, 2);
    vt.write("1\r\n2\r\n3\r\n4");
    let buf = vt.scroll_buffer();
    assert!(buf.len() >= 4);
    assert!(buf.contains(&"3".to_string()));
    assert!(buf.contains(&"4".to_string()));
}

#[test]
fn vt_decstbm_scrolls_only_the_selected_region_and_homes_cursor() {
    let mut vt = VirtualTerminal::new(6, 5);
    for row in 1..=5 {
        vt.write(&format!("\x1b[{row};1HR{row}"));
    }

    vt.write("\x1b[2;4r\x1b[1S");
    assert_eq!(vt.viewport(), vec!["R1", "R3", "R4", "", "R5"]);
    assert_eq!(vt.cursor_position(), (0, 0));

    vt.write("\x1b[1T\x1b[r");
    assert_eq!(vt.viewport(), vec!["R1", "", "R3", "R4", "R5"]);
    assert_eq!(vt.cursor_position(), (0, 0));
}

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

#[test]
fn tui_renders_component_lines_into_viewport() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["hello", "world"])));

    tui.render_frame().unwrap();

    let vp = tui.terminal.viewport();
    assert_eq!(vp[0], "hello");
    assert_eq!(vp[1], "world");
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

    comp = LinesComponent::new(vec!["alpha", "BETA"]);
    let mut tui2 = TUI::new(LoggingVirtualTerminal::new(20, 5));
    tui2.add_child(Box::new(comp));
    tui2.render_frame().unwrap();
    assert_eq!(tui2.terminal.viewport()[1], "BETA");
}

#[test]
fn tui_dispatch_input_reaches_focused_component() {
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

    assert_eq!(*captured.lock().unwrap(), "hello");
    assert_eq!(
        input.value(),
        "hello",
        "value must survive submit (pi parity)"
    );
}

#[test]
fn try_render_skips_when_not_requested() {
    let term = LoggingVirtualTerminal::new(20, 5);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec!["x"])));
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
    tui.render_frame().unwrap();
    assert!(tui.full_redraws() >= 1);
    let before = tui.full_redraws();
    tui.terminal.clear_writes();

    tui.request_render(true);
    tui.try_render().unwrap();
    assert!(
        tui.full_redraws() > before,
        "force request should trigger a full redraw"
    );
    let writes = tui.terminal.all_writes();
    assert!(
        writes.contains("\x1b[2J"),
        "force must clear like pi requestRender(true): {writes:?}"
    );
}

#[test]
fn differential_shrink_cleanup_moves_to_content_end_in_one_batch() {
    use support::MutableComponent;

    let lines = std::rc::Rc::new(std::cell::RefCell::new(vec![
        "keep-a".into(),
        "keep-b".into(),
        "gone-1".into(),
        "gone-2".into(),
        "gone-3".into(),
    ]));
    let term = LoggingVirtualTerminal::new(20, 10);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(MutableComponent {
        lines: lines.clone(),
    }));
    tui.render_frame().unwrap();
    tui.terminal.clear_writes();

    *lines.borrow_mut() = vec!["keep-A".into(), "keep-B".into()];
    tui.request_render(false);
    tui.render_frame().unwrap();

    let writes = tui.terminal.all_writes();
    let begins = writes.matches("\x1b[?2026h").count();
    assert_eq!(
        begins, 1,
        "shrink cleanup must stay in one sync batch, got {begins}: {writes:?}"
    );
    assert!(
        writes.contains("\x1b[2K"),
        "must clear surplus lines: {writes:?}"
    );
    let vp = tui.terminal.viewport();
    assert!(
        !vp.iter().any(|l| l.contains("gone")),
        "surplus rows must be gone: {vp:?}"
    );
}

#[test]
fn soft_resize_triggers_clearing_full_redraw() {
    use support::MutableComponent;

    let lines = std::rc::Rc::new(std::cell::RefCell::new(vec![
        "AAAAAAAA".to_string(),
        "BBBBBBBB".to_string(),
        "CCCCCCCC".to_string(),
    ]));
    let term = LoggingVirtualTerminal::new(8, 6);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(MutableComponent {
        lines: lines.clone(),
    }));
    tui.render_frame().unwrap();
    tui.terminal.clear_writes();

    tui.terminal.set_size_hint(6, 6);
    *lines.borrow_mut() = vec!["AAAAAA".to_string(), "BBBBBB".to_string()];
    tui.request_render(false);
    tui.render_frame().unwrap();

    let writes = tui.terminal.all_writes();
    assert!(
        writes.contains("\x1b[2J") && writes.contains("\x1b[3J"),
        "soft size change must fullRender(true) like pi: {writes:?}"
    );
    let vp = tui.terminal.viewport();
    assert!(
        !vp.iter().any(|l| l.contains('C')),
        "cleared resize must not leave stale rows: {vp:?}"
    );
}

#[test]
fn with_terminal_suspended_defers_paint_until_after_caller_mutates() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use support::MutableComponent;

    let lines = Rc::new(RefCell::new(vec!["short-draft".to_string()]));
    let term = LoggingVirtualTerminal::new(40, 12);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(MutableComponent {
        lines: lines.clone(),
    }));
    tui.render_frame().unwrap();
    let writes_before = tui.terminal.write_count();

    let _ = tui.with_terminal_suspended(|| "ok");

    assert_eq!(
        tui.terminal.write_count(),
        writes_before,
        "suspend must not paint before caller replaces editor text"
    );

    *lines.borrow_mut() = vec![
        "short-draft".into(),
        String::new(),
        "aaa".into(),
        String::new(),
        "bbb".into(),
    ];
    tui.request_render(false);
    assert!(tui.try_render().unwrap());

    let writes = tui.terminal.all_writes();
    assert!(
        !writes.contains("\x1b[2J"),
        "resume must not full-clear (inline TUI keeps scrollback above)"
    );

    let vp = tui.terminal.viewport().join("\n");
    assert!(vp.contains("aaa"), "viewport missing aaa:\n{vp}");
    assert!(vp.contains("bbb"), "viewport missing bbb:\n{vp}");
    let short_hits = vp.matches("short-draft").count();
    assert_eq!(
        short_hits, 1,
        "stale 1-line frame must not ghost above grown buffer:\n{vp}"
    );
}

#[test]
fn render_errors_when_line_overflows_terminal_width() {
    use xylitol_tui::tui::RenderError;

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
    let term = LoggingVirtualTerminal::new(5, 3);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(LinesComponent::new(vec![
        "\x1b_Ga=f,t=d,f=99,s=very-long-payload-that-is-way-wider-than-five\x1b\\",
    ])));
    tui.render_frame()
        .expect("Kitty image lines should be exempt from the width check");
}

#[test]
fn unchanged_lines_reuse_finalize_skip_width_checks() {
    use support::MutableComponent;

    let body: Vec<String> = (0..40)
        .map(|i| format!("预览标题{i}测宽与排版混合正文"))
        .collect();
    let lines = std::rc::Rc::new(std::cell::RefCell::new(body.clone()));
    let term = LoggingVirtualTerminal::new(80, 24);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(MutableComponent {
        lines: lines.clone(),
    }));

    tui.render_frame().expect("warm");
    assert_eq!(tui.finalize_width_checks_for_test(), 40);
    assert_eq!(tui.finalize_line_reuses_for_test(), 0);

    tui.clear_finalize_counters_for_test();
    tui.render_frame().expect("identical frame");
    assert_eq!(
        tui.finalize_line_reuses_for_test(),
        40,
        "stable content must reuse previous finalized lines"
    );
    assert_eq!(
        tui.finalize_width_checks_for_test(),
        0,
        "reused lines must not re-run visible_width"
    );

    tui.clear_finalize_counters_for_test();
    lines.borrow_mut()[7] = "仅改一行".into();
    tui.render_frame().expect("one line changed");
    assert_eq!(tui.finalize_width_checks_for_test(), 1);
    assert_eq!(tui.finalize_line_reuses_for_test(), 39);
}

#[test]
fn resize_still_width_checks_all_lines() {
    use support::MutableComponent;

    let lines = std::rc::Rc::new(std::cell::RefCell::new(vec![
        "你好世界".to_string(),
        "ok".to_string(),
    ]));
    let term = LoggingVirtualTerminal::new(20, 4);
    let mut tui = TUI::new(term);
    tui.add_child(Box::new(MutableComponent {
        lines: lines.clone(),
    }));
    tui.render_frame().expect("wide");
    tui.clear_finalize_counters_for_test();

    tui.terminal.set_size_hint(6, 4);
    let err = tui
        .render_frame()
        .expect_err("resize to underflow must re-check width");
    assert!(err.line_width > err.width);
    assert!(
        tui.finalize_line_reuses_for_test() == 0,
        "must not reuse finalized lines across width change"
    );
}

#[test]
fn overlay_preserves_trailing_content_styling() {
    use xylitol_tui::{OverlayAnchor, OverlayOptions};

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

    assert_eq!(tui.terminal.cell(0, 0).ch, 'X');
    assert_eq!(tui.terminal.cell(0, 2).ch, 'Z');
    assert_eq!(tui.terminal.cell(0, 3).ch, 'l');
    assert_eq!(tui.terminal.cell(0, 4).ch, 'o');
}

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

    tui.render_frame().unwrap();
    let vp = tui.terminal.viewport();
    assert_eq!(vp[0], "line1");
    assert_eq!(vp[1], "line2");

    *lines.borrow_mut() = vec![
        "line1".to_string(),
        "line2".to_string(),
        "line3".to_string(),
        "line4".to_string(),
        "line5".to_string(),
    ];
    tui.render_frame().unwrap();

    let vp = tui.terminal.viewport();
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

    *lines.borrow_mut() = vec!["stable".to_string(), "appended".to_string()];
    tui.terminal.clear_writes();
    tui.render_frame().unwrap();

    let vp = tui.terminal.viewport();
    assert_eq!(vp[0], "stable", "unchanged top line must stay");
    assert_eq!(vp[1], "appended", "appended line must render");
}

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

#[test]
fn input_render_handles_cjk_in_visible_window_without_panic() {
    use xylitol_tui::components::input::Input;
    use xylitol_tui::tui::Component;

    let mut input = Input::new();
    input.set_focused(true);
    feed_vt(&mut input, "你好世界测试");
    let lines = input.render(10);
    assert_eq!(lines.len(), 1, "input renders a single line");
    assert!(!lines[0].is_empty());
}

#[test]
fn mouse_capture_default_off_and_pairs_on_stop() {
    let mut vt = LoggingVirtualTerminal::new(20, 5);
    assert!(!vt.mouse_capture_active());
    assert_eq!(vt.mouse_enable_calls(), 0);
    assert_eq!(vt.mouse_disable_calls(), 0);

    vt.start();
    assert!(
        !vt.mouse_capture_active(),
        "start alone must not enable mouse capture"
    );

    vt.enable_mouse_capture();
    assert!(vt.mouse_capture_active());
    assert_eq!(vt.mouse_enable_calls(), 1);

    vt.stop();
    assert!(
        !vt.mouse_capture_active(),
        "stop must release active mouse capture"
    );
    assert_eq!(vt.mouse_disable_calls(), 1);

    vt.start();
    assert!(
        vt.mouse_capture_active(),
        "start after prior enable must restore desired capture"
    );
    assert_eq!(vt.mouse_enable_calls(), 2);

    vt.disable_mouse_capture();
    assert!(!vt.mouse_capture_active());
    assert_eq!(vt.mouse_disable_calls(), 2);
}
