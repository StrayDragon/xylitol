//! syntect-based code highlighter for the vendored markdown renderer (c370).
//!
//! Implements [`CodeHighlighter`] using syntect + two-face (pure-Rust fancy
//! regex backend, no oniguruma). Derived from codex's
//! `codex-rs/tui/src/render/highlight.rs` (minimal subset: find_syntax +
//! convert_style + highlight loop). Fixed CatppuccinMocha theme.
//!
//! Safety limits (from codex): inputs exceeding 512 KB or 10 000 lines fall
//! back to plain text (empty `Vec<StyleSegment>`), so rendering never blocks.

use std::sync::OnceLock;

use ratatui_core::style::{Color, Modifier, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;
use two_face::theme::EmbeddedThemeName;

use super::CodeHighlighter;
use super::StyleSegment;

/// Safety guardrails (from codex): reject oversized inputs to avoid
/// pathological CPU/memory usage. Callers fall back to plain text.
const MAX_HIGHLIGHT_BYTES: usize = 512 * 1024;
const MAX_HIGHLIGHT_LINES: usize = 10_000;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

/// Highlight code with a fixed theme, returning `Vec<StyleSegment>` with
/// global byte offsets (consumed by `segments_to_lines`). Empty Vec means
/// "no highlighting" (unrecognized language or oversized input) — callers
/// render plain text.
pub struct SyntectHighlighter {
    theme: syntect::highlighting::Theme,
}

impl SyntectHighlighter {
    pub fn new() -> Self {
        let theme = two_face::theme::extra()
            .get(EmbeddedThemeName::CatppuccinMocha)
            .clone();
        Self { theme }
    }
}

impl Default for SyntectHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeHighlighter for SyntectHighlighter {
    fn highlight(&self, lang: &str, code: &str) -> Vec<StyleSegment> {
        if code.is_empty()
            || code.len() > MAX_HIGHLIGHT_BYTES
            || code.lines().count() > MAX_HIGHLIGHT_LINES
        {
            return Vec::new();
        }
        let Some(syntax) = find_syntax(lang) else {
            return Vec::new();
        };
        let mut h = HighlightLines::new(syntax, &self.theme);
        let mut segments = Vec::new();
        let mut offset = 0usize;
        for line in LinesWithEndings::from(code) {
            let Ok(ranges) = h.highlight_line(line, syntax_set()) else {
                return Vec::new();
            };
            for (style, text) in ranges {
                let len = text.len();
                if len > 0 {
                    segments.push(StyleSegment {
                        start: offset,
                        end: offset + len,
                        style: convert_style(style),
                    });
                    offset += len;
                }
            }
        }
        segments
    }
}

/// syntect `Style` → ratatui `Style`. Skips background (avoid overwriting
/// terminal bg), keeps BOLD, skips italic/underline (poor terminal support,
/// see codex comments).
fn convert_style(syn_style: SyntectStyle) -> Style {
    let mut rt_style = Style::default();
    if let Some(fg) = convert_syntect_color(syn_style.foreground) {
        rt_style = rt_style.fg(fg);
    }
    if syn_style.font_style.contains(FontStyle::BOLD) {
        rt_style.add_modifier |= Modifier::BOLD;
    }
    rt_style
}

/// Decode syntect color alpha channel (bat/two-face convention):
/// - `0x00` → ANSI indexed palette
/// - `0x01` → terminal default (None)
/// - `0xFF`/other → RGB
fn convert_syntect_color(color: syntect::highlighting::Color) -> Option<Color> {
    match color.a {
        0x00 => Some(ansi_palette_color(color.r)),
        0x01 => None,
        _ => Some(Color::Rgb(color.r, color.g, color.b)),
    }
}

fn ansi_palette_color(index: u8) -> Color {
    match index {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        n => Color::Indexed(n),
    }
}

/// Find a syntax by language identifier. Aliases two-face cannot resolve on
/// its own are patched first (from codex).
fn find_syntax(lang: &str) -> Option<&'static SyntaxReference> {
    let ss = syntax_set();
    let patched = match lang {
        "csharp" | "c-sharp" => "c#",
        "golang" => "go",
        "python3" => "python",
        "shell" => "bash",
        _ => lang,
    };
    if let Some(s) = ss.find_syntax_by_token(patched) {
        return Some(s);
    }
    if let Some(s) = ss.find_syntax_by_name(patched) {
        return Some(s);
    }
    // Case-insensitive name fallback.
    let lower = patched.to_lowercase();
    if let Some(s) = ss
        .syntaxes()
        .iter()
        .find(|s| s.name.to_lowercase() == lower)
    {
        return Some(s);
    }
    ss.find_syntax_by_extension(lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rs_code_gets_highlighted() {
        let h = SyntectHighlighter::new();
        let segs = h.highlight("rs", "fn main() {}");
        assert!(!segs.is_empty(), "rust code should produce styled segments");
        // At least one segment carries a non-default style (color applied).
        assert!(
            segs.iter().any(|s| s.style.fg.is_some()),
            "some span has foreground color: {:?}",
            segs.iter().map(|s| s.style.fg).collect::<Vec<_>>()
        );
    }

    #[test]
    fn unknown_lang_returns_empty() {
        let h = SyntectHighlighter::new();
        let segs = h.highlight("not-a-real-lang-xyz", "code");
        assert!(segs.is_empty(), "unknown lang -> empty (fallback to plain)");
    }
}
