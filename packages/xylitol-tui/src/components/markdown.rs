//! Markdown rendering component.
//!
//! Ported from pi's `components/markdown.ts`. Uses `pulldown-cmark` for
//! parsing, then renders with user-provided theme hooks and optional syntax
//! highlighting via a callback (`highlight_code`).
//!
//! Copy / token policy (c530+): hierarchy via SGR (no `#` prefixes); links as
//! `text (url)`; no code fences or box-drawing tables; bold/italic = SGR only
//! (no visible `**`/`*`); `` ` `` / `~~` retained; blockquotes use a muted
//! `│ ` gutter (same token as quote body). UX SSOT:
//! `src/app/tui/design/markdown.md`.
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
    /// Heading line by level (`1..=6`). Caller applies color + weight/underline.
    pub heading: Box<dyn Fn(usize, &str) -> String>,
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

// ── Markdown component ──────────────────────────────────────────────────────

pub struct Markdown {
    text: String,
    padding_x: usize,
    padding_y: usize,
    default_text_style: Option<DefaultTextStyle>,
    theme: MarkdownTheme,

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
    ) -> Self {
        Self {
            text,
            padding_x,
            padding_y,
            theme,
            default_text_style,
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
        // GFM task lists — without this, `[` / `]` / `x` arrive as separate Text
        // events and tight-list rendering used to put each on its own line.
        opts.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(&normalized, opts);
        let events: Vec<Event> = parser.collect();

        // ---------------------------------------------------------------
        // Collect block-level events into styled strings.
        let mut rendered: Vec<MdLine> = Vec::new();
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
                    // Token-efficient: no `#` prefix; hierarchy via theme.heading(level) (c530).
                    let line = heading_lines.concat();
                    let n = level_to_usize(*level);
                    rendered.push(MdLine::raw((self.theme.heading)(n, &line)));
                    if !next_is_space(&events, idx) {
                        rendered.push(MdLine::raw(String::new()));
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
                    rendered.push(MdLine::raw(text.concat()));
                    if !next_is_space(&events, idx) && !next_is_list(&events, idx) {
                        rendered.push(MdLine::raw(String::new()));
                    }
                }

                Event::Start(Tag::CodeBlock(kind)) => {
                    idx += 1;
                    let lang = fenced_lang(kind);
                    let code = collect_text_until(&events, &mut idx);
                    for line in render_code_block_lines(self, &code, lang) {
                        rendered.push(MdLine::raw(line));
                    }
                    if !next_is_space(&events, idx) {
                        rendered.push(MdLine::raw(String::new()));
                    }
                }

                Event::Start(Tag::List(first_num)) => {
                    idx += 1;
                    let is_ordered = first_num.is_some() || list_is_ordered(&events, idx);
                    let start = first_num.unwrap_or(1) as usize;
                    rendered.extend(
                        render_list(self, &events, &mut idx, 0, content_width, is_ordered, start)
                            .into_iter()
                            .map(MdLine::prewrapped),
                    );
                }

                Event::Start(Tag::BlockQuote(_)) => {
                    idx += 1;
                    // quote theme already applies muted + italic — do not nest
                    // theme.italic (warning fg) or the body turns prominent.
                    let quote_text_fn = |s: &str| (self.theme.quote)(s);
                    let quote_prefix = get_style_prefix(&quote_text_fn);
                    let bar = (self.theme.quote_border)("│ ");
                    let bar_w = visible_width(&bar);
                    let body_width = content_width.saturating_sub(bar_w).max(1);

                    let mut quote_body = Vec::new();
                    while !at_list_end(&events, idx, "BlockQuote") {
                        quote_body.extend(render_events_block(
                            self,
                            &events,
                            &mut idx,
                            body_width,
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
                        for wl in wrap_text_with_ansi(&ql, body_width) {
                            rendered.push(MdLine::prewrapped(format!("{bar}{wl}")));
                        }
                    }
                    if !next_is_space(&events, idx) {
                        rendered.push(MdLine::raw(String::new()));
                    }
                }

                Event::Start(Tag::Table(_)) => {
                    idx += 1;
                    rendered.extend(
                        render_table(self, &events, &mut idx, content_width)
                            .into_iter()
                            .map(MdLine::prewrapped),
                    );
                    if !next_is_space(&events, idx) {
                        rendered.push(MdLine::raw(String::new()));
                    }
                }

                Event::Rule => {
                    idx += 1;
                    // Short rule — not a full-width wall (c530).
                    let w = content_width.clamp(4, 8);
                    rendered.push(MdLine::raw((self.theme.hr)(&"─".repeat(w))));
                    if !next_is_space(&events, idx) {
                        rendered.push(MdLine::raw(String::new()));
                    }
                }

                Event::SoftBreak | Event::HardBreak => {
                    idx += 1;
                }

                Event::Text(t) => {
                    rendered.push(MdLine::raw(apply_default_style(self, t)));
                    idx += 1;
                }

                _ => {
                    idx += 1;
                }
            }
        }

        // ── wrap + margins + background + padding ────────────────────────
        // Skip a second wrap for list/table/quote rows (hanging indent / cell
        // packing already fits content_width).
        let mut wrapped: Vec<String> = Vec::new();
        for line in rendered {
            if line.prewrapped || is_image_line(&line.text) {
                wrapped.push(line.text);
            } else {
                wrapped.extend(wrap_text_with_ansi(&line.text, content_width));
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

/// One Markdown output row before margin/pad.
///
/// `prewrapped` lines already respect `content_width` (list hanging indent,
/// table rows, quote wraps) and must not go through a second wrap pass.
struct MdLine {
    text: String,
    prewrapped: bool,
}

impl MdLine {
    fn raw(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            prewrapped: false,
        }
    }

    fn prewrapped(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            prewrapped: true,
        }
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
                // SGR bold only — no visible `**` (design/markdown.md).
                parts.push((md.theme.bold)(&inner));
                parts.push(style_prefix.to_string());
            }
            Event::Start(Tag::Emphasis) => {
                *idx += 1;
                let inner =
                    collect_inline_until(md, events, idx, default_fn, style_prefix).concat();
                // SGR italic only — no visible `*`.
                parts.push((md.theme.italic)(&inner));
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
            Event::TaskListMarker(checked) => {
                // Visible round-trip markers (design/markdown.md): `- [ ]` / `- [x]`.
                let marker = if *checked { "[x] " } else { "[ ] " };
                parts.push(default_fn(marker));
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
                // Leave break for the caller when a nested block follows (list /
                // paragraph / …); otherwise SoftBreak would become a stray `\n`
                // glued onto the parent item before `render_list` sees Start(List).
                if next_is_block_start(events, *idx + 1) {
                    break;
                }
                parts.push("\n".to_string());
                *idx += 1;
            }
            // Block starts belong to the caller (e.g. nested lists inside an
            // Item). Never skip them — that used to flatten nesting into the
            // parent bullet and scramble subsequent markers.
            Event::Start(
                Tag::List(_)
                | Tag::Item
                | Tag::Paragraph
                | Tag::Heading { .. }
                | Tag::CodeBlock(_)
                | Tag::BlockQuote(_)
                | Tag::Table(_)
                | Tag::TableHead
                | Tag::TableRow
                | Tag::TableCell
                | Tag::HtmlBlock,
            ) => break,
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

fn next_is_block_start(events: &[Event], idx: usize) -> bool {
    matches!(
        events.get(idx),
        Some(Event::Start(
            Tag::List(_)
                | Tag::Item
                | Tag::Paragraph
                | Tag::Heading { .. }
                | Tag::CodeBlock(_)
                | Tag::BlockQuote(_)
                | Tag::Table(_)
                | Tag::HtmlBlock,
        ))
    )
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

fn fenced_lang<'a>(kind: &'a pulldown_cmark::CodeBlockKind<'_>) -> Option<&'a str> {
    match kind {
        pulldown_cmark::CodeBlockKind::Fenced(s) if !s.is_empty() => Some(s.as_ref()),
        _ => None,
    }
}

/// Highlighted (or themed) code lines without fence chrome (c530).
fn render_code_block_lines(md: &Markdown, code: &str, lang: Option<&str>) -> Vec<String> {
    let indent = md.theme.code_block_indent.as_deref().unwrap_or("  ");
    if let Some(ref hc) = md.theme.highlight_code {
        hc(code, lang)
            .into_iter()
            .map(|hl| format!("{indent}{hl}"))
            .collect()
    } else {
        code.lines()
            .map(|cl| format!("{indent}{}", (md.theme.code_block)(cl)))
            .collect()
    }
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

/// Wrap item body and attach bullet / continuation prefixes. Returns whether any
/// non-empty content was pushed.
fn push_list_item_lines(
    lines: &mut Vec<String>,
    joined: &str,
    item_width: usize,
    first_prefix: &str,
    continuation: &str,
    rendered_any: &mut bool,
) -> bool {
    if joined.is_empty() {
        return false;
    }
    for wl in wrap_text_with_ansi(joined, item_width) {
        let prefix = if *rendered_any {
            continuation
        } else {
            first_prefix
        };
        lines.push(format!("{prefix}{wl}"));
        *rendered_any = true;
    }
    true
}

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
                            // Prefer pulldown's List(Some(n)) — item text is the
                            // body only, so list_is_ordered (digit-prefix heuristic)
                            // cannot see "1." for nested ordered lists.
                            let no = nested_start.is_some() || list_is_ordered(events, *idx);
                            lines.extend(render_list(md, events, idx, depth + 1, width, no, ns));
                            rendered_any = true;
                            continue;
                        }
                        Event::Start(Tag::Paragraph) => {
                            *idx += 1;
                            let joined = collect_inline_until(
                                md,
                                events,
                                idx,
                                &|s| apply_default_style(md, s),
                                "",
                            )
                            .concat();
                            push_list_item_lines(
                                &mut lines,
                                &joined,
                                item_width,
                                &first_prefix,
                                &continuation,
                                &mut rendered_any,
                            );
                        }
                        Event::Start(Tag::CodeBlock(kind)) => {
                            *idx += 1;
                            let lang = fenced_lang(kind);
                            let code = collect_text_until(events, idx);
                            for line in render_code_block_lines(md, &code, lang) {
                                let prefix = if rendered_any {
                                    continuation.as_str()
                                } else {
                                    first_prefix.as_str()
                                };
                                lines.push(format!("{prefix}{line}"));
                                rendered_any = true;
                            }
                        }
                        // Tight items: join consecutive inlines (Text / TaskListMarker /
                        // Strong / Link / …) into one line — never one Text event per row.
                        Event::Text(_)
                        | Event::Code(_)
                        | Event::TaskListMarker(_)
                        | Event::SoftBreak
                        | Event::HardBreak
                        | Event::Start(
                            Tag::Strong
                            | Tag::Emphasis
                            | Tag::Strikethrough
                            | Tag::Link { .. }
                            | Tag::Image { .. },
                        ) => {
                            let before = *idx;
                            let joined = collect_inline_until(
                                md,
                                events,
                                idx,
                                &|s| apply_default_style(md, s),
                                "",
                            )
                            .concat();
                            if *idx == before {
                                *idx += 1;
                                continue;
                            }
                            push_list_item_lines(
                                &mut lines,
                                &joined,
                                item_width,
                                &first_prefix,
                                &continuation,
                                &mut rendered_any,
                            );
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
            Event::Start(Tag::CodeBlock(kind)) => {
                *idx += 1;
                let lang = fenced_lang(kind);
                let code = collect_text_until(events, idx);
                // Keep highlight colors — do not wrap with quote_fn (muted/italic).
                lines.extend(render_code_block_lines(md, &code, lang));
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
        let heading = Box::new(|_level: usize, s: &str| s.to_string());
        MarkdownTheme {
            heading,
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
        let mut md = Markdown::new("hello world".into(), 1, 1, identity_theme(), None);
        let lines = md.render(30);
        assert!(lines.iter().any(|l| l.contains("hello")));
    }

    #[test]
    fn heading_has_no_hash_prefix() {
        let mut md = Markdown::new("# Title".into(), 1, 1, identity_theme(), None);
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
        );
        let text = visible_join(&mut md, 60);
        assert!(
            text.contains("docs (https://ex.com)"),
            "expected text (url):\n{text}"
        );
    }

    #[test]
    fn bold_italic_are_sgr_without_star_markers() {
        let mut theme = identity_theme();
        theme.bold = Box::new(|s| format!("\x1b[1m{s}\x1b[22m"));
        theme.italic = Box::new(|s| format!("\x1b[3m{s}\x1b[23m"));
        let mut md = Markdown::new(
            "a **bold** and *ital* and `code` and ~~x~~".into(),
            0,
            0,
            theme,
            None,
        );
        let raw = md.render(80).join("\n");
        let text = visible_join(&mut md, 80);
        assert!(
            !text.contains("**") && !text.contains("*ital*"),
            "bold/italic must not keep star markers:\n{text}"
        );
        assert!(text.contains("bold") && text.contains("ital"), "{text}");
        assert!(text.contains("`code`"), "{text}");
        assert!(text.contains("~~x~~"), "{text}");
        assert!(
            raw.contains("\x1b[1m") && raw.contains("\x1b[3m"),
            "must emit bold/italic SGR:\n{raw:?}"
        );
    }

    #[test]
    fn heading_levels_use_level_callback() {
        let mut theme = identity_theme();
        theme.heading = Box::new(|level, s| format!("H{level}:{s}"));
        let mut md = Markdown::new(
            "# One\n\n## Two\n\n### Three\n\n##### Five\n".into(),
            0,
            0,
            theme,
            None,
        );
        let text = visible_join(&mut md, 40);
        assert!(text.contains("H1:One"), "{text}");
        assert!(text.contains("H2:Two"), "{text}");
        assert!(text.contains("H3:Three"), "{text}");
        assert!(text.contains("H5:Five"), "{text}");
        assert!(!text.contains('#'), "no hash prefix:\n{text}");
    }

    #[test]
    fn quote_uses_muted_bar_prefix() {
        let mut md = Markdown::new("> hello quote".into(), 0, 0, identity_theme(), None);
        let text = visible_join(&mut md, 40);
        assert!(
            text.contains("│ hello quote"),
            "quote must use '│ ' gutter:\n{text}"
        );
    }

    #[test]
    fn quote_code_block_uses_highlight_and_bar() {
        let mut theme = identity_theme();
        theme.highlight_code = Some(Box::new(|code, lang| {
            let tag = lang.unwrap_or("?");
            code.lines().map(|l| format!("HL[{tag}]{l}")).collect()
        }));
        let mut md = Markdown::new(
            "> ```rust\n> fn main() {}\n> ```\n".into(),
            0,
            0,
            theme,
            None,
        );
        let text = visible_join(&mut md, 60);
        assert!(
            text.contains("│") && text.contains("HL[rust]fn main() {}"),
            "quote fence must highlight and keep gutter:\n{text}"
        );
        assert!(!text.contains("```"), "must not show fence chrome:\n{text}");
    }

    #[test]
    fn list_code_block_uses_highlight() {
        let mut theme = identity_theme();
        theme.highlight_code = Some(Box::new(|code, lang| {
            let tag = lang.unwrap_or("?");
            code.lines().map(|l| format!("HL[{tag}]{l}")).collect()
        }));
        let mut md = Markdown::new(
            "- item\n\n  ```py\n  x = 1\n  ```\n".into(),
            0,
            0,
            theme,
            None,
        );
        let text = visible_join(&mut md, 60);
        assert!(
            text.contains("HL[py]x = 1"),
            "list fence must highlight:\n{text}"
        );
    }

    #[test]
    fn quote_wrap_keeps_bar_on_continuation() {
        let mut md = Markdown::new(
            "> a very long quote line that should wrap under the gutter".into(),
            0,
            0,
            identity_theme(),
            None,
        );
        let lines: Vec<String> = md
            .render(24)
            .iter()
            .map(|l| strip_ansi_for_empty(l))
            .filter(|l| !l.trim().is_empty())
            .collect();
        assert!(lines.len() >= 2, "expected wrap:\n{lines:?}");
        for line in &lines {
            assert!(
                line.starts_with("│ "),
                "each wrapped quote line needs '│ ':\n{lines:?}"
            );
        }
    }

    #[test]
    fn renders_list() {
        let mut md = Markdown::new("- one\n- two".into(), 1, 1, identity_theme(), None);
        let lines = md.render(20);
        assert!(lines.iter().any(|l| l.contains("one")));
        assert!(lines.iter().any(|l| l.contains("two")));
    }

    #[test]
    fn task_list_checkbox_stays_on_one_line() {
        let mut md = Markdown::new(
            "- [ ] 终端验收\n- [x] 解析 prompt\n".into(),
            0,
            0,
            identity_theme(),
            None,
        );
        let text = visible_join(&mut md, 40);
        assert!(
            text.lines().any(|l| l.contains("- [ ] 终端验收")),
            "unchecked task must stay on one line:\n{text}"
        );
        assert!(
            text.lines().any(|l| l.contains("- [x] 解析 prompt")),
            "checked task must stay on one line:\n{text}"
        );
        // Regression: never split brackets onto their own rows.
        assert!(
            !text.lines().any(|l| {
                let t = l.trim();
                t == "[" || t == "]" || t == "x"
            }),
            "checkbox brackets must not be lone lines:\n{text}"
        );
    }

    #[test]
    fn tight_list_keeps_inline_styles_on_one_line() {
        let mut md = Markdown::new(
            "- see [docs](https://ex.com) and **bold**\n".into(),
            0,
            0,
            identity_theme(),
            None,
        );
        let text = visible_join(&mut md, 80);
        assert!(
            text.lines().any(|l| {
                l.contains("docs (https://ex.com)") && l.contains("bold") && l.contains("- ")
            }),
            "tight list inlines must share one bullet line:\n{text}"
        );
        assert!(
            !text.contains("**bold**"),
            "bold must not keep star markers:\n{text}"
        );
    }

    #[test]
    fn empty_text_returns_empty() {
        let mut md = Markdown::new("   ".into(), 0, 0, identity_theme(), None);
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
        );

        let text = visible_join(&mut md, 48);
        assert!(
            text.contains("Editor") && text.contains("multi-line"),
            "styled first cell must not absorb the next cell:\n{text}"
        );
        assert!(
            !text.contains("**Editor**"),
            "table bold cells must not keep star markers:\n{text}"
        );
    }

    #[test]
    fn hr_is_short() {
        let mut md = Markdown::new("---".into(), 0, 0, identity_theme(), None);
        let text = visible_join(&mut md, 80);
        let rule = text.lines().find(|l| l.contains('─')).unwrap_or("");
        assert!(
            rule.chars().filter(|c| *c == '─').count() <= 8,
            "hr should be short:\n{text}"
        );
    }

    #[test]
    fn nested_lists_keep_indent_and_ordered_markers() {
        let src = "1. 有序一项\n2. 有序二项\n   - 嵌套无序 A\n   - 嵌套无序 B\n     1. 再嵌套有序\n3. 有序三项含 [链接](https://example.com/list) 与 `code`\n";
        let mut md = Markdown::new(src.into(), 0, 0, identity_theme(), None);
        // trim_end only — strip_ansi_for_empty().trim() would erase list indent.
        let lines: Vec<String> = md
            .render(48)
            .iter()
            .map(|l| l.trim_end().to_string())
            .collect();
        let text = lines.join("\n");

        assert!(
            lines.iter().any(|l| l == "1. 有序一项"),
            "top-level item 1:\n{text}"
        );
        assert!(
            lines.iter().any(|l| l == "2. 有序二项"),
            "parent item must not swallow nested text:\n{text}"
        );
        assert!(
            lines.iter().any(|l| l == "    - 嵌套无序 A"),
            "nested unordered must indent:\n{text}"
        );
        assert!(
            lines.iter().any(|l| l == "        1. 再嵌套有序"),
            "nested ordered must keep 1. marker + deeper indent:\n{text}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("3. 有序三项含 链接 (https://example.com/list)")),
            "item 3 must keep link label+url on the bullet line:\n{text}"
        );
        assert!(
            !lines.iter().any(|l| l.trim() == "链接"),
            "link label must not be a lone wrapped line:\n{text}"
        );
        assert!(
            lines.iter().any(|l| l.contains('`') && l.contains("code")),
            "trailing inline code must still render:\n{text}"
        );
    }

    #[test]
    fn list_hanging_indent_survives_narrow_prewrap() {
        // Long URL forces wrap; continuation must keep spaces matching "3. "
        // (prewrapped list lines must not be re-wrapped by the outer pass).
        let src = "3. 有序三项含 [链接](https://example.com/list) 与 `code`\n";
        let mut md = Markdown::new(src.into(), 0, 0, identity_theme(), None);
        let lines: Vec<String> = md
            .render(40)
            .iter()
            .map(|l| l.trim_end().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        let text = lines.join("\n");
        assert!(
            lines.first().is_some_and(|l| l.starts_with("3. ")),
            "first line keeps marker:\n{text}"
        );
        assert!(
            lines.len() >= 2,
            "narrow width should wrap the long list item:\n{text}"
        );
        assert!(
            lines[1].starts_with("   "),
            "continuation must keep hanging indent (3 spaces), not reflow flush-left:\n{text}"
        );
        assert!(
            !lines[1].starts_with("3. "),
            "continuation must not repeat the marker:\n{text}"
        );
    }
}
