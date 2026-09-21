//! Lower-dock 待办栏: three-zone view, wrap (no ellipsis), height cap.

use xylitol_tui::{fg_rgb, overflow_more_border, visible_width};

use super::GlyphSet;
use crate::app::tui::bridge::todo_status_glyph;
use crate::app::tui::layout::LayoutTheme;
use crate::protocol::session::{TodoList, TodoStatus};

/// DESIGN `spacing.diff-side-by-side-min-cols`.
pub const TODO_BAR_BREAKPOINT: usize = 100;
pub const TODO_BAR_MAX_ROWS: usize = 6;
const HANGING_INDENT: usize = 4;
const COL_GUTTER: usize = 2;

pub fn todo_bar_max_rows(term_rows: usize) -> usize {
    TODO_BAR_MAX_ROWS.min((term_rows / 4).max(2))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TodoBarMode {
    Stack,
    Columns,
}

/// One painted run. Wide mode zips three columns onto one row; emphasis stays
/// on the doing cell, so a row is spans rather than one flag for the whole line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LayoutSpan {
    text: String,
    in_progress: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LayoutLine {
    text: String,
    spans: Vec<LayoutSpan>,
}

fn layout_line(text: String, in_progress: bool) -> LayoutLine {
    LayoutLine {
        spans: vec![LayoutSpan {
            text: text.clone(),
            in_progress,
        }],
        text,
    }
}

fn layout_line_spans(spans: Vec<LayoutSpan>) -> LayoutLine {
    let text = spans.iter().map(|span| span.text.as_str()).collect();
    LayoutLine { spans, text }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TodoBarLayout {
    lines: Vec<LayoutLine>,
    #[allow(dead_code)]
    mode: TodoBarMode,
    #[allow(dead_code)]
    overflow: bool,
    hits: Vec<TodoBarHit>,
}

#[derive(Debug, Clone, Copy)]
pub struct TodoBarParams<'a> {
    pub list: &'a TodoList,
    pub doing_open: bool,
    pub past_open: bool,
    pub pending_open: bool,
    pub scroll: usize,
    pub width: usize,
    pub max_rows: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoWing {
    Doing,
    Pending,
    Past,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TodoBarHit {
    pub row: usize,
    pub col_start: usize,
    pub col_end: usize,
    pub wing: TodoWing,
}

#[derive(Debug, Clone)]
pub struct TodoBarFrame {
    pub lines: Vec<String>,
    pub plain: Vec<String>,
    pub hits: Vec<TodoBarHit>,
}

pub fn todo_bar_line_count(glyphs: GlyphSet, params: TodoBarParams<'_>) -> usize {
    layout_todo_bar(glyphs, params).lines.len()
}

pub fn render_todo_bar(
    theme: LayoutTheme,
    glyphs: GlyphSet,
    params: TodoBarParams<'_>,
) -> TodoBarFrame {
    let width = params.width;
    let layout = layout_todo_bar(glyphs, params);
    let plain: Vec<String> = layout.lines.iter().map(|l| l.text.clone()).collect();
    TodoBarFrame {
        lines: layout
            .lines
            .into_iter()
            .map(|line| paint_line(theme, &line, width))
            .collect(),
        plain,
        hits: layout.hits,
    }
}

fn paint_line(theme: LayoutTheme, line: &LayoutLine, width: usize) -> String {
    let mut painted = String::new();
    for span in &line.spans {
        if span.text.is_empty() {
            continue;
        }
        let piece = if span.in_progress {
            fg_rgb(theme.palette().on_surface, &span.text)
        } else {
            theme.paint_muted(&span.text)
        };
        painted.push_str(&piece);
    }
    if width == 0 {
        return painted;
    }
    let w = visible_width(&line.text);
    if w >= width {
        painted
    } else {
        format!("{painted}{}", " ".repeat(width - w))
    }
}

fn layout_todo_bar(glyphs: GlyphSet, params: TodoBarParams<'_>) -> TodoBarLayout {
    let TodoBarParams {
        list,
        doing_open,
        past_open,
        pending_open,
        scroll,
        width,
        max_rows,
    } = params;
    if list.is_empty() {
        return TodoBarLayout {
            lines: Vec::new(),
            mode: TodoBarMode::Stack,
            overflow: false,
            hits: Vec::new(),
        };
    }
    let past: Vec<BarItem<'_>> = list
        .items
        .iter()
        .filter(|i| i.status == TodoStatus::Completed)
        .map(|i| BarItem {
            content: i.content.as_str(),
            status: i.status,
        })
        .collect();
    let doing: Vec<BarItem<'_>> = list
        .items
        .iter()
        .filter(|i| i.status == TodoStatus::InProgress)
        .map(|i| BarItem {
            content: i.content.as_str(),
            status: i.status,
        })
        .collect();
    let pending: Vec<BarItem<'_>> = list
        .items
        .iter()
        .filter(|i| i.status == TodoStatus::Pending)
        .map(|i| BarItem {
            content: i.content.as_str(),
            status: i.status,
        })
        .collect();
    let doing_h = doing_header(doing.len(), doing_open, glyphs);
    let pend_h = pending_header(pending.len(), pending_open, glyphs);
    let past_h = past_header(&past, past_open, glyphs);
    let mw = visible_width(glyphs.fold()).max(1);
    let cols = width.max(1);
    let use_cols = cols >= TODO_BAR_BREAKPOINT;
    let mut hits = Vec::new();
    let mut lines = if !use_cols {
        let mut out = Vec::new();
        push_stack_wing(
            &mut out,
            &mut hits,
            doing_h.as_deref(),
            &doing,
            doing_open,
            TodoWing::Doing,
            cols,
            mw,
        );
        push_stack_wing(
            &mut out,
            &mut hits,
            pend_h.as_deref(),
            &pending,
            pending_open,
            TodoWing::Pending,
            cols,
            mw,
        );
        push_stack_wing(
            &mut out,
            &mut hits,
            past_h.as_deref(),
            &past,
            past_open,
            TodoWing::Past,
            cols,
            mw,
        );
        out
    } else {
        let inner = cols.saturating_sub(COL_GUTTER * 2);
        let widths = column_widths(inner);
        let left = stack_col(doing_h.as_deref(), &doing, doing_open, widths[0]);
        let mid = stack_col(pend_h.as_deref(), &pending, pending_open, widths[1]);
        let right = stack_col(past_h.as_deref(), &past, past_open, widths[2]);
        let starts = [
            0,
            widths[0].saturating_add(COL_GUTTER),
            widths[0]
                .saturating_add(COL_GUTTER)
                .saturating_add(widths[1])
                .saturating_add(COL_GUTTER),
        ];
        for (i, (header, wing)) in [
            (doing_h.is_some(), TodoWing::Doing),
            (pend_h.is_some(), TodoWing::Pending),
            (past_h.is_some(), TodoWing::Past),
        ]
        .into_iter()
        .enumerate()
        {
            if header {
                hits.push(TodoBarHit {
                    row: 0,
                    col_start: starts[i],
                    col_end: starts[i].saturating_add(mw),
                    wing,
                });
            }
        }
        zip_cols([&left, &mid, &right], widths, COL_GUTTER)
    };
    let mode = if use_cols {
        TodoBarMode::Columns
    } else {
        TodoBarMode::Stack
    };
    let total = lines.len();
    let cap = max_rows.max(1);
    let overflow = total > cap;
    let max_scroll = total.saturating_sub(cap);
    let scroll = scroll.min(max_scroll);
    let vis_end = (scroll + cap).min(total);
    if overflow {
        hits.retain(|h| h.row >= scroll && h.row < vis_end);
        for h in &mut hits {
            h.row -= scroll;
        }
        lines = lines[scroll..vis_end].to_vec();
    }
    let more_above = scroll;
    let more_below = total.saturating_sub(vis_end);
    if !lines.is_empty() {
        for h in &mut hits {
            h.row += 1;
        }
        let mut framed = Vec::with_capacity(lines.len() + 2);
        framed.push(layout_line(
            if more_above > 0 {
                overflow_more_border(cols, true, more_above)
            } else {
                plain_border(cols)
            },
            false,
        ));
        framed.extend(lines);
        framed.push(layout_line(
            if more_below > 0 {
                overflow_more_border(cols, false, more_below)
            } else {
                plain_border(cols)
            },
            false,
        ));
        lines = framed;
    }
    TodoBarLayout {
        lines,
        mode,
        overflow,
        hits,
    }
}

#[derive(Clone, Copy)]
struct BarItem<'a> {
    content: &'a str,
    status: TodoStatus,
}

fn doing_header(n: usize, open: bool, glyphs: GlyphSet) -> Option<String> {
    let tri = if open { glyphs.unfold() } else { glyphs.fold() };
    Some(format!("{tri} {n} doing"))
}

fn past_header(items: &[BarItem<'_>], open: bool, glyphs: GlyphSet) -> Option<String> {
    let tri = if open { glyphs.unfold() } else { glyphs.fold() };
    Some(format!("{tri} {} completed", items.len()))
}

fn pending_header(n: usize, open: bool, glyphs: GlyphSet) -> Option<String> {
    let tri = if open { glyphs.unfold() } else { glyphs.fold() };
    Some(format!("{tri} {n} pending"))
}

const MIN_COL: usize = 8;

/// Wide-mode column inner widths (gutters excluded). Order: doing | pending | past.
/// Slots stay equal thirds regardless of fold so chevrons and sibling wrap do not jump.
fn column_widths(inner: usize) -> [usize; 3] {
    let equal = (inner / 3).max(MIN_COL);
    let leftover = inner.saturating_sub(equal.saturating_mul(3));
    [equal, equal, equal.saturating_add(leftover)]
}

#[allow(clippy::too_many_arguments)]
fn push_stack_wing(
    out: &mut Vec<LayoutLine>,
    hits: &mut Vec<TodoBarHit>,
    header: Option<&str>,
    body: &[BarItem<'_>],
    open: bool,
    wing: TodoWing,
    cols: usize,
    mw: usize,
) {
    if header.is_some() {
        hits.push(TodoBarHit {
            row: out.len(),
            col_start: 0,
            col_end: mw,
            wing,
        });
    }
    out.extend(stack_col(header, body, open, cols));
}

fn stack_col(
    header: Option<&str>,
    body: &[BarItem<'_>],
    open: bool,
    inner: usize,
) -> Vec<LayoutLine> {
    let mut rows = Vec::new();
    if let Some(h) = header {
        rows.push(layout_line(h.to_string(), false));
    }
    if open {
        for item in body {
            rows.extend(paint_item(item.content, item.status, inner));
        }
    }
    rows
}

fn paint_item(content: &str, status: TodoStatus, inner: usize) -> Vec<LayoutLine> {
    let in_progress = status == TodoStatus::InProgress;
    if in_progress {
        return wrap_words(content, inner.max(1))
            .into_iter()
            .map(|line| layout_line(line, true))
            .collect();
    }
    let wrap_w = inner.saturating_sub(HANGING_INDENT).max(1);
    let wrapped = wrap_words(content, wrap_w);
    let mark = format!("{} ", todo_status_glyph(status));
    debug_assert_eq!(visible_width(&mark), HANGING_INDENT);
    let hang = " ".repeat(HANGING_INDENT);
    wrapped
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            layout_line(
                format!("{}{line}", if i == 0 { &mark } else { &hang }),
                false,
            )
        })
        .collect()
}

fn zip_cols(cols: [&[LayoutLine]; 3], widths: [usize; 3], gutter: usize) -> Vec<LayoutLine> {
    let height = cols.iter().map(|c| c.len()).max().unwrap_or(0).max(1);
    let gap = " ".repeat(gutter);
    let mut lines = Vec::with_capacity(height);
    for r in 0..height {
        let mut spans = Vec::new();
        for (i, col) in cols.iter().enumerate() {
            if i > 0 {
                spans.push(LayoutSpan {
                    text: gap.clone(),
                    in_progress: false,
                });
            }
            let cell = col.get(r);
            spans.push(LayoutSpan {
                text: pad_visible(cell.map(|c| c.text.as_str()).unwrap_or(""), widths[i]),
                in_progress: cell.is_some_and(|c| c.spans.iter().any(|span| span.in_progress)),
            });
        }
        lines.push(layout_line_spans(spans));
    }
    if cols.iter().all(|c| c.is_empty()) {
        Vec::new()
    } else {
        lines
    }
}

fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let w = width.max(1);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        let trial = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if visible_width(&trial) <= w {
            cur = trial;
            continue;
        }
        if !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
        }
        if visible_width(word) <= w {
            cur = word.to_string();
        } else {
            let (hard, rest) = hard_wrap(word, w);
            lines.extend(hard);
            cur = rest;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

fn hard_wrap(token: &str, w: usize) -> (Vec<String>, String) {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for ch in token.chars() {
        let next = format!("{cur}{ch}");
        if visible_width(&next) <= w {
            cur = next;
            continue;
        }
        if !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
        }
        cur = ch.to_string();
        if visible_width(&cur) > w {
            lines.push(std::mem::take(&mut cur));
        }
    }
    (lines, cur)
}

fn pad_visible(s: &str, n: usize) -> String {
    let w = visible_width(s);
    if w >= n {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(n - w))
    }
}

fn plain_border(width: usize) -> String {
    "─".repeat(width.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::TodoItem;
    use xylitol_tui::utils::strip_ansi_codes;

    fn item(id: &str, content: &str, status: TodoStatus) -> TodoItem {
        TodoItem {
            id: id.into(),
            content: content.into(),
            status,
        }
    }

    fn sample() -> TodoList {
        TodoList::new(vec![
            item("1", "read glossary", TodoStatus::Completed),
            item("2", "skip overlay slot", TodoStatus::Completed),
            item("3", "write todo-bar copy", TodoStatus::InProgress),
            item("4", "paint lab states", TodoStatus::Pending),
            item("5", "graduate to product", TodoStatus::Pending),
        ])
    }

    fn params(
        list: &TodoList,
        past_open: bool,
        pending_open: bool,
        width: usize,
        max_rows: usize,
    ) -> TodoBarParams<'_> {
        params_folds(list, true, past_open, pending_open, width, max_rows)
    }

    fn params_folds(
        list: &TodoList,
        doing_open: bool,
        past_open: bool,
        pending_open: bool,
        width: usize,
        max_rows: usize,
    ) -> TodoBarParams<'_> {
        TodoBarParams {
            list,
            doing_open,
            past_open,
            pending_open,
            scroll: 0,
            width,
            max_rows,
        }
    }

    fn plain(
        list: &TodoList,
        past_open: bool,
        pending_open: bool,
        width: usize,
        max_rows: usize,
    ) -> String {
        let theme = LayoutTheme::product_dark();
        render_todo_bar(
            theme,
            GlyphSet::Unicode,
            params(list, past_open, pending_open, width, max_rows),
        )
        .lines
        .into_iter()
        .map(|l| strip_ansi_codes(&l))
        .collect::<Vec<_>>()
        .join("\n")
    }

    #[test]
    fn empty_list_occupies_zero_rows() {
        assert!(
            render_todo_bar(
                LayoutTheme::product_dark(),
                GlyphSet::Unicode,
                params(&TodoList::default(), false, false, 80, 6),
            )
            .lines
            .is_empty()
        );
        assert_eq!(
            todo_bar_line_count(
                GlyphSet::Unicode,
                params(&TodoList::default(), false, false, 80, 6)
            ),
            0
        );
    }

    #[test]
    fn folded_wings_hide_bodies() {
        let out = plain(&sample(), false, false, 80, 6);
        assert!(out.contains("▾ 1 doing"), "{out}");
        assert!(out.contains("write todo-bar copy"), "{out}");
        assert!(!out.contains("[~]"), "{out}");
        assert!(out.contains("▸ 2 completed"), "{out}");
        assert!(out.contains("▸ 2 pending"), "{out}");
        assert!(!out.contains("[x] read glossary"), "{out}");
        assert!(!out.contains("[ ] paint lab states"), "{out}");
        assert!(!out.contains("Todo ·"), "{out}");
        assert!(!out.contains('…'), "{out}");
        assert!(out.lines().next().is_some_and(|l| l.contains('─')), "{out}");
        assert!(out.lines().last().is_some_and(|l| l.contains('─')), "{out}");
        assert!(!out.contains("more"), "{out}");
    }

    #[test]
    fn long_title_wraps_with_hanging_indent_not_ellipsis() {
        let list = TodoList::new(vec![item(
            "1",
            "Keep the live checklist in the lower dock and wrap long titles not clipping them",
            TodoStatus::InProgress,
        )]);
        let out = plain(&list, false, false, 40, 12);
        assert!(!out.contains('…'), "{out}");
        assert!(out.contains("Keep the live"), "{out}");
        let lines: Vec<_> = out.lines().collect();
        assert!(lines.len() >= 2, "must wrap: {out}");
        assert!(
            out.contains("clipping them") || out.contains("not clipping"),
            "full title must remain: {out}"
        );
    }

    #[test]
    fn cjk_title_wraps_by_display_width_without_clipping() {
        let title = "测".repeat(40);
        let list = TodoList::new(vec![item("1", &title, TodoStatus::InProgress)]);
        let out = plain(&list, false, false, 20, 16);
        assert!(!out.contains('…') && !out.contains("..."), "{out}");
        let restored: String = out.chars().filter(|c| *c == '测').collect();
        assert_eq!(
            restored.chars().count(),
            40,
            "every scalar must survive wrap: {out}"
        );
        let lines: Vec<_> = out.lines().collect();
        assert!(lines.len() >= 2, "CJK must wrap at cell width: {out}");
    }

    #[test]
    fn persisted_overlong_snapshot_still_wraps() {
        let title = "a".repeat(120);
        let list = TodoList::new(vec![item("1", &title, TodoStatus::InProgress)]);
        let out = plain(&list, false, false, 24, 16);
        assert!(!out.contains('…'), "{out}");
        let restored: String = out.chars().filter(|c| *c == 'a').collect();
        assert_eq!(restored.len(), 120, "must not clip resume snapshot: {out}");
    }

    #[test]
    fn wide_terminal_tiles_three_columns() {
        let stacked = layout_todo_bar(GlyphSet::Unicode, params(&sample(), false, false, 80, 6));
        let wide = layout_todo_bar(GlyphSet::Unicode, params(&sample(), false, false, 100, 6));
        assert_eq!(stacked.mode, TodoBarMode::Stack);
        assert_eq!(wide.mode, TodoBarMode::Columns);
        assert!(
            wide.lines.len() < stacked.lines.len(),
            "columns should be shorter: stack={} cols={}",
            stacked.lines.len(),
            wide.lines.len()
        );
    }

    #[test]
    fn folding_keeps_equal_column_slots() {
        let long = "x".repeat(40);
        let list = TodoList::new(vec![
            item("1", "done", TodoStatus::Completed),
            item("2", "now", TodoStatus::InProgress),
            item("3", &long, TodoStatus::Pending),
        ]);
        let folded = layout_todo_bar(GlyphSet::Unicode, params(&list, false, false, 100, 6));
        let open = layout_todo_bar(
            GlyphSet::Unicode,
            params_folds(&list, false, false, true, 100, 6),
        );
        assert_eq!(folded.mode, TodoBarMode::Columns);
        assert_eq!(open.mode, TodoBarMode::Columns);
        let starts = |layout: &TodoBarLayout| -> Vec<(TodoWing, usize)> {
            layout
                .hits
                .iter()
                .filter(|h| h.row == 1)
                .map(|h| (h.wing, h.col_start))
                .collect()
        };
        assert_eq!(
            starts(&folded),
            starts(&open),
            "fold must not move column origins"
        );
        let x_rows =
            |layout: &TodoBarLayout| layout.lines.iter().filter(|l| l.text.contains('x')).count();
        assert_eq!(x_rows(&folded), 0, "folded pending hides body");
        assert!(
            x_rows(&open) >= 2,
            "open pending still wraps at a 1/3 slot, not a stolen sibling width:\n{}",
            open.lines
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn height_cap_marks_overflow() {
        let list = TodoList::new(vec![
            item("1", "read glossary", TodoStatus::Completed),
            item("2", "write todo-bar copy", TodoStatus::InProgress),
            item("3", "paint lab states", TodoStatus::Pending),
            item("4", "graduate to product", TodoStatus::Pending),
            item("5", "wire lower fixed zone", TodoStatus::Pending),
            item("6", "review dock stack", TodoStatus::Pending),
            item("7", "keep transcript visible", TodoStatus::Pending),
        ]);
        let layout = layout_todo_bar(GlyphSet::Unicode, params(&list, true, true, 80, 6));
        assert!(layout.overflow);
        assert_eq!(layout.lines.len(), 8);
        assert!(
            layout
                .lines
                .first()
                .is_some_and(|l| l.text.chars().all(|c| c == '─')),
            "plain top border: {:?}",
            layout.lines.first()
        );
        assert!(
            layout
                .lines
                .last()
                .is_some_and(|l| l.text.contains("↓ 4 more")),
            "{:?}",
            layout.lines.last()
        );
        assert!(
            !layout
                .lines
                .iter()
                .skip(1)
                .take(6)
                .any(|l| l.text.contains('↓') || l.text.contains('…')),
            "content must not be clipped: {:?}",
            layout.lines
        );
    }

    #[test]
    fn scrolled_bar_shows_more_above_and_below() {
        let list = TodoList::new(vec![
            item("1", "read glossary", TodoStatus::Completed),
            item("2", "write todo-bar copy", TodoStatus::InProgress),
            item("3", "paint lab states", TodoStatus::Pending),
            item("4", "graduate to product", TodoStatus::Pending),
            item("5", "wire lower fixed zone", TodoStatus::Pending),
            item("6", "review dock stack", TodoStatus::Pending),
            item("7", "keep transcript visible", TodoStatus::Pending),
        ]);
        let mut p = params(&list, true, true, 80, 6);
        p.scroll = 1;
        let layout = layout_todo_bar(GlyphSet::Unicode, p);
        assert!(
            layout
                .lines
                .first()
                .is_some_and(|l| l.text.contains("↑ 1 more")),
            "{:?}",
            layout.lines.first()
        );
        assert!(
            layout
                .lines
                .last()
                .is_some_and(|l| l.text.contains("↓ 3 more")),
            "{:?}",
            layout.lines.last()
        );
    }

    #[test]
    fn max_rows_follows_term_height_floor() {
        assert_eq!(todo_bar_max_rows(24), 6);
        assert_eq!(todo_bar_max_rows(16), 4);
        assert_eq!(todo_bar_max_rows(4), 2);
    }

    #[test]
    fn multiple_doing_items_keep_ssot_order() {
        let list = TodoList::new(vec![
            item("1", "first doing", TodoStatus::InProgress),
            item("2", "pending later", TodoStatus::Pending),
            item("3", "second doing", TodoStatus::InProgress),
        ]);
        let out = plain(&list, false, false, 80, 6);
        assert!(out.contains("▾ 2 doing"), "{out}");
        let first = out.find("first doing").expect("first doing");
        let second = out.find("second doing").expect("second doing");
        assert!(first < second, "SSOT order in doing column: {out}");
        assert!(!out.contains("[~]"), "{out}");
    }

    #[test]
    fn empty_columns_keep_zero_count_headers() {
        let list = TodoList::new(vec![
            item("1", "one", TodoStatus::Completed),
            item("2", "two", TodoStatus::Completed),
        ]);
        let out = plain(&list, true, true, 80, 6);
        assert!(out.contains("0 doing"), "{out}");
        assert!(out.contains("0 pending"), "{out}");
        assert!(out.contains("2 completed"), "{out}");
        assert!(out.contains("[x] one"), "{out}");
        assert!(!out.contains("[~]"), "{out}");
    }

    fn fg_at(line: &str, needle: &str) -> (u8, u8, u8) {
        let idx = line
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle} in {line}"));
        let prefix = &line[..idx];
        let marker = "\x1b[38;2;";
        let start = prefix
            .rfind(marker)
            .unwrap_or_else(|| panic!("no fg before {needle}"));
        let rest = &prefix[start + marker.len()..];
        let end = rest.find('m').expect("sgr");
        let mut parts = rest[..end].split(';');
        let r = parts.next().unwrap().parse().unwrap();
        let g = parts.next().unwrap().parse().unwrap();
        let b = parts.next().unwrap().parse().unwrap();
        (r, g, b)
    }

    #[test]
    fn wide_row_emphasis_stays_on_doing_cell() {
        let list = TodoList::new(vec![
            item("1", "doing now", TodoStatus::InProgress),
            item("2", "first pending", TodoStatus::Pending),
            item("3", "second pending", TodoStatus::Pending),
            item("4", "first done", TodoStatus::Completed),
            item("5", "second done", TodoStatus::Completed),
        ]);
        let theme = LayoutTheme::product_dark();
        let on = theme.palette().on_surface;
        let muted = theme.palette().muted;
        let frame = render_todo_bar(theme, GlyphSet::Unicode, params(&list, true, true, 120, 8));
        let row = frame
            .lines
            .iter()
            .find(|l| l.contains("doing now"))
            .expect("doing row");
        assert_eq!(fg_at(row, "doing now"), (on.r, on.g, on.b));
        assert_eq!(fg_at(row, "first pending"), (muted.r, muted.g, muted.b));
        assert_eq!(fg_at(row, "first done"), (muted.r, muted.g, muted.b));
        let below = frame
            .lines
            .iter()
            .find(|l| l.contains("second pending"))
            .expect("second row");
        assert_eq!(fg_at(below, "second pending"), (muted.r, muted.g, muted.b));
        assert_eq!(fg_at(below, "second done"), (muted.r, muted.g, muted.b));
        let on_sgr = format!("\x1b[38;2;{};{};{}m", on.r, on.g, on.b);
        assert!(
            !below.contains(&on_sgr),
            "a row with no doing cell must stay muted: {below}"
        );
    }
}
