//! ANSI escape code builders.
//!
//! Provides functions to wrap text in ANSI SGR (Select Graphic Rendition) sequences.
//! Follows our Codex-style theme: cyan for hints, magenta for accent, etc.
//!
//! # Example
//!
//! ```ignore
//! use crate::interface::tui::engine::ansi::*;
//! let styled = format!("{}{}{}", fg_cyan(), "hello", reset());
//! ```

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// ── SGR constants ─────────────────────────────────────────────────────────────

const CSI: &str = "\x1b[";

/// Reset all styles.
pub(crate) const fn reset() -> &'static str {
    "\x1b[0m"
}

/// Bold style.
pub(crate) const fn bold() -> &'static str {
    "\x1b[1m"
}

/// Dim style.
pub(crate) const fn dim() -> &'static str {
    "\x1b[2m"
}

/// Italic style.
pub(crate) const fn italic() -> &'static str {
    "\x1b[3m"
}

/// Underline style.
pub(crate) const fn underline() -> &'static str {
    "\x1b[4m"
}

/// Slow blink.
#[allow(dead_code)]
pub(crate) const fn blink() -> &'static str {
    "\x1b[5m"
}

/// Reverse video.
#[allow(dead_code)]
pub(crate) const fn reverse() -> &'static str {
    "\x1b[7m"
}

// ── Foreground colors (standard 8) ────────────────────────────────────────────

/// Black foreground.
pub(crate) const fn fg_black() -> &'static str {
    "\x1b[30m"
}
/// Red foreground.
pub(crate) const fn fg_red() -> &'static str {
    "\x1b[31m"
}
/// Green foreground.
pub(crate) const fn fg_green() -> &'static str {
    "\x1b[32m"
}
/// Yellow foreground.
pub(crate) const fn fg_yellow() -> &'static str {
    "\x1b[33m"
}
/// Blue foreground.
pub(crate) const fn fg_blue() -> &'static str {
    "\x1b[34m"
}
/// Magenta foreground.
pub(crate) const fn fg_magenta() -> &'static str {
    "\x1b[35m"
}
/// Cyan foreground.
pub(crate) const fn fg_cyan() -> &'static str {
    "\x1b[36m"
}
/// White foreground.
pub(crate) const fn fg_white() -> &'static str {
    "\x1b[37m"
}
/// Default foreground (terminal default).
pub(crate) const fn fg_default() -> &'static str {
    "\x1b[39m"
}

// ── Background colors (standard 8) ────────────────────────────────────────────

pub(crate) const fn bg_black() -> &'static str {
    "\x1b[40m"
}
pub(crate) const fn bg_red() -> &'static str {
    "\x1b[41m"
}
pub(crate) const fn bg_green() -> &'static str {
    "\x1b[42m"
}
pub(crate) const fn bg_yellow() -> &'static str {
    "\x1b[43m"
}
pub(crate) const fn bg_blue() -> &'static str {
    "\x1b[44m"
}
pub(crate) const fn bg_magenta() -> &'static str {
    "\x1b[45m"
}
pub(crate) const fn bg_cyan() -> &'static str {
    "\x1b[46m"
}
pub(crate) const fn bg_white() -> &'static str {
    "\x1b[47m"
}
pub(crate) const fn bg_default() -> &'static str {
    "\x1b[49m"
}

// ── 256-color ─────────────────────────────────────────────────────────────────

/// 256-color foreground: `\x1b[38;5;{n}m`.
pub(crate) fn fg_256(n: u8) -> String {
    format!("\x1b[38;5;{n}m")
}

/// 256-color background: `\x1b[48;5;{n}m`.
#[allow(dead_code)]
pub(crate) fn bg_256(n: u8) -> String {
    format!("\x1b[48;5;{n}m")
}

// ── Helpers for common theme styles ───────────────────────────────────────────

/// Wrap text in a style: apply `prefix` and append `reset()`.
fn styled(text: &str, prefix: &str) -> String {
    format!("{prefix}{text}{}", reset())
}

/// User message style: cyan + bold.
pub(crate) fn user(text: &str) -> String {
    styled(text, &format!("{}{}", fg_cyan(), bold()))
}

/// Assistant message style: default foreground.
pub(crate) fn assistant(text: &str) -> String {
    styled(text, "")
}

/// Dim style (system events, metadata).
pub(crate) fn dim_text(text: &str) -> String {
    styled(text, dim())
}

/// Hint style (cyan).
pub(crate) fn hint(text: &str) -> String {
    styled(text, fg_cyan())
}

/// Error style (red + bold).
pub(crate) fn error(text: &str) -> String {
    styled(text, &format!("{}{}", fg_red(), bold()))
}

/// Success style (green).
pub(crate) fn success(text: &str) -> String {
    styled(text, fg_green())
}

/// Accent style (magenta).
pub(crate) fn accent(text: &str) -> String {
    styled(text, fg_magenta())
}

/// Tool style (yellow + dim).
pub(crate) fn tool(text: &str) -> String {
    styled(text, &format!("{}{}", fg_yellow(), dim()))
}

/// Status bar: dim.
pub(crate) fn status_text(text: &str) -> String {
    styled(text, dim())
}

// ── String utilities ──────────────────────────────────────────────────────────

/// Compute the **visible** (display) width of a string by stripping ANSI escape
/// sequences and measuring Unicode width.
///
/// This is equivalent to pi's `visibleWidth()`.
pub(crate) fn visible_width(s: &str) -> usize {
    strip_ansi(s).width()
}

/// Strip ANSI escape sequences from a string.
/// Handles SGR (`\x1b[...m`), CSI (`\x1b[...` letter-terminated), and the
/// custom CURSOR_MARKER (`\x1b...\x07`, BEL-terminated) used to mark the
/// composer's hardware-cursor position. Without the BEL-terminated branch the
/// marker's trailing bytes would be counted as visible width, corrupting row
/// padding and overflow math.
pub(crate) fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                // CURSOR_MARKER / OSC-style: consume until BEL (\x07).
                Some('_') | Some(']') => {
                    for esc in chars.by_ref() {
                        if esc == '\x07' {
                            break;
                        }
                    }
                }
                // CSI-style: consume until a final byte (letter or ~).
                _ => {
                    for esc in chars.by_ref() {
                        if esc.is_ascii_alphabetic() || esc == '~' {
                            break;
                        }
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Truncate `text` to at most `max_width` visible columns, appending "..." when truncated.
///
/// ANSI escapes are stripped first so the measurement matches rendered width.
/// Unlike [`wrap_text`], this preserves leading/trailing whitespace verbatim,
/// which is required for composer input where every typed space matters.
pub(crate) fn truncate_visible(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if visible_width(text) <= max_width {
        return text.to_string();
    }
    if max_width <= 3 {
        return "...".chars().take(max_width).collect();
    }
    let plain = strip_ansi(text);
    let mut out = String::new();
    let mut width = 0usize;
    let target = max_width - 3;
    for ch in plain.chars() {
        let cw = ch.to_string().width();
        if width + cw > target {
            break;
        }
        out.push(ch);
        width += cw;
    }
    out.push_str("...");
    out
}

/// Wrap text to fit within `max_width` columns.
///
/// Preserves ANSI escape sequences.  Breaks at **word boundaries** (whitespace),
/// which is what users expect for text editor / transcript display.
/// Words longer than `max_width` are forcibly split.
///
/// Lines are separated by `\n` in the input produce line breaks.
pub(crate) fn wrap_text(text: &str, max_width: u16) -> Vec<String> {
    if max_width == 0 || text.is_empty() {
        return vec![text.to_string()];
    }
    let max = max_width as usize;

    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut current_width: usize = 0;

    // Split by whitespace, keeping track of separators
    // We use a simple approach: iterate through words
    let mut word_start = 0;
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    while word_start < len {
        // Skip leading whitespace (preserve a single space between words)
        let mut word_end = word_start;
        let mut leading_space = String::new();
        let mut has_leading_space = false;

        while word_end < len && chars[word_end].is_whitespace() && chars[word_end] != '\n' {
            leading_space.push(chars[word_end]);
            word_end += 1;
            has_leading_space = true;
        }

        // Check for explicit newline
        if word_end < len && chars[word_end] == '\n' {
            lines.push(current_line.clone());
            current_line.clear();
            current_width = 0;
            word_start = word_end + 1;
            continue;
        }

        // Find end of current word (non-whitespace)
        word_end = word_start;
        while word_end < len && !chars[word_end].is_whitespace() {
            word_end += 1;
        }

        if word_start == word_end {
            word_start += 1;
            continue;
        }

        let word: String = chars[word_start..word_end].iter().collect();
        let word_width = visible_width(&word);

        // If this is the first word on the line, add it even if it overflows
        if current_width == 0 {
            // Forcibly split long words
            if word_width > max {
                let mut pos = 0;
                let wchars: Vec<char> = word.chars().collect();
                while pos < wchars.len() {
                    let mut seg = String::new();
                    let mut seg_w = 0;
                    while pos < wchars.len() {
                        let c = wchars[pos];
                        let cw = visible_width(&c.to_string());
                        if seg_w + cw > max && !seg.is_empty() {
                            break;
                        }
                        seg.push(c);
                        seg_w += cw;
                        pos += 1;
                    }
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
                        current_line.clear();
                    }
                    current_width = seg_w;
                    current_line = seg;
                    if pos < wchars.len() || pos == word.len() {
                        // End this segment
                    }
                }
                // After splitting the long word, push the last segment
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                    current_line.clear();
                    current_width = 0;
                }
            } else {
                current_line = word;
                current_width = word_width;
            }
        } else {
            // Check if word fits with a space
            let space_width = 1;
            if current_width + space_width + word_width <= max {
                current_line.push(' ');
                current_line.push_str(&word);
                current_width += space_width + word_width;
            } else {
                // Start new line
                lines.push(current_line.clone());
                current_line = word;
                current_width = word_width;
            }
        }

        word_start = word_end;
    }

    // Don't forget the last line
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Pad a line to `width` columns with spaces, maintaining ANSI style integrity.
///
/// If the line's visible width is less than `width`, append spaces.  If the line
/// ends with an ANSI style prefix (e.g., `\x1b[0m\x1b[36m`), the padding is
/// injected before a final reset, preserving the style for future content.
pub(crate) fn pad_to_width(line: &str, width: u16) -> String {
    let w = width as usize;
    let vw = visible_width(line);
    if vw >= w {
        return line.to_string();
    }
    let padding = w - vw;
    // If the line ends with a reset, insert spaces before the reset.
    // Otherwise, just append spaces.
    if line.ends_with("\x1b[0m") {
        let prefix = &line[..line.len() - 4];
        format!("{prefix}{rest}\x1b[0m", rest = " ".repeat(padding))
    } else {
        format!("{line}{}", " ".repeat(padding))
    }
}

// ── Cursor movement ───────────────────────────────────────────────────────────

/// Cursor position marker for hardware cursor positioning.
/// See pi's `CURSOR_MARKER`.
pub(crate) const CURSOR_MARKER: &str = "\x1b_pi:c\x07";

/// Move cursor to absolute row/col (1-based).
pub(crate) fn cursor_goto(row: u16, col: u16) -> String {
    format!("\x1b[{row};{col}H")
}

/// Move cursor up N rows.
pub(crate) fn cursor_up(n: u16) -> String {
    format!("\x1b[{n}A")
}

/// Move cursor down N rows.
pub(crate) fn cursor_down(n: u16) -> String {
    format!("\x1b[{n}B")
}

/// Move cursor right N columns.
#[allow(dead_code)]
pub(crate) fn cursor_right(n: u16) -> String {
    format!("\x1b[{n}C")
}

/// Move cursor left N columns.
#[allow(dead_code)]
pub(crate) fn cursor_left(n: u16) -> String {
    format!("\x1b[{n}D")
}

/// Erase from cursor to end of line.
pub(crate) fn erase_line() -> &'static str {
    "\x1b[K"
}

/// Erase from cursor to end of screen.
pub(crate) fn erase_screen() -> &'static str {
    "\x1b[J"
}

/// Hide cursor.
pub(crate) fn hide_cursor() -> &'static str {
    "\x1b[?25l"
}

/// Show cursor.
pub(crate) fn show_cursor() -> &'static str {
    "\x1b[?25h"
}

/// Begin synchronized output (avoid flicker).
pub(crate) fn begin_sync() -> &'static str {
    "\x1b[?2026h"
}

/// End synchronized output.
pub(crate) fn end_sync() -> &'static str {
    "\x1b[?2026l"
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_plain() {
        assert_eq!(strip_ansi("hello"), "hello");
    }

    #[test]
    fn test_strip_ansi_sgr() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
    }

    #[test]
    fn test_strip_ansi_complex() {
        assert_eq!(strip_ansi("\x1b[38;5;32mblue\x1b[0m dim"), "blue dim");
    }

    #[test]
    fn test_visible_width_plain() {
        assert_eq!(visible_width("hello"), 5);
    }

    #[test]
    fn test_visible_width_ansi() {
        assert_eq!(visible_width("\x1b[31mhello\x1b[0m"), 5);
    }

    #[test]
    fn test_visible_width_unicode() {
        assert_eq!(visible_width("你好"), 4); // CJK chars are 2 wide
        assert_eq!(visible_width("aé"), 2);
    }

    #[test]
    fn test_wrap_text_no_wrap() {
        let lines = wrap_text("hello world", 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "hello world");
    }

    #[test]
    fn test_wrap_text_by_word() {
        let lines = wrap_text("hello world foo", 10);
        // "hello" (5) + " " (1) + "world" (5) = 11 > 10, so "hello world" wraps
        // Actually "hello world" = 11 > 10, so: "hello" then "world foo"
        // "world" (5) + " " (1) + "foo" (3) = 9 <= 10, stays on one line
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "hello");
        assert_eq!(lines[1], "world foo");
    }

    #[test]
    fn test_wrap_text_long_word() {
        let lines = wrap_text("helloWorld", 5);
        // Single long word: "hello" + "World"
        assert!(lines.len() >= 2);
        assert_eq!(lines[0], "hello");
        assert_eq!(lines[1], "World");
    }

    #[test]
    fn test_wrap_text_newline() {
        let lines = wrap_text("hello\nworld", 80);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "hello");
        assert_eq!(lines[1], "world");
    }

    #[test]
    fn test_pad_to_width_already_full() {
        let s = pad_to_width("hello", 5);
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_pad_to_width_needs_padding() {
        let s = pad_to_width("hi", 5);
        assert_eq!(s, "hi   ");
    }

    #[test]
    fn test_user_style_wraps() {
        let s = user("hello");
        assert!(s.starts_with("\x1b[36m"));
        assert!(s.ends_with("\x1b[0m"));
        assert_eq!(visible_width(&s), 5);
    }

    #[test]
    fn test_cursor_goto() {
        assert_eq!(cursor_goto(3, 5), "\x1b[3;5H");
    }

    #[test]
    fn test_erase_line_constant() {
        assert_eq!(erase_line(), "\x1b[K");
    }

    #[test]
    fn test_begin_end_sync() {
        assert_eq!(begin_sync(), "\x1b[?2026h");
        assert_eq!(end_sync(), "\x1b[?2026l");
    }
}
