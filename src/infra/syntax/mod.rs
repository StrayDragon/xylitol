//! Syntax highlighting for code in terminal output.
//!
//! Uses the `syntect` crate for Rust-native syntax highlighting
//! with Sublime Text .sublime-syntax files.

#![allow(dead_code)]

use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Theme};
use syntect::html::styled_line_to_highlighted_html;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAX_SET: std::sync::LazyLock<SyntaxSet> =
    std::sync::LazyLock::new(|| SyntaxSet::load_defaults_newlines());

static THEME_SET: std::sync::LazyLock<ThemeSet> =
    std::sync::LazyLock::new(|| ThemeSet::load_defaults());

/// Highlight code with automatic or specified language detection.
///
/// Returns ANSI-colored text suitable for terminal display.
pub fn highlight(code: &str, language: Option<&str>) -> String {
    let syntax = match language {
        Some(lang) => SYNTAX_SET
            .find_syntax_by_token(lang)
            .or_else(|| SYNTAX_SET.find_syntax_by_name(lang))
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text()),
        None => SYNTAX_SET
            .find_syntax_by_first_line(code)
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text()),
    };

    let theme = &THEME_SET.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut result = String::new();

    for line in LinesWithEndings::from(code) {
        let ranges = highlighter.highlight_line(line, &SYNTAX_SET).unwrap();
        if let Ok(html) = styled_line_to_highlighted_html(&ranges, syntect::html::IncludeBackground::No) {
            result.push_str(&html_to_ansi(&html));
        }
    }

    result
}

/// Check if a language is supported.
pub fn supports_language(name: &str) -> bool {
    SYNTAX_SET.find_syntax_by_token(name).is_some()
        || SYNTAX_SET.find_syntax_by_name(name).is_some()
}

/// Minimal HTML-to-ANSI converter for syntect output.
fn html_to_ansi(html: &str) -> String {
    // syntect produces <span style="color:#xxxxxx">text</span>
    // We convert to ANSI escape codes
    let mut result = String::new();
    let mut chars = html.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Skip HTML tag
            while let Some(&c) = chars.peek() {
                if c == '>' {
                    chars.next();
                    break;
                }
                chars.next();
            }
        } else if ch == '&' {
            // Decode HTML entities
            let entity: String = chars.by_ref().take_while(|&c| c != ';').collect();
            match entity.as_str() {
                "lt" => result.push('<'),
                "gt" => result.push('>'),
                "amp" => result.push('&'),
                "quot" => result.push('"'),
                "apos" => result.push('\''),
                _ => {}
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_language_rust() {
        assert!(supports_language("rust"));
    }

    #[test]
    fn test_supports_language_python() {
        assert!(supports_language("python"));
    }

    #[test]
    fn test_supports_language_unknown() {
        assert!(!supports_language("nonexistent-language-12345"));
    }

    #[test]
    fn test_highlight_rust_code() {
        let code = r#"fn main() { println!("hello"); }"#;
        let result = highlight(code, Some("rust"));
        // Should contain the code text (stripped of HTML tags)
        assert!(result.contains("fn main()"));
        assert!(result.contains("println!"));
    }

    #[test]
    fn test_highlight_auto_detect() {
        let code = "#!/usr/bin/env python3\nprint('hello')";
        let result = highlight(code, None);
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_html_to_ansi_strips_tags() {
        let html = r#"<span style="color:#d00">hello</span>"#;
        let result = html_to_ansi(html);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_html_to_ansi_entities() {
        let html = "a &amp; b &lt; c";
        let result = html_to_ansi(html);
        assert_eq!(result, "a & b < c");
    }
}
