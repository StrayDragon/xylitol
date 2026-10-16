//! syntect → ANSI lines for Markdown fences (Catppuccin Mocha).

use std::sync::OnceLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme};
use syntect::parsing::SyntaxSet;
use syntect::util::{LinesWithEndings, as_24_bit_terminal_escaped};

/// Skip highlighting above this many UTF-8 bytes.
pub const MAX_HIGHLIGHT_BYTES: usize = 512 * 1024;
/// Skip highlighting above this many lines.
pub const MAX_HIGHLIGHT_LINES: usize = 10_000;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME: OnceLock<Theme> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

fn theme() -> &'static Theme {
    THEME.get_or_init(|| {
        let set = two_face::theme::extra();
        set.get(two_face::theme::EmbeddedThemeName::CatppuccinMocha)
            .clone()
    })
}

fn plain_lines(code: &str) -> Vec<String> {
    code.lines().map(str::to_string).collect()
}

/// Highlight `code` for optional language id; returns ANSI-escaped lines.
pub fn highlight_code(code: &str, lang: Option<&str>) -> Vec<String> {
    if code.len() > MAX_HIGHLIGHT_BYTES || code.lines().count() > MAX_HIGHLIGHT_LINES {
        return plain_lines(code);
    }
    if code.is_empty() {
        return Vec::new();
    }

    let ss = syntax_set();
    let theme = theme();

    let syntax = lang
        .and_then(|l| ss.find_syntax_by_token(l))
        .or_else(|| ss.find_syntax_by_extension(lang.unwrap_or("")))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut out = Vec::new();
    for line in LinesWithEndings::from(code) {
        let ranges: Vec<(Style, &str)> = match highlighter.highlight_line(line, ss) {
            Ok(r) => r,
            Err(_) => return plain_lines(code),
        };
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        let trimmed = escaped.trim_end_matches(['\r', '\n']);
        out.push(trimmed.to_string());
    }
    out
}

pub fn highlight_code_owned(code: String, lang: Option<&str>) -> Vec<String> {
    highlight_code(&code, lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_fence_emits_ansi() {
        let lines = highlight_code("fn main() {\n    let x = 1;\n}\n", Some("rust"));
        assert!(!lines.is_empty());
        let joined = lines.join("\n");
        assert!(
            joined.contains('\x1b'),
            "expected ANSI escapes, got: {joined:?}"
        );
    }

    #[test]
    fn oversize_falls_back_plain() {
        let big = "a\n".repeat(MAX_HIGHLIGHT_LINES + 10);
        let lines = highlight_code(&big, Some("rust"));
        assert!(!lines.is_empty());
        assert!(
            !lines.iter().any(|l| l.contains('\x1b')),
            "oversize must not highlight"
        );
    }
}
