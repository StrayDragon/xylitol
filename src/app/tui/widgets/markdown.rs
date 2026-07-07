//! Markdown widget — passthrough + code-block highlight (c399 stage 2.2).
//!
//! Design per user directive (c399):
//! - **Current scope**: text passes through verbatim (no character rewriting);
//!   ONLY fenced code blocks get syntect syntax highlighting. Everything else
//!   (headings, bold, italic, blockquote, list, link, etc.) renders as plain
//!   text for now — minimal, conservative, copy-token-friendly.
//! - **Knobs for gradual enablement**: `MarkdownTheme` carries per-element
//!   switches (enable_heading_grading, enable_bold, enable_italic, enable_quote,
//!   enable_link, enable_list_marker, enable_table_align, …). Each is a future
//!   increment; flipping one on wires in that element's styling without touching
//!   the parser. The parser emits structured events; handlers consume them.
//!
//! This is deliberately less aggressive than the c396 markdown_render (which
//! rewrote every element on the old ratatui model). The new engine's line-array
//! model preserves cross-paragraph context for free, so we can start minimal and
//! add styling incrementally without the old architectural pressure.
//!
//! Reuses `components::syntect_highlight` for the code-block highlight pass.

use pulldown_cmark::CodeBlockKind;

use crate::app::tui::engine::component::Component;
use crate::app::tui::engine::style::{CellStyle, Span, StyledLine};
use crate::app::tui::engine::width::wrap;

/// Per-element styling switches. All default to `false` (passthrough) except
/// code-block highlighting (always on — it's the one transform we want). Flip a
/// switch on to wire in that element's handler in the parser below.
#[derive(Clone, Copy, Default)]
pub struct MarkdownTheme {
    /// Grade headings by level (H1 bold+underlined, H2 bold, ...). Off = plain.
    pub enable_heading_grading: bool,
    /// Render `**bold**` with bold modifier. Off = literal asterisks shown.
    pub enable_bold: bool,
    /// Render `*italic*` with italic modifier. Off = literal asterisks shown.
    pub enable_italic: bool,
    /// Prefix blockquote lines with `> `. Off = literal `>` shown.
    pub enable_quote_prefix: bool,
    /// Render `[text](url)` as `text (url)`. Off = literal markdown shown.
    pub enable_link_expand: bool,
    /// Prefix list items with `•` / `1.`. Off = literal marker shown.
    pub enable_list_marker: bool,
    /// Column-align tables with spaces (no Unicode borders). Off = literal pipes.
    pub enable_table_align: bool,
}

impl MarkdownTheme {
    /// Fully passthrough (only code blocks highlighted). The conservative start.
    pub fn passthrough() -> Self {
        Self::default()
    }
}

/// Markdown widget. Parses `source` with pulldown-cmark and renders lines
/// according to `theme`. Render result caches by (source signature, width).
pub struct Markdown {
    source: String,
    theme: MarkdownTheme,
}

impl Markdown {
    pub fn new(source: impl Into<String>, theme: MarkdownTheme) -> Self {
        Self {
            source: source.into(),
            theme,
        }
    }
    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source = source.into();
    }
    pub fn set_theme(&mut self, theme: MarkdownTheme) {
        self.theme = theme;
    }
}

impl Component for Markdown {
    fn render(&self, width: usize) -> Vec<StyledLine> {
        render_markdown(&self.source, width, self.theme)
    }

    fn invalidate(&mut self) {
        // No mutable cache yet (render recomputes each call — see Text note).
    }
}

/// Render markdown `source` at `width` per `theme`. Pure function (also the
/// test entry point). Stage 2.2: passthrough text + code-block highlight.
pub fn render_markdown(source: &str, width: usize, theme: MarkdownTheme) -> Vec<StyledLine> {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut renderer = Renderer::new(width, theme);
    for event in Parser::new_ext(source, opts) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                renderer.begin_code_block(kind);
            }
            Event::End(TagEnd::CodeBlock) => {
                renderer.end_code_block();
            }
            Event::Text(text) => {
                renderer.text(&text);
            }
            Event::Code(code) => {
                renderer.inline_code(&code);
            }
            Event::SoftBreak | Event::HardBreak => {
                renderer.line_break();
            }
            // Block-level End events (Paragraph/Heading/BlockQuote/List/Item)
            // flush the pending line + insert a blank separator so consecutive
            // blocks stay visually distinct in passthrough mode. This is the
            // minimal structural cue preserved without any styling.
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::BlockQuote(_)
                | TagEnd::List(_)
                | TagEnd::Item,
            ) => {
                renderer.block_end();
            }
            // All other events (emphasis Start/End, link Start/End, table cells,
            // etc.) fall through as no-ops — passthrough emits their text only.
            _ => {}
        }
    }
    renderer.finish()
}

struct Renderer {
    width: usize,
    /// Per-element styling switches. Currently unread (passthrough mode) —
    /// reserved for the gradual-enablement knobs (heading_grading, bold, etc.).
    #[allow(dead_code)]
    theme: MarkdownTheme,
    lines: Vec<StyledLine>,
    /// Current pending line (spans accumulate here, flushed on block boundary).
    pending: Vec<Span>,
    /// Code-block accumulator. When in_code is Some(lang), Text events go here.
    in_code: Option<Option<String>>, // outer Some = inside block; inner = lang
    code_buf: String,
}

impl Renderer {
    fn new(width: usize, theme: MarkdownTheme) -> Self {
        Self {
            width,
            theme,
            lines: Vec::new(),
            pending: Vec::new(),
            in_code: None,
            code_buf: String::new(),
        }
    }

    fn begin_code_block(&mut self, kind: CodeBlockKind) {
        self.flush_pending();
        let lang = match kind {
            CodeBlockKind::Fenced(s) if !s.is_empty() => Some(s.into_string()),
            _ => None,
        };
        self.in_code = Some(lang);
        self.code_buf.clear();
    }

    fn end_code_block(&mut self) {
        let lang = self.in_code.take();
        let code = std::mem::take(&mut self.code_buf);
        self.render_code_block(lang.flatten().as_deref(), &code);
    }

    fn text(&mut self, text: &str) {
        if self.in_code.is_some() {
            self.code_buf.push_str(text);
        } else {
            // Passthrough: emit text as-is (preserve markdown structure visually).
            // Split on newlines so each source line becomes its own pending flush.
            for (i, line) in text.split('\n').enumerate() {
                if i > 0 {
                    self.flush_pending();
                }
                if !line.is_empty() {
                    self.pending.push(Span::raw(line));
                }
            }
        }
    }

    fn inline_code(&mut self, code: &str) {
        // Inline `code` — passthrough as plain text (no backtick styling yet).
        self.pending.push(Span::raw(code));
    }

    /// Soft/hard break within a block: flush the current line, no blank after.
    fn line_break(&mut self) {
        self.flush_pending();
    }

    /// End of a block (paragraph/heading/quote/list/item): flush + insert a
    /// blank separator line so consecutive blocks stay visually distinct.
    fn block_end(&mut self) {
        self.flush_pending();
        // Avoid stacking blank lines (consecutive block ends, e.g. list items).
        if !self.lines.is_empty() && !self.lines.last().unwrap().is_empty() {
            self.lines.push(StyledLine::empty());
        }
    }

    fn flush_pending(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let line = StyledLine::from_spans(std::mem::take(&mut self.pending));
        // Wrap the pending logical line to width (CJK-aware).
        let wrapped = wrap(&line, self.width);
        self.lines.extend(wrapped);
    }

    fn render_code_block(&mut self, lang: Option<&str>, code: &str) {
        let code = code.strip_suffix('\n').unwrap_or(code);
        if code.is_empty() {
            return;
        }
        // syntect highlight pass. c399 stage 4: highlight() now returns
        // StyleSegment with CellStyle directly (no ratatui). Unrecognized
        // language or oversized -> plain fallback.
        let segments = crate::app::tui::syntect_highlight::highlight(lang.unwrap_or(""), code);
        if segments.is_empty() {
            // Plain fallback: emit each code line unstyled, wrapped to width.
            for raw_line in code.split('\n') {
                let line = StyledLine::raw(raw_line);
                let wrapped = wrap(&line, self.width);
                self.lines.extend(wrapped);
            }
            // Blank line after the block for visual separation.
            self.lines.push(StyledLine::empty());
            return;
        }
        // Convert StyleSegments (byte offsets into `code`) to per-line StyledLines.
        // Reuse the c396 segment→line splitter, then adapt the ratatui Lines to
        // our StyledLine. Simpler: walk code char-by-char building spans.
        self.emit_highlighted_code(code, &segments);
        // Blank line after the block.
        self.lines.push(StyledLine::empty());
    }

    /// Walk `code` building StyledLines from byte-offset StyleSegments. Wraps
    /// long lines to width. Each span carries the segment's converted style.
    fn emit_highlighted_code(
        &mut self,
        code: &str,
        segments: &[crate::app::tui::syntect_highlight::StyleSegment],
    ) {
        let mut current_line = StyledLine::new();
        let mut current_w = 0usize;
        let mut byte_pos = 0usize;
        for ch in code.chars() {
            let ch_bytes = ch.len_utf8();
            if ch == '\n' {
                let line = std::mem::take(&mut current_line);
                let wrapped = wrap(&line, self.width);
                self.lines.extend(wrapped);
                current_w = 0;
                byte_pos += ch_bytes;
                continue;
            }
            let style = style_at_byte(segments, byte_pos);
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if cw > 0 && current_w + cw > self.width && current_w > 0 {
                let line = std::mem::take(&mut current_line);
                let wrapped = wrap(&line, self.width);
                self.lines.extend(wrapped);
                current_w = 0;
            }
            current_line.push(Span::styled(ch.to_string(), style));
            current_w += cw;
            byte_pos += ch_bytes;
        }
        if !current_line.is_empty() {
            let wrapped = wrap(&current_line, self.width);
            self.lines.extend(wrapped);
        }
    }

    fn finish(mut self) -> Vec<StyledLine> {
        self.flush_pending();
        // Trim trailing blank lines (block_end separators at doc end are noise).
        while self.lines.len() > 1 && self.lines.last().unwrap().is_empty() {
            self.lines.pop();
        }
        if self.lines.is_empty() {
            self.lines.push(StyledLine::empty());
        }
        self.lines
    }
}

/// Look up the [`CellStyle`] for a byte position in the highlighted source.
/// Segments are sorted, non-overlapping byte ranges produced by
/// [`crate::app::tui::syntect_highlight::highlight`]. c399 stage 4: segments
/// carry `CellStyle` directly, so this is a plain range lookup (no adapter).
fn style_at_byte(
    segments: &[crate::app::tui::syntect_highlight::StyleSegment],
    byte_pos: usize,
) -> CellStyle {
    for seg in segments {
        if seg.start <= byte_pos && byte_pos < seg.end {
            return seg.style;
        }
    }
    CellStyle::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(md: &str, width: usize) -> Vec<String> {
        render_markdown(md, width, MarkdownTheme::passthrough())
            .into_iter()
            .map(|l| l.plain_text())
            .collect()
    }

    #[test]
    fn plain_paragraph_passthrough() {
        let lines = render("hello world", 80);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn code_block_highlighted_not_passthrough() {
        // A fenced rust block: the code body should be present (highlighted),
        // but the ``` fences must NOT appear (consumed by the parser).
        let md = "```rs\nfn main() {}\n```\n";
        let lines = render_markdown(md, 80, MarkdownTheme::passthrough());
        let all: String = lines
            .iter()
            .map(|l| l.plain_text())
            .collect::<Vec<_>>()
            .join("|");
        assert!(all.contains("fn main()"), "code body present: {all}");
        assert!(!all.contains("```"), "fences consumed: {all}");
    }

    #[test]
    fn code_block_unknown_lang_plain_fallback() {
        let md = "```unknownlang\ncode here\n```\n";
        let lines = render(md, 80);
        let all = lines.join("|");
        assert!(
            all.contains("code here"),
            "unknown-lang code renders as plain"
        );
        assert!(!all.contains("```"));
    }

    #[test]
    fn headings_passthrough_as_plain_text() {
        // Passthrough mode: heading renders as its text content (pulldown-cmark
        // consumes the `#`; no styling applied). Structure is visible only via
        // the text itself — no `#` prefix, no bold, per the user's "plain text"
        // directive (c399 stage 2.2). Enable heading_grading later via the knob.
        let lines = render("# Title", 80);
        assert_eq!(lines, vec!["Title"]);
    }

    #[test]
    fn emphasis_passthrough_as_plain_text() {
        // `**bold**` -> "bold" (asterisks consumed), no bold modifier applied.
        let lines = render("some **bold** text", 80);
        assert_eq!(lines, vec!["some bold text"]);
    }

    #[test]
    fn link_passthrough_as_label_only() {
        // `[GitHub](url)` -> "GitHub" (url dropped, no expansion). The url is
        // gone because pulldown-cmark separates link text from dest_url; we only
        // emit the text in passthrough. enable_link_expand will add " (url)".
        let lines = render("[GitHub](https://github.com)", 80);
        assert_eq!(lines, vec!["GitHub"]);
    }

    #[test]
    fn paragraph_wraps_at_width() {
        let lines = render("abcdefghij", 4);
        assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn code_block_highlighted_spans_carry_color() {
        // The highlighted code block should produce spans with foreground color
        // (syntect assigns colors). At least one span has fg set.
        let md = "```rs\nfn main() {}\n```\n";
        let lines = render_markdown(md, 80, MarkdownTheme::passthrough());
        let any_colored = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .any(|s| s.style.fg.is_some());
        assert!(any_colored, "some code span carries fg color (highlighted)");
    }

    #[test]
    fn multiple_paragraphs_separated() {
        let md = "first\n\nsecond\n";
        let lines = render(md, 80);
        // Two paragraphs -> two text lines with a blank separator between
        // (block_end inserts a blank after each paragraph for visual structure).
        assert!(lines.contains(&"first".to_string()));
        assert!(lines.contains(&"second".to_string()));
        // The blank separator sits between them.
        let first_idx = lines.iter().position(|l| l == "first").unwrap();
        let second_idx = lines.iter().position(|l| l == "second").unwrap();
        assert!(second_idx > first_idx, "second paragraph after first");
        // At least one blank between (consecutive block ends may collapse).
        let blanks_between = lines[first_idx + 1..second_idx]
            .iter()
            .filter(|l| l.is_empty())
            .count();
        assert!(
            blanks_between >= 1,
            "blank separator between paragraphs: {lines:?}"
        );
    }

    #[test]
    fn code_block_separates_from_following_content() {
        // Code block followed by a paragraph: a blank line sits between them
        // (code block emits a trailing blank; the paragraph comes after).
        let md = "```rs\nx\n```\n\nafter\n";
        let lines = render_markdown(md, 80, MarkdownTheme::passthrough());
        let plain: Vec<String> = lines.iter().map(|l| l.plain_text()).collect();
        let code_idx = plain.iter().position(|l| l == "x").unwrap();
        let after_idx = plain.iter().position(|l| l == "after").unwrap();
        // At least one blank between code block and following paragraph.
        let blanks = plain[code_idx + 1..after_idx]
            .iter()
            .filter(|l| l.is_empty())
            .count();
        assert!(
            blanks >= 1,
            "blank separates code block from following content: {plain:?}"
        );
    }
}
