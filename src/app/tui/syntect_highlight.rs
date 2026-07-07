//! syntect-based code highlighter (c395, migrated to CellStyle in c399 stage 4).
//!
//! Self-contained module: `highlight()` returns `Vec<StyleSegment>` whose style
//! is the engine's own [`CellStyle`] (no ratatui dependency). The c396 layer
//! previously emitted `ratatui_core::style::Style`; c399 stage 4 removed ratatui
//! entirely, so this layer now emits `CellStyle` directly and the transitional
//! `engine_ratatui_style_adapter` is gone.
//!
//! `SyntectHighlighter` is migrated from the vendored `highlight/syntect_bridge.rs`
//! (derived from codex's `render/highlight.rs` minimal subset). Fixed
//! CatppuccinMocha/Latte theme picked via `COLORFGBG`.

use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;
use two_face::theme::EmbeddedThemeName;

use crate::app::tui::engine::style::{CellStyle, Color};

/// A styled byte range within source code (global byte offsets).
#[derive(Debug, Clone, Copy)]
pub struct StyleSegment {
    pub start: usize,
    pub end: usize,
    pub style: CellStyle,
}

/// Safety guardrails (from codex): reject oversized inputs to avoid
/// pathological CPU/memory usage. Callers fall back to plain text.
const MAX_HIGHLIGHT_BYTES: usize = 512 * 1024;
const MAX_HIGHLIGHT_LINES: usize = 10_000;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

/// Highlight code with a fixed theme, returning `Vec<StyleSegment>` with
/// global byte offsets. Empty Vec = no highlighting (unrecognized language or
/// oversized input) — callers render plain text.
pub fn highlight(lang: &str, code: &str) -> Vec<StyleSegment> {
    if code.is_empty()
        || code.len() > MAX_HIGHLIGHT_BYTES
        || code.lines().count() > MAX_HIGHLIGHT_LINES
    {
        return Vec::new();
    }
    let Some(syntax) = find_syntax(lang) else {
        return Vec::new();
    };
    let theme = theme();
    let mut h = HighlightLines::new(syntax, &theme);
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

/// Pick the highlight theme based on terminal background lightness (c396).
///
/// Pick the embedded syntect theme (`Mocha` for dark, `Latte` for light) from
/// a raw `COLORFGBG` value. Delegates the dark/light classification to the
/// shared [`crate::app::tui::theme_detect::classify_colorfgbg`]; this function
/// only maps [`TerminalTheme`] → [`EmbeddedThemeName`] (code-highlight theme).
///
/// The classification logic is shared so Palette/cursor and code-block
/// highlighting agree on one theme. OSC 11 background probing is a future
/// enhancement (see `theme_detect` module docs).
fn pick_theme_name(colorfgbg: Option<&str>) -> EmbeddedThemeName {
    match crate::app::tui::theme_detect::classify_colorfgbg(colorfgbg) {
        crate::app::tui::theme_detect::TerminalTheme::Light => EmbeddedThemeName::CatppuccinLatte,
        crate::app::tui::theme_detect::TerminalTheme::Dark => EmbeddedThemeName::CatppuccinMocha,
    }
}

static THEME_NAME: OnceLock<EmbeddedThemeName> = OnceLock::new();

fn theme() -> syntect::highlighting::Theme {
    let name =
        *THEME_NAME.get_or_init(|| pick_theme_name(std::env::var("COLORFGBG").ok().as_deref()));
    two_face::theme::extra().get(name).clone()
}

/// syntect `Style` → engine `CellStyle`. Skips background, keeps BOLD, skips
/// italic/underline (poor terminal support, see codex comments).
fn convert_style(syn_style: SyntectStyle) -> CellStyle {
    let mut style = CellStyle::default();
    if let Some(fg) = convert_syntect_color(syn_style.foreground) {
        style.fg = Some(fg);
    }
    if syn_style.font_style.contains(FontStyle::BOLD) {
        style.bold = true;
    }
    style
}

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
        let segs = highlight("rs", "fn main() {}");
        assert!(!segs.is_empty(), "rust code produces styled segments");
        assert!(
            segs.iter().any(|s| s.style.fg.is_some()),
            "some span has foreground color"
        );
    }

    #[test]
    fn unknown_lang_returns_empty() {
        let segs = highlight("not-a-real-lang-xyz", "code");
        assert!(segs.is_empty(), "unknown lang -> empty (fallback)");
    }

    #[test]
    fn style_segment_carries_cell_style() {
        // c399 stage 4: segments now carry CellStyle directly (no ratatui).
        let segs = highlight("rs", "fn main() {}");
        for s in &segs {
            // CellStyle fields are accessible; fg is Option<Color>.
            let _ = s.style.fg;
            let _ = s.style.bold;
        }
    }

    // ── c396 theme selection (pure logic, no env races) ───────────────────

    #[test]
    fn pick_theme_light_when_colorfgbs_light() {
        // bg code 15 (white) → light background → Latte.
        assert_eq!(
            pick_theme_name(Some("0;15")),
            EmbeddedThemeName::CatppuccinLatte
        );
        assert_eq!(
            pick_theme_name(Some("7;7")),
            EmbeddedThemeName::CatppuccinLatte
        );
    }

    #[test]
    fn pick_theme_dark_when_colorfgbs_dark() {
        // bg code 0 (black) → dark background → Mocha.
        assert_eq!(
            pick_theme_name(Some("15;0")),
            EmbeddedThemeName::CatppuccinMocha
        );
        assert_eq!(
            pick_theme_name(Some("default;0")),
            EmbeddedThemeName::CatppuccinMocha
        );
    }

    #[test]
    fn pick_theme_dark_when_colorfgbs_unset() {
        assert_eq!(pick_theme_name(None), EmbeddedThemeName::CatppuccinMocha);
    }

    #[test]
    fn pick_theme_dark_when_colorfgbs_unparseable() {
        assert_eq!(
            pick_theme_name(Some("garbage")),
            EmbeddedThemeName::CatppuccinMocha
        );
        assert_eq!(
            pick_theme_name(Some("onlyonefield")),
            EmbeddedThemeName::CatppuccinMocha
        );
    }
}
