//! XyEvent → ratatui rendering for the inline TUI.
//!
//! Two responsibilities:
//! - [`draw_tail_frame`]: paint the mutable tail region (input line, spinner,
//!   current streaming line, last tool status) via `Terminal::draw`. This is
//!   freely redrawable each frame.
//! - committed scrollback lines are produced by [`app::TuiApp`] (see
//!   `commit_lines_for`) and flushed via `Terminal::insert_before`; this module
//!   only formats them into [`Line`] values.
//!
//! Pure functions are preferred (no terminal side effects) so they can be
//! unit-tested without a real terminal.

use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::terminal::Frame;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;

use crate::app::tui::app::TuiApp;
use crate::app::tui::theme;
use crate::domain::lifecycle::XyEvent;

/// Render the mutable tail region (the `Viewport::Inline` area).
///
/// pi-style layout, bottom-aligned within the tail area:
///   - streaming assistant text   (only while a turn streams)
///   - thinking/loading indicator (only while a turn streams; spinner + dim italic)
///   - ❯ input prompt             (always; subtle background block)
pub fn draw_tail_frame(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let p = theme::palette();

    // ── Fill the entire tail region with the input background (修复 c340 §7 #1)
    // ── so it reads as a continuous bottom-anchored block with no floating
    // gap. Without this, Viewport::Inline(3) leaves visually-empty rows above
    // the bottom-aligned content, giving a "not anchored to bottom"错觉.
    let bg = p.input_bg();
    let buf = frame.buffer_mut();
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            buf[(x, y)].set_bg(bg);
        }
    }

    // ── Compose lines top-to-bottom ───────────────────────────────
    let mut lines: Vec<Line> = Vec::new();

    if app.is_streaming() {
        // Streaming assistant text (the in-progress line), normal style.
        if let Some(text) = app.current_streaming_line()
            && !text.is_empty()
        {
            lines.push(Line::styled((*text).to_string(), p.assistant()));
        }

        // Thinking/loading indicator: spinner glyph + italic dim label.
        // Prefers a concrete tool/status line when one is active, else shows
        // the generic "Thinking…" label so the user sees activity.
        let indicator = app.indicator_label();
        lines.push(Line::styled(indicator, p.thinking()));
    }

    // ── Input prompt line (pi-style background block) ─────────────
    // The input line is ALWAYS active and pinned to the bottom row so the user
    // can type at any time — even mid-stream (Enter interrupts + sends a new
    // prompt). Cursor always sits here, never on the streaming text.
    let prompt_text = format!("❯ {}", app.input_buffer());
    let input_style = Style::default().bg(p.input_bg());
    lines.push(Line::styled(prompt_text, input_style));

    // ── Bottom-align and render ───────────────────────────────────
    // Render each line directly via `Line::render` (Line implements Widget in
    // ratatui-core) instead of the Paragraph built-in widget (c341: drop all
    // built-in widgets; hand-roll for inline mode).
    let n = u16::try_from(lines.len()).unwrap_or(area.height);
    let top = area.y + area.height.saturating_sub(n);
    let buf = frame.buffer_mut();
    for (i, line) in lines.iter().enumerate() {
        let row_y = top.saturating_add(i as u16);
        if row_y >= area.bottom() {
            break;
        }
        let row_area = Rect {
            x: area.x,
            y: row_y,
            width: area.width,
            height: 1,
        };
        line.render(row_area, buf);
    }

    // ── Cursor placement (always, on the input line) ──────────────
    // The cursor ALWAYS sits on the input prompt — never on streaming text.
    // Absolute terminal coords; display width so CJK aligns correctly.
    use unicode_width::UnicodeWidthStr;
    let prefix_len = UnicodeWidthStr::width("❯ ") as u16;
    let input_len = UnicodeWidthStr::width(app.input_buffer()) as u16;
    let y = area.y + area.height.saturating_sub(1);
    let x = area.x + prefix_len + input_len;
    frame.set_cursor_position((x.min(area.right().saturating_sub(1)), y));
}

/// Decide whether an [`XyEvent`] finalizes one or more scrollback lines.
///
/// Returns the formatted lines to commit (then `insert_before` flushes them).
/// TextDelta is accumulated by `TuiApp`; here we only emit lines for events
/// that complete a block: ToolExecutionEnd (summary), Error, ModelSelect.
pub fn commit_lines_for(event: &XyEvent) -> Vec<Line<'static>> {
    let p = theme::palette();
    match event {
        XyEvent::ToolExecutionEnd {
            name,
            result,
            is_error,
            ..
        } => {
            let style = if *is_error { p.error() } else { p.tool() };
            let preview: String = result.lines().take(3).collect::<Vec<_>>().join("\n");
            let suffix = if result.lines().count() > 3 {
                "…"
            } else {
                ""
            };
            vec![Line::styled(format!("[{name}] {preview}{suffix}"), style)]
        }
        XyEvent::Error(msg) => vec![Line::styled(format!("error: {msg}"), p.error())],
        XyEvent::ModelSelect { model_id, .. } => {
            vec![Line::styled(format!("model: {model_id}"), p.text_dim())]
        }
        _ => Vec::new(),
    }
}

/// Format the user's submitted prompt as a scrollback line (修复 c340 §7 #3).
///
/// Called on Enter before `driver.run`, so the user sees their own message in
/// the conversation history above the streaming reply. Styled distinctly
/// (bold cyan `❯` prefix) to distinguish from the assistant reply.
pub fn user_message_line(prompt: &str) -> Line<'static> {
    Line::styled(format!("❯ {prompt}"), theme::palette().user_prompt())
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

    /// The label text (used by the TUI app to build the indicator).
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn render(&self, glyph_style: Style, label_style: Style) -> Line<'static> {
        Line::from(vec![
            Span::styled(self.glyph.to_string(), glyph_style),
            Span::styled(self.label.clone(), label_style),
        ])
    }
}

#[cfg(test)]
mod user_message_tests {
    use super::user_message_line;

    #[test]
    fn user_message_contains_prompt_with_prefix() {
        let line = user_message_line("fix the bug");
        // The line carries the ❯ prefix and the prompt text.
        let text = line.to_string();
        assert!(text.contains('❯'), "prefix present: {text}");
        assert!(text.contains("fix the bug"), "prompt text present: {text}");
    }

    #[test]
    fn user_message_empty_prompt_still_has_prefix() {
        let line = user_message_line("");
        let text = line.to_string();
        assert!(text.contains('❯'), "prefix present even for empty: {text}");
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

    /// Idle + empty input: cursor sits right after the "❯ " prompt prefix.
    #[test]
    fn cursor_after_prompt_prefix_when_idle_empty() {
        let app = TuiApp::default();
        let mut term = render_term(&app);
        term.backend_mut().assert_cursor_position((2u16, 23u16));
    }

    /// Diagnostic: render Chinese input and dump the raw input-row cells to see
    /// whether CJK chars are stored with spurious spacing.
    #[test]
    fn diag_chinese_input_row_cells() {
        let mut app = TuiApp::default();
        for c in "你好".chars() {
            app.push_char(c);
        }
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let cells: Vec<String> = (0..12u16)
            .map(|x| buf[(x, 23)].symbol().to_string())
            .collect();
        // CJK chars occupy 2 cells each in the buffer: "你" at x=2 (with its
        // second column as an empty filler at x=3), "好" at x=4. This is
        // ratatui's correct double-width handling; a real terminal renders
        // them contiguously without visible gaps.
        assert_eq!(cells[0], "❯");
        assert_eq!(cells[1], " ");
        assert_eq!(cells[2], "你");
        assert_eq!(cells[4], "好", "cells: {cells:?}");
    }

    /// With typed input "abc", the cursor sits after "❯ abc" (col 5).
    #[test]
    fn cursor_after_typed_input() {
        let mut app = TuiApp::default();
        for c in "abc".chars() {
            app.push_char(c);
        }
        let mut term = render_term(&app);
        term.backend_mut().assert_cursor_position((5u16, 23u16));
    }

    /// Chinese input "你好" (2 chars, each 2 display cols wide): cursor sits
    /// after "❯ 你好" → col 2 + 4 = 6. Guards the CJK display-width fix.
    #[test]
    fn cursor_after_chinese_input() {
        let mut app = TuiApp::default();
        for c in "你好".chars() {
            app.push_char(c);
        }
        let mut term = render_term(&app);
        term.backend_mut().assert_cursor_position((6u16, 23u16));
    }

    /// The prompt glyph must be present in the rendered grid's last line.
    #[test]
    fn last_row_contains_prompt() {
        let app = TuiApp::default();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        // Dump every non-empty row to see where content actually landed.
        let mut dump = String::new();
        for y in 0..24u16 {
            let row: String = (0..80u16)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            let trimmed = row.trim_end();
            if !trimmed.is_empty() {
                dump.push_str(&format!("row {y:>2}: {trimmed:?}\n"));
            }
        }
        let last_row: String = (0..80u16)
            .map(|x| buf[(x, 23)].symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(
            last_row.contains('❯'),
            "last row: {last_row:?}\nfull grid dump:\n{dump}"
        );
    }

    /// While streaming, the tail shows a thinking indicator line ABOVE the
    /// input line (pi-style). The indicator carries the spinner glyph.
    #[test]
    fn streaming_shows_thinking_above_input() {
        let mut app = TuiApp::default();
        app.start_stream();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let row_text = |y: u16| -> String {
            (0..80u16)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect::<String>()
                .trim_end()
                .to_string()
        };
        // Input line is the last row (23) and stays active (❯); indicator must
        // be the row directly above it (22).
        let input = row_text(23);
        let indicator = row_text(22);
        assert!(indicator.starts_with('⠋'), "indicator row: {indicator:?}");
        assert!(input.starts_with('❯'), "input row: {input:?}");
    }

    /// While idle, there is NO thinking indicator — only the input line.
    #[test]
    fn idle_has_no_thinking_indicator() {
        let app = TuiApp::default();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let any_thinking = (0..24u16).any(|y| {
            (0..80u16)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect::<String>()
                .contains("Thinking")
        });
        assert!(!any_thinking, "idle should not show Thinking indicator");
    }

    /// The input line carries a background color (pi-style Box block).
    #[test]
    fn input_line_has_background() {
        let app = TuiApp::default();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        // The prompt cell should have a non-default background.
        let cell = &buf[(0, 23)];
        assert_ne!(
            cell.bg,
            ratatui_core::style::Color::Reset,
            "input line cell should have a background block"
        );
    }

    /// 修复 c340 §7 #1: the entire tail region (all 3 rows of Viewport::Inline(3))
    /// carries the input background, so the area reads as one continuous
    /// bottom-anchored block with no floating gap.
    #[test]
    fn tail_region_fully_filled_with_background() {
        let app = TuiApp::default();
        let term = render_term(&app);
        let buf = term.backend().buffer();
        let expected_bg = crate::app::tui::theme::palette().input_bg();
        // Tail = bottom 3 rows of a 24-row terminal: rows 21, 22, 23.
        for y in 21..24u16 {
            for x in 0..80u16 {
                assert_eq!(
                    buf[(x, y)].bg,
                    expected_bg,
                    "row {y} col {x} should have input_bg (filled tail)"
                );
            }
        }
    }
}
