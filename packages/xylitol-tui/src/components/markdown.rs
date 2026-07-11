//! Markdown rendering component.
//!
//! Ported from pi's `components/markdown.ts`. Uses `pulldown-cmark` for
//! parsing, then renders with user-provided theme hooks and optional syntax
//! highlighting via a callback (`highlight_code`).
//!
//! Copy / token policy (c530): hierarchy via SGR (no `#` prefixes); links as
//! `text (url)`; no code fences or box-drawing tables; inline `` ` `` / `**` /
//! `*` / `~~` retained for round-trip. UX SSOT: `src/app/tui/design/markdown.md`.
//!
//! The `MarkdownTheme` is deliberately a struct of boxed closures so consumers
//! (e.g. the main crate with syntect) can inject their own styling pipeline
//! without the TUI crate depending on heavy highlighting libraries.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::terminal_image::is_image_line;
use crate::tui::Component;
use crate::utils::{apply_background_to_line, visible_width, wrap_text_with_ansi};

// ── theme / options ─────────────────────────────────────────────────────────

#[allow(clippy::type_complexity)]
pub struct DefaultTextStyle {
    pub color: Option<Box<dyn Fn(&str) -> String>>,
    pub bg_color: Option<Box<dyn Fn(&str) -> String>>,
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub underline: bool,
}

#[allow(clippy::type_complexity)]
pub struct MarkdownTheme {
    pub heading: Box<dyn Fn(&str) -> String>,
    pub link: Box<dyn Fn(&str) -> String>,
    pub link_url: Box<dyn Fn(&str) -> String>,
    pub code: Box<dyn Fn(&str) -> String>,
    pub code_block: Box<dyn Fn(&str) -> String>,
    pub code_block_border: Box<dyn Fn(&str) -> String>,
    pub quote: Box<dyn Fn(&str) -> String>,
    pub quote_border: Box<dyn Fn(&str) -> String>,
    pub hr: Box<dyn Fn(&str) -> String>,
    pub list_bullet: Box<dyn Fn(&str) -> String>,
    pub bold: Box<dyn Fn(&str) -> String>,
    pub italic: Box<dyn Fn(&str) -> String>,
    pub strikethrough: Box<dyn Fn(&str) -> String>,
    pub underline: Box<dyn Fn(&str) -> String>,
    pub highlight_code: Option<Box<dyn Fn(&str, Option<&str>) -> Vec<String>>>,
    pub code_block_indent: Option<String>,
}

#[derive(Default)]
pub struct MarkdownOptions {
    pub preserve_ordered_list_markers: bool,
    pub preserve_backslash_escapes: bool,
}

// ── Markdown component ──────────────────────────────────────────────────────

pub struct Markdown {
    text: String,
    padding_x: usize,
    padding_y: usize,
    default_text_style: Option<DefaultTextStyle>,
    theme: MarkdownTheme,
    _options: MarkdownOptions,

    cached_text: Option<String>,
    cached_width: Option<usize>,
    cached_lines: Option<Vec<String>>,
}

impl Markdown {
    pub fn new(
        text: String,
        padding_x: usize,
        padding_y: usize,
        theme: MarkdownTheme,
        default_text_style: Option<DefaultTextStyle>,
        options: Option<MarkdownOptions>,
    ) -> Self {
        Self {
            text,
            padding_x,
            padding_y,
            theme,
            default_text_style,
            _options: options.unwrap_or_default(),
            cached_text: None,
            cached_width: None,
            cached_lines: None,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.invalidate();
    }
}

impl Component for Markdown {
    fn render(&mut self, width: usize) -> Vec<String> {
        if let Some(ref lines) = self.cached_lines
            && self.cached_text.as_deref() == Some(&self.text)
            && self.cached_width == Some(width)
        {
            return lines.clone();
        }

        let content_width = width.saturating_sub(self.padding_x * 2).max(1);
        if self.text.trim().is_empty() {
            self.cache_result(width, vec![]);
            return vec![];
        }

        let normalized = self.text.replace('\t', "   ");

        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);

        let parser = Parser::new_ext(&normalized, opts);
        let events: Vec<Event> = parser.collect();

        // ---------------------------------------------------------------
        // Collect block-level events into styled strings.
        let mut rendered: Vec<String> = Vec::new();
        let mut idx = 0;
        while idx < events.len() {
            let event = &events[idx];
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    idx += 1;
                    let heading_lines = collect_inline_until(
                        self,
                        &events,
                        &mut idx,
                        &|s| apply_default_style(self, s),
                        "",
                    );
                    // Token-efficient: no `#` prefix; hierarchy via SGR (c530 / design/markdown.md).
                    let line = heading_lines.concat();
                    let n = level_to_usize(*level);
                    let styled = match n {
                        1 => {
                            (self.theme.heading)(&(self.theme.bold)(&(self.theme.underline)(&line)))
                        }
                        2 => {
                            (self.theme.heading)(&(self.theme.bold)(&(self.theme.underline)(&line)))
                        }
                        3 | 4 => (self.theme.heading)(&(self.theme.bold)(&line)),
                        _ => (self.theme.heading)(&line),
                    };
                    rendered.push(styled);
                    if !next_is_space(&events, idx) {
                        rendered.push(String::new());
                    }
                }

                Event::Start(Tag::Paragraph) => {
                    idx += 1;
                    let text = collect_inline_until(
                        self,
                        &events,
                        &mut idx,
                        &|s| apply_default_style(self, s),
                        "",
                    );
                    rendered.push(text.concat());
                    if !next_is_space(&events, idx) && !next_is_list(&events, idx) {
                        rendered.push(String::new());
                    }
                }

                Event::Start(Tag::CodeBlock(kind)) => {
                    idx += 1;
                    let lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(s) => {
                            if s.is_empty() {
                                None
                            } else {
                                Some(s.as_ref())
                            }
                        }
                        pulldown_cmark::CodeBlockKind::Indented => None,
                    };
                    let code = collect_text_until(&events, &mut idx);
                    let indent = self.theme.code_block_indent.as_deref().unwrap_or("  ");
                    // No fence / language bar / line numbers (c530).
                    if let Some(ref hc) = self.theme.highlight_code {
                        for hl in hc(&code, lang) {
                            rendered.push(format!("{indent}{hl}"));
                        }
                    } else {
                        for cl in code.lines() {
                            rendered.push(format!("{indent}{}", (self.theme.code_block)(cl)));
                        }
                    }
                    if !next_is_space(&events, idx) {
                        rendered.push(String::new());
                    }
                }

                Event::Start(Tag::List(first_num)) => {
                    idx += 1;
                    let is_ordered = first_num.is_some() || list_is_ordered(&events, idx);
                    let start = first_num.unwrap_or(1) as usize;
                    rendered.extend(render_list(
                        self,
                        &events,
                        &mut idx,
                        0,
                        content_width,
                        is_ordered,
                        start,
                    ));
                }

                Event::Start(Tag::BlockQuote(_)) => {
                    idx += 1;
                    let quote_text_fn = |s: &str| (self.theme.quote)(&(self.theme.italic)(s));
                    let quote_prefix = get_style_prefix(&quote_text_fn);

                    let mut quote_body = Vec::new();
                    while !at_list_end(&events, idx, "BlockQuote") {
                        quote_body.extend(render_events_block(
                            self,
                            &events,
                            &mut idx,
                            content_width.max(1),
                            &quote_text_fn,
                            &quote_prefix,
                        ));
                    }
                    // skip End(BlockQuote)
                    skip_end_tag(&events, &mut idx);

                    while quote_body.last().is_some_and(|l| l.is_empty()) {
                        quote_body.pop();
                    }

                    for ql in quote_body {
                        for wl in wrap_text_with_ansi(&ql, content_width.max(1)) {
                            // Dim/italic only — no │ / box decoration (c530).
                            rendered.push(wl);
                        }
                    }
                    if !next_is_space(&events, idx) {
                        rendered.push(String::new());
                    }
                }

                Event::Start(Tag::Table(_)) => {
                    idx += 1;
                    rendered.extend(render_table(self, &events, &mut idx, content_width));
                    if !next_is_space(&events, idx) {
                        rendered.push(String::new());
                    }
                }

                Event::Rule => {
                    idx += 1;
                    // Short rule — not a full-width wall (c530).
                    let w = content_width.clamp(4, 8);
                    rendered.push((self.theme.hr)(&"─".repeat(w)));
                    if !next_is_space(&events, idx) {
                        rendered.push(String::new());
                    }
                }

                Event::SoftBreak | Event::HardBreak => {
                    idx += 1;
                }

                Event::Text(t) => {
                    rendered.push(apply_default_style(self, t));
                    idx += 1;
                }

                _ => {
                    idx += 1;
                }
            }
        }

        // ── wrap + margins + background + padding ────────────────────────
        let mut wrapped: Vec<String> = Vec::new();
        for line in rendered {
            if is_image_line(&line) {
                wrapped.push(line);
            } else {
                wrapped.extend(wrap_text_with_ansi(&line, content_width));
            }
        }

        let left = " ".repeat(self.padding_x);
        let right = " ".repeat(self.padding_x);
        let mut content: Vec<String> = Vec::new();
        for line in wrapped {
            if is_image_line(&line) {
                content.push(line);
                continue;
            }
            let with_margins = format!("{left}{line}{right}");
            if let Some(ref bg) = self
                .default_text_style
                .as_ref()
                .and_then(|s| s.bg_color.as_ref())
            {
                content.push(apply_background_to_line(&with_margins, width, bg));
            } else {
                let vis = visible_width(&with_margins);
                let pad = width.saturating_sub(vis);
                content.push(format!("{with_margins}{}", " ".repeat(pad)));
            }
        }

        let empty = " ".repeat(width);
        let pad_line = if let Some(ref bg) = self
            .default_text_style
            .as_ref()
            .and_then(|s| s.bg_color.as_ref())
        {
            apply_background_to_line(&empty, width, bg)
        } else {
            empty.clone()
        };

        let mut result = Vec::new();
        for _ in 0..self.padding_y {
            result.push(pad_line.clone());
        }
        result.extend(content);
        for _ in 0..self.padding_y {
            result.push(pad_line.clone());
        }

        let result = if result.is_empty() {
            vec![String::new()]
        } else {
            result
        };
        self.cache_result(width, result.clone());
        result
    }

    fn handle_input(&mut self, _event: crate::tui::InputEvent) {}
    fn invalidate(&mut self) {
        self.cached_text = None;
        self.cached_width = None;
        self.cached_lines = None;
    }
}

impl Markdown {
    fn cache_result(&mut self, width: usize, lines: Vec<String>) {
        self.cached_text = Some(self.text.clone());
        self.cached_width = Some(width);
        self.cached_lines = Some(lines);
    }
}

// ── style helpers ───────────────────────────────────────────────────────────

fn apply_default_style(md: &Markdown, text: &str) -> String {
    let Some(ref style) = md.default_text_style else {
        return text.to_string();
    };
    let mut styled = text.to_string();
    if let Some(ref color) = style.color {
        styled = color(&styled);
    }
    if style.bold {
        styled = (md.theme.bold)(&styled);
    }
    if style.italic {
        styled = (md.theme.italic)(&styled);
    }
    if style.strikethrough {
        styled = (md.theme.strikethrough)(&styled);
    }
    if style.underline {
        styled = (md.theme.underline)(&styled);
    }
    styled
}

fn get_style_prefix(f: &dyn Fn(&str) -> String) -> String {
    let sentinel = "\0";
    let styled = f(sentinel);
    let idx = styled.find(sentinel).unwrap_or(0);
    styled[..idx].to_string()
}

// ── event iterators ─────────────────────────────────────────────────────────

fn collect_inline_until(
    md: &Markdown,
    events: &[Event],
    idx: &mut usize,
    default_fn: &dyn Fn(&str) -> String,
    style_prefix: &str,
) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();

    while *idx < events.len() {
        match &events[*idx] {
            Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::Heading(_))
            | Event::End(TagEnd::TableHead)
            | Event::End(TagEnd::TableRow)
            | Event::End(_) => break,

            Event::Start(Tag::Strong) => {
                *idx += 1;
                let inner =
                    collect_inline_until(md, events, idx, default_fn, style_prefix).concat();
                let marked = format!("**{inner}**");
                parts.push((md.theme.bold)(&marked));
                parts.push(style_prefix.to_string());
            }
            Event::Start(Tag::Emphasis) => {
                *idx += 1;
                let inner =
                    collect_inline_until(md, events, idx, default_fn, style_prefix).concat();
                let marked = format!("*{inner}*");
                parts.push((md.theme.italic)(&marked));
                parts.push(style_prefix.to_string());
            }
            Event::Start(Tag::Strikethrough) => {
                *idx += 1;
                let inner =
                    collect_inline_until(md, events, idx, default_fn, style_prefix).concat();
                let marked = format!("~~{inner}~~");
                parts.push((md.theme.strikethrough)(&marked));
                parts.push(style_prefix.to_string());
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                *idx += 1;
                let inner =
                    collect_inline_until(md, events, idx, default_fn, style_prefix).concat();
                let url = dest_url.as_ref();
                let label = if strip_ansi_for_empty(&inner).is_empty() {
                    url.to_string()
                } else {
                    inner
                };
                let labeled = (md.theme.link)(&label);
                let urled = (md.theme.link_url)(url);
                parts.push(format!("{labeled} ({urled})"));
                parts.push(style_prefix.to_string());
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                *idx += 1;
                let inner =
                    collect_inline_until(md, events, idx, default_fn, style_prefix).concat();
                let url = dest_url.as_ref();
                let label = if strip_ansi_for_empty(&inner).is_empty() {
                    url.to_string()
                } else {
                    inner
                };
                let labeled = (md.theme.link)(&label);
                let urled = (md.theme.link_url)(url);
                parts.push(format!("{labeled} ({urled})"));
                parts.push(style_prefix.to_string());
            }
            Event::Code(code) => {
                let marked = format!("`{code}`");
                parts.push((md.theme.code)(&marked));
                parts.push(style_prefix.to_string());
                *idx += 1;
            }
            Event::Text(t) => {
                parts.push(default_fn(t));
                *idx += 1;
            }
            Event::Html(raw) => {
                parts.push(default_fn(raw));
                *idx += 1;
            }
            Event::SoftBreak | Event::HardBreak => {
                parts.push("\n".to_string());
                *idx += 1;
            }
            _ => *idx += 1,
        }
    }

    // Skip closing tag
    if *idx < events.len() {
        match &events[*idx] {
            Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::Heading(_))
            | Event::End(TagEnd::TableHead)
            | Event::End(TagEnd::TableRow)
            | Event::End(TagEnd::Strong)
            | Event::End(TagEnd::Emphasis)
            | Event::End(TagEnd::Strikethrough)
            | Event::End(TagEnd::Link)
            | Event::End(TagEnd::Image) => *idx += 1,
            _ => {}
        }
    }

    parts
}

/// True empty after stripping CSI / OSC for label fallback.
fn strip_ansi_for_empty(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == '\u{7}' {
                            break;
                        }
                        if ch == '\\' {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}

fn collect_text_until(events: &[Event], idx: &mut usize) -> String {
    let mut text = String::new();
    while *idx < events.len() {
        match &events[*idx] {
            Event::Text(t) => {
                text.push_str(t);
                *idx += 1;
            }
            Event::End(TagEnd::CodeBlock) => {
                *idx += 1;
                break;
            }
            Event::SoftBreak | Event::HardBreak => {
                text.push('\n');
                *idx += 1;
            }
            _ => *idx += 1,
        }
    }
    text
}

fn next_is_space(events: &[Event], idx: usize) -> bool {
    matches!(events.get(idx), Some(Event::SoftBreak | Event::HardBreak))
}

fn next_is_list(events: &[Event], idx: usize) -> bool {
    matches!(events.get(idx), Some(Event::Start(Tag::List(_))))
}

fn list_is_ordered(events: &[Event], start_idx: usize) -> bool {
    let mut i = start_idx;
    while i < events.len() {
        if let Event::Start(Tag::Item) = &events[i] {
            i += 1;
            if let Some(Event::Text(t)) = events.get(i) {
                let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
                return digits > 0 && t.as_bytes().get(digits) == Some(&b'.');
            }
        } else {
            i += 1;
        }
    }
    false
}

fn at_list_end(events: &[Event], idx: usize, tag: &str) -> bool {
    match events.get(idx) {
        Some(Event::End(TagEnd::BlockQuote(_))) if tag == "BlockQuote" => true,
        Some(Event::End(TagEnd::List(_))) if tag == "List" => true,
        Some(Event::End(TagEnd::Table)) if tag == "Table" => true,
        _ => false,
    }
}

fn skip_end_tag(events: &[Event], idx: &mut usize) {
    if *idx < events.len() && matches!(events[*idx], Event::End(_)) {
        *idx += 1;
    }
}

// ── list rendering ──────────────────────────────────────────────────────────

fn render_list(
    md: &Markdown,
    events: &[Event],
    idx: &mut usize,
    depth: usize,
    width: usize,
    is_ordered: bool,
    start_number: usize,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let indent = "    ".repeat(depth);
    let mut item_index = 0usize;

    while *idx < events.len() {
        match &events[*idx] {
            Event::End(TagEnd::List(_)) => {
                *idx += 1;
                break;
            }
            Event::End(TagEnd::Item) => {
                *idx += 1;
                item_index += 1;
            }
            Event::Start(Tag::Item) => {
                *idx += 1;

                let marker = if is_ordered {
                    format!("{}. ", start_number + item_index)
                } else {
                    "- ".to_string()
                };
                let first_prefix = format!("{indent}{}", (md.theme.list_bullet)(&marker));
                let continuation = format!("{indent}{}", " ".repeat(visible_width(&marker)));
                let item_width = width.saturating_sub(visible_width(&first_prefix)).max(1);
                let mut rendered_any = false;

                loop {
                    if *idx >= events.len() {
                        break;
                    }
                    match &events[*idx] {
                        Event::End(TagEnd::Item) | Event::End(TagEnd::List(_)) => break,

                        Event::Start(Tag::List(nested_start)) => {
                            *idx += 1;
                            let ns = nested_start.unwrap_or(1) as usize;
                            let no = list_is_ordered(events, *idx);
                            lines.extend(render_list(md, events, idx, depth + 1, width, no, ns));
                            rendered_any = true;
                            continue;
                        }
                        Event::Start(Tag::Paragraph) => {
                            *idx += 1;
                            let text = collect_inline_until(
                                md,
                                events,
                                idx,
                                &|s| apply_default_style(md, s),
                                "",
                            );
                            let joined = text.concat();
                            if joined.is_empty() {
                                continue;
                            }
                            for wl in wrap_text_with_ansi(&joined, item_width) {
                                let prefix = if rendered_any {
                                    continuation.clone()
                                } else {
                                    first_prefix.clone()
                                };
                                lines.push(format!("{prefix}{wl}"));
                                rendered_any = true;
                            }
                        }
                        Event::Text(t) => {
                            *idx += 1;
                            let styled = apply_default_style(md, t);
                            if styled.trim().is_empty() {
                                continue;
                            }
                            for wl in wrap_text_with_ansi(&styled, item_width) {
                                let prefix = if rendered_any {
                                    continuation.clone()
                                } else {
                                    first_prefix.clone()
                                };
                                lines.push(format!("{prefix}{wl}"));
                                rendered_any = true;
                            }
                        }
                        _ => *idx += 1,
                    }
                }

                if !rendered_any {
                    lines.push(first_prefix);
                }
            }
            _ => *idx += 1,
        }
    }

    lines
}

fn render_events_block(
    md: &Markdown,
    events: &[Event],
    idx: &mut usize,
    width: usize,
    quote_fn: &dyn Fn(&str) -> String,
    quote_prefix: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    while *idx < events.len() {
        match &events[*idx] {
            Event::End(TagEnd::BlockQuote(_))
            | Event::End(TagEnd::List(_))
            | Event::End(TagEnd::Table) => break,
            Event::End(TagEnd::Paragraph) => {
                *idx += 1;
                break;
            }
            Event::Start(Tag::Paragraph) => {
                *idx += 1;
                let text = collect_inline_until(md, events, idx, quote_fn, quote_prefix);
                lines.push(text.concat());
            }
            Event::Start(Tag::List(first_num)) => {
                *idx += 1;
                let is_ordered = first_num.is_some() || list_is_ordered(events, *idx);
                let start = first_num.unwrap_or(1) as usize;
                lines.extend(render_list(md, events, idx, 0, width, is_ordered, start));
            }
            Event::Start(Tag::CodeBlock(_)) => {
                let text = collect_text_until(events, idx);
                lines.push(text);
            }
            Event::Text(t) => {
                lines.push(quote_fn(t));
                *idx += 1;
            }
            _ => *idx += 1,
        }
    }
    lines
}

// ── table rendering ─────────────────────────────────────────────────────────

fn render_table(md: &Markdown, events: &[Event], idx: &mut usize, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut headers: Vec<String> = Vec::new();
    let mut body_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut in_header = false;

    while *idx < events.len() {
        match &events[*idx] {
            Event::End(TagEnd::Table) => {
                *idx += 1;
                break;
            }
            Event::Start(Tag::TableHead) => {
                *idx += 1;
                in_header = true;
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                *idx += 1;
                if !current_row.is_empty() {
                    headers = std::mem::take(&mut current_row);
                }
                in_header = false;
            }
            Event::End(TagEnd::TableRow) => {
                *idx += 1;
                if in_header || headers.is_empty() {
                    headers = std::mem::take(&mut current_row);
                    in_header = false;
                } else if !current_row.is_empty() {
                    body_rows.push(std::mem::take(&mut current_row));
                }
            }
            Event::Start(Tag::TableCell) => {
                *idx += 1;
                let cell =
                    collect_inline_until(md, events, idx, &|s| apply_default_style(md, s), "");
                current_row.push(cell.concat());
            }
            _ => *idx += 1,
        }
    }

    let num_cols = headers.len();
    if num_cols == 0 {
        return lines;
    }
    let overhead = table_overhead(num_cols);
    let available = width.saturating_sub(overhead);
    if available < num_cols {
        return lines;
    }

    let max_unbroken = 30;
    let mut natural: Vec<usize> = vec![0; num_cols];
    let mut min_word: Vec<usize> = vec![1; num_cols];
    for (i, h) in headers.iter().enumerate() {
        natural[i] = visible_width(h);
        min_word[i] = longest_word_width(h, max_unbroken).max(1);
    }
    for row in &body_rows {
        for (i, cell) in row.iter().enumerate() {
            natural[i] = natural[i].max(visible_width(cell));
            min_word[i] = min_word[i].max(longest_word_width(cell, max_unbroken).max(1));
        }
    }

    let widths = compute_column_widths(&natural, &min_word, available);

    // Space-aligned columns (scheme A) — no box drawing, no decorative `|` (c530).
    let join_row = |parts: &[String]| -> String { parts.join(" ") };

    // Header (bold + underline)
    let hw: Vec<Vec<String>> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| wrap_text_with_ansi(h, widths[i]))
        .collect();
    let hr_cnt = hw.iter().map(|c| c.len()).max().unwrap_or(1);
    for ri in 0..hr_cnt {
        let parts: Vec<String> = hw
            .iter()
            .enumerate()
            .map(|(ci, cw)| {
                let text = cw.get(ri).cloned().unwrap_or_default();
                let pad = widths[ci].saturating_sub(visible_width(&text));
                let cell = format!("{text}{}", " ".repeat(pad));
                (md.theme.underline)(&(md.theme.bold)(&cell))
            })
            .collect();
        lines.push(join_row(&parts));
    }

    // Body
    for row in &body_rows {
        let rw: Vec<Vec<String>> = row
            .iter()
            .enumerate()
            .map(|(i, c)| wrap_text_with_ansi(c, widths[i]))
            .collect();
        let rc = rw.iter().map(|c| c.len()).max().unwrap_or(1);
        for li in 0..rc {
            let parts: Vec<String> = rw
                .iter()
                .enumerate()
                .map(|(ci, cw)| {
                    let text = cw.get(li).cloned().unwrap_or_default();
                    format!(
                        "{text}{}",
                        " ".repeat(widths[ci].saturating_sub(visible_width(&text)))
                    )
                })
                .collect();
            lines.push(join_row(&parts));
        }
    }

    lines
}

// ── utilities ───────────────────────────────────────────────────────────────

fn level_to_usize(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn longest_word_width(text: &str, max: usize) -> usize {
    text.split_whitespace()
        .map(visible_width)
        .max()
        .unwrap_or(0)
        .min(max)
}

fn table_overhead(num_cols: usize) -> usize {
    // Scheme A: single space between columns.
    num_cols.saturating_sub(1)
}

fn compute_column_widths(natural: &[usize], min_word: &[usize], available: usize) -> Vec<usize> {
    let n = natural.len();
    if n == 0 {
        return Vec::new();
    }

    let fit_to_available = |preferred: &[usize]| -> Vec<usize> {
        let mut widths = vec![1; n];
        let mut remaining = available.saturating_sub(n);
        let mut grew = true;
        while grew && remaining > 0 {
            grew = false;
            for i in 0..n {
                if widths[i] < preferred[i].max(1) && remaining > 0 {
                    widths[i] += 1;
                    remaining -= 1;
                    grew = true;
                }
            }
        }
        widths
    };

    let total_natural: usize = natural.iter().sum();
    if total_natural <= available {
        let preferred: Vec<usize> = (0..n).map(|i| natural[i].max(min_word[i])).collect();
        if preferred.iter().sum::<usize>() <= available {
            return preferred;
        }
        return fit_to_available(&preferred);
    }

    let min_total: usize = min_word.iter().sum();
    if min_total > available {
        return fit_to_available(min_word);
    }

    let extra = available.saturating_sub(min_total);
    let total_grow: usize = (0..n).map(|i| natural[i].saturating_sub(min_word[i])).sum();
    let mut widths: Vec<usize> = (0..n).map(|i| min_word[i]).collect();
    if total_grow > 0 {
        for i in 0..n {
            widths[i] += (natural[i].saturating_sub(min_word[i]) * extra)
                .checked_div(total_grow)
                .unwrap_or(0);
        }
    }
    // Rounding
    let allocated: usize = widths.iter().sum();
    let mut remaining = available.saturating_sub(allocated);
    let mut grew = true;
    while grew && remaining > 0 {
        grew = false;
        for i in 0..n {
            if widths[i] < natural[i] && remaining > 0 {
                widths[i] += 1;
                remaining -= 1;
                grew = true;
            }
        }
    }
    widths
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_theme() -> MarkdownTheme {
        let id = Box::new(|s: &str| s.to_string());
        MarkdownTheme {
            heading: id.clone(),
            link: id.clone(),
            link_url: id.clone(),
            code: id.clone(),
            code_block: id.clone(),
            code_block_border: id.clone(),
            quote: id.clone(),
            quote_border: id.clone(),
            hr: id.clone(),
            list_bullet: id.clone(),
            bold: id.clone(),
            italic: id.clone(),
            strikethrough: id.clone(),
            underline: id.clone(),
            highlight_code: None,
            code_block_indent: None,
        }
    }

    fn visible_join(md: &mut Markdown, width: usize) -> String {
        md.render(width)
            .iter()
            .map(|l| strip_ansi_for_empty(l))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn renders_paragraph() {
        let mut md = Markdown::new("hello world".into(), 1, 1, identity_theme(), None, None);
        let lines = md.render(30);
        assert!(lines.iter().any(|l| l.contains("hello")));
    }

    #[test]
    fn heading_has_no_hash_prefix() {
        let mut md = Markdown::new("# Title".into(), 1, 1, identity_theme(), None, None);
        let text = visible_join(&mut md, 30);
        assert!(text.contains("Title"));
        assert!(
            !text.lines().any(|l| l.trim_start().starts_with('#')),
            "heading must not use # prefix:\n{text}"
        );
    }

    #[test]
    fn code_block_has_no_fence() {
        let mut md = Markdown::new(
            "```rust\nfn main() {}\n```".into(),
            1,
            1,
            identity_theme(),
            None,
            None,
        );
        let text = visible_join(&mut md, 40);
        assert!(text.contains("fn main() {}"));
        assert!(
            !text.contains("```"),
            "code block must not emit fence lines:\n{text}"
        );
    }

    #[test]
    fn image_is_alt_url_form() {
        let mut md = Markdown::new(
            "![diagram](https://ex.com/a.png)".into(),
            0,
            0,
            identity_theme(),
            None,
            None,
        );
        let text = visible_join(&mut md, 60);
        assert!(
            text.contains("diagram (https://ex.com/a.png)"),
            "expected alt (url):\n{text}"
        );
    }

    #[test]
    fn link_is_text_url_form() {
        let mut md = Markdown::new(
            "see [docs](https://ex.com)".into(),
            0,
            0,
            identity_theme(),
            None,
            None,
        );
        let text = visible_join(&mut md, 60);
        assert!(
            text.contains("docs (https://ex.com)"),
            "expected text (url):\n{text}"
        );
    }

    #[test]
    fn inline_markers_roundtrip() {
        let mut md = Markdown::new(
            "a **bold** and *ital* and `code` and ~~x~~".into(),
            0,
            0,
            identity_theme(),
            None,
            None,
        );
        let text = visible_join(&mut md, 80);
        assert!(text.contains("**bold**"), "{text}");
        assert!(text.contains("*ital*"), "{text}");
        assert!(text.contains("`code`"), "{text}");
        assert!(text.contains("~~x~~"), "{text}");
    }

    #[test]
    fn quote_has_no_bar() {
        let mut md = Markdown::new("> hello quote".into(), 0, 0, identity_theme(), None, None);
        let text = visible_join(&mut md, 40);
        assert!(text.contains("hello quote"));
        assert!(!text.contains('│'), "quote must not use box bar:\n{text}");
    }

    #[test]
    fn renders_list() {
        let mut md = Markdown::new("- one\n- two".into(), 1, 1, identity_theme(), None, None);
        let lines = md.render(20);
        assert!(lines.iter().any(|l| l.contains("one")));
        assert!(lines.iter().any(|l| l.contains("two")));
    }

    #[test]
    fn empty_text_returns_empty() {
        let mut md = Markdown::new("   ".into(), 0, 0, identity_theme(), None, None);
        assert!(md.render(20).is_empty());
    }

    #[test]
    fn table_is_space_aligned_without_box() {
        let mut md = Markdown::new(
            "| a | b |\n|---|---|\n| 1 | 2 |".into(),
            0,
            0,
            identity_theme(),
            None,
            None,
        );
        let text = visible_join(&mut md, 40);
        assert!(text.contains('a') && text.contains('1'));
        assert!(
            !text.contains('┌') && !text.contains('│') && !text.contains('└'),
            "table must not use box drawing:\n{text}"
        );
        assert!(
            !text.contains('|'),
            "table must not emit decorative pipes:\n{text}"
        );
    }

    #[test]
    fn table_respects_width_when_min_words_exceed_available_space() {
        let mut md = Markdown::new(
            "\
| Feature | Status | Notes |
|---------|--------|-------|
| Autocomplete | file-path + slash-command with fd recursive search | another-long-unbroken-token |
"
            .into(),
            0,
            0,
            identity_theme(),
            None,
            None,
        );

        let lines = md.render(72);
        for line in lines {
            assert!(
                visible_width(&line) <= 72,
                "table line exceeds width: {} > 72: {line:?}",
                visible_width(&line)
            );
        }
    }

    #[test]
    fn table_keeps_styled_cells_separate() {
        let mut md = Markdown::new(
            "| Feature | Status |\n|---------|--------|\n| **Editor** | multi-line |\n".into(),
            0,
            0,
            identity_theme(),
            None,
            None,
        );

        let text = visible_join(&mut md, 48);
        assert!(
            text.contains("**Editor**") && text.contains("multi-line"),
            "styled first cell must not absorb the next cell:\n{text}"
        );
    }

    #[test]
    fn hr_is_short() {
        let mut md = Markdown::new("---".into(), 0, 0, identity_theme(), None, None);
        let text = visible_join(&mut md, 80);
        let rule = text.lines().find(|l| l.contains('─')).unwrap_or("");
        assert!(
            rule.chars().filter(|c| *c == '─').count() <= 8,
            "hr should be short:\n{text}"
        );
    }
}
