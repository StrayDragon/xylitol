use xylitol_tui::{bold, dim, fg_rgb, truncate_to_width, visible_width};

use super::GlyphSet;
use crate::layout::LayoutTheme;
use xylitol_plugin_sdk::session::{TodoList, TodoStatus};

pub const TODO_BAR_MAX_ROWS: usize = 6;
const HANGING_INDENT: usize = 4;

pub fn todo_bar_max_rows(term_rows: usize) -> usize {
    TODO_BAR_MAX_ROWS.min((term_rows / 4).max(2))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TodoLineStyle {
    Muted,
    SummaryBody,
    Doing,
    Pending,
    Done,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LayoutSpan {
    text: String,
    style: TodoLineStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LayoutLine {
    text: String,
    spans: Vec<LayoutSpan>,
}

impl LayoutLine {
    fn new(spans: Vec<LayoutSpan>) -> Self {
        let text = spans.iter().map(|s| s.text.as_str()).collect();
        Self { text, spans }
    }

    fn styled(text: String, style: TodoLineStyle) -> Self {
        Self::new(vec![LayoutSpan { text, style }])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TodoBarParams<'a> {
    pub list: &'a TodoList,
    pub expanded: bool,
    pub scroll: usize,
    pub width: usize,
    pub max_rows: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TodoBarHit {
    pub row: usize,
    pub col_start: usize,
    pub col_end: usize,
}

#[derive(Debug, Clone)]
pub struct TodoBarFrame {
    pub lines: Vec<String>,
    pub plain: Vec<String>,
    pub hits: Vec<TodoBarHit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TodoBarLayout {
    lines: Vec<LayoutLine>,
    hits: Vec<TodoBarHit>,
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

fn paint_span(theme: LayoutTheme, span: &LayoutSpan) -> String {
    let palette = theme.palette();
    match span.style {
        TodoLineStyle::Muted => theme.paint_muted(&span.text),
        TodoLineStyle::SummaryBody => fg_rgb(palette.on_surface, &span.text),
        TodoLineStyle::Doing => bold(&fg_rgb(palette.on_surface, &span.text)),
        TodoLineStyle::Pending => theme.paint_muted(&span.text),
        TodoLineStyle::Done => dim(&fg_rgb(palette.tool, &span.text)),
        TodoLineStyle::Hint => dim(&fg_rgb(palette.tool, &span.text)),
    }
}

fn paint_line(theme: LayoutTheme, line: &LayoutLine, width: usize) -> String {
    let mut painted = String::new();
    for span in &line.spans {
        if span.text.is_empty() {
            continue;
        }
        painted.push_str(&paint_span(theme, span));
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

fn summary_line(list: &TodoList, glyphs: GlyphSet, width: usize, expanded: bool) -> LayoutLine {
    let total = list.items.len();
    let done = list
        .items
        .iter()
        .filter(|i| i.status == TodoStatus::Completed)
        .count();
    let lead = format!("{} 待办 {done}/{total}", glyphs.todo_summary(expanded));
    let current = list
        .items
        .iter()
        .find(|i| i.status == TodoStatus::InProgress)
        .map(|i| i.content.as_str());
    let Some(current) = current else {
        return LayoutLine::styled(lead, TodoLineStyle::Muted);
    };
    let budget = width.saturating_sub(visible_width(&lead) + 3).max(1);
    let body = truncate_to_width(current, budget, "…", false);
    LayoutLine::new(vec![
        LayoutSpan {
            text: format!("{lead} · "),
            style: TodoLineStyle::Muted,
        },
        LayoutSpan {
            text: body,
            style: TodoLineStyle::SummaryBody,
        },
    ])
}

fn entry_lines(
    glyphs: GlyphSet,
    status: TodoStatus,
    content: &str,
    width: usize,
) -> Vec<LayoutLine> {
    let (glyph, style) = match status {
        TodoStatus::InProgress => (glyphs.todo_doing(), TodoLineStyle::Doing),
        TodoStatus::Pending => (glyphs.todo_pending(), TodoLineStyle::Pending),
        TodoStatus::Completed => (glyphs.todo_done(), TodoLineStyle::Done),
    };
    let wrap_w = width.saturating_sub(HANGING_INDENT).max(1);
    let wrapped = wrap_words(content, wrap_w);
    let lead = format!("  {glyph} ");
    let hang = " ".repeat(HANGING_INDENT);
    wrapped
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let prefix = if i == 0 { lead.as_str() } else { hang.as_str() };
            LayoutLine::styled(format!("{prefix}{line}"), style)
        })
        .collect()
}

fn grouped_entries(list: &TodoList, glyphs: GlyphSet, width: usize) -> Vec<LayoutLine> {
    let mut out = Vec::new();
    for status in [
        TodoStatus::InProgress,
        TodoStatus::Pending,
        TodoStatus::Completed,
    ] {
        for item in list.items.iter().filter(|i| i.status == status) {
            out.extend(entry_lines(
                glyphs,
                item.status,
                item.content.as_str(),
                width,
            ));
        }
    }
    out
}

fn layout_todo_bar(glyphs: GlyphSet, params: TodoBarParams<'_>) -> TodoBarLayout {
    let TodoBarParams {
        list,
        expanded,
        scroll,
        width,
        max_rows,
    } = params;
    if list.is_empty() {
        return TodoBarLayout {
            lines: Vec::new(),
            hits: Vec::new(),
        };
    }
    let summary = summary_line(list, glyphs, width, expanded);
    let summary_hit = TodoBarHit {
        row: 0,
        col_start: 0,
        col_end: visible_width(&summary.text).max(1),
    };
    if !expanded {
        return TodoBarLayout {
            lines: vec![summary],
            hits: vec![summary_hit],
        };
    }
    let body = grouped_entries(list, glyphs, width);
    let cap = max_rows.max(2);
    let budget = cap.saturating_sub(1);
    let mut lines = vec![summary];
    if body.len() <= budget {
        lines.extend(body);
        return TodoBarLayout {
            lines,
            hits: vec![summary_hit],
        };
    }
    let visible = budget.saturating_sub(1).max(1);
    let max_scroll = body.len().saturating_sub(visible);
    let scroll = scroll.min(max_scroll);
    let remaining = body.len().saturating_sub(visible);
    lines.extend(body.into_iter().skip(scroll).take(visible));
    lines.push(LayoutLine::styled(
        format!("… 还有 {remaining} 项 · 滚轮查看"),
        TodoLineStyle::Hint,
    ));
    TodoBarLayout {
        lines,
        hits: vec![summary_hit],
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
#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::session::TodoItem;
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

    fn params(list: &TodoList, expanded: bool, width: usize, max_rows: usize) -> TodoBarParams<'_> {
        TodoBarParams {
            list,
            expanded,
            scroll: 0,
            width,
            max_rows,
        }
    }

    fn frame(list: &TodoList, expanded: bool, width: usize, max_rows: usize) -> TodoBarFrame {
        render_todo_bar(
            LayoutTheme::product_dark(),
            GlyphSet::Unicode,
            params(list, expanded, width, max_rows),
        )
    }

    fn plain(list: &TodoList, expanded: bool, width: usize, max_rows: usize) -> String {
        frame(list, expanded, width, max_rows)
            .lines
            .into_iter()
            .map(|l| strip_ansi_codes(&l))
            .collect::<Vec<_>>()
            .join("\n")
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
    fn empty_list_occupies_zero_rows() {
        let empty = TodoList::default();
        assert!(frame(&empty, false, 80, 6).lines.is_empty());
        assert!(frame(&empty, true, 80, 6).lines.is_empty());
        assert_eq!(
            todo_bar_line_count(GlyphSet::Unicode, params(&empty, false, 80, 6)),
            0
        );
    }

    #[test]
    fn collapsed_bar_is_a_single_summary_line() {
        let out = plain(&sample(), false, 80, 6);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines.len(), 1, "collapsed bar takes one row: {out}");
        assert!(out.contains("待办 2/5"), "done/total summary: {out}");
        assert!(out.contains("write todo-bar copy"), "current task: {out}");
        assert!(!out.contains('─'), "no outline: {out}");
        assert!(!out.contains("doing"), "no wing headers: {out}");
        assert!(
            !out.contains("[x]") && !out.contains("[ ]"),
            "no checkbox glyphs: {out}"
        );
    }

    #[test]
    fn summary_omits_body_without_in_progress() {
        let list = TodoList::new(vec![item("1", "done thing", TodoStatus::Completed)]);
        let out = plain(&list, false, 80, 6);
        assert!(out.contains("待办 1/1"), "{out}");
        assert!(!out.contains('·'), "no body separator without doing: {out}");
    }

    #[test]
    fn expanded_bar_groups_by_status_in_ssot_order() {
        let list = TodoList::new(vec![
            item("1", "pending first", TodoStatus::Pending),
            item("2", "doing one", TodoStatus::InProgress),
            item("3", "done one", TodoStatus::Completed),
            item("4", "doing two", TodoStatus::InProgress),
        ]);
        let out = plain(&list, true, 80, 6);
        let at = |needle: &str| {
            out.find(needle)
                .unwrap_or_else(|| panic!("missing {needle}: {out}"))
        };
        assert!(
            at("doing one") < at("doing two"),
            "SSOT order within doing: {out}"
        );
        assert!(
            at("doing two") < at("pending first"),
            "doing before pending: {out}"
        );
        assert!(
            at("pending first") < at("done one"),
            "pending before completed: {out}"
        );
    }

    #[test]
    fn expanded_bar_marks_status_with_glyphs() {
        let out = plain(&sample(), true, 80, 8);
        assert!(out.contains("→ write todo-bar copy"), "doing arrow: {out}");
        assert!(out.contains("· paint lab states"), "pending dot: {out}");
        assert!(out.contains("✓ read glossary"), "done check: {out}");
    }

    #[test]
    fn status_colours_follow_three_brightness_steps() {
        let theme = LayoutTheme::product_dark();
        let palette = theme.palette();
        let bar = frame(&sample(), true, 80, 8);
        let doing = bar
            .lines
            .iter()
            .find(|l| l.contains("write todo-bar copy") && l.contains("→"))
            .expect("doing row");
        assert_eq!(
            fg_at(doing, "write todo-bar copy"),
            (
                palette.on_surface.r,
                palette.on_surface.g,
                palette.on_surface.b
            )
        );
        assert!(doing.contains("\x1b[1m"), "doing body is bold: {doing:?}");
        let pending = bar
            .lines
            .iter()
            .find(|l| l.contains("paint lab states") && l.contains("·"))
            .expect("pending row");
        assert_eq!(
            fg_at(pending, "paint lab states"),
            (palette.muted.r, palette.muted.g, palette.muted.b)
        );
        let done = bar
            .lines
            .iter()
            .find(|l| l.contains("read glossary") && l.contains("✓"))
            .expect("done row");
        assert_eq!(
            fg_at(done, "read glossary"),
            (palette.tool.r, palette.tool.g, palette.tool.b)
        );
        assert!(done.contains("\x1b[2m"), "done body is dim: {done:?}");
    }

    #[test]
    fn light_theme_keeps_three_distinct_steps() {
        let theme = LayoutTheme::from_palette(xylitol_tui::Palette::light());
        let palette = theme.palette();
        let bar = render_todo_bar(theme, GlyphSet::Unicode, params(&sample(), true, 80, 8));
        let done = bar
            .lines
            .iter()
            .find(|l| l.contains("read glossary"))
            .expect("done row");
        assert_eq!(
            fg_at(done, "read glossary"),
            (palette.tool.r, palette.tool.g, palette.tool.b)
        );
        let doing = bar
            .lines
            .iter()
            .find(|l| l.contains("write todo-bar copy") && l.contains("→"))
            .expect("doing row");
        assert_eq!(
            fg_at(doing, "write todo-bar copy"),
            (
                palette.on_surface.r,
                palette.on_surface.g,
                palette.on_surface.b
            )
        );
    }

    #[test]
    fn summary_row_is_the_only_click_target() {
        let collapsed = frame(&sample(), false, 80, 6);
        assert_eq!(collapsed.hits.len(), 1);
        assert_eq!(collapsed.hits[0].row, 0);
        assert_eq!(collapsed.hits[0].col_start, 0);
        assert!(collapsed.hits[0].col_end > 0);
        let expanded = frame(&sample(), true, 80, 8);
        assert_eq!(expanded.hits.len(), 1, "no per-wing hit regions");
        assert_eq!(expanded.hits[0].row, 0);
    }

    #[test]
    fn overflow_keeps_row_cap_and_hints_remaining() {
        let mut items = Vec::new();
        items.push(item("0", "doing now", TodoStatus::InProgress));
        for i in 1..7 {
            items.push(item(
                &i.to_string(),
                &format!("pending {i}"),
                TodoStatus::Pending,
            ));
        }
        let list = TodoList::new(items);
        let out = plain(&list, true, 80, 6);
        let lines: Vec<_> = out.lines().collect();
        assert_eq!(lines.len(), 6, "row cap holds: {out}");
        assert!(lines[0].contains("待办 0/7"), "summary stays first: {out}");
        assert!(
            lines[5].contains("还有") && lines[5].contains("项"),
            "hint row: {out}"
        );
        assert!(!out.contains("more"), "no border-style more cue: {out}");
    }

    #[test]
    fn scroll_moves_the_visible_window() {
        let mut items = Vec::new();
        for i in 1..8 {
            items.push(item(
                &i.to_string(),
                &format!("pending {i}"),
                TodoStatus::Pending,
            ));
        }
        let list = TodoList::new(items);
        let mut p = params(&list, true, 80, 6);
        p.scroll = 2;
        let out = layout_todo_bar(GlyphSet::Unicode, p);
        let joined = out
            .lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !joined.contains("pending 1"),
            "scrolled past first entries: {joined}"
        );
        assert!(
            joined.contains("pending 3"),
            "window starts at scroll: {joined}"
        );
    }

    #[test]
    fn long_title_wraps_with_hanging_indent_not_ellipsis() {
        let list = TodoList::new(vec![item(
            "1",
            "Keep the live checklist in the lower dock and wrap long titles not clipping them",
            TodoStatus::InProgress,
        )]);
        let out = plain(&list, true, 40, 12);
        let body: String = out
            .lines()
            .filter(|l| !l.contains("待办"))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            !body.contains('…'),
            "entry body must never ellipsize: {out}"
        );
        assert!(out.contains("Keep the live"), "{out}");
        assert!(out.lines().count() >= 3, "must wrap: {out}");
        assert!(
            out.contains("clipping them") || out.contains("not clipping"),
            "full title must remain: {out}"
        );
    }

    #[test]
    fn cjk_title_wraps_by_display_width_without_clipping() {
        let title = "测".repeat(40);
        let list = TodoList::new(vec![item("1", &title, TodoStatus::InProgress)]);
        let out = plain(&list, true, 20, 20);
        let restored: String = out
            .lines()
            .filter(|l| !l.contains("待办"))
            .flat_map(|l| l.chars())
            .filter(|c| *c == '测')
            .collect();
        assert_eq!(
            restored.chars().count(),
            40,
            "every scalar must survive wrap: {out}"
        );
        assert!(
            out.lines().count() >= 3,
            "CJK must wrap at cell width: {out}"
        );
    }

    #[test]
    fn ascii_glyph_set_falls_back() {
        let bar = render_todo_bar(
            LayoutTheme::product_dark(),
            GlyphSet::Ascii,
            params(&sample(), true, 80, 8),
        );
        let joined = bar
            .lines
            .iter()
            .map(|l| strip_ansi_codes(l))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("> write todo-bar copy"),
            "ascii doing: {joined}"
        );
        assert!(
            joined.contains(". paint lab states"),
            "ascii pending: {joined}"
        );
        assert!(joined.contains("x read glossary"), "ascii done: {joined}");
    }

    #[test]
    fn max_rows_follows_term_height_floor() {
        assert_eq!(todo_bar_max_rows(24), 6);
        assert_eq!(todo_bar_max_rows(16), 4);
        assert_eq!(todo_bar_max_rows(4), 2);
    }
}
