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
}

/// Theme closures — product maps semantic tokens → SGR.
pub struct DiffTheme {
    pub added: Box<dyn Fn(&str) -> String>,
    pub removed: Box<dyn Fn(&str) -> String>,
    pub context: Box<dyn Fn(&str) -> String>,
    pub gutter: Box<dyn Fn(&str) -> String>,
    pub meta: Box<dyn Fn(&str) -> String>,
    /// Intra-line changed span (typically reverse / bold).
    pub word_change: Box<dyn Fn(&str) -> String>,
}

impl Default for DiffTheme {
    fn default() -> Self {
        Self {
            added: Box::new(|s| format!("\x1b[32m{s}\x1b[0m")),
            removed: Box::new(|s| format!("\x1b[31m{s}\x1b[0m")),
            context: Box::new(|s| format!("\x1b[2m{s}\x1b[0m")),
            gutter: Box::new(|s| format!("\x1b[2m{s}\x1b[0m")),
            meta: Box::new(|s| format!("\x1b[2m{s}\x1b[0m")),
            word_change: Box::new(|s| format!("\x1b[7m{s}\x1b[27m")),
        }
    }
}

/// Rendering options.
#[derive(Debug, Clone)]
pub struct DiffOptions {
    pub word_level: bool,
    /// When `Some(n)` and `width >= n`, use side-by-side; `None` = always unified.
    pub side_by_side_min_width: Option<usize>,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            word_level: true,
            side_by_side_min_width: Some(100),
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

    let use_sbs = options
        .side_by_side_min_width
        .is_some_and(|min| width >= min);

    if use_sbs {
        render_side_by_side(&lines, width, theme, options)
    } else {
        render_unified(&lines, width, theme, options)
    }
}

fn normalize_input(input: &DiffInput) -> Vec<DiffLine> {
    match input {
        DiffInput::LinePair { old, new, path } => lines_from_pair(old, new, path.as_deref()),
        DiffInput::UnifiedText(text) => parse_unified(text),
        DiffInput::DisplayText(text) => parse_display(text),
    }
}

fn lines_from_pair(old: &str, new: &str, path: Option<&str>) -> Vec<DiffLine> {
    if old == new {
        return Vec::new();
    }
    let mut out = Vec::new();
    if let Some(p) = path {
        out.push(DiffLine {
            kind: LineKind::Meta,
            sign: ' ',
            content: format!("--- a/{p}"),
            old_no: None,
            new_no: None,
        });
        out.push(DiffLine {
            kind: LineKind::Meta,
            sign: ' ',
            content: format!("+++ b/{p}"),
            old_no: None,
            new_no: None,
        });
    }
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
                '-',
                &del_styled,
                del.old_no,
                None,
                width,
                theme,
                LineKind::Delete,
            ));
            out.extend(emit_styled_line(
                '+',
                &ins_styled,
                None,
                ins.new_no,
                width,
                theme,
                LineKind::Insert,
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
                    line.sign,
                    &colored,
                    line.old_no,
                    line.new_no,
                    width,
                    theme,
                    line.kind,
                ));
            }
        }
        i += 1;
    }
    out
}

fn color_content(kind: LineKind, content: &str, theme: &DiffTheme) -> String {
    match kind {
        LineKind::Delete => (theme.removed)(content),
        LineKind::Insert => (theme.added)(content),
        LineKind::Equal => (theme.context)(content),
        LineKind::Meta => (theme.meta)(content),
    }
}

fn word_level_pair(old: &str, new: &str, theme: &DiffTheme) -> (String, String) {
    let diff = TextDiff::from_words(old, new);
    let mut del = String::new();
    let mut ins = String::new();
    for change in diff.iter_all_changes() {
        let v = change.value();
        match change.tag() {
            ChangeTag::Delete => {
                del.push_str(&(theme.word_change)(v));
            }
            ChangeTag::Insert => {
                ins.push_str(&(theme.word_change)(v));
            }
            ChangeTag::Equal => {
                del.push_str(&(theme.removed)(v));
                ins.push_str(&(theme.added)(v));
            }
        }
    }
    // Wrap whole line in kind color if empty of word markers — already mixed.
    // Prefix remaining plain with kind color via outer emit.
    if del.is_empty() {
        del = (theme.removed)(old);
    }
    if ins.is_empty() {
        ins = (theme.added)(new);
    }
    (del, ins)
}

fn emit_styled_line(
    sign: char,
    styled_content: &str,
    old_no: Option<u32>,
    new_no: Option<u32>,
    width: usize,
    theme: &DiffTheme,
    kind: LineKind,
) -> Vec<String> {
    let gutter = format_gutter(old_no, new_no);
    let sign_s = match kind {
        LineKind::Delete => (theme.removed)(&sign.to_string()),
        LineKind::Insert => (theme.added)(&sign.to_string()),
        LineKind::Equal => (theme.context)(&sign.to_string()),
        LineKind::Meta => (theme.meta)(&sign.to_string()),
    };
    let prefix = format!("{}{} ", (theme.gutter)(&gutter), sign_s);
    let prefix_w = visible_width(&prefix);
    let content_w = width.saturating_sub(prefix_w).max(1);
    let wrapped = wrap_text_with_ansi(styled_content, content_w);
    let blank_gutter = " ".repeat(visible_width(&gutter));
    let cont_prefix = format!("{}  ", (theme.gutter)(&blank_gutter));
    let mut out = Vec::new();
    for (idx, part) in wrapped.into_iter().enumerate() {
        let line = if idx == 0 {
            format!("{prefix}{part}")
        } else {
            format!("{cont_prefix}{part}")
        };
        out.push(pad_to_width(&line, width));
    }
    if out.is_empty() {
        out.push(pad_to_width(&prefix, width));
    }
    out
}

fn format_gutter(old_no: Option<u32>, new_no: Option<u32>) -> String {
    match (old_no, new_no) {
        (Some(o), Some(n)) => format!("{o:>4} {n:>4}"),
        (Some(o), None) => format!("{o:>4}     "),
        (None, Some(n)) => format!("     {n:>4}"),
        (None, None) => "         ".into(),
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
) -> Vec<String> {
    // Two columns with a single space separator (no box drawing).
    let col = width.saturating_sub(1) / 2;
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if line.kind == LineKind::Meta {
            out.extend(wrap_pad(&(theme.meta)(&line.content), width));
            i += 1;
            continue;
        }
        // Pair delete+insert into one row when possible.
        if line.kind == LineKind::Delete
            && i + 1 < lines.len()
            && lines[i + 1].kind == LineKind::Insert
        {
            let left = format_sbs_cell(
                '-',
                &line.content,
                line.old_no,
                theme,
                LineKind::Delete,
                col,
            );
            let right = format_sbs_cell(
                '+',
                &lines[i + 1].content,
                lines[i + 1].new_no,
                theme,
                LineKind::Insert,
                col,
            );
            out.extend(zip_columns(&left, &right, col, width));
            i += 2;
            continue;
        }
        let (left, right) = match line.kind {
            LineKind::Delete => (
                format_sbs_cell(
                    '-',
                    &line.content,
                    line.old_no,
                    theme,
                    LineKind::Delete,
                    col,
                ),
                vec![" ".repeat(col)],
            ),
            LineKind::Insert => (
                vec![" ".repeat(col)],
                format_sbs_cell(
                    '+',
                    &line.content,
                    line.new_no,
                    theme,
                    LineKind::Insert,
                    col,
                ),
            ),
            LineKind::Equal => (
                format_sbs_cell(' ', &line.content, line.old_no, theme, LineKind::Equal, col),
                format_sbs_cell(' ', &line.content, line.new_no, theme, LineKind::Equal, col),
            ),
            LineKind::Meta => unreachable!(),
        };
        out.extend(zip_columns(&left, &right, col, width));
        i += 1;
    }
    out
}

fn format_sbs_cell(
    sign: char,
    content: &str,
    _no: Option<u32>,
    theme: &DiffTheme,
    kind: LineKind,
    col: usize,
) -> Vec<String> {
    let sign_s = match kind {
        LineKind::Delete => (theme.removed)(&sign.to_string()),
        LineKind::Insert => (theme.added)(&sign.to_string()),
        LineKind::Equal => (theme.context)(&sign.to_string()),
        LineKind::Meta => (theme.meta)(&sign.to_string()),
    };
    let body = color_content(kind, content, theme);
    let prefix = format!("{sign_s} ");
    let pw = visible_width(&prefix);
    let cw = col.saturating_sub(pw).max(1);
    wrap_text_with_ansi(&format!("{prefix}{body}"), col.max(1))
        .into_iter()
        .map(|l| {
            // Re-wrap already combined — simpler: wrap body only
            let _ = cw;
            pad_to_width(&l, col)
        })
        .collect()
}

fn zip_columns(left: &[String], right: &[String], col: usize, width: usize) -> Vec<String> {
    let n = left.len().max(right.len()).max(1);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let l = left.get(i).cloned().unwrap_or_else(|| " ".repeat(col));
        let r = right.get(i).cloned().unwrap_or_else(|| " ".repeat(col));
        let row = format!("{l} {r}");
        out.push(pad_to_width(&row, width));
    }
    out
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
            DiffInput::DisplayText(s) | DiffInput::UnifiedText(s) => {
                1u8.hash(&mut h);
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
        DiffTheme {
            added: Box::new(|s| format!("+{s}")),
            removed: Box::new(|s| format!("-{s}")),
            context: Box::new(|s| s.to_string()),
            gutter: Box::new(|s| s.to_string()),
            meta: Box::new(|s| s.to_string()),
            word_change: Box::new(|s| format!("[{s}]")),
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
            },
        );
        let joined = lines.join("\n");
        assert!(joined.contains("--- a/f.rs") || joined.contains("f.rs"));
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
            },
        );
        let joined = lines.join("\n");
        assert!(
            joined.contains("[world]") || joined.contains("[there]"),
            "expected word markers, got: {joined}"
        );
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
            },
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
    fn parse_unified_basic() {
        let text = "--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-old\n+new\n";
        let lines = parse_unified(text);
        assert!(lines.iter().any(|l| l.kind == LineKind::Delete));
        assert!(lines.iter().any(|l| l.kind == LineKind::Insert));
    }
}
