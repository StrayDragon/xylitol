//! `MarkdownRenderer` — render a finalized markdown string into `Vec<Line>`
//! for the ratatui buffer (c355).
//!
//! Backed by `pulldown-cmark` (pull parser → `Event` iterator). The renderer is
//! a small stack-based state machine: inline spans (`Strong`/`Emphasis`/`Code`)
//! accumulate into the current line's `Span` vector; block elements
//! (`Heading`/`CodeBlock`/`BlockQuote`/`Item`) flush and start a new line.
//!
//! Both user input and assistant replies render through this same component,
//! differing only in the injected [`MarkdownStyle`] (spec tui60). MVP element
//! coverage (spec tui61): headings, bold/code inline, fenced code blocks
//! (language label + background, NO syntect highlighting), unordered lists,
//! blockquotes, CJK-aware paragraph wrapping (reusing `wrap_to_width`). MVP
//! excludes syntect, tables, and streaming incremental markdown parsing.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui_core::style::{Color, Style};
use ratatui_core::text::{Line, Span};

use crate::app::tui::render::wrap_to_width;
use crate::app::tui::theme::Palette;

/// Style tokens injected by the caller. User input and assistant replies share
/// the renderer but pass different styles (spec tui60).
#[derive(Clone, Copy)]
pub struct MarkdownStyle {
    /// Normal paragraph body.
    pub text: Style,
    /// Heading (`#` … `######`): bold + accent.
    pub heading: Style,
    /// Inline `` `code` ``: inverted-ish tone.
    pub code_inline: Style,
    /// Fenced code block background (applied to every cell of the block).
    pub code_block_bg: Option<Color>,
    /// Code block language label color (the ` ```rs ` tag line).
    pub code_block_lang: Style,
    /// Unordered list item marker (`•`/`-`).
    pub list_marker: Style,
    /// Blockquote body: dimmed.
    pub quote: Style,
    /// Bold span modifier (applied on top of the surrounding text style).
    pub strong: Style,
}

impl MarkdownStyle {
    /// Style for rendering user input (matches `Palette::user_prompt`).
    pub fn for_user(p: &Palette) -> Self {
        Self {
            text: p.user_prompt(),
            heading: p.user_prompt(),
            code_inline: Style::default().fg(Color::Cyan),
            code_block_bg: Some(Color::Black),
            code_block_lang: p.text_dim(),
            list_marker: p.user_prompt(),
            quote: p.text_dim(),
            strong: Style::default().add_modifier(ratatui_core::style::Modifier::BOLD),
        }
    }

    /// Style for rendering assistant replies (matches `Palette::assistant`).
    pub fn for_assistant(p: &Palette) -> Self {
        Self {
            text: p.assistant(),
            heading: Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui_core::style::Modifier::BOLD),
            code_inline: Style::default().fg(Color::Yellow),
            code_block_bg: Some(Color::Black),
            code_block_lang: p.text_dim(),
            list_marker: p.primary(),
            quote: p.text_dim(),
            strong: Style::default().add_modifier(ratatui_core::style::Modifier::BOLD),
        }
    }

    /// Style for rendering finalized thinking/reasoning text (c366). Same
    /// structural rendering as assistant (markdown parsed), but the whole
    /// palette is dimmed so reasoning stays visually subordinate to the reply.
    pub fn for_thinking(p: &Palette) -> Self {
        Self {
            text: p.thinking(),
            // No BOLD on heading — keep thinking visually quiet.
            heading: p.thinking(),
            code_inline: p.text_dim(),
            code_block_bg: Some(Color::Black),
            code_block_lang: p.text_dim(),
            list_marker: p.text_dim(),
            quote: p.text_dim(),
            strong: Style::default().add_modifier(ratatui_core::style::Modifier::ITALIC),
        }
    }
}

/// Render a finalized markdown string into wrapped `Line`s at `width` (CJK-aware
/// via `wrap_to_width`), styled per `style`.
///
/// Streaming text MUST NOT pass through here (spec tui61 streaming-stays-plain);
/// only finalized text committed to scrollback is rendered.
pub fn render_markdown(text: &str, width: u16, style: &MarkdownStyle) -> Vec<Line<'static>> {
    let mut renderer = Renderer::new(width, *style);
    for event in Parser::new(text) {
        renderer.handle_event(event);
    }
    renderer.finish();
    renderer.lines
}

// ── state machine ─────────────────────────────────────────────────────────

/// Stack frame tracking the enclosing block context. Inline emphasis nesting
/// is tracked separately in `Renderer::inline_stack` so block flushes don't
/// disturb span accumulation.
#[derive(Clone, Copy)]
#[allow(dead_code)] // HeadingLevel/Quote kind retained for future per-level styling.
enum Block {
    Paragraph,
    Heading(HeadingLevel),
    /// Fenced/indented code block. The language label is tracked separately in
    /// `Renderer::code_lang_pending` (consumed on the first code-text flush).
    #[allow(clippy::enum_variant_names)] // `CodeBlock` reads clearer than `Code`.
    CodeBlock,
    Quote,
    ListItem,
}

struct Renderer {
    width: u16,
    style: MarkdownStyle,
    /// Completed lines.
    lines: Vec<Line<'static>>,
    /// Spans of the in-progress line (inline accumulation).
    pending: Vec<Span<'static>>,
    /// Block context stack (top = innermost).
    blocks: Vec<Block>,
    /// Inline modifier stack: each entry means "subsequent Text uses this
    /// style merged on top of the block's base text style".
    inline_mods: Vec<Style>,
    /// Whether the current code block has not yet emitted its language-label
    /// line (set on CodeBlock start, cleared after the first Text/flush).
    code_lang_pending: Option<Option<String>>,
    /// Current blockquote nesting depth (0 = not in a quote). Each level adds
    /// a `▎ ` prefix to every rendered row (c366).
    quote_depth: usize,
}

impl Renderer {
    fn new(width: u16, style: MarkdownStyle) -> Self {
        Self {
            width,
            style,
            lines: Vec::new(),
            pending: Vec::new(),
            blocks: Vec::new(),
            inline_mods: Vec::new(),
            code_lang_pending: None,
            quote_depth: 0,
        }
    }

    /// The base text style for the current position: the block context's text
    /// style, with any active inline modifiers merged on top.
    fn current_style(&self) -> Style {
        let base = match self.blocks.last() {
            Some(Block::Heading(_)) => self.style.heading,
            Some(Block::Quote) => self.style.quote,
            Some(Block::CodeBlock) => self.style.text, // code styling per-line in flush
            _ => self.style.text,
        };
        self.inline_mods.iter().fold(base, |acc, m| acc.patch(*m))
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_block_or_inline(tag),
            Event::End(end) => self.end_block_or_inline(end),
            Event::Text(s) => self.push_text(&s, self.current_style()),
            Event::Code(s) => self.push_text(&s, self.style.code_inline),
            Event::SoftBreak | Event::HardBreak => self.flush_inline(),
            Event::Rule => {
                self.flush_inline();
                self.emit_line(Line::styled(
                    "─".repeat(self.width.max(1) as usize),
                    self.style.quote,
                ));
            }
            // MVP ignores HTML/Math/footnotes/task-list markers (spec tui61 scope).
            _ => {}
        }
    }

    fn start_block_or_inline(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.blocks.push(Block::Paragraph);
            }
            Tag::Heading { level, .. } => {
                self.flush_inline();
                self.blocks.push(Block::Heading(level));
            }
            Tag::CodeBlock(kind) => {
                self.flush_inline();
                let lang = match kind {
                    CodeBlockKind::Fenced(s) if !s.is_empty() => Some(s.into_string()),
                    _ => None,
                };
                self.code_lang_pending = Some(lang);
                self.blocks.push(Block::CodeBlock);
            }
            Tag::BlockQuote(_) => {
                self.flush_inline();
                self.quote_depth += 1;
                self.blocks.push(Block::Quote);
            }
            Tag::List(None) => {
                // Unordered list container; individual items come as Tag::Item.
                // (Ordered lists fall through to MVP-ignore — only List(None) is in scope.)
            }
            Tag::Item => {
                self.flush_inline();
                self.blocks.push(Block::ListItem);
                // Emit the bullet marker as the first span of the item line.
                self.pending
                    .push(Span::styled("• ".to_string(), self.style.list_marker));
            }
            Tag::Strong => self.inline_mods.push(self.style.strong),
            Tag::Emphasis => {
                self.inline_mods
                    .push(Style::default().add_modifier(ratatui_core::style::Modifier::ITALIC));
            }
            // MVP scope excludes tables, definition lists, footnotes, links-as-blocks.
            _ => {}
        }
    }

    fn end_block_or_inline(&mut self, end: TagEnd) {
        match end {
            TagEnd::Paragraph | TagEnd::Item => {
                self.flush_inline();
                self.blocks.pop();
            }
            TagEnd::Heading(_) => {
                self.flush_inline();
                self.blocks.pop();
            }
            TagEnd::CodeBlock => {
                self.flush_inline();
                self.blocks.pop();
                self.code_lang_pending = None;
            }
            TagEnd::BlockQuote(_) => {
                self.flush_inline();
                self.blocks.pop();
                self.quote_depth = self.quote_depth.saturating_sub(1);
            }
            TagEnd::List(_) => { /* container close — no per-line action */ }
            TagEnd::Strong | TagEnd::Emphasis => {
                self.inline_mods.pop();
            }
            _ => {}
        }
    }

    /// Push a completed line into `self.lines`, prefixing it with the
    /// blockquote bar (`▎ ` × depth) when inside a quote (c366). The prefix
    /// goes on every physical row so wrapped continuation rows carry it too.
    fn emit_line(&mut self, mut line: Line<'static>) {
        if self.quote_depth > 0 {
            let prefix = "▎ ".repeat(self.quote_depth);
            let mut spans = vec![Span::styled(prefix, self.style.quote)];
            spans.append(&mut line.spans);
            line.spans = spans;
        }
        self.lines.push(line);
    }

    /// Append text with a style. If inside a code block, wrap each line of the
    /// text to `width` and stamp the code-block background. Otherwise, if the
    /// accumulated pending spans exceed `width`, wrap by flushing the current
    /// word boundary (simple greedy wrap on the pending string).
    fn push_text(&mut self, s: &str, style: Style) {
        if matches!(self.blocks.last(), Some(Block::CodeBlock)) {
            self.push_code_text(s, style);
            return;
        }
        // Inline emphasis/code: merge with current block base via provided style.
        // `style` already reflects the caller's choice (code_inline or current_style).
        self.pending.push(Span::styled(s.to_string(), style));
    }

    /// Code-block text: wrap to width minus an indent, prefix the language
    /// label on the first line, stamp the code-block background.
    fn push_code_text(&mut self, s: &str, _style: Style) {
        // If we still owe a language label line, emit it first.
        if let Some(Some(lang)) = self.code_lang_pending.take() {
            let label = format!("  {lang}");
            let mut label_line = Line::from(vec![Span::styled(label, self.style.code_block_lang)]);
            if let Some(bg) = self.style.code_block_bg {
                label_line = label_line.style(Style::default().bg(bg));
            }
            self.emit_line(label_line);
        }
        // Emit each line of the code text with indent + background.
        let indent = "  ";
        for raw_line in s.lines() {
            let rows = wrap_to_width(raw_line, self.width.saturating_sub(indent.len() as u16));
            for row in rows {
                let mut line = Line::from(vec![Span::styled(
                    format!("{indent}{row}"),
                    self.style.text,
                )]);
                if let Some(bg) = self.style.code_block_bg {
                    line = line.style(Style::default().bg(bg));
                }
                self.emit_line(line);
            }
        }
    }

    /// Flush the accumulated pending spans into one (or more, if wrapping)
    /// completed lines, applying the block-appropriate style + background.
    fn flush_inline(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let spans = std::mem::take(&mut self.pending);
        // Reconstruct the line from its spans, then wrap by display width using
        // the concatenated string. Spans crossing the wrap boundary are split
        // naively (the wrap helper already handles CJK width); for MVP the
        // whole pending line is treated as one styled chunk when wrapping, and
        // multi-span styling is preserved only on the first physical row.
        let full: String = spans.iter().map(|s| s.content.to_string()).collect();
        let rows = wrap_to_width(&full, self.width);
        let mut made_lines: Vec<Line<'static>> = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            if i == 0 {
                // Preserve span structure on the first physical row.
                made_lines.push(Line::from(spans.clone()));
            } else {
                // Continuation rows: single span with the base text style.
                let style = self.current_style();
                made_lines.push(Line::from(vec![Span::styled(row.clone(), style)]));
            }
        }
        // Apply block-level background (code blocks carry bg on every row).
        if matches!(self.blocks.last(), Some(Block::CodeBlock))
            && let Some(bg) = self.style.code_block_bg
        {
            made_lines = made_lines
                .into_iter()
                .map(|l| l.style(Style::default().bg(bg)))
                .collect();
        }
        for line in made_lines {
            self.emit_line(line);
        }
    }

    fn finish(&mut self) {
        self.flush_inline();
        if self.lines.is_empty() {
            self.lines.push(Line::raw(""));
        }
    }
}

#[cfg(test)]
mod tests {
    //! TestBackend-based behavior assertions for each markdown element (spec tui41).
    //! Each element is rendered via `render_markdown` then drawn to a buffer and
    //! the row text is asserted, so the tests are implementation-agnostic.

    use super::*;
    use crate::app::tui::theme::palette;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::buffer::CellWidth;
    use ratatui_core::layout::Rect;
    use ratatui_core::terminal::Terminal;
    use ratatui_core::widgets::Widget;

    fn render_to_buf(text: &str, width: u16) -> (ratatui_core::buffer::Buffer, u16) {
        let style = MarkdownStyle::for_assistant(&palette());
        let lines = render_markdown(text, width, &style);
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
        let mut prev_width: u16 = 1;
        for x in 0..width {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            let is_filler =
                sym.is_empty() || (sym == " " && prev_width == 2) || cell.cell_width() == 0;
            if is_filler {
                continue;
            }
            if let Some(ch) = sym.chars().next() {
                out.push(ch);
            }
            prev_width = cell.cell_width().max(1) as u16;
        }
        out.trim_end().to_string()
    }

    #[test]
    fn heading_renders() {
        let (buf, _) = render_to_buf("# Title", 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("Title"), "heading text visible: {row}");
    }

    #[test]
    fn bold_inline_renders() {
        let (buf, _) = render_to_buf("some **bold** word", 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("bold"), "bold text visible: {row}");
        assert!(row.contains("some"), "surrounding text visible: {row}");
    }

    #[test]
    fn inline_code_renders() {
        let (buf, _) = render_to_buf("use `cargo` now", 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("cargo"), "inline code visible: {row}");
    }

    #[test]
    fn fenced_code_block_has_language_label() {
        let md = "```rs\nfn main() {}\n```\n";
        let (buf, _) = render_to_buf(md, 40);
        // First row is the language label "  rs".
        let label_row = row_text(&buf, 0, 40);
        assert!(
            label_row.contains("rs"),
            "language label present on first row: {label_row}"
        );
        // Second row contains the code body.
        let body_row = row_text(&buf, 1, 40);
        assert!(
            body_row.contains("fn main()"),
            "code body visible on second row: {body_row}"
        );
    }

    #[test]
    fn code_block_background_applied() {
        let md = "```rs\nfn main() {}\n```\n";
        let (buf, _) = render_to_buf(md, 40);
        // The code body cell carries a non-Reset background.
        let bg = buf[(0, 1)].bg;
        assert_ne!(
            bg,
            ratatui_core::style::Color::Reset,
            "code block row has a distinct background (got {bg:?})"
        );
    }

    #[test]
    fn unordered_list_renders_marker() {
        let md = "- first\n- second\n";
        let (buf, h) = render_to_buf(md, 40);
        assert!(h >= 2, "two list items -> at least 2 rows, got {h}");
        let row0 = row_text(&buf, 0, 40);
        assert!(row0.contains("first"), "first item text visible: {row0}");
        // A list marker glyph is present somewhere on the row.
        assert!(
            row0.contains('•') || row0.contains('-'),
            "list marker visible: {row0}"
        );
    }

    #[test]
    fn blockquote_renders_with_prefix() {
        // c366: blockquote carries a `▎` prefix on every row (spec tui66).
        let md = "> a quote\n";
        let (buf, _) = render_to_buf(md, 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("a quote"), "quote body visible: {row}");
        assert!(row.contains('▎'), "blockquote prefix visible: {row}");
    }

    #[test]
    fn nested_blockquote_indents_prefix() {
        // c366: nested blockquote carries an extra prefix (spec tui66).
        let md = ">> nested\n";
        let (buf, _) = render_to_buf(md, 40);
        let row = row_text(&buf, 0, 40);
        // Two prefix glyphs for depth-2 quote.
        let prefix_count = row.chars().filter(|&c| c == '▎').count();
        assert!(
            prefix_count >= 2,
            "nested blockquote has >=2 prefix glyphs, got {prefix_count}: {row}"
        );
    }

    #[test]
    fn thinking_text_renders_markdown() {
        // c366: ThinkingText renders through markdown (spec tui65). A code
        // block in thinking content renders with structure, not flat text.
        let p = palette();
        let style = MarkdownStyle::for_thinking(&p);
        let md = "```\nfn think() {}\n```\n";
        let lines = render_markdown(md, 40, &style);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(
            text.contains("fn think()"),
            "thinking code block body parsed (not flat): {text}"
        );
    }

    #[test]
    fn paragraph_wraps_cjk_at_display_width() {
        let md = "你好世界再见你好世界再见";
        let (buf, _) = render_to_buf(md, 4);
        // 12 CJK chars at width 4 -> 6 rows of 2 chars.
        assert_eq!(row_text(&buf, 0, 4), "你好");
        assert_eq!(row_text(&buf, 1, 4), "世界");
    }

    #[test]
    fn user_vs_assistant_share_renderer() {
        // Both styles render the same text; structural output identical (the
        // difference is only styling, asserted at the type level by using the
        // same render_markdown entry point).
        let p = palette();
        let user_lines = render_markdown("# t", 40, &MarkdownStyle::for_user(&p));
        let asst_lines = render_markdown("# t", 40, &MarkdownStyle::for_assistant(&p));
        let user_text: String = user_lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        let asst_text: String = asst_lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert_eq!(
            user_text, asst_text,
            "user/assistant share structural rendering (only styles differ)"
        );
    }
}
