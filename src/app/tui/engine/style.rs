//! Self-owned lightweight style types + ANSI serialization (c399).
//!
//! pi-tui line-array model: widgets produce `Vec<StyledLine>`, the engine diffs
//! by line equality, and writes only changed lines. Diffing needs a canonical
//! string form, so each `StyledLine` serializes to an ANSI escape string. This
//! module owns the style vocabulary, the SGR serialization, and the line model —
//! fully independent of ratatui (which we removed in c399: ratatui `Style` has
//! no ANSI serialization, and the line-array diff must self-control it).
//!
//! Representation choice: `StyledLine { spans: Vec<Span> }` (not `Vec<Cell>`).
//! Spans are closer to the markdown renderer's output (consecutive runs of the
//! same style), more compact, and diffing serializes to a string anyway. Width
//! is computed via `unicode_width` over the concatenated span text.

use std::fmt::Write as _;

/// A terminal color. Mirrors the subset of SGR color encodings the engine emits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    /// 256-color palette index (SGR 38;5;n / 48;5;n).
    Indexed(u8),
    /// 24-bit true color (SGR 38;2;r;g;b / 48;2;r;g;b).
    Rgb(u8, u8, u8),
    /// Terminal default (reset).
    Reset,
}

impl Color {
    /// SGR parameter suffix for a FOREGROUND set of this color.
    /// Full parameter string (e.g. `31` for Red, `38;5;202` for Indexed,
    /// `38;2;1;2;3` for Rgb, `39` for Reset).
    fn sgr_fg(self) -> String {
        match self {
            Color::Black => "30".into(),
            Color::Red => "31".into(),
            Color::Green => "32".into(),
            Color::Yellow => "33".into(),
            Color::Blue => "34".into(),
            Color::Magenta => "35".into(),
            Color::Cyan => "36".into(),
            Color::Gray => "37".into(),
            Color::DarkGray => "90".into(),
            Color::LightRed => "91".into(),
            Color::LightGreen => "92".into(),
            Color::LightYellow => "93".into(),
            Color::LightBlue => "94".into(),
            Color::LightMagenta => "95".into(),
            Color::LightCyan => "96".into(),
            Color::White => "97".into(),
            Color::Indexed(n) => format!("38;5;{n}"),
            Color::Rgb(r, g, b) => format!("38;2;{r};{g};{b}"),
            Color::Reset => "39".into(),
        }
    }

    /// SGR parameter suffix for a BACKGROUND set. 16-color shifts by 10 from fg;
    /// 256/truecolor use 48 instead of 38.
    fn sgr_bg(self) -> String {
        match self {
            Color::Black => "40".into(),
            Color::Red => "41".into(),
            Color::Green => "42".into(),
            Color::Yellow => "43".into(),
            Color::Blue => "44".into(),
            Color::Magenta => "45".into(),
            Color::Cyan => "46".into(),
            Color::Gray => "47".into(),
            Color::DarkGray => "100".into(),
            Color::LightRed => "101".into(),
            Color::LightGreen => "102".into(),
            Color::LightYellow => "103".into(),
            Color::LightBlue => "104".into(),
            Color::LightMagenta => "105".into(),
            Color::LightCyan => "106".into(),
            Color::White => "107".into(),
            Color::Indexed(n) => format!("48;5;{n}"),
            Color::Rgb(r, g, b) => format!("48;2;{r};{g};{b}"),
            Color::Reset => "49".into(),
        }
    }
}

/// Text style attributes (the SGR subset the engine emits).
///
/// Default (all None/false) is the terminal's current style — the engine appends
/// a full reset per line (`\x1b[0m`) so styles never cross lines (the "styles do
/// not cross lines" invariant from rendering-engine.md Step 4 applyLineResets).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CellStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub crossed_out: bool,
    pub reverse: bool,
}

impl CellStyle {
    /// Empty style (terminal default). Const construction (the engine uses it
    /// as a starting point for builder-style chaining in const contexts).
    pub const fn empty() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            crossed_out: false,
            reverse: false,
        }
    }

    /// Whether this style produces no SGR escape at all (nothing to emit).
    pub fn is_empty(self) -> bool {
        self == Self::default()
    }

    pub fn fg(mut self, c: Color) -> Self {
        self.fg = Some(c);
        self
    }
    pub fn bg(mut self, c: Color) -> Self {
        self.bg = Some(c);
        self
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
    pub fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    /// Serialize to an SGR escape sequence that APPLIES this style from a reset
    /// state. Empty style -> empty string (caller wraps line end with reset).
    ///
    /// Order: reset is NOT emitted here (the per-line reset lives in
    /// `StyledLine::to_ansi`). We emit fg/bg/colors then attribute bits.
    pub fn to_sgr(self) -> String {
        let mut params: Vec<String> = Vec::new();
        if let Some(fg) = self.fg {
            params.push(fg.sgr_fg());
        }
        if let Some(bg) = self.bg {
            params.push(bg.sgr_bg());
        }
        if self.bold {
            params.push("1".into());
        }
        if self.dim {
            params.push("2".into());
        }
        if self.italic {
            params.push("3".into());
        }
        if self.underline {
            params.push("4".into());
        }
        if self.reverse {
            params.push("7".into());
        }
        if self.crossed_out {
            params.push("9".into());
        }
        if params.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", params.join(";"))
        }
    }
}

/// A run of text with a single style. Building block of [`StyledLine`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Span {
    pub text: String,
    pub style: CellStyle,
}

impl Span {
    pub fn new(text: impl Into<String>, style: CellStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
    /// Unstyled span (terminal default).
    pub fn raw(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: CellStyle::default(),
        }
    }
    /// Styled span.
    pub fn styled(text: impl Into<String>, style: CellStyle) -> Self {
        Self::new(text, style)
    }
}

/// One terminal line: a sequence of same-or-varying-style spans.
///
/// The engine's line-array is `Vec<StyledLine>`; diffing compares
/// [`to_ansi`] output. Width is computed over concatenated span text via
/// `unicode_width`.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct StyledLine {
    pub spans: Vec<Span>,
}

impl StyledLine {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_spans(spans: Vec<Span>) -> Self {
        Self { spans }
    }
    pub fn raw(text: impl Into<String>) -> Self {
        Self {
            spans: vec![Span::raw(text)],
        }
    }
    /// Empty line (no spans) — renders as a blank row.
    pub fn empty() -> Self {
        Self { spans: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() || self.spans.iter().all(|s| s.text.is_empty())
    }

    pub fn push(&mut self, span: Span) {
        self.spans.push(span);
    }

    /// Concatenated plain text of all spans (no styling).
    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }

    /// Display width of the line (CJK-aware, via `unicode_width`).
    pub fn width(&self) -> usize {
        use unicode_width::UnicodeWidthStr;
        self.spans
            .iter()
            .map(|s| UnicodeWidthStr::width(s.text.as_str()))
            .sum()
    }

    /// Serialize to an ANSI string for terminal output. Emits each span's SGR
    /// then text; does NOT append a trailing reset (the engine's applyLineResets
    /// step appends `\x1b[0m\x1b]8;;\x07` per line — rendering-engine.md Step 4).
    /// Empty spans are skipped (no SGR for empty text).
    pub fn to_ansi(&self) -> String {
        let mut out = String::new();
        for span in &self.spans {
            if span.text.is_empty() {
                continue;
            }
            if !span.style.is_empty() {
                let _ = out.write_str(&span.style.to_sgr());
            }
            let _ = out.write_str(&span.text);
        }
        out
    }

    /// Per-line reset sequence the engine appends after each non-image line
    /// (SGR reset + OSC 8 hyperlink reset). rendering-engine.md Step 4 invariant.
    pub const LINE_RESET: &'static str = "\x1b[0m\x1b]8;;\x07";
}

/// Zero-width APC marker a `Focusable` widget embeds at the on-screen cursor
/// position (rendering-engine.md Step 4 `extractCursorPosition`; ui-components.md
/// Step 4). The engine strips it and positions the hardware cursor there for IME.
/// Terminals ignore unknown APC sequences, so it does not render.
pub const CURSOR_MARKER: &str = "\x1b_pi:c\x07";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_style_emits_nothing() {
        assert_eq!(CellStyle::default().to_sgr(), "");
        assert!(CellStyle::default().is_empty());
    }

    #[test]
    fn bold_emits_sgr_1() {
        assert_eq!(CellStyle::default().bold().to_sgr(), "\x1b[1m");
    }

    #[test]
    fn bold_italic_combines_params() {
        let s = CellStyle::default().bold().italic();
        assert_eq!(s.to_sgr(), "\x1b[1;3m");
    }

    #[test]
    fn fg_red_emits_31() {
        assert_eq!(CellStyle::default().fg(Color::Red).to_sgr(), "\x1b[31m");
    }

    #[test]
    fn fg_indexed_256color() {
        assert_eq!(
            CellStyle::default().fg(Color::Indexed(202)).to_sgr(),
            "\x1b[38;5;202m"
        );
    }

    #[test]
    fn fg_rgb_truecolor() {
        assert_eq!(
            CellStyle::default().fg(Color::Rgb(1, 2, 3)).to_sgr(),
            "\x1b[38;2;1;2;3m"
        );
    }

    #[test]
    fn bg_and_fg_both() {
        let s = CellStyle::default().fg(Color::Red).bg(Color::Blue);
        assert_eq!(s.to_sgr(), "\x1b[31;44m");
    }

    #[test]
    fn full_style_all_params() {
        let s = CellStyle {
            fg: Some(Color::Green),
            bg: Some(Color::Yellow),
            bold: true,
            italic: true,
            underline: true,
            dim: true,
            crossed_out: true,
            reverse: true,
        };
        // order: fg, bg, bold(1), dim(2), italic(3), underline(4), reverse(7), crossed_out(9)
        assert_eq!(s.to_sgr(), "\x1b[32;43;1;2;3;4;7;9m");
    }

    #[test]
    fn styled_line_to_ansi_single_span() {
        let line = StyledLine::from_spans(vec![Span::styled("hi", CellStyle::default().bold())]);
        assert_eq!(line.to_ansi(), "\x1b[1mhi");
    }

    #[test]
    fn styled_line_to_ansi_multi_span_style_change() {
        let line = StyledLine::from_spans(vec![
            Span::styled("red ", CellStyle::default().fg(Color::Red)),
            Span::raw("plain"),
        ]);
        assert_eq!(line.to_ansi(), "\x1b[31mred plain");
    }

    #[test]
    fn styled_line_empty_skips_sgr() {
        let line = StyledLine::from_spans(vec![
            Span::styled("", CellStyle::default().bold()),
            Span::raw("x"),
        ]);
        assert_eq!(line.to_ansi(), "x");
    }

    #[test]
    fn styled_line_plain_text_concat() {
        let line = StyledLine::from_spans(vec![
            Span::raw("hello "),
            Span::styled("world", CellStyle::default().bold()),
        ]);
        assert_eq!(line.plain_text(), "hello world");
    }

    #[test]
    fn styled_line_width_ascii() {
        let line = StyledLine::raw("hello");
        assert_eq!(line.width(), 5);
    }

    #[test]
    fn styled_line_width_cjk_double() {
        // CJK chars are display-width 2 each.
        let line = StyledLine::raw("你好");
        assert_eq!(line.width(), 4);
    }

    #[test]
    fn styled_line_width_mixed() {
        let line = StyledLine::raw("a你b好c");
        // a(1) + 你(2) + b(1) + 好(2) + c(1) = 7
        assert_eq!(line.width(), 7);
    }

    #[test]
    fn styled_line_equality_for_diff() {
        // The engine diffs lines by equality — identical content+style must be ==.
        let a = StyledLine::from_spans(vec![Span::styled("x", CellStyle::default().bold())]);
        let b = StyledLine::from_spans(vec![Span::styled("x", CellStyle::default().bold())]);
        assert_eq!(a, b);
        let c = StyledLine::from_spans(vec![Span::raw("x")]);
        assert_ne!(a, c);
    }

    #[test]
    fn line_reset_constant() {
        assert_eq!(StyledLine::LINE_RESET, "\x1b[0m\x1b]8;;\x07");
    }
}
