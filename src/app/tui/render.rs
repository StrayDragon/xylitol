//! XyEvent → UI rendering for the inline TUI.
//!
//! Responsibilities after c365 componentization:
//! - [`draw_tail_frame`]: a thin wrapper that delegates to the [`Tail`] widget
//!   (which composes `MutableLine` / `ThinkingIndicator` / `InputPrompt`) and
//!   positions the frame cursor on the input line. All tail rendering logic
//!   lives in `components/`.
//! - the [`RenderedLine`] UI-data type + the single `xyevent_to_rendered` seam
//!   (spec tui42: rendering consumes only `RenderedLine`, never `XyEvent`).
//! - wrapping helpers (`wrap_to_width` / `wrap_line_to_width`) shared by the
//!   commit path and the widgets.
//! - [`StatusLine`] (spinner glyph + label carrier).
//!
//! Pure functions are preferred (no terminal side effects) so they can be
//! unit-tested without a real terminal. The c365 escape-sequence path
//! (`raw_render.rs`) was removed: streaming text now grows in the ratatui
//! buffer (transparent-bg mutable line at the tail top), verifiable via
//! TestBackend.

use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::terminal::Frame;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;

use crate::app::tui::app::TuiApp;
use crate::app::tui::components::tail::input_cursor_position;
use crate::app::tui::components::{Tail, TranscriptLine};
use crate::app::tui::theme;
use crate::domain::lifecycle::XyEvent;

/// Render the mutable tail region (the `Viewport::Inline` area).
///
/// Delegates to the [`Tail`] widget (c365 componentization): `MutableLine`
/// (pending streaming text, transparent bg) → `ThinkingIndicator` →
/// `InputPrompt`, bottom-anchored. The cursor is positioned on the input line.
pub fn draw_tail_frame(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let width = area.width;
    Tail::new(app, width).render(area, frame.buffer_mut());
    let (x, y) = input_cursor_position(area, app);
    frame.set_cursor_position((x, y));
}

// ── UI/UX ↔ business-flow boundary (spec tui42) ──────────────────────────
//
// `RenderedLine` is the UI-only data type. Business events (`XyEvent`) are
// translated into `RenderedLine` at a SINGLE seam (`xyevent_to_rendered`); the
// rendering layer (widgets) consumes only `RenderedLine` and never matches
// `XyEvent` variants or calls Driver/agent methods.

/// A UI-only representation of one finalized scrollback line.
///
/// This is the type the rendering layer consumes. Adding a new message kind is
/// a new variant + a `to_line` arm; business-event churn does not touch the
/// rendering layer (the seam function absorbs it).
pub enum RenderedLine {
    /// The user's submitted prompt, echoed into history.
    UserInput(String),
    /// A finalized assistant text line (streamed text committed on boundary).
    AssistantText(String),
    /// A tool-execution summary line (name + result preview + error flag).
    ToolSummary {
        name: String,
        preview: String,
        is_error: bool,
    },
    /// A status/notification line (model switch, generic error, etc.).
    Status(String),
}

impl RenderedLine {
    /// Render this UI data into a styled ratatui `Line`. The styling lives here
    /// (UI concern), not in the business layer.
    pub fn to_line(&self) -> Line<'static> {
        let p = theme::palette();
        match self {
            RenderedLine::UserInput(prompt) => Line::styled(format!("❯ {prompt}"), p.user_prompt()),
            RenderedLine::AssistantText(text) => Line::styled(text.clone(), p.assistant()),
            RenderedLine::ToolSummary {
                name,
                preview,
                is_error,
            } => {
                let style = if *is_error { p.error() } else { p.tool() };
                Line::styled(format!("[{name}] {preview}"), style)
            }
            RenderedLine::Status(msg) => Line::styled(msg.clone(), p.text_dim()),
        }
    }
}

/// The SINGLE seam: translate a business `XyEvent` into UI-only `RenderedLine`s.
///
/// This is the only place the rendering layer learns about `XyEvent`. Everything
/// downstream (`to_line`, the widgets) consumes `RenderedLine` and is insulated
/// from domain-event vocabulary churn (spec tui42).
pub fn xyevent_to_rendered(event: &XyEvent) -> Vec<RenderedLine> {
    match event {
        XyEvent::ToolExecutionEnd {
            name,
            result,
            is_error,
            ..
        } => {
            let preview: String = result.lines().take(3).collect::<Vec<_>>().join("\n");
            let preview = if result.lines().count() > 3 {
                format!("{preview}…")
            } else {
                preview
            };
            vec![RenderedLine::ToolSummary {
                name: name.clone(),
                preview,
                is_error: *is_error,
            }]
        }
        XyEvent::Error(msg) => vec![RenderedLine::Status(format!("error: {msg}"))],
        XyEvent::ModelSelect { model_id, .. } => {
            vec![RenderedLine::Status(format!("model: {model_id}"))]
        }
        // TextDelta/ThinkingDelta/ToolStart/ToolUpdate/TurnStart/TurnEnd do not
        // produce finalized scrollback lines here — TextDelta is accumulated by
        // TuiApp and committed on boundaries; others update tail status only.
        _ => Vec::new(),
    }
}

/// The user's submitted prompt as a [`RenderedLine`] (修复 c340 §7 #3).
///
/// Called on Enter before `driver.run`, so the user sees their own message in
/// the conversation history above the streaming reply. Committed via the same
/// `insert_before` path as assistant/tool lines (c365: unified commit).
pub fn user_message_rendered(prompt: &str) -> RenderedLine {
    RenderedLine::UserInput(prompt.to_string())
}

/// Wrap a single logical line of text into multiple physical lines that each
/// fit within `width` display columns. Honors CJK double-width characters via
/// `unicode_width`.
pub fn wrap_to_width(text: &str, width: u16) -> Vec<String> {
    use unicode_width::UnicodeWidthChar;
    let max_w = width as usize;
    if max_w == 0 {
        return vec![text.to_string()];
    }
    let mut rows: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w: usize = 0;
    for ch in text.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if w == 0 {
            // Zero-width (combining marks etc.) — append without advancing.
            cur.push(ch);
            continue;
        }
        if cur_w + w > max_w && !cur.is_empty() {
            rows.push(std::mem::take(&mut cur));
            cur_w = 0;
        }
        cur.push(ch);
        cur_w += w;
    }
    rows.push(cur);
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

/// Wrap a styled [`Line`] into multiple [`Line`]s that each fit `width`.
/// Preserves the original style on every wrapped row.
pub fn wrap_line_to_width(line: &Line<'_>, width: u16) -> Vec<Line<'static>> {
    let full: String = line.spans.iter().map(|s| s.content.to_string()).collect();
    let style = line.spans.first().map(|s| s.style).unwrap_or_default();
    wrap_to_width(&full, width)
        .into_iter()
        .map(|row| Line::styled(row, style))
        .collect()
}

/// Internal helper: a status line carries a glyph + label.
pub struct StatusLine {
    glyph: &'static str,
    label: String,
}

impl StatusLine {
    pub fn new(glyph: &'static str, label: impl Into<String>) -> Self {
        Self {
            glyph,
            label: label.into(),
        }
    }

    /// The label text (used by the ThinkingIndicator widget to build the label).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The glyph (e.g. "⚙", "✓"). Kept for future richer status rendering.
    #[allow(dead_code)]
    pub fn glyph(&self) -> &'static str {
        self.glyph
    }

    pub fn render(&self, glyph_style: Style, label_style: Style) -> Line<'static> {
        Line::from(vec![
            Span::styled(self.glyph.to_string(), glyph_style),
            Span::styled(self.label.clone(), label_style),
        ])
    }
}

/// Total physical row count a slice of [`RenderedLine`]s occupies at `width`
/// (each wrapped CJK-aware via [`TranscriptLine`]). Used by the commit path to
/// size the `insert_before` area before rendering.
pub fn commit_height(lines: &[RenderedLine], width: u16) -> u16 {
    lines
        .iter()
        .map(|l| TranscriptLine::new(l, width).row_count())
        .sum::<u16>()
        .max(1)
}

/// Render a slice of finalized [`RenderedLine`]s directly into a [`Buffer`],
/// stacked top-to-bottom, each wrapped to `width` via [`TranscriptLine`].
///
/// This is the pure rendering core for the `insert_before` commit path, shared
/// with the `TestBackend` harness (spec tui41). `InlineTerminal::commit_to_scrollback`
/// calls this inside its `insert_before` closure; tests call it on a
/// `TestBackend` buffer.
pub fn render_commit_lines_into_buf(
    lines: &[RenderedLine],
    width: u16,
    buf: &mut ratatui_core::buffer::Buffer,
) {
    let mut y = 0u16;
    for line in lines {
        let widget = TranscriptLine::new(line, width);
        let h = widget.row_count().max(1);
        widget.render(Rect::new(0, y, width, h), buf);
        y = y.saturating_add(h);
    }
}

#[cfg(test)]
mod user_message_tests {
    use super::{user_message_rendered, wrap_to_width};

    #[test]
    fn user_message_contains_prompt_with_prefix() {
        let line = user_message_rendered("fix the bug").to_line();
        let text = line.to_string();
        assert!(text.contains('❯'), "prefix present: {text}");
        assert!(text.contains("fix the bug"), "prompt text present: {text}");
    }

    #[test]
    fn user_message_empty_prompt_still_has_prefix() {
        let line = user_message_rendered("").to_line();
        let text = line.to_string();
        assert!(text.contains('❯'), "prefix present even for empty: {text}");
    }

    #[test]
    fn wrap_short_text_one_row() {
        let rows = wrap_to_width("hello", 80);
        assert_eq!(rows, vec!["hello"]);
    }

    #[test]
    fn wrap_long_text_splits_at_width() {
        let rows = wrap_to_width("abcdefghij", 4);
        assert_eq!(rows, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn wrap_cjk_uses_display_width_not_char_count() {
        let rows = wrap_to_width("你好世界再见", 4);
        assert_eq!(rows, vec!["你好", "世界", "再见"]);
    }

    #[test]
    fn wrap_empty_text_returns_one_empty_row() {
        let rows = wrap_to_width("", 80);
        assert_eq!(rows, vec![""]);
    }
}

#[cfg(test)]
mod commit_harness {
    //! TestBackend harness for the insert_before commit path (spec tui41),
    //! now via the `TranscriptLine` widget (c365 componentization).
    //!
    //! Covers `render_commit_lines_into_buf` via `Terminal::insert_before` +
    //! `Viewport::Inline` — the path that commits finalized scrollback lines.
    //! These tests PIN the CJK + wrapping behavior so the widget selection
    //! decision stays safe: whichever rendering implementation is chosen, the
    //! assertions (behavior, not implementation) must still pass.

    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::buffer::CellWidth;
    use ratatui_core::terminal::{Terminal, TerminalOptions, Viewport};

    /// Build an inline terminal (width × height), draw the tail from `app`,
    /// commit `lines` via `insert_before` + `render_commit_lines_into_buf`,
    /// then redraw the tail. Returns the terminal for scrollback assertions.
    fn commit_and_render(
        app: &TuiApp,
        width: u16,
        height: u16,
        lines: &[RenderedLine],
    ) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        let mut term = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(height),
            },
        )
        .unwrap();
        term.draw(|f| draw_tail_frame(f, app)).unwrap();
        let commit_height: u16 = lines
            .iter()
            .map(|l| TranscriptLine::new(l, width).row_count())
            .sum::<u16>()
            .max(1);
        term.insert_before(commit_height, |buf| {
            render_commit_lines_into_buf(lines, width, buf)
        })
        .unwrap();
        term.draw(|f| draw_tail_frame(f, app)).unwrap();
        term
    }

    /// Read a buffer row as plain text, collapsing ratatui double-width filler
    /// cells (implementation-agnostic: works whether rendering uses a library
    /// widget or a hand-roll).
    fn row_text(buf: &ratatui_core::buffer::Buffer, y: u16, width: u16) -> String {
        let mut out = String::new();
        let mut prev_width: u16 = 1;
        for x in 0..width {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            let is_filler =
                sym.is_empty() || (sym == " " && prev_width == 2) || cell.cell_width() == 0;
            if is_filler {
                continue;
            }
            if let Some(ch) = sym.chars().next() {
                out.push(ch);
            }
            prev_width = cell.cell_width().max(1) as u16;
        }
        out.trim_end().to_string()
    }

    #[test]
    fn commit_ascii_line_appears_in_scrollback() {
        let app = TuiApp::default();
        let line = RenderedLine::AssistantText("hello world".into());
        let term = commit_and_render(&app, 40, 10, &[line]);
        let sb = term.backend().scrollback();
        let found = (0..sb.area.height).any(|y| row_text(sb, y, 40).contains("hello world"));
        assert!(found, "committed ASCII line missing from scrollback");
    }

    #[test]
    fn commit_cjk_line_double_width_visible() {
        let app = TuiApp::default();
        let line = RenderedLine::AssistantText("你好".into());
        let term = commit_and_render(&app, 40, 10, &[line]);
        let sb = term.backend().scrollback();
        let found = (0..sb.area.height).any(|y| row_text(sb, y, sb.area.width).contains("你好"));
        assert!(found, "committed CJK line missing from scrollback");
    }

    #[test]
    fn commit_long_line_wraps_to_width() {
        let app = TuiApp::default();
        let line = RenderedLine::AssistantText("abcdefghij".into());
        let term = commit_and_render(&app, 4, 10, &[line]);
        let sb = term.backend().scrollback();
        assert_eq!(row_text(sb, 0, 4), "abcd");
        assert_eq!(row_text(sb, 1, 4), "efgh");
        assert_eq!(row_text(sb, 2, 4), "ij");
    }

    #[test]
    fn commit_cjk_long_line_wraps_by_display_width() {
        let app = TuiApp::default();
        let line = RenderedLine::AssistantText("你好世界再见".into());
        let term = commit_and_render(&app, 4, 10, &[line]);
        let sb = term.backend().scrollback();
        let w = sb.area.width;
        assert_eq!(row_text(sb, 0, w), "你好");
        assert_eq!(row_text(sb, 1, w), "世界");
        assert_eq!(row_text(sb, 2, w), "再见");
    }

    #[test]
    fn after_commit_tail_shows_only_input_prompt() {
        let app = TuiApp::default();
        let line = RenderedLine::AssistantText("assistant reply".into());
        let term = commit_and_render(&app, 40, 10, &[line]);
        let buf = term.backend().buffer();
        // With TAIL_HEIGHT=6 the tail is rows 4-9: border@4, input@8, border@9.
        // The input row (8) must contain the ❯ prompt, and must NOT contain the
        // committed reply text (that lives in scrollback above).
        let input_row = row_text(buf, 8, 40);
        assert!(
            input_row.contains('❯'),
            "tail input line missing prompt: {input_row}"
        );
        assert!(
            !input_row.contains("assistant reply"),
            "tail should not show committed reply text: {input_row}"
        );
    }
}

#[cfg(test)]
mod cursor_tests {
    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::terminal::Terminal;

    fn render_term(app: &TuiApp) -> Terminal<TestBackend> {
        let mut term = Terminal::new(TestBackend::new(80, 24)).unwrap();
        term.draw(|f| draw_tail_frame(f, app)).unwrap();
        term
    }

    fn row_text(buf: &ratatui_core::buffer::Buffer, y: u16) -> String {
        (0..80u16)
            .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn cursor_after_prompt_prefix_when_idle_empty() {
        let app = TuiApp::default();
        let mut term = render_term(&app);
        // Idle panel fills the 6-row tail (rows 18-23): border@18, input@22,
        // border@23. Cursor sits after the `❯ ` prefix on the input row.
        term.backend_mut().assert_cursor_position((2u16, 22u16));
    }

    #[test]
    fn diag_chinese_input_row_cells() {
        let mut app = TuiApp::default();
        for c in "你好".chars() {
            app.push_char(c);
        }
        let term = render_term(&app);
        let buf = term.backend().buffer();
        // Input is on row 22 (panel inner bottom, above the bottom border@23).
        let cells: Vec<String> = (0..12u16)
            .map(|x| buf[(x, 22)].symbol().to_string())
            .collect();
        assert_eq!(cells[0], "❯");
        assert_eq!(cells[1], " ");
        assert_eq!(cells[2], "你");
        assert_eq!(cells[4], "好", "cells: {cells:?}");
    }

    #[test]
    fn cursor_after_typed_input() {
        let mut app = TuiApp::default();
        for c in "abc".chars() {
            app.push_char(c);
        }
        let mut term = render_term(&app);
        // `❯ abc` on row 22: cursor at col 2 + 3 = 5.
        term.backend_mut().assert_cursor_position((5u16, 22u16));
    }

    #[test]
    fn cursor_after_chinese_input() {
        let mut app = TuiApp::default();
        for c in "你好".chars() {
            app.push_char(c);
        }
        let mut term = render_term(&app);
        // `❯ 你好` on row 22: 2 + 4 = 6.
        term.backend_mut().assert_cursor_position((6u16, 22u16));
    }

    #[test]
    fn input_row_contains_prompt() {
        let app = TuiApp::default();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        // Input sits on row 22 (panel inner bottom, above the bottom border@23).
        let input_row: String = (0..80u16)
            .map(|x| buf[(x, 22)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(input_row.contains('❯'), "input row 22: {input_row:?}");
        // The bottom border is on row 23.
        let border_row: String = (0..80u16)
            .map(|x| buf[(x, 23)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(
            border_row.starts_with('─'),
            "bottom border row 23: {border_row:?}"
        );
    }

    #[test]
    fn streaming_shows_status_and_thinking_above_input() {
        let mut app = TuiApp::default();
        app.start_stream();
        // Push a pending tail so the panel anchors to the bottom (rows 19-23):
        // mutable@18, border@19, status@20, thinking@21, input@22, border@23.
        app.handle_xy_event(XyEvent::TextDelta("x".into()));
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let status = row_text(buf, 20);
        let thinking = row_text(buf, 21);
        let input = row_text(buf, 22);
        assert!(status.starts_with('⠋'), "status row 20: {status:?}");
        assert!(status.contains("Working"), "status label: {status:?}");
        assert_eq!(thinking, "Thinking…", "thinking row 21: {thinking:?}");
        assert!(input.starts_with('❯'), "input row 22: {input:?}");
    }

    /// c365 buffer route: streaming text IS in the ratatui buffer (the mutable
    /// line at the tail top), NOT escape-written outside it. This reverses the
    /// escape-era test that asserted the buffer had NO streaming text. The
    /// mutable line is top-anchored (row 20 — the viewport top in a 24-row
    /// terminal with TAIL_HEIGHT=4), flush against the scrollback above.
    #[test]
    fn streaming_text_is_in_ratatui_buffer_mutable_row() {
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("typing-stream-text".into()));
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let full: String = (0..24u16)
            .map(|y| row_text(buf, y))
            .collect::<Vec<_>>()
            .join("|");
        assert!(
            full.contains("typing-stream-text"),
            "streaming text must be in the ratatui buffer (mutable row): {full:?}"
        );
    }

    #[test]
    fn idle_tail_has_no_pending_or_thinking() {
        let app = TuiApp::default();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let any_spinner = (0..24u16).any(|y| row_text(buf, y).contains('⠋'));
        assert!(!any_spinner, "idle should show no spinner");
    }

    #[test]
    fn idle_has_no_thinking_indicator() {
        let app = TuiApp::default();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let any_thinking = (0..24u16).any(|y| row_text(buf, y).contains("Thinking"));
        assert!(!any_thinking, "idle should not show Thinking indicator");
    }

    #[test]
    fn input_line_has_background() {
        let app = TuiApp::default();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        // Input row 22 carries the panel bg (the bottom-panel fill).
        let cell = &buf[(0, 22)];
        assert_ne!(
            cell.bg,
            ratatui_core::style::Color::Reset,
            "input line cell should have a panel background"
        );
    }

    #[test]
    fn input_row_carries_panel_background() {
        let app = TuiApp::default();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let expected_bg = crate::app::tui::theme::palette().panel_bg();
        for x in 0..80u16 {
            assert_eq!(
                buf[(x, 22)].bg,
                expected_bg,
                "input row 22 col {x} should have panel_bg"
            );
        }
    }

    /// c365 route B: the mutable streaming row is transparent (no panel bg)
    /// so it blends into the scrollback; the panel inner rows (status,
    /// thinking, input) carry the panel bg. In an 80×24 terminal with
    /// TAIL_HEIGHT=6: mutable@18, border@19, status@20, thinking@21, input@22.
    #[test]
    fn streaming_mutable_transparent_panel_inner_has_bg() {
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("abcdefghij".into()));
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let bg = crate::app::tui::theme::palette().panel_bg();
        // Panel inner rows (status 20, thinking 21, input 22) carry panel bg.
        assert_eq!(buf[(0, 20)].bg, bg, "status row has bg");
        assert_eq!(buf[(0, 21)].bg, bg, "thinking row has bg");
        assert_eq!(buf[(0, 22)].bg, bg, "input row has bg");
        // Mutable row (18, flush above the panel border) does NOT carry panel
        // bg — it blends into the scrollback body.
        assert_eq!(
            buf[(0, 18)].bg,
            ratatui_core::style::Color::Reset,
            "mutable row 18 should be transparent, text={:?}",
            row_text(buf, 18)
        );
        // The mutable row carries the streaming text.
        assert!(
            row_text(buf, 18).contains("abcdefghij"),
            "mutable row has the streaming text: {:?}",
            row_text(buf, 18)
        );
    }
}
