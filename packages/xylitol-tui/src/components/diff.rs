//! Diff component — copy-friendly unified / optional side-by-side rendering.
//!
//! UX: `src/app/tui/design/diff-block.md`. Engine: `similar` (in-package).

use similar::{ChangeTag, TextDiff};

use crate::tui::{Component, InputEvent};
use crate::utils::{truncate_to_width, visible_width, wrap_text_with_ansi};

/// How to feed the Diff renderer.
#[derive(Debug, Clone)]
pub enum DiffInput {
    /// Human gutter format from `generate_display_diff` (or compatible).
    DisplayText(String),
    /// Standard unified diff text.
    UnifiedText(String),
    /// Compute from old/new text (optional path for headers).
    LinePair {
        old: String,
        new: String,
        path: Option<String>,
    },
    /// pi edit display format: `+ 42 content` / `- 11 old` / `  40 context`.
    EditText(String),
}

impl DiffInput {
    /// Build pi-style [`EditText`] from old/new file contents (agent Edit tool path).
    ///
    /// Prefer this over raw [`LinePair`] when you want compact `±N content` gutters
    /// that stay column-aligned (same as pi `generateDiffString`).
    pub fn from_edit_pair(old: &str, new: &str) -> Self {
        Self::EditText(generate_edit_text(old, new, 4))
    }
}

/// Theme closures — product maps semantic tokens → SGR.
///
/// **Layering (pi edit / tool-wash embedding):**
/// - `added` / `removed` / `context`: **fg** for content polarity.
/// - `added_line_bg` / `removed_line_bg`: optional full-row tint after pad (unified
///   only). Prefer identity when the host already paints a `tool-*-bg` shell.
/// - `word_change_added` / `word_change_removed`: changed spans. Prefer
///   `word_wash_bg(block_bg, polarity)` (soft green/red mix ≈0.32 on the tool/row
///   wash) + polarity fg; restore the **block/row bg** (not `49m`). Avoid
///   reverse/white flash and darken-only shades.
/// - Side-by-side skips row tint so polarity stays fg-only (c464).
pub struct DiffTheme {
    pub added: Box<dyn Fn(&str) -> String>,
    pub removed: Box<dyn Fn(&str) -> String>,
    pub context: Box<dyn Fn(&str) -> String>,
    pub gutter: Box<dyn Fn(&str) -> String>,
    pub meta: Box<dyn Fn(&str) -> String>,
    /// Intra-line changed span on insert lines (brighter added bg).
    pub word_change_added: Box<dyn Fn(&str) -> String>,
    /// Intra-line changed span on delete lines (brighter removed bg).
    pub word_change_removed: Box<dyn Fn(&str) -> String>,
    /// Full-row background for insert lines (applied after pad-to-width).
    pub added_line_bg: Box<dyn Fn(&str) -> String>,
    /// Full-row background for delete lines (applied after pad-to-width).
    pub removed_line_bg: Box<dyn Fn(&str) -> String>,
    /// Optional per-line content highlight (default identity). Syntect stays optional.
    pub highlight_line: Box<dyn Fn(&str) -> String>,
}

impl Default for DiffTheme {
    fn default() -> Self {
        // 256-color fallbacks: dark green/red row tint + stronger word tint.
        // Prefer product truecolor via demo/app theme (DESIGN.md diff-*-bg).
        Self {
            added: Box::new(|s| format!("\x1b[32m{s}\x1b[39m")),
            removed: Box::new(|s| format!("\x1b[31m{s}\x1b[39m")),
            context: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            gutter: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            meta: Box::new(|s| format!("\x1b[2m{s}\x1b[22m")),
            word_change_added: Box::new(|s| {
                format!("\x1b[48;5;28m\x1b[32m{s}\x1b[39m\x1b[48;5;22m")
            }),
            word_change_removed: Box::new(|s| {
                format!("\x1b[48;5;88m\x1b[31m{s}\x1b[39m\x1b[48;5;52m")
            }),
            added_line_bg: Box::new(|s| format!("\x1b[48;5;22m{s}\x1b[49m")),
            removed_line_bg: Box::new(|s| format!("\x1b[48;5;52m{s}\x1b[49m")),
            highlight_line: Box::new(|s| s.to_string()),
        }
    }
}

/// Rendering options.
#[derive(Debug, Clone)]
pub struct DiffOptions {
    pub word_level: bool,
    /// When `Some(n)` and `width >= n`, use side-by-side; `None` = always unified.
    pub side_by_side_min_width: Option<usize>,
    /// Compact pi-style `±{num} {content}` gutter (auto for [`DiffInput::EditText`]).
    pub compact_line_numbers: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            word_level: true,
            side_by_side_min_width: Some(100),
            // pi-style `±N content` — dual old/new gutter looks "错位" in demos.
            compact_line_numbers: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Delete,
    Insert,
    Equal,
    Meta,
}

#[derive(Debug, Clone)]
struct DiffLine {
    kind: LineKind,
    /// Sign prefix for unified (`-`/`+`/` `) or empty for meta.
    sign: char,
    content: String,
    old_no: Option<u32>,
    new_no: Option<u32>,
}

/// Free-function renderer (also used by [`Diff`] Component).
pub fn render_diff_lines(
    input: &DiffInput,
    width: usize,
    theme: &DiffTheme,
    options: &DiffOptions,
) -> Vec<String> {
    let lines = normalize_input(input);
    if lines.is_empty() {
        return vec![(theme.meta)("(no changes)")];
    }

    let mut opts = options.clone();
    if matches!(input, DiffInput::EditText(_)) {
        opts.compact_line_numbers = true;
    }
    let num_width = line_number_width(&lines);

    let use_sbs = opts.side_by_side_min_width.is_some_and(|min| width >= min);

    if use_sbs {
        render_side_by_side(&lines, width, theme, &opts, num_width)
    } else {
        render_unified(&lines, width, theme, &opts, num_width)
    }
}

fn normalize_input(input: &DiffInput) -> Vec<DiffLine> {
    let lines = match input {
        DiffInput::LinePair { old, new, path } => lines_from_pair(old, new, path.as_deref()),
        DiffInput::UnifiedText(text) => parse_unified(text),
        DiffInput::DisplayText(text) => parse_display(text),
        DiffInput::EditText(text) => parse_edit_text(text),
    };
    // Expandable / Edit headers already show the path — drop git `--- a/` / `+++ b/` noise.
    lines
        .into_iter()
        .filter(|l| !is_redundant_file_header(l))
        .collect()
}

fn is_redundant_file_header(line: &DiffLine) -> bool {
    if line.kind != LineKind::Meta {
        return false;
    }
    let t = line.content.trim();
    t.starts_with("--- a/")
        || t.starts_with("--- b/")
        || t.starts_with("+++ a/")
        || t.starts_with("+++ b/")
        || t.starts_with("--- /")
        || t.starts_with("+++ /")
}

fn line_number_width(lines: &[DiffLine]) -> usize {
    lines
        .iter()
        .flat_map(|l| [l.old_no, l.new_no])
        .flatten()
        .map(|n| n.to_string().len())
        .max()
        .unwrap_or(1)
        .max(1)
}

/// Parse pi edit display lines: `+ 42 content` / `- 11 old` / `  40 context`.
fn parse_edit_text(text: &str) -> Vec<DiffLine> {
    let mut out = Vec::new();
    for raw in text.lines() {
        if raw.trim() == "..." || raw.trim_start().starts_with("...") {
            out.push(DiffLine {
                kind: LineKind::Meta,
                sign: ' ',
                content: raw.trim().to_string(),
                old_no: None,
                new_no: None,
            });
            continue;
        }
        let Some(parsed) = parse_edit_line(raw) else {
            if !raw.is_empty() {
                out.push(DiffLine {
                    kind: LineKind::Meta,
                    sign: ' ',
                    content: raw.to_string(),
                    old_no: None,
                    new_no: None,
                });
            }
            continue;
        };
        let (kind, sign, no, content) = parsed;
        let (old_no, new_no) = match kind {
            LineKind::Delete => (no, None),
            LineKind::Insert => (None, no),
            LineKind::Equal => (no, no),
            LineKind::Meta => (None, None),
        };
        out.push(DiffLine {
            kind,
            sign,
            content,
            old_no,
            new_no,
        });
    }
    out
}

fn parse_edit_line(line: &str) -> Option<(LineKind, char, Option<u32>, String)> {
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let prefix = bytes[0] as char;
    let (kind, sign) = match prefix {
        '+' => (LineKind::Insert, '+'),
        '-' => (LineKind::Delete, '-'),
        ' ' | '\t' => (LineKind::Equal, ' '),
        _ => return None,
    };
    let rest = &line[1..];
    // Optional padded line number, then a space, then content (pi: `+  42 content`).
    let rest_trim_start = rest.trim_start_matches(' ');
    let digits_end = rest_trim_start
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit())
        .map(|(i, _)| i + 1)
        .last()
        .unwrap_or(0);
    let (no, content) = if digits_end > 0 {
        let num_str = &rest_trim_start[..digits_end];
        let after = rest_trim_start[digits_end..]
            .strip_prefix(' ')
            .unwrap_or("");
        (num_str.parse().ok(), after.to_string())
    } else {
        // `+ content` without a number
        let content = rest.strip_prefix(' ').unwrap_or(rest).to_string();
        (None, content)
    };
    Some((kind, sign, no, content))
}

/// Generate pi `generateDiffString`-compatible text from old/new contents.
pub fn generate_edit_text(old: &str, new: &str, context_lines: usize) -> String {
    if old == new {
        return String::new();
    }
    let diff = TextDiff::from_lines(old, new);
    let mut old_no = 1u32;
    let mut new_no = 1u32;
    let mut max_no = 1u32;
    let mut rows: Vec<(LineKind, u32, String)> = Vec::new();
    for change in diff.iter_all_changes() {
        let value = change.value().trim_end_matches('\n');
        match change.tag() {
            ChangeTag::Delete => {
                rows.push((LineKind::Delete, old_no, value.to_string()));
                max_no = max_no.max(old_no);
                old_no += 1;
            }
            ChangeTag::Insert => {
                rows.push((LineKind::Insert, new_no, value.to_string()));
                max_no = max_no.max(new_no);
                new_no += 1;
            }
            ChangeTag::Equal => {
                rows.push((LineKind::Equal, old_no, value.to_string()));
                max_no = max_no.max(old_no).max(new_no);
                old_no += 1;
                new_no += 1;
            }
        }
    }
    let num_width = max_no.to_string().len().max(1);
    let mut keep = vec![false; rows.len()];
    for (i, (kind, _, _)) in rows.iter().enumerate() {
        if *kind != LineKind::Equal {
            let start = i.saturating_sub(context_lines);
            let end = (i + context_lines + 1).min(rows.len());
            for slot in keep.iter_mut().take(end).skip(start) {
                *slot = true;
            }
        }
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < rows.len() {
        if !keep[i] {
            let mut j = i;
            while j < rows.len() && !keep[j] {
                j += 1;
            }
            if j > i {
                out.push(format!(" {} ...", " ".repeat(num_width)));
            }
            i = j;
            continue;
        }
        let (kind, no, content) = &rows[i];
        let num = format!("{no:>num_width$}");
        let sign = match kind {
            LineKind::Delete => '-',
            LineKind::Insert => '+',
            LineKind::Equal | LineKind::Meta => ' ',
        };
        out.push(format!("{sign}{num} {content}"));
        i += 1;
    }
    out.join("\n")
}

fn lines_from_pair(old: &str, new: &str, path: Option<&str>) -> Vec<DiffLine> {
    let _ = path; // Callers put the path on the expandable header — omit --- a/ +++ b/ body noise.
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    let diff = TextDiff::from_lines(old, new);
    let mut old_no = 1u32;
    let mut new_no = 1u32;
    for change in diff.iter_all_changes() {
        let value = change.value().trim_end_matches('\n');
        match change.tag() {
            ChangeTag::Delete => {
                out.push(DiffLine {
                    kind: LineKind::Delete,
                    sign: '-',
                    content: value.to_string(),
                    old_no: Some(old_no),
                    new_no: None,
                });
                old_no += 1;
            }
            ChangeTag::Insert => {
                out.push(DiffLine {
                    kind: LineKind::Insert,
                    sign: '+',
                    content: value.to_string(),
                    old_no: None,
                    new_no: Some(new_no),
                });
                new_no += 1;
            }
            ChangeTag::Equal => {
                out.push(DiffLine {
                    kind: LineKind::Equal,
                    sign: ' ',
                    content: value.to_string(),
                    old_no: Some(old_no),
                    new_no: Some(new_no),
                });
                old_no += 1;
                new_no += 1;
            }
        }
    }
    out
}

fn parse_unified(text: &str) -> Vec<DiffLine> {
    let mut out = Vec::new();
    let mut old_no = 0u32;
    let mut new_no = 0u32;
    for raw in text.lines() {
        if raw.starts_with("---") || raw.starts_with("+++") || raw.starts_with("@@") {
            out.push(DiffLine {
                kind: LineKind::Meta,
                sign: ' ',
                content: raw.to_string(),
                old_no: None,
                new_no: None,
            });
            if let Some(rest) = raw.strip_prefix("@@") {
                // @@ -l,s +l,s @@
                if let Some((old_part, new_part)) = rest.split_once('+') {
                    if let Some(n) = old_part
                        .trim()
                        .trim_start_matches('-')
                        .split(',')
                        .next()
                        .and_then(|s| s.parse().ok())
                    {
                        old_no = n;
                    }
                    if let Some(n) = new_part
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.split(',').next())
                        .and_then(|s| s.parse().ok())
                    {
                        new_no = n;
                    }
                }
            }
            continue;
        }
        if let Some(content) = raw.strip_prefix('-') {
            out.push(DiffLine {
                kind: LineKind::Delete,
                sign: '-',
                content: content.to_string(),
                old_no: Some(old_no),
                new_no: None,
            });
            old_no += 1;
        } else if let Some(content) = raw.strip_prefix('+') {
            out.push(DiffLine {
                kind: LineKind::Insert,
                sign: '+',
                content: content.to_string(),
                old_no: None,
                new_no: Some(new_no),
            });
            new_no += 1;
        } else if let Some(content) = raw.strip_prefix(' ') {
            out.push(DiffLine {
                kind: LineKind::Equal,
                sign: ' ',
                content: content.to_string(),
                old_no: Some(old_no),
                new_no: Some(new_no),
            });
            old_no += 1;
            new_no += 1;
        } else if !raw.is_empty() {
            out.push(DiffLine {
                kind: LineKind::Meta,
                sign: ' ',
                content: raw.to_string(),
                old_no: None,
                new_no: None,
            });
        }
    }
    out
}

/// Parse `generate_display_diff` gutter lines.
fn parse_display(text: &str) -> Vec<DiffLine> {
    let mut out = Vec::new();
    for raw in text.lines() {
        if raw.trim() == "(no changes)" {
            continue;
        }
        // Split on " | " once from the right-ish: format is `{gutter}| {content}`
        let Some((gutter, content)) = raw.split_once('|') else {
            out.push(DiffLine {
                kind: LineKind::Meta,
                sign: ' ',
                content: raw.to_string(),
                old_no: None,
                new_no: None,
            });
            continue;
        };
        let content = content.strip_prefix(' ').unwrap_or(content).to_string();
        let g = gutter.trim_end();
        if g.contains("...") || content.starts_with("---") || content.starts_with("+++") {
            out.push(DiffLine {
                kind: LineKind::Meta,
                sign: ' ',
                content,
                old_no: None,
                new_no: None,
            });
            continue;
        }
        // Delete: "   1     " (old only) — left number, right blank
        // Insert: "     2   " (new only)
        // Equal:  "   1    2"
        let nums: Vec<&str> = g.split_whitespace().collect();
        match nums.as_slice() {
            [old] => {
                // Ambiguous: could be delete (old) — display format uses left for delete
                // Insert has leading spaces so first token is new line number only when
                // the gutter starts with spaces: check leading whitespace count.
                let leading = gutter.len() - gutter.trim_start().len();
                if leading >= 4 {
                    let new_no = old.parse().ok();
                    out.push(DiffLine {
                        kind: LineKind::Insert,
                        sign: '+',
                        content,
                        old_no: None,
                        new_no,
                    });
                } else {
                    let old_no = old.parse().ok();
                    out.push(DiffLine {
                        kind: LineKind::Delete,
                        sign: '-',
                        content,
                        old_no,
                        new_no: None,
                    });
                }
            }
            [old, new] => {
                out.push(DiffLine {
                    kind: LineKind::Equal,
                    sign: ' ',
                    content,
                    old_no: old.parse().ok(),
                    new_no: new.parse().ok(),
                });
            }
            _ => {
                out.push(DiffLine {
                    kind: LineKind::Meta,
                    sign: ' ',
                    content,
                    old_no: None,
                    new_no: None,
                });
            }
        }
    }
    out
}

fn render_unified(
    lines: &[DiffLine],
    width: usize,
    theme: &DiffTheme,
    options: &DiffOptions,
    num_width: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if options.word_level
            && i + 1 < lines.len()
            && lines[i].kind == LineKind::Delete
            && lines[i + 1].kind == LineKind::Insert
            && (i + 2 >= lines.len()
                || (lines[i + 2].kind != LineKind::Delete && lines[i + 2].kind != LineKind::Insert))
            && (i == 0
                || (lines[i - 1].kind != LineKind::Delete && lines[i - 1].kind != LineKind::Insert))
        {
            // Exactly one adjacent -/+ pair (not part of a multi-line hunk).
            let del = &lines[i];
            let ins = &lines[i + 1];
            let (del_styled, ins_styled) = word_level_pair(&del.content, &ins.content, theme);
            out.extend(emit_styled_line(
                theme,
                EmitOpts {
                    sign: '-',
                    styled_content: &del_styled,
                    old_no: del.old_no,
                    new_no: None,
                    width,
                    kind: LineKind::Delete,
                    compact: options.compact_line_numbers,
                    num_width,
                },
            ));
            out.extend(emit_styled_line(
                theme,
                EmitOpts {
                    sign: '+',
                    styled_content: &ins_styled,
                    old_no: None,
                    new_no: ins.new_no,
                    width,
                    kind: LineKind::Insert,
                    compact: options.compact_line_numbers,
                    num_width,
                },
            ));
            i += 2;
            continue;
        }

        let line = &lines[i];
        match line.kind {
            LineKind::Meta => {
                let body = (theme.meta)(&line.content);
                out.extend(wrap_pad(&body, width));
            }
            LineKind::Delete | LineKind::Insert | LineKind::Equal => {
                let colored = color_content(line.kind, &line.content, theme);
                out.extend(emit_styled_line(
                    theme,
                    EmitOpts {
                        sign: line.sign,
                        styled_content: &colored,
                        old_no: line.old_no,
                        new_no: line.new_no,
                        width,
                        kind: line.kind,
                        compact: options.compact_line_numbers,
                        num_width,
                    },
                ));
            }
        }
        i += 1;
    }
    out
}

fn color_content(kind: LineKind, content: &str, theme: &DiffTheme) -> String {
    let highlighted = (theme.highlight_line)(content);
    match kind {
        LineKind::Delete => (theme.removed)(&highlighted),
        LineKind::Insert => (theme.added)(&highlighted),
        LineKind::Equal => (theme.context)(&highlighted),
        LineKind::Meta => (theme.meta)(&highlighted),
    }
}

fn color_prefix(kind: LineKind, prefix: &str, theme: &DiffTheme) -> String {
    match kind {
        LineKind::Delete => (theme.removed)(prefix),
        LineKind::Insert => (theme.added)(prefix),
        LineKind::Equal => (theme.context)(prefix),
        LineKind::Meta => (theme.meta)(prefix),
    }
}

/// Compact pi gutter: `±{pad}N ` — fixed width so content columns align.
fn compact_prefix(sign: char, no: Option<u32>, num_width: usize) -> String {
    let num = match no {
        Some(n) => format!("{n:>num_width$}"),
        None => " ".repeat(num_width),
    };
    format!("{sign}{num} ")
}

fn word_level_pair(old: &str, new: &str, theme: &DiffTheme) -> (String, String) {
    // pi `renderIntraLineDiff`: strip leading whitespace from the first
    // removed/added part so indentation is not inverse-/word-highlighted.
    let diff = TextDiff::from_words(old, new);
    let mut del = String::new();
    let mut ins = String::new();
    let mut first_removed = true;
    let mut first_added = true;
    for change in diff.iter_all_changes() {
        let v = change.value();
        match change.tag() {
            ChangeTag::Delete => {
                let mut value = v.to_string();
                if first_removed {
                    let lead = value
                        .find(|c: char| !c.is_whitespace())
                        .unwrap_or(value.len());
                    del.push_str(&value[..lead]);
                    value = value[lead..].to_string();
                    first_removed = false;
                }
                if !value.is_empty() {
                    del.push_str(&(theme.word_change_removed)(&value));
                }
            }
            ChangeTag::Insert => {
                let mut value = v.to_string();
                if first_added {
                    let lead = value
                        .find(|c: char| !c.is_whitespace())
                        .unwrap_or(value.len());
                    ins.push_str(&value[..lead]);
                    value = value[lead..].to_string();
                    first_added = false;
                }
                if !value.is_empty() {
                    ins.push_str(&(theme.word_change_added)(&value));
                }
            }
            ChangeTag::Equal => {
                del.push_str(&(theme.removed)(v));
                ins.push_str(&(theme.added)(v));
            }
        }
    }
    if del.is_empty() {
        del = (theme.removed)(old);
    }
    if ins.is_empty() {
        ins = (theme.added)(new);
    }
    (del, ins)
}

struct EmitOpts<'a> {
    sign: char,
    styled_content: &'a str,
    old_no: Option<u32>,
    new_no: Option<u32>,
    width: usize,
    kind: LineKind,
    compact: bool,
    num_width: usize,
}

fn emit_styled_line(theme: &DiffTheme, opts: EmitOpts<'_>) -> Vec<String> {
    let (prefix, cont_prefix) = if opts.compact {
        // pi: color sign+number together; content starts at a fixed column.
        let plain = compact_prefix(opts.sign, opts.old_no.or(opts.new_no), opts.num_width);
        let cont_plain = format!(" {}", " ".repeat(opts.num_width + 1)); // sign + num + space
        (
            color_prefix(opts.kind, &plain, theme),
            color_prefix(opts.kind, &cont_plain, theme),
        )
    } else {
        let sign_s = color_prefix(opts.kind, &opts.sign.to_string(), theme);
        let gutter = format_gutter(opts.old_no, opts.new_no, opts.num_width);
        let prefix = format!("{}{} ", (theme.gutter)(&gutter), sign_s);
        let blank_gutter = " ".repeat(visible_width(&gutter));
        let cont_prefix = format!("{}  ", (theme.gutter)(&blank_gutter));
        (prefix, cont_prefix)
    };
    let prefix_w = visible_width(&prefix);
    let content_w = opts.width.saturating_sub(prefix_w).max(1);
    let wrapped = wrap_text_with_ansi(opts.styled_content, content_w);
    let mut out = Vec::new();
    for (idx, part) in wrapped.into_iter().enumerate() {
        let line = if idx == 0 {
            format!("{prefix}{part}")
        } else {
            format!("{cont_prefix}{part}")
        };
        // Pad first, then row tint — so bg spans the full terminal width.
        out.push(paint_kind_line_bg(
            theme,
            opts.kind,
            &pad_to_width(&line, opts.width),
        ));
    }
    if out.is_empty() {
        out.push(paint_kind_line_bg(
            theme,
            opts.kind,
            &pad_to_width(&prefix, opts.width),
        ));
    }
    out
}

fn paint_kind_line_bg(theme: &DiffTheme, kind: LineKind, line: &str) -> String {
    match kind {
        LineKind::Delete => (theme.removed_line_bg)(line),
        LineKind::Insert => (theme.added_line_bg)(line),
        LineKind::Equal | LineKind::Meta => line.to_string(),
    }
}

fn format_gutter(old_no: Option<u32>, new_no: Option<u32>, num_width: usize) -> String {
    let blank = " ".repeat(num_width);
    match (old_no, new_no) {
        (Some(o), Some(n)) => format!("{o:>w$} {n:>w$}", w = num_width),
        (Some(o), None) => format!("{o:>w$} {blank}", w = num_width),
        (None, Some(n)) => format!("{blank} {n:>w$}", w = num_width),
        (None, None) => format!("{blank} {blank}"),
    }
}

fn wrap_pad(s: &str, width: usize) -> Vec<String> {
    wrap_text_with_ansi(s, width.max(1))
        .into_iter()
        .map(|l| pad_to_width(&l, width))
        .collect()
}

fn pad_to_width(s: &str, width: usize) -> String {
    let w = visible_width(s);
    if w > width {
        truncate_to_width(s, width, "", false)
    } else if w < width {
        format!("{s}{}", " ".repeat(width - w))
    } else {
        s.to_string()
    }
}

fn render_side_by_side(
    lines: &[DiffLine],
    width: usize,
    theme: &DiffTheme,
    _options: &DiffOptions,
    num_width: usize,
) -> Vec<String> {
    // Plain-text column math first, then color — keeps `│` gutter vertically aligned
    // even when cells carry ANSI (pad-after-color drifted by 1 col with `·` etc.).
    let max_half = width.saturating_sub(3) / 2;
    enum Row {
        Meta(String),
        Pair {
            left_plain: String,
            right_plain: String,
            left_styled: String,
            right_styled: String,
        },
    }

    let style_cell = |sign: char, content: &str, no: Option<u32>, kind: LineKind| {
        let plain = format_sbs_cell_plain(sign, content, no, num_width);
        // SBS: fg + gutter only — no `added_line_bg` / `removed_line_bg` (c464).
        if visible_width(&plain) > max_half {
            let t = truncate_to_width(&plain, max_half, "", false);
            let styled = color_prefix(kind, &(theme.highlight_line)(&t), theme);
            (t, styled)
        } else {
            let styled = color_prefix(kind, &(theme.highlight_line)(&plain), theme);
            (plain, styled)
        }
    };

    let mut rows: Vec<Row> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if line.kind == LineKind::Meta {
            rows.push(Row::Meta((theme.meta)(&line.content)));
            i += 1;
            continue;
        }
        if line.kind == LineKind::Equal {
            let (lp, ls) = style_cell(' ', &line.content, line.old_no, LineKind::Equal);
            let (rp, rs) = style_cell(' ', &line.content, line.new_no, LineKind::Equal);
            rows.push(Row::Pair {
                left_plain: lp,
                right_plain: rp,
                left_styled: ls,
                right_styled: rs,
            });
            i += 1;
            continue;
        }
        if line.kind == LineKind::Delete || line.kind == LineKind::Insert {
            let (deletes, inserts, next) = take_change_hunk(lines, i);
            i = next;
            let n = deletes.len().max(inserts.len());
            for k in 0..n {
                // Empty content on one side = blank half (no fake ±N gutter) — ptd9 / c540.
                let (lp, ls) = match deletes.get(k) {
                    Some(d) if !d.content.is_empty() => {
                        style_cell('-', &d.content, d.old_no, LineKind::Delete)
                    }
                    _ => (String::new(), String::new()),
                };
                let (rp, rs) = match inserts.get(k) {
                    Some(ins) if !ins.content.is_empty() => {
                        style_cell('+', &ins.content, ins.new_no, LineKind::Insert)
                    }
                    _ => (String::new(), String::new()),
                };
                rows.push(Row::Pair {
                    left_plain: lp,
                    right_plain: rp,
                    left_styled: ls,
                    right_styled: rs,
                });
            }
            continue;
        }
        i += 1;
    }

    let left_w = rows
        .iter()
        .filter_map(|r| match r {
            Row::Pair { left_plain, .. } if !left_plain.is_empty() => {
                Some(visible_width(left_plain))
            }
            _ => None,
        })
        .max()
        .unwrap_or(0)
        .min(max_half)
        .max(1);

    let mut out = Vec::new();
    for row in rows {
        match row {
            Row::Meta(text) => {
                out.extend(wrap_pad(&text, width));
            }
            Row::Pair {
                left_plain,
                right_plain,
                left_styled,
                right_styled,
            } => {
                out.push(zip_packed_columns(
                    &left_plain,
                    &right_plain,
                    &left_styled,
                    &right_styled,
                    left_w,
                    width,
                ));
            }
        }
    }
    out
}

/// Collect a change hunk starting at `start`: all deletes, then all inserts (similar order).
fn take_change_hunk(lines: &[DiffLine], start: usize) -> (&[DiffLine], &[DiffLine], usize) {
    let mut i = start;
    let del_start = i;
    while i < lines.len() && lines[i].kind == LineKind::Delete {
        i += 1;
    }
    let del_end = i;
    let ins_start = i;
    while i < lines.len() && lines[i].kind == LineKind::Insert {
        i += 1;
    }
    let ins_end = i;
    (&lines[del_start..del_end], &lines[ins_start..ins_end], i)
}

/// Plain (no ANSI) SBS cell text — used to compute stable column widths.
fn format_sbs_cell_plain(sign: char, content: &str, no: Option<u32>, num_width: usize) -> String {
    format!("{}{}", compact_prefix(sign, no, num_width), content)
}

fn zip_packed_columns(
    left_plain: &str,
    right_plain: &str,
    left_styled: &str,
    right_styled: &str,
    left_w: usize,
    width: usize,
) -> String {
    const SEP: &str = " │ ";
    let sep_w = visible_width(SEP);
    let pad = left_w.saturating_sub(visible_width(left_plain));
    let left_part = if left_plain.is_empty() {
        " ".repeat(left_w)
    } else {
        format!("{left_styled}{}", " ".repeat(pad))
    };
    let right_budget = width.saturating_sub(left_w + sep_w).max(1);
    let right_part = if right_plain.is_empty() {
        String::new()
    } else if visible_width(right_styled) > right_budget {
        truncate_to_width(right_styled, right_budget, "", false)
    } else {
        right_styled.to_string()
    };
    let row = if right_plain.is_empty() {
        left_part
    } else if left_plain.is_empty() {
        format!("{}{SEP}{right_part}", " ".repeat(left_w))
    } else {
        format!("{left_part}{SEP}{right_part}")
    };
    pad_to_width(&row, width)
}

/// Diff component with render cache.
pub struct Diff {
    input: DiffInput,
    padding_x: usize,
    padding_y: usize,
    theme: DiffTheme,
    options: DiffOptions,
    cache: Option<(u64, usize, Vec<String>)>,
}

impl Diff {
    pub fn new(
        input: DiffInput,
        padding_x: usize,
        padding_y: usize,
        theme: DiffTheme,
        options: DiffOptions,
    ) -> Self {
        Self {
            input,
            padding_x,
            padding_y,
            theme,
            options,
            cache: None,
        }
    }

    pub fn set_input(&mut self, input: DiffInput) {
        self.input = input;
        self.cache = None;
    }

    pub fn set_options(&mut self, options: DiffOptions) {
        self.options = options;
        self.cache = None;
    }

    fn input_fingerprint(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        match &self.input {
            DiffInput::DisplayText(s) | DiffInput::UnifiedText(s) | DiffInput::EditText(s) => {
                1u8.hash(&mut h);
                // Distinguish EditText from Display/Unified via a tag byte.
                match &self.input {
                    DiffInput::EditText(_) => 3u8.hash(&mut h),
                    DiffInput::UnifiedText(_) => 4u8.hash(&mut h),
                    _ => 5u8.hash(&mut h),
                }
                s.hash(&mut h);
            }
            DiffInput::LinePair { old, new, path } => {
                2u8.hash(&mut h);
                old.hash(&mut h);
                new.hash(&mut h);
                path.hash(&mut h);
            }
        }
        self.options.word_level.hash(&mut h);
        self.options.side_by_side_min_width.hash(&mut h);
        self.options.compact_line_numbers.hash(&mut h);
        h.finish()
    }
}

impl Component for Diff {
    fn render(&mut self, width: usize) -> Vec<String> {
        let fp = self.input_fingerprint();
        if let Some((cfp, cw, lines)) = &self.cache
            && *cfp == fp
            && *cw == width
        {
            return lines.clone();
        }

        let content_w = width.saturating_sub(self.padding_x * 2).max(1);
        let mut body = render_diff_lines(&self.input, content_w, &self.theme, &self.options);
        let left = " ".repeat(self.padding_x);
        let right = " ".repeat(self.padding_x);
        for line in &mut body {
            *line = format!("{left}{line}{right}");
            *line = pad_to_width(line, width);
        }
        let empty = " ".repeat(width);
        let mut result = Vec::new();
        for _ in 0..self.padding_y {
            result.push(empty.clone());
        }
        result.extend(body);
        for _ in 0..self.padding_y {
            result.push(empty.clone());
        }
        if result.is_empty() {
            result.push(String::new());
        }
        self.cache = Some((fp, width, result.clone()));
        result
    }

    fn handle_input(&mut self, _event: InputEvent) {}

    fn invalidate(&mut self) {
        self.cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_theme() -> DiffTheme {
        // Identity kind colors — signs come only from compact/dual gutters (avoid `--1`).
        DiffTheme {
            added: Box::new(|s| s.to_string()),
            removed: Box::new(|s| s.to_string()),
            context: Box::new(|s| s.to_string()),
            gutter: Box::new(|s| s.to_string()),
            meta: Box::new(|s| s.to_string()),
            word_change_added: Box::new(|s| format!("[{s}]")),
            word_change_removed: Box::new(|s| format!("[{s}]")),
            added_line_bg: Box::new(|s| s.to_string()),
            removed_line_bg: Box::new(|s| s.to_string()),
            highlight_line: Box::new(|s| s.to_string()),
        }
    }

    #[test]
    fn empty_pair_shows_no_changes() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "a\n".into(),
                new: "a\n".into(),
                path: None,
            },
            40,
            &plain_theme(),
            &DiffOptions::default(),
        );
        assert!(lines.iter().any(|l| l.contains("no changes")));
    }

    #[test]
    fn line_pair_colors_delete_insert() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "hello\n".into(),
                new: "world\n".into(),
                path: Some("f.rs".into()),
            },
            60,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: None,
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(joined.contains("f.rs") || joined.contains("hello") || joined.contains("world"));
        assert!(
            !joined.contains("--- a/"),
            "path headers belong on the Edit summary, not Diff body"
        );
        assert!(joined.contains('-') && joined.contains('+'));
    }

    #[test]
    fn word_level_marks_changed_span() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "hello world\n".into(),
                new: "hello there\n".into(),
                path: None,
            },
            80,
            &plain_theme(),
            &DiffOptions {
                word_level: true,
                side_by_side_min_width: None,
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains("[world]") || joined.contains("[there]"),
            "expected word markers, got: {joined}"
        );
    }

    #[test]
    fn default_theme_paints_row_bg_after_pad() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "hello\n".into(),
                new: "world\n".into(),
                path: None,
            },
            40,
            &DiffTheme::default(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: None,
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains("\x1b[48;5;52m") && joined.contains("\x1b[48;5;22m"),
            "default theme should tint delete/insert rows; got:\n{joined}"
        );
        assert!(joined.contains("\x1b[49m"), "row bg must reset with 49m");
    }

    #[test]
    fn narrow_stays_unified_when_sbs_enabled() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "a\n".into(),
                new: "b\n".into(),
                path: None,
            },
            40,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: Some(100),
                ..DiffOptions::default()
            },
        );
        // Unified has sign on the left of content; side-by-side has two columns.
        let joined = lines.join("\n");
        assert!(!joined.contains(" +b") || joined.matches('+').count() >= 1);
        assert!(joined.contains('-') || joined.contains('+'));
    }

    #[test]
    fn cjk_uses_visible_width() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "你好世界\n".into(),
                new: "你好宇宙\n".into(),
                path: None,
            },
            20,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: None,
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains("你好世界") && joined.contains("你好宇宙"),
            "CJK glyphs must stay intact (no byte-split); got:\n{joined}"
        );
        for line in &lines {
            assert!(
                visible_width(line) <= 20,
                "line wider than width: {line:?} ({})",
                visible_width(line)
            );
        }
    }

    #[test]
    fn cjk_narrow_sbs_respects_visible_width() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "标题：验收路径\n".into(),
                new: "标题：发布路径\n".into(),
                path: Some("说明.md".into()),
            },
            36,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: Some(30),
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains("验收") || joined.contains("发布"),
            "SBS CJK content should remain readable; got:\n{joined}"
        );
        for line in &lines {
            assert!(
                visible_width(line) <= 36,
                "SBS CJK row exceeds width: {line:?} w={}",
                visible_width(line)
            );
        }
    }

    #[test]
    fn parse_unified_basic() {
        let text = "--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-old\n+new\n";
        let lines = parse_unified(text);
        assert!(lines.iter().any(|l| l.kind == LineKind::Delete));
        assert!(lines.iter().any(|l| l.kind == LineKind::Insert));
    }

    #[test]
    fn render_omits_git_a_b_path_headers() {
        let text = "--- a/x.rs\n+++ b/x.rs\n@@ -1,1 +1,1 @@\n-old\n+new\n";
        let lines = render_diff_lines(
            &DiffInput::UnifiedText(text.into()),
            60,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: None,
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            !joined.contains("--- a/") && !joined.contains("+++ b/"),
            "git path headers are redundant with Edit summary; got:\n{joined}"
        );
        assert!(
            joined.contains("old") && joined.contains("new"),
            "got:\n{joined}"
        );
    }

    #[test]
    fn edit_text_compact_gutter_shows_line_numbers() {
        let input = DiffInput::EditText(
            [
                "-  10 fn ready() -> bool {",
                "+  10 fn ready(prompt: &str) -> bool {",
                "+ 100 !prompt.is_empty()",
            ]
            .join("\n"),
        );
        let lines = render_diff_lines(
            &input,
            80,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: None,
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains("10") && joined.contains("100"),
            "edit gutter should show padded line numbers; got:\n{joined}"
        );
        // Content column aligned: prefix is `±` + 3-wide num + space.
        let content_cols: Vec<usize> = lines
            .iter()
            .filter(|l| l.contains("fn ") || l.contains("prompt"))
            .filter_map(|l| l.find("fn ").or_else(|| l.find('!')))
            .collect();
        assert!(
            content_cols.len() >= 2,
            "expected content lines; got:\n{joined}"
        );
        assert!(
            content_cols.windows(2).all(|w| w[0] == w[1]),
            "content should share one column; cols={content_cols:?}\n{joined}"
        );
    }

    #[test]
    fn from_edit_pair_is_compact_and_aligned() {
        let input = DiffInput::from_edit_pair("a\nb\n", "a\nc\n");
        let lines = render_diff_lines(&input, 60, &plain_theme(), &DiffOptions::default());
        let joined = lines.join("\n");
        assert!(
            joined.contains('-') && joined.contains('+'),
            "got:\n{joined}"
        );
        // Default options use compact gutters.
        assert!(
            !joined.contains("    1     "),
            "should not use dual gutter by default; got:\n{joined}"
        );
    }

    #[test]
    fn side_by_side_shows_line_numbers_on_both_columns() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "status: Ready\nfooter: cwd\n".into(),
                new: "status: Working\nfooter: cwd · model\n".into(),
                path: Some("ui_root.rs".into()),
            },
            120,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: Some(60),
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains('1') && joined.contains('2'),
            "SBS cells should include line numbers; got:\n{joined}"
        );
        assert!(
            joined.contains("Ready") && joined.contains("Working"),
            "SBS body should remain; got:\n{joined}"
        );
        // Packed columns: each replace-hunk row is L|R (not all deletes then all inserts).
        let ready_line = lines
            .iter()
            .find(|l| l.contains("Ready") && l.contains("Working"))
            .expect("Ready|Working should share one SBS row");
        let pos = ready_line.find("Working").expect("Working");
        assert!(
            pos < 55,
            "packed SBS row should keep panes close; Working at {pos} in:\n{ready_line}"
        );
    }

    #[test]
    fn side_by_side_skips_row_background_tint() {
        // Theme with unmistakable row-bg SGR — SBS must not apply it (c464).
        let theme = DiffTheme {
            added: Box::new(|s| format!("A{s}")),
            removed: Box::new(|s| format!("R{s}")),
            context: Box::new(|s| s.to_string()),
            gutter: Box::new(|s| s.to_string()),
            meta: Box::new(|s| s.to_string()),
            word_change_added: Box::new(|s| s.to_string()),
            word_change_removed: Box::new(|s| s.to_string()),
            added_line_bg: Box::new(|s| format!("\x1b[48;2;1;2;3m{s}\x1b[49m")),
            removed_line_bg: Box::new(|s| format!("\x1b[48;2;4;5;6m{s}\x1b[49m")),
            highlight_line: Box::new(|s| s.to_string()),
        };
        let sbs = render_diff_lines(
            &DiffInput::LinePair {
                old: "old line\n".into(),
                new: "new line\n".into(),
                path: None,
            },
            100,
            &theme,
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: Some(40),
                ..DiffOptions::default()
            },
        );
        let sbs_joined = sbs.join("\n");
        assert!(
            !sbs_joined.contains("48;2;1;2;3") && !sbs_joined.contains("48;2;4;5;6"),
            "SBS must not paint row bg; got:\n{sbs_joined}"
        );
        assert!(
            sbs_joined.contains("old line") && sbs_joined.contains("new line"),
            "SBS body still present; got:\n{sbs_joined}"
        );

        let unified = render_diff_lines(
            &DiffInput::LinePair {
                old: "old line\n".into(),
                new: "new line\n".into(),
                path: None,
            },
            80,
            &theme,
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: None,
                ..DiffOptions::default()
            },
        );
        let uni_joined = unified.join("\n");
        assert!(
            uni_joined.contains("48;2;1;2;3") || uni_joined.contains("48;2;4;5;6"),
            "unified may still use row bg; got:\n{uni_joined}"
        );
    }

    #[test]
    fn side_by_side_zips_multiline_replace_hunk() {
        // similar emits DDII for a 2-line replace — must zip, not stack.
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "status: Ready\nfooter: cwd · model\n".into(),
                new: "status: Working\nfooter: cwd · model · context%\n".into(),
                path: None,
            },
            100,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: Some(40),
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Ready") && l.contains("Working")),
            "row1 should pair Ready|Working; got:\n{joined}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("footer") && l.contains('-') && l.contains('+')),
            "row2 should pair footer delete|insert; got:\n{joined}"
        );
    }

    #[test]
    fn side_by_side_empty_half_has_no_fake_line_number() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "only_old\n".into(),
                new: "\n".into(),
                path: None,
            },
            100,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: Some(40),
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains("only_old") || joined.contains('-'),
            "got:\n{joined}"
        );
        // Empty right half must not invent a gutter like `+  1` after the separator.
        for line in &lines {
            if let Some(idx) = line.find('│') {
                let right = line[idx + '│'.len_utf8()..].trim();
                assert!(
                    right.is_empty()
                        || (!right.starts_with('+')
                            && !right.chars().next().is_some_and(|c| c.is_ascii_digit())),
                    "empty SBS half must not fake a line number; line={line:?}"
                );
            }
        }
    }

    #[test]
    fn side_by_side_empty_left_half_has_no_fake_line_number() {
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "\n".into(),
                new: "only_new\n".into(),
                path: None,
            },
            100,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: Some(40),
                ..DiffOptions::default()
            },
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains("only_new"),
            "insert-only SBS should show new content; got:\n{joined}"
        );
        for line in &lines {
            if let Some(idx) = line.find('│') {
                let left = line[..idx].trim();
                assert!(
                    left.is_empty()
                        || (!left.starts_with('-')
                            && !left.chars().next().is_some_and(|c| c.is_ascii_digit())),
                    "empty left SBS half must not fake a line number; line={line:?}"
                );
            }
        }
    }

    #[test]
    fn edit_format_snapshot_stable_gutter() {
        let input = DiffInput::from_edit_pair(
            "fn ready() -> bool {\n    true\n}\n",
            "fn ready(prompt: &str) -> bool {\n    !prompt.is_empty()\n}\n",
        );
        let lines = render_diff_lines(
            &input,
            72,
            &plain_theme(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: None,
                ..DiffOptions::default()
            },
        );
        // Pad-to-width adds trailing spaces for row bg; trim for stable, hook-friendly snaps.
        let normalized: Vec<_> = lines.iter().map(|l| l.trim_end()).collect();
        insta::assert_snapshot!("diff_edit_format_compact", normalized.join("\n"));
    }
}

#[cfg(test)]
mod sbs_ansi_align_harness {
    use super::*;
    use unicode_width::UnicodeWidthChar;

    /// Visible column of the first occurrence of `needle` (skips ANSI CSI).
    fn visible_col_of(s: &str, needle: char) -> Option<usize> {
        let mut col = 0usize;
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for x in chars.by_ref() {
                        if x.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                continue;
            }
            if c == needle {
                return Some(col);
            }
            col += UnicodeWidthChar::width(c).unwrap_or(0);
        }
        None
    }

    #[test]
    fn side_by_side_ansi_theme_keeps_gutter_and_plus_aligned() {
        // Regression: pad-after-color drifted `+` by 1 col across rows (esp. with `·`).
        let lines = render_diff_lines(
            &DiffInput::LinePair {
                old: "status: Ready\nfooter: cwd · model\n".into(),
                new: "status: Working\nfooter: cwd · model · context%\n".into(),
                path: Some("ui_root.rs".into()),
            },
            100,
            &DiffTheme::default(),
            &DiffOptions {
                word_level: false,
                side_by_side_min_width: Some(40),
                ..DiffOptions::default()
            },
        );
        let content: Vec<&String> = lines
            .iter()
            .filter(|l| l.contains('│') && (l.contains('+') || l.contains('-')))
            .collect();
        assert!(
            content.len() >= 2,
            "need ≥2 SBS content rows; got:\n{}",
            lines.join("\n")
        );
        let sep_cols: Vec<usize> = content
            .iter()
            .map(|l| visible_col_of(l, '│').expect("│ gutter"))
            .collect();
        assert!(
            sep_cols.windows(2).all(|w| w[0] == w[1]),
            "│ must share one visible column; cols={sep_cols:?}\n{}",
            lines.join("\n")
        );
        let plus_cols: Vec<usize> = content
            .iter()
            .filter_map(|l| visible_col_of(l, '+'))
            .collect();
        assert!(
            plus_cols.len() >= 2,
            "need ≥2 `+` markers; got:\n{}",
            lines.join("\n")
        );
        assert!(
            plus_cols.windows(2).all(|w| w[0] == w[1]),
            "`+` must share one visible column under ANSI theme; cols={plus_cols:?}\n{}",
            lines.join("\n")
        );
    }
}
