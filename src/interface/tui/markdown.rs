//! Markdown rendering for the TUI.
//!
//! Delegates to `leaf-core` for parsing and styling, exposing a stable
//! `pub(crate)` interface to the rest of xylitol.

use leaf_core::{LinkSpan, ThemePreset};
use ratatui::style::{Color, Modifier};
use ratatui::text::Line;
use unicode_width::UnicodeWidthStr;

/// Markdown renderer with syntax highlighting support.
///
/// Internally delegates to [`leaf_core::MarkdownRenderer`].
pub(crate) struct MarkdownRenderer {
    inner: leaf_core::MarkdownRenderer,
    /// When true, links get LightBlue + UNDERLINED styling.
    /// Defaults to false per spec r2: avoid forcing link decoration
    /// unless explicitly enabled.
    link_styled: bool,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self {
            inner: leaf_core::MarkdownRenderer::new(),
            link_styled: false,
        }
    }
}

impl MarkdownRenderer {
    pub(crate) fn set_link_styled(&mut self, enabled: bool) {
        self.link_styled = enabled;
    }

    pub(crate) fn set_theme(&mut self, theme_name: &str) -> bool {
        let preset = match theme_name {
            "arctic" | "Arctic" => ThemePreset::Arctic,
            "forest" | "Forest" => ThemePreset::Forest,
            "ocean-dark" | "OceanDark" | "base16-ocean.dark" => ThemePreset::OceanDark,
            "solarized-dark" | "SolarizedDark" | "Solarized (dark)" => ThemePreset::SolarizedDark,
            _ => return false,
        };
        self.inner.set_theme(preset);
        true
    }

    /// Render a markdown string to ratatui lines.
    pub(crate) fn render(&self, source: &str, width: u16) -> Vec<Line<'static>> {
        let output = self.inner.render(source, width as usize);
        let mut lines: Vec<Line<'static>> = output.lines;

        if self.link_styled {
            apply_link_style(&mut lines, &output.links);
        }

        lines
    }

    /// Available theme names for UI selection.
    pub(crate) fn available_themes() -> Vec<&'static str> {
        vec!["Arctic", "Forest", "OceanDark", "SolarizedDark"]
    }
}

/// Apply UNDERLINED + LightBlue to spans that overlap with detected link positions.
fn apply_link_style(lines: &mut [Line<'_>], links: &[LinkSpan]) {
    for link in links {
        if link.line_idx >= lines.len() {
            continue;
        }
        let line = &mut lines[link.line_idx];
        let mut col: usize = 0;
        for span in line.spans.iter_mut() {
            let span_width = UnicodeWidthStr::width(span.content.as_ref());
            let span_end = col + span_width;
            if span_end > link.start_col && col < link.end_col {
                span.style = span
                    .style
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::UNDERLINED);
            }
            col = span_end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_plain(source: &str) -> String {
        let renderer = MarkdownRenderer::default();
        let lines = renderer.render(source, 80);
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn test_headings() {
        let md = "# H1\n## H2\n### H3\n#### H4";
        insta::assert_snapshot!("md_headings", render_to_plain(md));
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        insta::assert_snapshot!("md_code_block", render_to_plain(md));
    }

    #[test]
    fn test_inline_styles() {
        let md = "Normal **bold** *italic* ~~strike~~ `code`";
        insta::assert_snapshot!("md_inline_styles", render_to_plain(md));
    }

    #[test]
    fn test_links() {
        let md = "[click here](https://example.com) and plain text";
        insta::assert_snapshot!("md_links", render_to_plain(md));
    }

    #[test]
    fn test_blockquote() {
        let md = "> quoted line\n> second line\n\nnormal";
        insta::assert_snapshot!("md_blockquote", render_to_plain(md));
    }

    #[test]
    fn test_unordered_list() {
        let md = "- item 1\n- item 2\n  - nested\n- item 3";
        insta::assert_snapshot!("md_unordered_list", render_to_plain(md));
    }

    #[test]
    fn test_ordered_list() {
        let md = "1. first\n2. second\n3. third";
        insta::assert_snapshot!("md_ordered_list", render_to_plain(md));
    }

    #[test]
    fn test_task_list() {
        let md = "- [x] done\n- [ ] pending";
        insta::assert_snapshot!("md_task_list", render_to_plain(md));
    }

    #[test]
    fn test_horizontal_rule() {
        let md = "above\n\n---\n\nbelow";
        insta::assert_snapshot!("md_horizontal_rule", render_to_plain(md));
    }

    #[test]
    fn test_wide_chars_cjk() {
        let md = "中文内容 **粗体** and `代码`";
        insta::assert_snapshot!("md_wide_chars_cjk", render_to_plain(md));
    }

    #[test]
    fn test_table_rendering() {
        let md = "| Col A | Col B |\n|-------|-------|\n| 1     | 2     |";
        insta::assert_snapshot!("md_table", render_to_plain(md));
    }

    #[test]
    fn test_table_multi_row() {
        let md = "| Name | Value | Note |\n|------|-------|------|\n| a | 1 | ok |\n| b | 2 | - |";
        insta::assert_snapshot!("md_table_multi_row", render_to_plain(md));
    }

    #[test]
    fn test_table_single_column() {
        let md = "| Only |\n|------|\n| val  |";
        insta::assert_snapshot!("md_table_single_col", render_to_plain(md));
    }

    // -- link_styled configuration tests ---

    fn render_with_link_styled(source: &str, styled: bool) -> Vec<Line<'static>> {
        let mut renderer = MarkdownRenderer::default();
        renderer.set_link_styled(styled);
        renderer.render(source, 80)
    }

    #[test]
    fn test_link_default_no_forced_style() {
        use ratatui::style::{Color, Modifier};

        let md = "[click](https://example.com)";
        let lines = render_with_link_styled(md, false);
        let link_span = lines
            .first()
            .and_then(|l| l.spans.first())
            .expect("link text span");

        assert!(
            !link_span.style.add_modifier.contains(Modifier::UNDERLINED),
            "link_styled=false should not force UNDERLINED, got: {:?}",
            link_span.style
        );
        assert_ne!(
            link_span.style.fg,
            Some(Color::LightBlue),
            "link_styled=false should not force LightBlue"
        );
    }

    #[test]
    fn test_link_styled_enabled() {
        use ratatui::style::{Color, Modifier};

        let md = "[click](https://example.com)";
        let lines = render_with_link_styled(md, true);

        let has_styled_link = lines.iter().any(|line| {
            line.spans.iter().any(|span| {
                span.style.fg == Some(Color::LightBlue)
                    && span.style.add_modifier.contains(Modifier::UNDERLINED)
            })
        });

        assert!(
            has_styled_link,
            "link_styled=true should have at least one span with UNDERLINED + LightBlue"
        );
    }

    #[test]
    fn test_link_url_suffix_always_shown() {
        let md = "[text](https://example.com)";
        let plain = render_to_plain(md);
        assert!(
            plain.contains("https://example.com") || plain.contains("text"),
            "Link text or URL should appear in rendered output"
        );
    }
}
