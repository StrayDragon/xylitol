//! Markdown rendering for the TUI.
//!
//! Converts markdown text to ratatui [`Line`] items with basic formatting
//! (**bold**, *italic*, `code`, headers, code blocks with syntax highlighting).

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Markdown renderer with syntax highlighting support.
pub(crate) struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = ts.themes["base16-ocean.dark"].clone();
        Self {
            syntax_set: ss,
            theme,
        }
    }
}

impl MarkdownRenderer {
    /// Render a markdown string to ratatui lines, wrapped at `base_width`.
    pub(crate) fn render<'a>(&'a self, text: &'a str, _base_width: u16) -> Vec<Line<'a>> {
        let mut lines: Vec<Line<'_>> = Vec::new();
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_buf: Vec<String> = Vec::new();

        for raw_line in text.lines() {
            let trimmed = raw_line.trim();

            if trimmed.starts_with("```") {
                if in_code_block {
                    // End of code block — render.
                    let lang_token = if code_lang.is_empty() {
                        None
                    } else {
                        Some(code_lang.as_str())
                    };
                    for hl_line in self.highlight_lines(&code_buf, lang_token) {
                        lines.push(hl_line);
                    }
                    code_buf.clear();
                    in_code_block = false;
                    code_lang.clear();
                } else {
                    in_code_block = true;
                    code_lang = trimmed.trim_start_matches("```").trim().to_string();
                }
                continue;
            }

            if in_code_block {
                code_buf.push(raw_line.to_string());
                continue;
            }

            if raw_line.is_empty() {
                lines.push(Line::from(""));
                continue;
            }

            // Headers.
            if let Some(content) = raw_line.strip_prefix("### ") {
                lines.push(Line::from(Span::styled(
                    content,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )));
                continue;
            }
            if let Some(content) = raw_line.strip_prefix("## ") {
                lines.push(Line::from(Span::styled(
                    content,
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )));
                continue;
            }
            if let Some(content) = raw_line.strip_prefix("# ") {
                lines.push(Line::from(Span::styled(
                    content,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )));
                continue;
            }

            // Regular paragraph line — parse inline formatting.
            lines.push(Line::from(self.parse_inline(raw_line)));
        }

        // Close unclosed code block.
        if in_code_block && !code_buf.is_empty() {
            let lang_token = if code_lang.is_empty() {
                None
            } else {
                Some(code_lang.as_str())
            };
            for hl_line in self.highlight_lines(&code_buf, lang_token) {
                lines.push(hl_line);
            }
        }

        lines
    }

    /// Parse inline formatting: **bold**, *italic*, `code`.
    fn parse_inline(&self, text: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            // Inline code `...`
            if let Some((before, content)) = find_backtick_delimited(remaining) {
                if !before.is_empty() {
                    spans.extend(self.parse_bold_italic(before));
                }
                spans.push(Span::styled(
                    content.to_string(),
                    Style::default().fg(Color::Green).bg(Color::DarkGray),
                ));
                remaining = &remaining[before.len() + content.len() + 2..];
                continue;
            }

            spans.extend(self.parse_bold_italic(remaining));
            break;
        }

        spans
    }

    /// Parse **bold** and *italic* from the remaining text.
    fn parse_bold_italic(&self, text: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            // Bold **...**
            if let Some((before, content)) = find_double_star_delimited(remaining) {
                if !before.is_empty() {
                    // Check for italic inside before text.
                    spans.extend(self.parse_italic_only(before));
                }
                spans.push(Span::styled(
                    content.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                remaining = &remaining[before.len() + content.len() + 4..];
                continue;
            }

            spans.extend(self.parse_italic_only(remaining));
            break;
        }

        spans
    }

    /// Parse *italic* only (no bold).
    fn parse_italic_only(&self, text: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            if let Some((before, content)) = find_single_star_delimited(remaining) {
                if !before.is_empty() {
                    spans.push(Span::raw(before.to_string()));
                }
                spans.push(Span::styled(
                    content.to_string(),
                    Style::default().add_modifier(Modifier::ITALIC),
                ));
                remaining = &remaining[before.len() + content.len() + 2..];
            } else {
                spans.push(Span::raw(remaining.to_string()));
                break;
            }
        }

        spans
    }

    /// Highlight code lines using syntect, returning ratatui lines.
    fn highlight_lines(&self, code_lines: &[String], lang: Option<&str>) -> Vec<Line<'static>> {
        if code_lines.is_empty() {
            return Vec::new();
        }

        let syntax = lang
            .and_then(|l| self.syntax_set.find_syntax_by_token(l))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();
        let code = code_lines.join("\n");

        for line in LinesWithEndings::from(&code) {
            let ranges = highlighter.highlight_line(line, &self.syntax_set).unwrap();
            let spans: Vec<Span<'_>> = ranges
                .into_iter()
                .map(|(style, text)| {
                    let fg = style.foreground;
                    let mut ratatui_style = Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b));
                    if style.font_style.contains(FontStyle::BOLD) {
                        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                    }
                    if style.font_style.contains(FontStyle::ITALIC) {
                        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                    }
                    Span::styled(
                        // Strip trailing newline for cleaner display.
                        text.trim_end_matches('\n').to_string(),
                        ratatui_style,
                    )
                })
                .collect();
            lines.push(Line::from(spans));
        }

        lines
    }
}

// ── Inline delimiter helpers ────────────────────────────────────────────────

/// Find text between backticks: \`...\`.
fn find_backtick_delimited(text: &str) -> Option<(&str, &str)> {
    let open = text.find('`')?;
    let after_open = &text[open + 1..];
    let close = after_open.find('`')?;
    Some((&text[..open], &after_open[..close]))
}

/// Find text between double stars: \*\*...\*\*.
fn find_double_star_delimited(text: &str) -> Option<(&str, &str)> {
    let open = text.find("**")?;
    let after_open = &text[open + 2..];
    let close = after_open.find("**")?;
    Some((&text[..open], &after_open[..close]))
}

/// Find text between single stars: \*...\*.
fn find_single_star_delimited(text: &str) -> Option<(&str, &str)> {
    let open = text.find('*')?;
    let after_open = &text[open + 1..];
    let close = after_open.find('*')?;
    Some((&text[..open], &after_open[..close]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold_parsing() {
        let r = MarkdownRenderer::default();
        let spans = r.parse_inline("hello **world** here");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "hello ");
        assert_eq!(spans[2].content, " here");
    }

    #[test]
    fn test_inline_code() {
        let r = MarkdownRenderer::default();
        let spans = r.parse_inline("use `code` here");
        assert_eq!(spans.len(), 3);
    }

    #[test]
    fn test_headers() {
        let r = MarkdownRenderer::default();
        let lines = r.render("# Big\n## Medium\n### Small", 80);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_code_block() {
        let r = MarkdownRenderer::default();
        let lines = r.render("text\n```rust\nfn main() {}\n```\nmore", 80);
        assert_eq!(lines.len(), 3);
    }
}
