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
    /// H1 (bold + underlined).
    pub h1: Style,
    /// H2 (bold).
    pub h2: Style,
    /// H3 (bold + italic).
    pub h3: Style,
    /// H4 (italic).
    pub h4: Style,
    /// H5 (italic).
    pub h5: Style,
    /// H6 (italic).
    pub h6: Style,
    /// Inline `code` foreground.
    pub code_fg: Color,
    /// Blockquote text (italic + dimmed).
    pub quote: Style,
}

impl RenderStyle {
    /// Heading style for `level` (1-based). Matches codex MarkdownStyles grading
    /// (c396): H1 bold+underlined, H2 bold, H3 bold+italic, H4-H6 italic.
    fn heading(&self, level: HeadingLevel) -> Style {
        match level {
            HeadingLevel::H1 => self.h1,
            HeadingLevel::H2 => self.h2,
            HeadingLevel::H3 => self.h3,
            HeadingLevel::H4 => self.h4,
            HeadingLevel::H5 => self.h5,
            HeadingLevel::H6 => self.h6,
        }
    }

    /// Style for assistant replies (default theme).
    pub fn for_assistant() -> Self {
        Self {
            text: Style::default(),
            h1: Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
            h2: Style::default().add_modifier(Modifier::BOLD),
            h3: Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::ITALIC),
            h4: Style::default().add_modifier(Modifier::ITALIC),
            h5: Style::default().add_modifier(Modifier::ITALIC),
            h6: Style::default().add_modifier(Modifier::ITALIC),
            code_fg: Color::Cyan,
            quote: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        }
    }

    /// Style for user input echo (cyan/bold).
    pub fn for_user() -> Self {
        let base = Style::default().fg(Color::Cyan);
        Self {
            text: base,
            h1: base
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
            h2: base.add_modifier(Modifier::BOLD),
            h3: base
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::ITALIC),
            h4: base.add_modifier(Modifier::ITALIC),
            h5: base.add_modifier(Modifier::ITALIC),
            h6: base.add_modifier(Modifier::ITALIC),
            code_fg: Color::Cyan,
            quote: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        }
    }

    /// Style for thinking/reasoning (dimmed).
    pub fn for_thinking() -> Self {
        let dim = Color::DarkGray;
        let base = Style::default().fg(dim);
        Self {
            text: base,
            h1: base
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
            h2: base.add_modifier(Modifier::BOLD),
            h3: base
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::ITALIC),
            h4: base.add_modifier(Modifier::ITALIC),
            h5: base.add_modifier(Modifier::ITALIC),
            h6: base.add_modifier(Modifier::ITALIC),
            code_fg: Color::DarkGray,
            quote: base.add_modifier(Modifier::ITALIC),
        }
    }
}

/// Render finalized markdown text into styled `Line`s (c395).
///
/// `width` is used for code-block line wrapping. Streaming text MUST NOT pass
/// here (only finalized, committed text).
pub fn render_markdown(text: &str, width: u16, style: RenderStyle) -> Vec<Line<'static>> {
    use pulldown_cmark::Options;
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut writer = Writer::new(width as usize, style);
    for event in Parser::new_ext(text, opts) {
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
    /// Current blockquote nesting depth (c396). Each level adds a "> " prefix
    /// to every physical row of the blockquote, so `> > nested` carries a
    /// doubled prefix. Reset to 0 outside any blockquote.
    blockquote_depth: usize,
    /// Whether the current pending line still needs its blockquote prefix
    /// emitted (c396). Set by flush_line and BlockQuote start; cleared by
    /// ensure_blockquote_prefix once emitted. Avoids doubled prefixes.
    prefix_pending: bool,
    /// Ordered-list counter (c396). `Some(n)` = current ordered list emits
    /// `{n}. ` markers and increments; `None` = unordered list emits `• `.
    /// Reset at `TagEnd::List`.
    list_counter: Option<u64>,
    /// Code block accumulator.
    code_lang: Option<String>,
    code_buf: String,
    /// Pending link URL (appended as " (url)" at End(Link)).
    pending_link_url: Option<String>,
    /// Display width of the current pending line (for paragraph wrapping).
    pending_width: usize,
    /// Table cell accumulator: cells of the current row (plain text).
    table_row_cells: Vec<String>,
    /// All rows of the current table (header first). Each row is a Vec of
    /// cell strings; rendered as aligned columns at End(Table).
    table_rows: Vec<Vec<String>>,
}

#[derive(Clone, Copy)]
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
            blockquote_depth: 0,
            prefix_pending: false,
            list_counter: None,
            code_lang: None,
            code_buf: String::new(),
            pending_link_url: None,
            pending_width: 0,
            table_row_cells: Vec::new(),
            table_rows: Vec::new(),
        }
    }

    /// The current text style: base (paragraph/heading/quote) + inline mods.
    fn current_style(&self) -> Style {
        let base = match self.blocks.last() {
            Some(Block::Heading(level)) => self.style.heading(*level),
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
            // c396: emit the blockquote prefix at the start of each physical
            // line (flush sets prefix_pending; wrap and newline both flush).
            if self.pending.is_empty() {
                self.ensure_blockquote_prefix();
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
        // c396: the blockquote prefix is emitted lazily by
        // `ensure_blockquote_prefix()` on the next text push, so we do NOT
        // emit it here (emitting here caused doubled prefixes on nested
        // blockquotes because BlockQuote start also triggers a flush).
        self.prefix_pending = self.blockquote_depth > 0;
    }

    /// Emit the blockquote "> " prefix for the current line if one is pending
    /// (set by flush_line / BlockQuote start). Called lazily before the first
    /// text/span lands on a fresh line, so the prefix appears exactly once per
    /// physical row (including wrapped continuation rows — wrap calls flush,
    /// which sets prefix_pending again).
    fn ensure_blockquote_prefix(&mut self) {
        if self.prefix_pending {
            let prefix = "> ".repeat(self.blockquote_depth);
            self.push_span(Span::styled(prefix, self.style.quote));
            self.prefix_pending = false;
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
                // codex-style leading hash prefix (c396): "# " for H1, "## " for
                // H2, etc. — keeps the document outline visible at a glance.
                let prefix = format!("{} ", "#".repeat(level as usize));
                let prefix_style = self.style.heading(level);
                self.push_span(Span::styled(prefix, prefix_style));
                self.blocks.push(Block::Heading(level));
            }
            Tag::CodeBlock(kind) => {
                self.flush_line();
                // Unconditional blank-line separation before the code block:
                // streaming commit splits at paragraph boundaries (c377
                // fence-aware), so each paragraph is render_markdown'd
                // independently and the renderer can't rely on prior lines
                // existing. Always pad so the block stands apart.
                // TODO(c397): once render granularity decouples to full-message,
                // gate this on `!self.lines.is_empty()` like codex's needs_newline.
                self.lines.push(Line::raw(""));
                let lang = match kind {
                    CodeBlockKind::Fenced(s) if !s.is_empty() => Some(s.into_string()),
                    _ => None,
                };
                self.code_lang = lang;
                self.code_buf.clear();
                self.blocks.push(Block::CodeBlock);
            }
            Tag::BlockQuote(_) => {
                self.flush_line(); // ends the prior line
                self.blockquote_depth += 1;
                self.blocks.push(Block::BlockQuote);
                // The blockquote's first line needs its prefix emitted when
                // the first text lands (lazy via ensure_blockquote_prefix).
                self.prefix_pending = true;
            }
            Tag::List(start) => {
                // c396: track ordered-list start. `Some(n)` → ordered, emit
                // `{n}. ` markers and increment per item; `None` → unordered.
                self.list_counter = start;
            }
            Tag::Item => {
                self.flush_line();
                self.blocks.push(Block::ListItem);
                if let Some(n) = self.list_counter {
                    // Ordered: "1. " / "2. " ... in light_blue (codex style).
                    let marker = format!("{}. ", n);
                    self.push_span(Span::styled(marker, Style::default().fg(Color::LightBlue)));
                    self.list_counter = Some(n + 1);
                } else {
                    self.push_span(Span::styled("• ".to_string(), self.style.text));
                }
            }
            Tag::Table(_) => {
                self.flush_line();
                self.table_rows.clear();
            }
            Tag::TableHead | Tag::TableRow => {
                self.table_row_cells.clear();
            }
            Tag::TableCell => {
                // Cell text accumulates into pending; we capture it at End(TableCell).
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
                // Decrement depth BEFORE flushing so the post-flush prefix
                // (for the line AFTER the blockquote) reflects the outer scope.
                self.blockquote_depth = self.blockquote_depth.saturating_sub(1);
                self.flush_line();
                self.blocks.pop();
            }
            TagEnd::List(_) => {
                // Reset the ordered-list counter when the list ends.
                self.list_counter = None;
            }
            TagEnd::Table => {
                self.flush_table();
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                // Collect the row's cells (pending held cell text sequentially).
                let row = std::mem::take(&mut self.table_row_cells);
                self.table_rows.push(row);
            }
            TagEnd::TableCell => {
                // Capture cell text: join pending spans.
                let cell: String = self.pending.iter().map(|s| s.content.to_string()).collect();
                self.pending.clear();
                self.pending_width = 0;
                self.table_row_cells.push(cell.trim().to_string());
            }
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
        // Trailing blank line for visual separation from following content.
        self.lines.push(Line::raw(""));
        self.code_buf.clear();
    }

    /// Render accumulated table rows as aligned columns (no border lines).
    /// Each column is padded to its max cell width; header row is bold.
    fn flush_table(&mut self) {
        let rows = std::mem::take(&mut self.table_rows);
        if rows.is_empty() {
            return;
        }
        // Compute column widths (max cell display width per column).
        let n_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if n_cols == 0 {
            return;
        }
        let col_widths: Vec<usize> = (0..n_cols)
            .map(|c| {
                rows.iter()
                    .filter_map(|r| {
                        r.get(c)
                            .map(|cell| unicode_width::UnicodeWidthStr::width(cell.as_str()))
                    })
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        for (row_idx, row) in rows.iter().enumerate() {
            let is_header = row_idx == 0;
            let style = if is_header {
                self.style.h2
            } else {
                self.style.text
            };
            let mut spans: Vec<Span<'static>> = Vec::new();
            for (c, cell) in row.iter().enumerate() {
                if c > 0 {
                    spans.push(Span::styled("  ".to_string(), style));
                }
                let cw = col_widths.get(c).copied().unwrap_or(0);
                let cell_w = unicode_width::UnicodeWidthStr::width(cell.as_str());
                spans.push(Span::styled(cell.clone(), style));
                if cell_w < cw {
                    spans.push(Span::styled(" ".repeat(cw - cell_w), style));
                }
            }
            self.lines.push(Line::from(spans));
        }
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

    /// Read the ratatui Style of the first non-empty cell on row `y`.
    fn row_style(
        buf: &ratatui_core::buffer::Buffer,
        y: u16,
        width: u16,
    ) -> ratatui_core::style::Style {
        for x in 0..width {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            if !sym.is_empty() && sym != " " {
                return cell.style();
            }
        }
        ratatui_core::style::Style::default()
    }

    #[test]
    fn heading_has_hash_prefix() {
        // c396: headings carry a leading "# " / "## " prefix matching level.
        let (buf, _) = render_to_buf("## Subsection", 40);
        assert!(
            row_text(&buf, 0, 40).starts_with("## "),
            "H2 has ## prefix: {:?}",
            row_text(&buf, 0, 40)
        );
    }

    #[test]
    fn heading_h1_is_bold_and_underlined() {
        // c396/tui78: H1 MUST be bold + underlined.
        let (buf, _) = render_to_buf("# Title", 40);
        let style = row_style(&buf, 0, 40);
        assert!(
            style.add_modifier.contains(Modifier::BOLD),
            "H1 has BOLD: {:?}",
            style.add_modifier
        );
        assert!(
            style.add_modifier.contains(Modifier::UNDERLINED),
            "H1 has UNDERLINED: {:?}",
            style.add_modifier
        );
    }

    #[test]
    fn heading_h3_is_bold_and_italic() {
        // c396/tui78: H3 MUST be bold + italic.
        let (buf, _) = render_to_buf("### Sub", 40);
        let style = row_style(&buf, 0, 40);
        assert!(
            style.add_modifier.contains(Modifier::BOLD),
            "H3 has BOLD: {:?}",
            style.add_modifier
        );
        assert!(
            style.add_modifier.contains(Modifier::ITALIC),
            "H3 has ITALIC: {:?}",
            style.add_modifier
        );
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
    fn ordered_list_renders_incrementing_numbers() {
        // c396/tui79: ordered list items carry auto-incrementing "1. "/"2. " markers.
        let md = "1. first\n2. second\n3. third\n";
        let (buf, h) = render_to_buf(md, 40);
        assert!(h >= 3, "three items -> at least 3 rows, got {h}");
        assert!(
            row_text(&buf, 0, 40).starts_with("1. "),
            "row0 has 1. marker: {:?}",
            row_text(&buf, 0, 40)
        );
        assert!(
            row_text(&buf, 1, 40).starts_with("2. "),
            "row1 has 2. marker: {:?}",
            row_text(&buf, 1, 40)
        );
        assert!(
            row_text(&buf, 2, 40).starts_with("3. "),
            "row2 has 3. marker: {:?}",
            row_text(&buf, 2, 40)
        );
    }

    #[test]
    fn ordered_list_custom_start() {
        // c396/tui79: ordered list with explicit start value.
        let md = "3. alpha\n4. beta\n";
        let (buf, _h) = render_to_buf(md, 40);
        assert!(
            row_text(&buf, 0, 40).starts_with("3. "),
            "row0 has 3. marker: {:?}",
            row_text(&buf, 0, 40)
        );
        assert!(
            row_text(&buf, 1, 40).starts_with("4. "),
            "row1 has 4. marker: {:?}",
            row_text(&buf, 1, 40)
        );
    }

    #[test]
    fn blockquote_has_gt_prefix() {
        // c396/tui66: blockquote lines MUST carry a visible "> " prefix.
        let md = "> a quote\n";
        let (buf, _) = render_to_buf(md, 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("a quote"), "quote body: {row}");
        assert!(row.starts_with("> "), "blockquote has > prefix: {row}");
    }

    #[test]
    fn nested_blockquote_has_doubled_prefix() {
        // c396/tui66: nested blockquote (>>) carries a doubled "> > " prefix.
        let md = ">> nested\n";
        let (buf, _) = render_to_buf(md, 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("nested"), "nested body: {row}");
        assert!(
            row.starts_with("> > "),
            "nested blockquote has > > prefix: {row}"
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
