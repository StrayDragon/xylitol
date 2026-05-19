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
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes["base16-ocean.dark"].clone();
        Self { syntax_set, theme }
    }
}

impl MarkdownRenderer {
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
                    _ => {}
                },
                Event::Text(text) => {
                    if current.is_empty() {
                        start_line(&mut current, blockquote_depth, &mut list_stack, in_item);
                    }
                    let style =
                        current_style(bold, italic, strike, heading, !link_stack.is_empty());
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
