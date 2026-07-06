//! Width utilities for `StyledLine` (c399).
//!
//! pi-tui's width utilities (skill `ui-components.md` Step 0) operate on ANSI
//! strings because widgets emit strings. Our widgets emit structured
//! `StyledLine { spans }`, so these utilities work on `StyledLine` directly —
//! width is `unicode_width` over concatenated span text (no ANSI parsing needed),
//! and truncate/wrap split spans at char boundaries while preserving the style
//! of the source span on each piece.
//!
//! These are the backstop for the engine's hard width invariant
//! (`rendering-engine.md` Step 6): a widget whose `render(width)` returns a line
//! wider than `width` is a hard error. Widgets use `truncate`/`wrap` to honor it.

use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use super::style::{Span, StyledLine};

/// Display width of text that may contain ANSI escape sequences (SGR/OSC/APC).
/// Escape sequences contribute 0 width. Used by the engine to compute the IME
/// cursor column from the text before a `CURSOR_MARKER` (which is itself an APC,
/// so it's correctly zero-width here).
pub fn marker_aware_width(s: &str) -> usize {
    // Strip ANSI escape sequences (CSI/OSC/APC/other), then measure visible text.
    let mut clean = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\x1b' {
            clean.push(ch);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii() && (0x40..=0x7E).contains(&(c as u32)) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                for c in chars.by_ref() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' {
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                }
            }
            Some('_') => {
                // APC: ESC _ ... terminated by ST (ESC \) OR BEL (\x07).
                // CURSOR_MARKER uses BEL termination; some DCS/APC use ST.
                chars.next();
                for c in chars.by_ref() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' {
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                }
            }
            Some(_) => {
                chars.next();
            }
            None => {}
        }
    }
    UnicodeWidthStr::width(clean.as_str())
}

/// Truncate a line to at most `max_width` display columns, appending `ellipsis`
/// (default `…`) if any content was cut. Wide chars (CJK/emoji) are never split:
/// if the next char would overflow, the char moves to the cut (truncated output
/// ends before it). Pass `""` for `ellipsis` to truncate without a marker.
///
/// `CURSOR_MARKER` (zero-width APC) is preserved in place — it contributes 0 to
/// width and must survive truncation so the engine can still locate the IME
/// cursor after a truncate (e.g. an Input widget truncating its rendered view).
pub fn truncate(line: &StyledLine, max_width: usize, ellipsis: &str) -> StyledLine {
    if line.width() <= max_width {
        return line.clone();
    }
    let ellipsis_w = if ellipsis.is_empty() {
        0
    } else {
        unicode_width::UnicodeWidthStr::width(ellipsis)
    };
    let budget = max_width.saturating_sub(ellipsis_w);
    let mut out = StyledLine::new();
    let mut used = 0usize;
    'outer: for span in &line.spans {
        let mut piece = String::new();
        for ch in span.text.chars() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
            // Zero-width chars (combining marks, the CURSOR_MARKER APC) pass
            // through without counting toward the budget — the marker must
            // survive truncation so the engine can still locate the IME cursor.
            if cw == 0 {
                piece.push(ch);
                continue;
            }
            if used + cw > budget {
                if !piece.is_empty() {
                    out.push(Span::new(std::mem::take(&mut piece), span.style));
                }
                break 'outer;
            }
            piece.push(ch);
            used += cw;
        }
        if !piece.is_empty() {
            out.push(Span::new(piece, span.style));
        }
    }
    if !ellipsis.is_empty() && line.width() > max_width {
        out.push(Span::raw(ellipsis));
    }
    out
}

/// Word-wrap a line to at most `width` display columns, returning one or more
/// lines. Breaks at the last char that fits (char-level wrap, CJK-safe — matches
/// the existing `wrap_to_width` behavior in `render.rs`). Each wrapped line
/// carries the style of its source spans (a span split across a break produces
/// two spans, one per line, both with the original style).
///
/// An empty/blank input yields a single empty line (callers rely on at least
/// one line per paragraph). `CURSOR_MARKER` (zero-width) passes through.
pub fn wrap(line: &StyledLine, width: usize) -> Vec<StyledLine> {
    if width == 0 {
        return vec![line.clone()];
    }
    let mut rows: Vec<StyledLine> = Vec::new();
    let mut cur = StyledLine::new();
    let mut cur_w = 0usize;
    for span in &line.spans {
        let mut piece = String::new();
        for ch in span.text.chars() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
            if cw == 0 {
                piece.push(ch);
                continue;
            }
            if cur_w + cw > width && cur_w > 0 {
                // flush current piece before wrapping
                if !piece.is_empty() {
                    cur.push(Span::new(std::mem::take(&mut piece), span.style));
                }
                rows.push(std::mem::take(&mut cur));
                cur_w = 0;
            }
            piece.push(ch);
            cur_w += cw;
        }
        if !piece.is_empty() {
            cur.push(Span::new(piece, span.style));
        }
    }
    rows.push(cur);
    if rows.is_empty() {
        rows.push(StyledLine::empty());
    }
    rows
}

/// Wrap a single span-less string to `width` as `StyledLine`s (all default
/// style). Convenience for widgets that render plain text without per-span
/// styling. CJK-aware via `unicode_width`.
pub fn wrap_plain(text: &str, width: usize) -> Vec<StyledLine> {
    if width == 0 {
        return vec![StyledLine::raw(text)];
    }
    let mut rows: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            rows.push(std::mem::take(&mut cur));
            cur_w = 0;
            continue;
        }
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if cw == 0 {
            cur.push(ch);
            continue;
        }
        if cur_w + cw > width && cur_w > 0 {
            rows.push(std::mem::take(&mut cur));
            cur_w = 0;
        }
        cur.push(ch);
        cur_w += cw;
    }
    rows.push(cur);
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows.into_iter().map(StyledLine::raw).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::engine::style::{CellStyle, Color};

    #[test]
    fn truncate_no_op_when_within_width() {
        let line = StyledLine::raw("hello");
        let out = truncate(&line, 10, "…");
        assert_eq!(out.plain_text(), "hello");
    }

    #[test]
    fn truncate_ascii_adds_ellipsis() {
        let line = StyledLine::raw("hello world");
        let out = truncate(&line, 8, "…");
        assert_eq!(out.plain_text(), "hello w…");
        assert!(out.width() <= 8);
    }

    #[test]
    fn truncate_no_ellipsis_marker() {
        let line = StyledLine::raw("abcdefghij");
        let out = truncate(&line, 4, "");
        assert_eq!(out.plain_text(), "abcd");
    }

    #[test]
    fn truncate_cjk_never_splits_wide_char() {
        // 你好 = 4 cols. Truncate to 3: must keep only 你 (2 cols), not split 好.
        let line = StyledLine::raw("你好");
        let out = truncate(&line, 3, "");
        assert_eq!(out.plain_text(), "你");
        assert_eq!(out.width(), 2);
    }

    #[test]
    fn truncate_preserves_style_on_kept_part() {
        let line = StyledLine::from_spans(vec![Span::styled(
            "red text",
            CellStyle::default().fg(Color::Red),
        )]);
        let out = truncate(&line, 4, "");
        assert_eq!(out.spans.len(), 1);
        assert_eq!(out.spans[0].style.fg, Some(Color::Red));
        assert_eq!(out.spans[0].text, "red ");
    }

    #[test]
    fn wrap_short_line_one_row() {
        let line = StyledLine::raw("hello");
        let rows = wrap(&line, 80);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].plain_text(), "hello");
    }

    #[test]
    fn wrap_long_line_splits_at_width() {
        let line = StyledLine::raw("abcdefghij");
        let rows = wrap(&line, 4);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].plain_text(), "abcd");
        assert_eq!(rows[1].plain_text(), "efgh");
        assert_eq!(rows[2].plain_text(), "ij");
    }

    #[test]
    fn wrap_cjk_uses_display_width() {
        let line = StyledLine::raw("你好世界再见");
        let rows = wrap(&line, 4);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].plain_text(), "你好");
        assert_eq!(rows[1].plain_text(), "世界");
        assert_eq!(rows[2].plain_text(), "再见");
    }

    #[test]
    fn wrap_preserves_span_style_across_break() {
        let line = StyledLine::from_spans(vec![Span::styled(
            "redtext",
            CellStyle::default().fg(Color::Red),
        )]);
        let rows = wrap(&line, 3);
        assert!(rows.len() >= 2);
        for row in &rows {
            for span in &row.spans {
                assert_eq!(
                    span.style.fg,
                    Some(Color::Red),
                    "style carried to wrapped row"
                );
            }
        }
    }

    #[test]
    fn wrap_mixed_style_splits_spans_correctly() {
        let line = StyledLine::from_spans(vec![
            Span::styled("ab", CellStyle::default().bold()),
            Span::raw("cdef"),
        ]);
        let rows = wrap(&line, 3);
        // "abc" then "def"
        assert_eq!(rows[0].plain_text(), "abc");
        assert_eq!(rows[1].plain_text(), "def");
        // first row has the bold span + plain span
        assert_eq!(rows[0].spans.len(), 2);
        assert!(rows[0].spans[0].style.bold);
    }

    #[test]
    fn wrap_plain_multiline_splits_on_newline() {
        let rows = wrap_plain("a\nb\nc", 80);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].plain_text(), "a");
        assert_eq!(rows[1].plain_text(), "b");
        assert_eq!(rows[2].plain_text(), "c");
    }

    #[test]
    fn wrap_plain_cjk() {
        let rows = wrap_plain("你好世界", 4);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].plain_text(), "你好");
        assert_eq!(rows[1].plain_text(), "世界");
    }

    #[test]
    fn wrap_empty_returns_one_empty_row() {
        let rows = wrap_plain("", 80);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_empty());
    }

    // ── marker_aware_width (ANSI stripping for IME cursor column) ─────────

    #[test]
    fn marker_aware_width_plain_ascii() {
        assert_eq!(marker_aware_width("hello"), 5);
    }

    #[test]
    fn marker_aware_width_cjk() {
        assert_eq!(marker_aware_width("你好"), 4);
    }

    #[test]
    fn marker_aware_width_strips_sgr() {
        // \x1b[31m + "hi" = width 2 (color adds nothing).
        assert_eq!(marker_aware_width("\x1b[31mhi"), 2);
    }

    #[test]
    fn marker_aware_width_strips_apc_cursor_marker() {
        // "ab" + CURSOR_MARKER (APC) + "cd" = width 4; the marker is zero-width.
        const CURSOR_MARKER: &str = "\x1b_pi:c\x07";
        assert_eq!(marker_aware_width(&format!("ab{CURSOR_MARKER}cd")), 4);
    }

    #[test]
    fn marker_aware_width_only_escapes() {
        assert_eq!(marker_aware_width("\x1b[1m\x1b[0m"), 0);
    }
}
