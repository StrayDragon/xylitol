//! Self-researched lightweight markdown renderer (c395).
//!
//! Based on pulldown-cmark (pull parser → Event iterator) + syntect (code
//! highlighting). Outputs `Vec<Line<'static>>` with terminal-friendly styling
//! only (bold/italic/color via ratatui Style + Modifier) — NO box-drawing
//! borders, NO tree connectors, NO table border lines. Code blocks use
//! syntect highlighting without background/border/language-label.
//!
//! Architecture: a stateful `Writer` consumes `pulldown_cmark::Event`s,
//! accumulating spans into the current line and flushing on block boundaries.
//! Inline emphasis (strong/emphasis/code) stacks a style modifier.
#![allow(clippy::enum_variant_names, clippy::doc_lazy_continuation)]

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::{Line, Span};

use crate::app::tui::components::syntect_highlight;

/// Style tokens injected by the caller. Each role (user/assistant/thinking)
/// builds its own `RenderStyle`; the renderer is shared.
#[derive(Clone, Copy)]
pub struct RenderStyle {
    /// Normal paragraph text.
    pub text: Style,
    /// Headings (bold variant of text).
    pub heading: Style,
    /// Inline `code` foreground.
    pub code_fg: Color,
    /// Blockquote text (italic + dimmed).
    pub quote: Style,
}

impl RenderStyle {
    /// Style for assistant replies (default theme).
    pub fn for_assistant() -> Self {
        Self {
            text: Style::default(),
            heading: Style::default().add_modifier(Modifier::BOLD),
            code_fg: Color::Cyan,
            quote: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        }
    }

    /// Style for user input echo (cyan/bold).
    pub fn for_user() -> Self {
        Self {
            text: Style::default().fg(Color::Cyan),
            heading: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            code_fg: Color::Cyan,
            quote: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        }
    }

    /// Style for thinking/reasoning (dimmed).
    pub fn for_thinking() -> Self {
        let dim = Color::DarkGray;
        Self {
            text: Style::default().fg(dim),
            heading: Style::default().fg(dim).add_modifier(Modifier::BOLD),
            code_fg: Color::DarkGray,
            quote: Style::default().fg(dim).add_modifier(Modifier::ITALIC),
        }
    }
}

/// Render finalized markdown text into styled `Line`s (c395).
///
/// `width` is used for code-block line wrapping. Streaming text MUST NOT pass
/// here (only finalized, committed text).
pub fn render_markdown(text: &str, width: u16, style: RenderStyle) -> Vec<Line<'static>> {
    let mut writer = Writer::new(width as usize, style);
    for event in Parser::new(text) {
        writer.handle_event(event);
    }
    writer.finish();
    writer.lines
}

// ── Writer state machine ──────────────────────────────────────────────────

/// Track the display width of the current pending line so paragraphs wrap
/// at `width` (CJK-aware). Appended in `push_str`.
struct Writer {
    width: usize,
    style: RenderStyle,
    /// Completed lines.
    lines: Vec<Line<'static>>,
    /// Spans of the in-progress line.
    pending: Vec<Span<'static>>,
    /// Current inline style (bold/italic/code stacked).
    inline_mods: Vec<Modifier>,
    /// Inline code active (uses code_fg color).
    in_code: bool,
    /// Block context stack.
    blocks: Vec<Block>,
    /// Code block accumulator.
    code_lang: Option<String>,
    code_buf: String,
    /// Pending link URL (appended as " (url)" at End(Link)).
    pending_link_url: Option<String>,
    /// Display width of the current pending line (for paragraph wrapping).
    pending_width: usize,
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // HeadingLevel retained for future per-level styling.
enum Block {
    Paragraph,
    Heading(HeadingLevel),
    CodeBlock,
    BlockQuote,
    ListItem,
}

impl Writer {
    fn new(width: usize, style: RenderStyle) -> Self {
        Self {
            width,
            style,
            lines: Vec::new(),
            pending: Vec::new(),
            inline_mods: Vec::new(),
            in_code: false,
            blocks: Vec::new(),
            code_lang: None,
            code_buf: String::new(),
            pending_link_url: None,
            pending_width: 0,
        }
    }

    /// The current text style: base (paragraph/heading/quote) + inline mods.
    fn current_style(&self) -> Style {
        let base = match self.blocks.last() {
            Some(Block::Heading(_)) => self.style.heading,
            Some(Block::BlockQuote) => self.style.quote,
            _ => self.style.text,
        };
        let mods: Modifier = self
            .inline_mods
            .iter()
            .copied()
            .fold(Modifier::empty(), |acc, m| acc.union(m));
        let mut s = base.add_modifier(mods);
        if self.in_code {
            s = s.fg(self.style.code_fg);
        }
        s
    }

    fn push_str(&mut self, text: &str) {
        let style = self.current_style();
        // Walk char-by-char: wrap at `width` (CJK-aware), flush on '\n'.
        for ch in text.chars() {
            if ch == '\n' {
                self.flush_line();
                continue;
            }
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if cw > 0 && self.pending_width + cw > self.width && self.pending_width > 0 {
                self.flush_line();
            }
            // Merge into last span if same style, else push new span.
            if let Some(last) = self.pending.last_mut()
                && last.style == style
            {
                last.content = format!("{}{}", last.content, ch).into();
            } else {
                self.pending.push(Span::styled(ch.to_string(), style));
            }
            self.pending_width += cw;
        }
    }

    /// Push a span (e.g. list marker), merging with previous if same style.
    /// Accounts for the span's display width in the pending-line width.
    fn push_span(&mut self, span: Span<'static>) {
        let w = unicode_width::UnicodeWidthStr::width(span.content.as_ref());
        if let Some(last) = self.pending.last_mut()
            && last.style == span.style
        {
            last.content = format!("{}{}", last.content, span.content).into();
        } else {
            self.pending.push(span);
        }
        self.pending_width += w;
    }

    fn flush_line(&mut self) {
        if !self.pending.is_empty() {
            let line = Line::from(std::mem::take(&mut self.pending));
            self.lines.push(line);
            self.pending_width = 0;
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_block_or_inline(tag),
            Event::End(end) => self.end_block_or_inline(end),
            Event::Text(s) => {
                if matches!(self.blocks.last(), Some(Block::CodeBlock)) {
                    self.code_buf.push_str(&s);
                } else {
                    self.push_str(&s);
                }
            }
            Event::Code(s) => {
                self.in_code = true;
                self.push_str(&s);
                self.in_code = false;
            }
            Event::SoftBreak | Event::HardBreak => self.flush_line(),
            Event::Rule => {
                self.flush_line();
            }
            // MVP ignores HTML/Math/footnotes/task-list markers.
            _ => {}
        }
    }

    fn start_block_or_inline(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.blocks.push(Block::Paragraph);
            }
            Tag::Heading { level, .. } => {
                self.flush_line();
                self.blocks.push(Block::Heading(level));
            }
            Tag::CodeBlock(kind) => {
                self.flush_line();
                let lang = match kind {
                    CodeBlockKind::Fenced(s) if !s.is_empty() => Some(s.into_string()),
                    _ => None,
                };
                self.code_lang = lang;
                self.code_buf.clear();
                self.blocks.push(Block::CodeBlock);
            }
            Tag::BlockQuote(_) => {
                self.flush_line();
                self.blocks.push(Block::BlockQuote);
            }
            Tag::List(None) => { /* unordered list container */ }
            Tag::Item => {
                self.flush_line();
                self.blocks.push(Block::ListItem);
                self.push_span(Span::styled("• ".to_string(), self.style.text));
            }
            Tag::Strong => self.inline_mods.push(Modifier::BOLD),
            Tag::Emphasis => self.inline_mods.push(Modifier::ITALIC),
            Tag::Link { dest_url, .. } => {
                // codex-style "text (url)" — the url is appended at End(Link).
                self.pending_link_url = Some(dest_url.into_string());
            }
            _ => {}
        }
    }

    fn end_block_or_inline(&mut self, end: TagEnd) {
        match end {
            TagEnd::Paragraph | TagEnd::Item => {
                self.flush_line();
                self.blocks.pop();
            }
            TagEnd::Heading(_) => {
                self.flush_line();
                self.blocks.pop();
            }
            TagEnd::CodeBlock => {
                self.flush_code_block();
                self.blocks.pop();
                self.code_lang = None;
            }
            TagEnd::BlockQuote(_) => {
                self.flush_line();
                self.blocks.pop();
            }
            TagEnd::List(_) => {}
            TagEnd::Strong | TagEnd::Emphasis => {
                self.inline_mods.pop();
            }
            TagEnd::Link => {
                if let Some(url) = self.pending_link_url.take() {
                    let paren_style = self.style.text.fg(Color::DarkGray);
                    let url_style = self
                        .style
                        .text
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::UNDERLINED);
                    self.push_span(Span::styled(" (".to_string(), paren_style));
                    self.push_span(Span::styled(url, url_style));
                    self.push_span(Span::styled(")".to_string(), paren_style));
                }
            }
            _ => {}
        }
    }

    fn flush_code_block(&mut self) {
        let lang = self.code_lang.as_deref().unwrap_or("");
        let segments = syntect_highlight::highlight(lang, &self.code_buf);
        let code_lines = syntect_highlight::segments_to_lines(
            &self.code_buf,
            &segments,
            "",
            Style::default(),
            self.width,
        );
        self.lines.extend(code_lines);
        self.code_buf.clear();
    }

    fn finish(&mut self) {
        self.flush_line();
        if self.lines.is_empty() {
            self.lines.push(Line::raw(""));
        }
    }
}

#[cfg(test)]
mod tests {
    //! Behavior assertions for the self-researched renderer (spec tui41).
    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::buffer::CellWidth;
    use ratatui_core::layout::Rect;
    use ratatui_core::terminal::Terminal;
    use ratatui_core::widgets::Widget;

    fn render_to_buf(text: &str, width: u16) -> (ratatui_core::buffer::Buffer, u16) {
        let lines = render_markdown(text, width, RenderStyle::for_assistant());
        let height = lines.len() as u16;
        let mut term = Terminal::new(TestBackend::new(width, height.max(1))).unwrap();
        term.draw(|f| {
            let area = Rect::new(0, 0, width, height.max(1));
            f.render_widget(ratatui_widgets::paragraph::Paragraph::new(lines), area);
        })
        .unwrap();
        (term.backend().buffer().clone(), height)
    }

    fn row_text(buf: &ratatui_core::buffer::Buffer, y: u16, width: u16) -> String {
        let mut out = String::new();
        let mut prev_w: u16 = 1;
        for x in 0..width {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            let is_filler = sym.is_empty() || (sym == " " && prev_w == 2) || cell.cell_width() == 0;
            if is_filler {
                continue;
            }
            if let Some(ch) = sym.chars().next() {
                out.push(ch);
            }
            prev_w = cell.cell_width().max(1) as u16;
        }
        out.trim_end().to_string()
    }

    #[test]
    fn heading_renders() {
        let (buf, _) = render_to_buf("# Title", 40);
        assert!(row_text(&buf, 0, 40).contains("Title"));
    }

    #[test]
    fn bold_inline_renders() {
        let (buf, _) = render_to_buf("some **bold** word", 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("bold") && row.contains("some"));
    }

    #[test]
    fn inline_code_renders() {
        let (buf, _) = render_to_buf("use `cargo` now", 40);
        assert!(row_text(&buf, 0, 40).contains("cargo"));
    }

    #[test]
    fn code_block_highlights_and_no_border() {
        let md = "```rs\nfn main() {}\n```\n";
        let (buf, h) = render_to_buf(md, 40);
        let all: String = (0..h)
            .map(|y| row_text(&buf, y, 40))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(all.contains("fn main()"), "code body: {all}");
        // No border glyphs.
        assert!(!all.contains('╭') && !all.contains('╰') && !all.contains('│'));
    }

    #[test]
    fn list_renders_with_marker_no_tree_connector() {
        let md = "- a\n- b\n";
        let (buf, h) = render_to_buf(md, 40);
        let row0 = row_text(&buf, 0, 40);
        assert!(row0.contains('a'));
        assert!(!row0.contains('├') && !row0.contains('└'));
    }

    #[test]
    fn blockquote_italic_no_pipe() {
        let md = "> a quote\n";
        let (buf, _) = render_to_buf(md, 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("a quote"));
        assert!(
            !row.contains('│') && !row.contains('▎'),
            "no pipe prefix: {row}"
        );
    }

    #[test]
    fn link_renders_text_and_url() {
        let md = "[GitHub](https://github.com)";
        let (buf, _) = render_to_buf(md, 60);
        let row = row_text(&buf, 0, 60);
        assert!(row.contains("GitHub") && row.contains("https://github.com"));
    }
}
