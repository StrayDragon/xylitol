//! Markdown rendering for the TUI.
//!
//! Uses `pulldown-cmark` to parse CommonMark and maps events into
//! ratatui `Line` / `Span` values.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

#[derive(Debug, Clone, Copy)]
enum ListKind {
    Bullet,
    Ordered,
}

#[derive(Debug, Clone, Copy)]
struct ListState {
    kind: ListKind,
    next_number: u64,
}

/// Markdown renderer with syntax highlighting support.
pub(crate) struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme: Theme,
    /// When true, links get LightBlue + UNDERLINED styling.
    /// Defaults to false per spec r2: avoid forcing link decoration
    /// unless explicitly enabled.
    link_styled: bool,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes["base16-ocean.dark"].clone();
        Self {
            syntax_set,
            theme,
            link_styled: false,
        }
    }
}

impl MarkdownRenderer {
    pub(crate) fn set_link_styled(&mut self, enabled: bool) {
        self.link_styled = enabled;
    }

    pub(crate) fn set_theme(&mut self, theme_name: &str) -> bool {
        let theme_set = ThemeSet::load_defaults();
        let Some(theme) = theme_set.themes.get(theme_name) else {
            return false;
        };
        self.theme = theme.clone();
        true
    }

    /// Render a markdown string to ratatui lines.
    pub(crate) fn render(&self, source: &str, _width: u16) -> Vec<Line<'static>> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_TABLES);

        let parser = Parser::new_ext(source, options);

        let mut out: Vec<Line<'static>> = Vec::new();
        let mut current: Vec<Span<'static>> = Vec::new();

        let mut bold = 0u32;
        let mut italic = 0u32;
        let mut strike = 0u32;
        let mut heading: Option<HeadingLevel> = None;
        let mut link_stack: Vec<String> = Vec::new();
        let mut blockquote_depth: usize = 0;
        let mut list_stack: Vec<ListState> = Vec::new();
        let mut in_item = false;

        let mut in_code_block: Option<String> = None;
        let mut code_buf = String::new();

        let mut in_table_head = false;
        let mut cell_index: usize = 0;

        for event in parser {
            // Code blocks are buffered and rendered with syntect once closed.
            if in_code_block.is_some() {
                match event {
                    Event::End(TagEnd::CodeBlock) => {
                        let lang = in_code_block.take().unwrap_or_default();
                        let lang = lang.trim();
                        let lang = if lang.is_empty() { None } else { Some(lang) };

                        for line in self.highlight_code_block(&code_buf, lang) {
                            out.push(line);
                        }
                        code_buf.clear();
                    }
                    Event::Text(text)
                    | Event::Code(text)
                    | Event::Html(text)
                    | Event::InlineHtml(text) => {
                        code_buf.push_str(text.as_ref());
                    }
                    Event::SoftBreak | Event::HardBreak => {
                        code_buf.push('\n');
                    }
                    _ => {}
                }
                continue;
            }

            match event {
                Event::Start(tag) => match tag {
                    Tag::Heading { level, .. } => {
                        if !current.is_empty() {
                            push_line(&mut out, &mut current);
                        }
                        heading = Some(level);
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    Tag::Strong => bold = bold.saturating_add(1),
                    Tag::Emphasis => italic = italic.saturating_add(1),
                    Tag::Strikethrough => strike = strike.saturating_add(1),
                    Tag::Link { dest_url, .. } => link_stack.push(dest_url.to_string()),
                    Tag::BlockQuote(_) => {
                        if !current.is_empty() {
                            push_line(&mut out, &mut current);
                        }
                        blockquote_depth = blockquote_depth.saturating_add(1);
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    Tag::List(start) => {
                        let state = match start {
                            Some(n) => ListState {
                                kind: ListKind::Ordered,
                                next_number: n,
                            },
                            None => ListState {
                                kind: ListKind::Bullet,
                                next_number: 1,
                            },
                        };
                        list_stack.push(state);
                    }
                    Tag::Item => {
                        if !current.is_empty() {
                            push_line(&mut out, &mut current);
                        }
                        in_item = true;
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    Tag::Paragraph => {
                        if !current.is_empty() {
                            push_line(&mut out, &mut current);
                        }
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    Tag::CodeBlock(kind) => {
                        if !current.is_empty() {
                            push_line(&mut out, &mut current);
                        }
                        let lang = match kind {
                            CodeBlockKind::Fenced(info) => info.to_string(),
                            CodeBlockKind::Indented => String::new(),
                        };
                        in_code_block = Some(lang);
                    }
                    Tag::Table(_) if !current.is_empty() => {
                        push_line(&mut out, &mut current);
                    }
                    Tag::TableHead => {
                        in_table_head = true;
                        cell_index = 0;
                    }
                    Tag::TableRow => {
                        cell_index = 0;
                    }
                    Tag::TableCell if cell_index > 0 => {
                        current.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                    }
                    _ => {}
                },
                Event::End(tag_end) => match tag_end {
                    TagEnd::Heading(_level) => {
                        heading = None;
                        push_line(&mut out, &mut current);
                        out.push(Line::from(""));
                    }
                    TagEnd::Strong => bold = bold.saturating_sub(1),
                    TagEnd::Emphasis => italic = italic.saturating_sub(1),
                    TagEnd::Strikethrough => strike = strike.saturating_sub(1),
                    TagEnd::Link => {
                        if let Some(dest) = link_stack.pop()
                            && !dest.is_empty()
                            && !current.is_empty()
                        {
                            current.push(Span::styled(
                                format!(" ({dest})"),
                                Style::default().fg(Color::DarkGray),
                            ));
                        }
                    }
                    TagEnd::BlockQuote(_kind) => {
                        if !current.is_empty() {
                            push_line(&mut out, &mut current);
                        }
                        blockquote_depth = blockquote_depth.saturating_sub(1);
                        out.push(Line::from(""));
                    }
                    TagEnd::Item => {
                        in_item = false;
                        push_line(&mut out, &mut current);
                    }
                    TagEnd::List(_ordered) => {
                        list_stack.pop();
                        out.push(Line::from(""));
                    }
                    TagEnd::Paragraph => {
                        push_line(&mut out, &mut current);
                        out.push(Line::from(""));
                    }
                    TagEnd::Table => {
                        if !current.is_empty() {
                            push_line(&mut out, &mut current);
                        }
                        out.push(Line::from(""));
                    }
                    TagEnd::TableHead => {
                        push_line(&mut out, &mut current);
                        in_table_head = false;
                    }
                    TagEnd::TableRow => {
                        push_line(&mut out, &mut current);
                    }
                    TagEnd::TableCell => {
                        cell_index += 1;
                        if in_table_head && let Some(last) = current.last_mut() {
                            *last = Span::styled(
                                last.content.to_string(),
                                last.style.add_modifier(Modifier::BOLD),
                            );
                        }
                    }
                    _ => {}
                },
                Event::Text(text) => {
                    if current.is_empty() {
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    let link_active = self.link_styled && !link_stack.is_empty();
                    let style = current_style(bold, italic, strike, heading, link_active);
                    current.push(Span::styled(text.to_string(), style));
                }
                Event::Code(code) => {
                    if current.is_empty() {
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    current.push(Span::styled(
                        code.to_string(),
                        Style::default()
                            .fg(Color::Green)
                            .bg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                Event::SoftBreak => {
                    if current.is_empty() {
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    current.push(Span::raw(" "));
                }
                Event::HardBreak => {
                    push_line(&mut out, &mut current);
                }
                Event::Rule => {
                    if !current.is_empty() {
                        push_line(&mut out, &mut current);
                    }
                    out.push(Line::from(Span::styled(
                        "─".repeat(32),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                Event::TaskListMarker(checked) => {
                    if current.is_empty() {
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    let marker = if checked { "[x] " } else { "[ ] " };
                    current.push(Span::styled(marker, Style::default().fg(Color::DarkGray)));
                }
                Event::Html(html) | Event::InlineHtml(html) => {
                    if current.is_empty() {
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    current.push(Span::styled(
                        html.to_string(),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                Event::FootnoteReference(label) => {
                    if current.is_empty() {
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    current.push(Span::styled(
                        format!("[^{label}]"),
                        Style::default().fg(Color::LightMagenta),
                    ));
                }
                Event::InlineMath(math) | Event::DisplayMath(math) => {
                    if current.is_empty() {
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    current.push(Span::styled(
                        math.to_string(),
                        Style::default().fg(Color::LightMagenta),
                    ));
                }
            }
        }

        if !current.is_empty() {
            push_line(&mut out, &mut current);
        }

        out
    }

    fn highlight_code_block(&self, code: &str, lang: Option<&str>) -> Vec<Line<'static>> {
        let syntax = lang
            .and_then(|token| self.syntax_set.find_syntax_by_token(token))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();

        for line in LinesWithEndings::from(code) {
            let ranges = match highlighter.highlight_line(line, &self.syntax_set) {
                Ok(r) => r,
                Err(_) => {
                    lines.push(Line::from(line.trim_end_matches('\n').to_string()));
                    continue;
                }
            };

            let spans: Vec<Span<'static>> = ranges
                .into_iter()
                .map(|(style, text)| {
                    let fg = style.foreground;
                    let mut s = Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b));
                    if style.font_style.contains(FontStyle::BOLD) {
                        s = s.add_modifier(Modifier::BOLD);
                    }
                    if style.font_style.contains(FontStyle::ITALIC) {
                        s = s.add_modifier(Modifier::ITALIC);
                    }
                    Span::styled(text.trim_end_matches('\n').to_string(), s)
                })
                .collect();

            lines.push(Line::from(spans));
        }

        lines
    }
}

fn push_line(out: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if spans.is_empty() {
        out.push(Line::from(""));
        return;
    }
    if let Some(last) = spans.last_mut() {
        let trimmed = last.content.trim_end();
        if trimmed != last.content.as_ref() {
            *last = Span::styled(trimmed.to_owned(), last.style);
        }
    }
    out.push(Line::from(std::mem::take(spans)));
}

fn start_line(
    spans: &mut Vec<Span<'static>>,
    blockquote_depth: usize,
    list_stack: &mut [ListState],
    in_item: bool,
) {
    if blockquote_depth > 0 {
        spans.push(Span::styled(
            "│ ".repeat(blockquote_depth),
            Style::default().fg(Color::DarkGray),
        ));
    }

    if in_item {
        let indent = "  ".repeat(list_stack.len().saturating_sub(1));
        if !indent.is_empty() {
            spans.push(Span::raw(indent));
        }

        if let Some(state) = list_stack.last_mut() {
            match state.kind {
                ListKind::Bullet => {
                    spans.push(Span::styled("- ", Style::default().fg(Color::DarkGray)))
                }
                ListKind::Ordered => {
                    let n = state.next_number;
                    state.next_number = state.next_number.saturating_add(1);
                    spans.push(Span::styled(
                        format!("{n}. "),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }
        }
    }
}

fn current_style(
    bold: u32,
    italic: u32,
    strike: u32,
    heading: Option<HeadingLevel>,
    link_active: bool,
) -> Style {
    let mut style = Style::default();
    if bold > 0 {
        style = style.add_modifier(Modifier::BOLD);
    }
    if italic > 0 {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if strike > 0 {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }

    if let Some(level) = heading {
        style = style.add_modifier(Modifier::BOLD);
        style = style.fg(match level {
            HeadingLevel::H1 => Color::White,
            HeadingLevel::H2 => Color::LightCyan,
            HeadingLevel::H3 => Color::Cyan,
            _ => Color::LightBlue,
        });
    }

    if link_active {
        style = style
            .fg(Color::LightBlue)
            .add_modifier(Modifier::UNDERLINED);
    }

    style
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
        let md = "[click](https://example.com)";
        let lines = render_with_link_styled(md, true);
        let link_span = lines
            .first()
            .and_then(|l| l.spans.first())
            .expect("link text span");

        assert!(
            link_span.style.add_modifier.contains(Modifier::UNDERLINED),
            "link_styled=true should apply UNDERLINED"
        );
        assert_eq!(
            link_span.style.fg,
            Some(Color::LightBlue),
            "link_styled=true should apply LightBlue"
        );
    }

    #[test]
    fn test_link_url_suffix_always_shown() {
        let md = "[text](https://example.com)";
        let plain_off = {
            let r = MarkdownRenderer::default();
            let lines = r.render(md, 80);
            lines
                .iter()
                .flat_map(|l| l.spans.iter())
                .map(|s| s.content.as_ref())
                .collect::<String>()
        };
        assert!(
            plain_off.contains("(https://example.com)"),
            "URL suffix should appear regardless of link_styled"
        );
    }
}
