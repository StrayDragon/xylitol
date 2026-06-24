//! TuiRenderer — differential render loop that tracks previous lines and writes
//! only changed ANSI output to stdout.
//!
//! This also handles CURSOR_MARKER stripping and hardware cursor positioning.
//!
//! # Usage
//!
//! ```ignore
//! let mut renderer = TuiRenderer::new(stdout);
//! loop {
//!     let lines = compose_layout(&app, width, height);
//!     renderer.render(&lines, width, height)?;
//! }
//! ```

use std::io::{self, Write};

use crate::interface::tui::engine::ansi;
use crate::interface::tui::engine::diff;

/// Differential renderer that writes minimal ANSI output to update the terminal.
pub(crate) struct TuiRenderer<W: Write> {
    prev_lines: Vec<String>,
    prev_width: u16,
    prev_height: u16,
    cursor_row: u16,
    cursor_col: u16,
    writer: W,
    first_render: bool,
}

impl<W: Write> TuiRenderer<W> {
    pub(crate) fn new(writer: W) -> Self {
        Self {
            prev_lines: Vec::new(),
            prev_width: 0,
            prev_height: 0,
            cursor_row: 0,
            cursor_col: 0,
            writer,
            first_render: true,
        }
    }

    pub(crate) fn writer(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Handle terminal resize — force a full redraw.
    pub(crate) fn on_resize(&mut self, new_width: u16, new_height: u16) {
        self.prev_lines.clear();
        self.prev_width = new_width;
        self.prev_height = new_height;
        self.first_render = true;
    }

    /// Render new lines. Strips CURSOR_MARKER and positions hardware cursor.
    pub(crate) fn render(
        &mut self,
        lines: &[String],
        width: u16,
        height: u16,
    ) -> io::Result<usize> {
        let mut written = 0;

        // ── Step 1: Strip CURSOR_MARKER and find cursor position ──
        let (clean_lines, cursor_pos) = strip_cursor_marker(lines);
        let cursor_row = cursor_pos.map(|(r, _)| r);
        let cursor_col = cursor_pos.map(|(_, c)| c);

        // ── Step 2: Full or differential render ───────────────────
        if self.first_render || width != self.prev_width || height != self.prev_height {
            written += self.full_redraw(&clean_lines, cursor_row, cursor_col)?;
        } else {
            let output =
                diff::build_diff_output(&self.prev_lines, &clean_lines, self.cursor_row, 0);
            if !output.is_empty() {
                self.writer.write_all(&output)?;
                written += output.len();
            }
            // Position cursor even if no content changed
            if let Some((r, c)) = cursor_pos {
                let pos = ansi::cursor_goto(r + 1, c + 1);
                self.writer.write_all(pos.as_bytes())?;
                written += pos.len();
                self.cursor_row = r;
                self.cursor_col = c;
            }
        }

        self.writer.flush()?;
        self.prev_lines = clean_lines;
        self.prev_width = width;
        self.prev_height = height;
        self.first_render = false;

        Ok(written)
    }

    /// Full redraw of all lines.
    fn full_redraw(
        &mut self,
        lines: &[String],
        cursor_row: Option<u16>,
        cursor_col: Option<u16>,
    ) -> io::Result<usize> {
        let mut output = String::new();

        output.push_str(ansi::begin_sync());
        output.push_str(ansi::hide_cursor());
        output.push_str(&ansi::cursor_goto(1, 1));

        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                output.push_str("\r\n");
            }
            output.push_str(ansi::erase_line());
            output.push_str(line);
        }

        // Position hardware cursor if we found a marker
        if let (Some(r), Some(c)) = (cursor_row, cursor_col) {
            output.push_str(&ansi::cursor_goto(r + 1, c + 1));
            self.cursor_row = r;
            self.cursor_col = c;
        } else {
            // Default: cursor at end of last line
            let last_row = lines.len().saturating_sub(1) as u16;
            output.push_str(&ansi::cursor_goto(last_row + 1, 1));
            self.cursor_row = last_row;
            self.cursor_col = 0;
        }
        output.push_str(ansi::show_cursor());
        output.push_str(ansi::end_sync());

        self.writer.write_all(output.as_bytes())?;
        Ok(output.len())
    }
}

/// Scan lines for CURSOR_MARKER, strip it, and return (clean_lines, cursor_position).
fn strip_cursor_marker(lines: &[String]) -> (Vec<String>, Option<(u16, u16)>) {
    let mut clean = Vec::with_capacity(lines.len());
    let mut cursor_pos: Option<(u16, u16)> = None;

    for (row, line) in lines.iter().enumerate() {
        if let Some(idx) = line.find(ansi::CURSOR_MARKER) {
            let col = ansi::visible_width(&line[..idx]) as u16;
            let stripped = line.replacen(ansi::CURSOR_MARKER, "", 1);
            clean.push(stripped);
            cursor_pos = Some((row as u16, col));
        } else {
            clean.push(line.clone());
        }
    }

    (clean, cursor_pos)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_render_full() {
        let buf = {
            let mut buf = Vec::new();
            let mut renderer = TuiRenderer::new(&mut buf);
            let lines = vec!["hello".to_string(), "world".to_string()];
            let written = renderer.render(&lines, 80, 24).unwrap();
            assert!(written > 0);
            buf
        };
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("hello"));
        assert!(output.contains("world"));
        assert!(output.contains(ansi::hide_cursor()));
        assert!(output.contains(ansi::show_cursor()));
    }

    #[test]
    fn test_differential_render() {
        let mut buf = Vec::new();
        let third_written = {
            let mut renderer = TuiRenderer::new(&mut buf);
            let lines = vec!["a".to_string(), "b".to_string()];

            let w1 = renderer.render(&lines, 80, 24).unwrap();
            assert!(w1 > 0, "first render produced output");

            let w2 = renderer.render(&lines, 80, 24).unwrap();
            assert_eq!(w2, 0, "no output for unchanged content");

            let lines2 = vec!["a".to_string(), "X".to_string()];
            renderer.render(&lines2, 80, 24).unwrap()
        };
        assert!(third_written > 0, "should write changes");
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("X"));
    }

    #[test]
    fn test_resize_triggers_full_redraw() {
        let buf = {
            let mut buf = Vec::new();
            let mut renderer = TuiRenderer::new(&mut buf);
            let lines = vec!["hello".to_string()];
            renderer.render(&lines, 80, 24).unwrap();
            renderer.on_resize(120, 30);
            renderer.render(&lines, 120, 30).unwrap();
            buf
        };
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(ansi::hide_cursor()));
    }

    #[test]
    fn test_cursor_marker_stripped() {
        let marker_line = format!("  ▸ hello{}", ansi::CURSOR_MARKER);
        let lines = vec![marker_line];
        let (clean, pos) = strip_cursor_marker(&lines);
        assert_eq!(clean.len(), 1);
        assert!(!clean[0].contains(ansi::CURSOR_MARKER));
        assert!(pos.is_some());
        let (r, c) = pos.unwrap();
        assert_eq!(r, 0);
        assert_eq!(c, 9); // "  ▸ hello" = 9 visible width
    }

    #[test]
    fn test_cursor_marker_not_found() {
        let lines = vec!["hello".to_string()];
        let (clean, pos) = strip_cursor_marker(&lines);
        assert_eq!(clean, lines);
        assert!(pos.is_none());
    }

    #[test]
    fn test_write_through() {
        let buf = {
            let mut buf = Vec::new();
            let mut renderer = TuiRenderer::new(&mut buf);
            renderer.writer().write_all(b"hello").unwrap();
            buf
        };
        assert_eq!(buf, b"hello");
    }

    /// r5 no-leftover: when a changed line gets SHORTER, the diff writer MUST
    /// erase the trailing stale characters so the new line fully replaces the
    /// old one (no ghosting of the previous frame's longer text).
    #[test]
    fn test_diff_shorter_line_erases_trailing() {
        let mut buf = Vec::new();
        {
            let mut renderer = TuiRenderer::new(&mut buf);
            // Frame 1: composer line is long
            let f1 = vec!["  ▸ hello world long text".to_string()];
            renderer.render(&f1, 80, 24).unwrap();
            // Frame 2: same line, much shorter — trailing chars must be cleared
            let f2 = vec!["  ▸ hi".to_string()];
            renderer.render(&f2, 80, 24).unwrap();
        }
        let out = String::from_utf8(buf).unwrap();
        // The diff path must emit erase_line for the changed line so that
        // "hello world long text" does not linger after "hi".
        assert!(
            out.contains(ansi::erase_line()),
            "diff path must erase the changed line to avoid ghosting"
        );
    }

    /// r5 no-leftover: replacing one prompt marker line with another (e.g.
    /// composer text change) must not leave the old prompt behind. Simulates
    /// the `▸ T` → `▸ hi` transition that produced ghosting in manual testing.
    #[test]
    fn test_diff_replaces_prompt_line_without_ghost() {
        let mut buf = Vec::new();
        {
            let mut renderer = TuiRenderer::new(&mut buf);
            let f1 = vec![format!("  ▸ T{}", ansi::CURSOR_MARKER)];
            renderer.render(&f1, 80, 24).unwrap();
            let f2 = vec![format!("  ▸ hi{}", ansi::CURSOR_MARKER)];
            renderer.render(&f2, 80, 24).unwrap();
        }
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains(ansi::erase_line()),
            "changed line must be erased before rewriting"
        );
    }
}
