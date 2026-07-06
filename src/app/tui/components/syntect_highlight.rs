//! syntect-based code highlighter + segment→line converter (c395).
//!
//! Self-contained module: no dependency on the vendored ratatui-markdown.
//! `StyleSegment` + `segments_to_lines` are migrated from the vendored
//! `highlight/segment.rs`; `SyntectHighlighter` is migrated from
//! `highlight/syntect_bridge.rs` (derived from codex's
//! `render/highlight.rs` minimal subset). Fixed CatppuccinMocha theme.

use std::sync::OnceLock;

use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;
use two_face::theme::EmbeddedThemeName;

/// A styled byte range within source code (global byte offsets).
#[derive(Debug, Clone, Copy)]
pub struct StyleSegment {
    pub start: usize,
    pub end: usize,
    pub style: Style,
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
/// Probes the `COLORFGBS` environment variable (common to iTerm2/Alacritty/
/// Tmux/Kitty/GNOME Terminal): format `"fg;bg"` where the background code
/// `>= 7` indicates a light background → light theme (CatppuccinLatte); else
/// dark theme (CatppuccinMocha). Falls back to dark when `COLORFGBS` is unset
/// or unparseable.
///
/// OSC 11 background query is intentionally NOT used (would race with user
/// input under the inline viewport); see c396 design D5.
fn pick_theme_name(colorfgbs: Option<&str>) -> EmbeddedThemeName {
    let Some(val) = colorfgbs else {
        return EmbeddedThemeName::CatppuccinMocha;
    };
    let parts: Vec<&str> = val.split(';').collect();
    if parts.len() >= 2
        && let Ok(bg) = parts[1].trim().parse::<u8>()
    {
        return if bg >= 7 {
            EmbeddedThemeName::CatppuccinLatte
        } else {
            EmbeddedThemeName::CatppuccinMocha
        };
    }
    EmbeddedThemeName::CatppuccinMocha
}

static THEME_NAME: OnceLock<EmbeddedThemeName> = OnceLock::new();

fn theme() -> syntect::highlighting::Theme {
    let name =
        *THEME_NAME.get_or_init(|| pick_theme_name(std::env::var("COLORFGBS").ok().as_deref()));
    two_face::theme::extra().get(name).clone()
}

/// syntect `Style` → ratatui `Style`. Skips background, keeps BOLD, skips
/// italic/underline (poor terminal support, see codex comments).
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

/// Convert highlighted segments into per-line `Vec<Line>` with width wrapping.
/// `prefix` is prepended to every line (e.g. indent); pass "" for none.
pub fn segments_to_lines(
    source: &str,
    segments: &[StyleSegment],
    prefix: &str,
    prefix_style: Style,
    max_width: usize,
) -> Vec<Line<'static>> {
    let prefix_width = unicode_width::UnicodeWidthStr::width(prefix);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut line_start: usize = 0;
    for raw_line in source.split('\n') {
        let line_end = line_start + raw_line.len();
        let line_segs: Vec<StyleSegment> = segments
            .iter()
            .filter(|s| s.start < line_end && s.end > line_start)
            .map(|s| StyleSegment {
                start: s.start.saturating_sub(line_start),
                end: s.end.min(line_end).saturating_sub(line_start),
                style: s.style,
            })
            .filter(|s| s.start < s.end)
            .collect();
        let mut wrapped = wrap_line(
            raw_line.replace('\t', "    ").as_str(),
            &line_segs,
            prefix,
            prefix_width,
            prefix_style,
            max_width,
        );
        lines.append(&mut wrapped);
        line_start = line_end + 1;
    }
    lines
}

fn wrap_line(
    text: &str,
    segments: &[StyleSegment],
    prefix: &str,
    prefix_width: usize,
    prefix_style: Style,
    max_width: usize,
) -> Vec<Line<'static>> {
    let mut result = Vec::new();
    if text.is_empty() {
        let mut spans: Vec<Span<'static>> = Vec::new();
        if !prefix.is_empty() {
            spans.push(Span::styled(prefix.to_string(), prefix_style));
        }
        result.push(Line::from(spans));
        return result;
    }
    let sorted: Vec<(usize, usize, Style)> =
        segments.iter().map(|s| (s.start, s.end, s.style)).collect();
    let mut seg_idx = 0;
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    if !prefix.is_empty() {
        current_spans.push(Span::styled(prefix.to_string(), prefix_style));
    }
    let mut current_len = prefix_width;
    let mut byte_pos: usize = 0;
    for ch in text.chars() {
        let char_byte_start = byte_pos;
        byte_pos += ch.len_utf8();
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_len + cw > max_width && current_len > prefix_width {
            result.push(Line::from(std::mem::take(&mut current_spans)));
            current_spans = Vec::new();
            if !prefix.is_empty() {
                current_spans.push(Span::styled(prefix.to_string(), prefix_style));
            }
            current_len = prefix_width;
        }
        let style = style_at_byte(&sorted, &mut seg_idx, char_byte_start);
        if let Some(last) = current_spans.last_mut()
            && last.style == style
        {
            last.content = format!("{}{}", last.content, ch).into();
            current_len += cw;
            continue;
        }
        current_spans.push(Span::styled(ch.to_string(), style));
        current_len += cw;
    }
    if !current_spans.is_empty() {
        result.push(Line::from(current_spans));
    }
    result
}

fn style_at_byte(
    segments: &[(usize, usize, Style)],
    seg_idx: &mut usize,
    byte_pos: usize,
) -> Style {
    while *seg_idx < segments.len() && segments[*seg_idx].1 <= byte_pos {
        *seg_idx += 1;
    }
    if *seg_idx < segments.len() && segments[*seg_idx].0 <= byte_pos {
        segments[*seg_idx].2
    } else {
        Style::default()
    }
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
    fn segments_to_lines_multiline() {
        let source = "line1\nline2\nline3";
        let lines = segments_to_lines(source, &[], "", Style::default(), 80);
        assert_eq!(lines.len(), 3);
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
