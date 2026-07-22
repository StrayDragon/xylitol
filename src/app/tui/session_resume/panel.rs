//! Resume session panel (pi SessionSelectorComponent port; c1065).

use std::collections::{HashMap, HashSet};

use crossterm::event::KeyEvent;
use xylitol_tui::{
    Component, Input, InputEvent, matches_key_event, printable_from_key_event, truncate_to_width,
    visible_width, with_keybindings,
};

use crate::app::core::driver::SessionListEntry;
use crate::app::tui::keybindings::matches_binding;
use crate::app::tui::layout::LayoutTheme;
use crate::protocol::ports::format_session_age;

use super::search::{NameFilter, SessionScope, SortMode, filter_and_sort};

/// Visible session rows in the resume list viewport (pi SessionList.maxVisible = 10).
const MAX_VISIBLE_SESSIONS: usize = 10;

/// Max display columns for the preview (name / first_message) column.
/// Remaining horizontal space pads before the full session id column.
const MAX_PREVIEW_COLS: usize = 40;

/// Gap between preview | id | meta columns.
const COL_GAP: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionResumeAction {
    Switch(String),
    Rename { id: String, name: String },
    Delete(String),
    None,
}

pub struct SessionResumePanel {
    all_entries: Vec<SessionListEntry>,
    scope: SessionScope,
    sort: SortMode,
    name_filter: NameFilter,
    show_path: bool,
    selected: usize,
    folded_parents: HashSet<String>,
    rename: Option<(String, Input)>,
    confirming_delete: Option<String>,
    filter: Input,
    loading: bool,
    current_session_id: Option<String>,
    current_cwd: String,
    status_line: Option<String>,
    theme: LayoutTheme,
}

impl SessionResumePanel {
    pub fn new(theme: LayoutTheme) -> Self {
        Self {
            all_entries: Vec::new(),
            scope: SessionScope::Current,
            sort: SortMode::Threaded,
            name_filter: NameFilter::All,
            show_path: false,
            selected: 0,
            folded_parents: HashSet::new(),
            rename: None,
            confirming_delete: None,
            filter: Input::new(),
            loading: false,
            current_session_id: None,
            current_cwd: ".".into(),
            status_line: None,
            theme,
        }
    }

    pub fn set_current_cwd(&mut self, cwd: impl Into<String>) {
        self.current_cwd = cwd.into();
    }

    pub fn load_entries(&mut self, entries: Vec<SessionListEntry>, current_id: Option<String>) {
        self.all_entries = entries;
        self.current_session_id = current_id;
        self.loading = false;
        self.selected = 0;
        self.folded_parents.clear();
        self.rename = None;
        self.confirming_delete = None;
        self.status_line = None;
        self.filter.set_value(String::new());
        self.clamp_selection();
    }

    pub fn set_loading(&mut self, loaded: usize, total: usize) {
        self.loading = true;
        self.all_entries.clear();
        self.status_line = Some(if total == 0 {
            "Loading sessions…".into()
        } else {
            format!("Loading sessions… {loaded}/{total}")
        });
    }

    pub fn apply_rename(&mut self, id: &str, name: &str) {
        if let Some(entry) = self.all_entries.iter_mut().find(|e| e.id == id) {
            entry.name = Some(name.to_string());
        }
    }

    pub fn remove_entry(&mut self, id: &str) {
        self.all_entries.retain(|e| e.id != id);
        self.clamp_selection();
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_line = Some(msg.into());
    }

    pub fn invalidate(&mut self) {
        self.filter.invalidate();
    }

    /// Esc in rename/delete sub-state; returns true when consumed.
    pub fn cancel_substate(&mut self) -> bool {
        if self.rename.take().is_some() {
            return true;
        }
        if self.confirming_delete.take().is_some() {
            return true;
        }
        false
    }

    fn visible_rows(&self) -> Vec<SessionListEntry> {
        if self.loading {
            return Vec::new();
        }
        let rows = filter_and_sort(
            &self.all_entries,
            self.scope,
            &self.current_cwd,
            self.name_filter,
            self.sort,
            self.filter.value(),
        );
        if self.sort != SortMode::Threaded || self.folded_parents.is_empty() {
            return rows;
        }
        let by_id: HashMap<String, SessionListEntry> = self
            .all_entries
            .iter()
            .map(|e| (e.id.clone(), e.clone()))
            .collect();
        rows.into_iter()
            .filter(|e| !is_hidden_by_fold(e, &self.folded_parents, &by_id))
            .collect()
    }

    fn has_children(&self, id: &str) -> bool {
        self.all_entries
            .iter()
            .any(|e| e.parent_session_id.as_deref() == Some(id))
    }

    fn clamp_selection(&mut self) {
        let n = self.visible_rows().len();
        if n == 0 {
            self.selected = 0;
        } else if self.selected >= n {
            self.selected = n - 1;
        }
    }

    fn selected_entry(&self) -> Option<SessionListEntry> {
        self.visible_rows().get(self.selected).cloned()
    }

    fn scope_header_title(&self) -> &'static str {
        match self.scope {
            SessionScope::Current => "Current Folder",
            SessionScope::All => "All Sessions",
        }
    }

    pub fn render(&self, width: usize) -> Vec<String> {
        let w = width.max(1);
        let mut lines = Vec::new();

        if self.loading {
            if let Some(s) = &self.status_line {
                lines.push(self.theme.paint_muted(s));
            }
            return lines;
        }

        let scope_title = self.scope_header_title();
        let header = format!(
            "Resume Session ({scope_title})  {} | {}  Name: {}  Sort: {}",
            if self.scope == SessionScope::Current {
                "◉ Current"
            } else {
                "○ Current"
            },
            if self.scope == SessionScope::All {
                "◉ All"
            } else {
                "○ All"
            },
            self.name_filter.label(),
            self.sort.label(),
        );
        lines.push(
            self.theme
                .paint_muted(&truncate_to_width(&header, w, "…", true)),
        );

        lines.push(self.theme.paint_muted(
            "tab scope · re:<pattern> · \"phrase\" exact · ctrl+s sort · ctrl+n named · ctrl+d delete · ctrl+p path · ctrl+r rename",
        ));
        lines.push(self.theme.paint_muted(&truncate_to_width(
            "> filter… (type to search; re: / \"phrase\" supported)",
            w,
            "…",
            true,
        )));

        let filter_line = if self.filter.value().is_empty() {
            "> ".to_string()
        } else {
            format!("> {}", self.filter.value())
        };
        lines.push(truncate_to_width(&filter_line, w, "…", true));

        if let Some((id, input)) = &self.rename {
            lines.push(
                self.theme
                    .paint_muted(&format!("Rename {id}: (Enter save, Esc cancel)")),
            );
            lines.push(format!("> {}", input.value()));
            return lines;
        }

        if let Some(id) = &self.confirming_delete {
            lines.push(
                self.theme
                    .paint_muted(&format!("Delete session {id}? Enter confirm · Esc cancel")),
            );
        }

        if let Some(s) = &self.status_line {
            lines.push(self.theme.paint_status(s));
        }

        let rows = self.visible_rows();
        if rows.is_empty() {
            lines.push(self.theme.paint_muted(" (no matching sessions)"));
            return lines;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let total = rows.len();
        let max_vis = MAX_VISIBLE_SESSIONS.min(total);
        let start = self
            .selected
            .saturating_sub(max_vis / 2)
            .min(total.saturating_sub(max_vis));
        let end = (start + max_vis).min(total);

        let id_col_w = rows[start..end]
            .iter()
            .map(|e| visible_width(e.id.as_str()))
            .max()
            .unwrap_or(0);

        for (i, entry) in rows.iter().enumerate().take(end).skip(start) {
            // Preview is name / first_message only — id lives in its own column (never
            // duplicate the id as the preview fallback).
            let primary = entry
                .name
                .as_deref()
                .filter(|s| !s.is_empty())
                .or(entry.first_message.as_deref().filter(|s| !s.is_empty()))
                .unwrap_or("—");
            let mut label = format!("{}{primary}", entry.tree_prefix);
            if self.current_session_id.as_deref() == Some(entry.id.as_str()) {
                label.push_str(" *");
            }
            if self.sort == SortMode::Threaded
                && self.has_children(&entry.id)
                && self.folded_parents.contains(&entry.id)
            {
                label = format!("⊞ {label}");
            }
            let age = format_session_age(entry.modified_unix, now);
            let right = format!("{}  {age}", entry.message_count);
            let is_selected = i == self.selected;
            let body = format_session_row_body(&label, entry.id.as_str(), &right, w, id_col_w);

            let line = if is_selected {
                let marked = format!("{}{body}", self.theme.paint_tool_name("› "));
                self.theme.paint_selected_row(&marked, w)
            } else {
                format!("  {body}")
            };
            lines.push(line);

            if self.show_path {
                let path = entry
                    .path
                    .as_deref()
                    .or(entry.cwd.as_deref())
                    .unwrap_or("—");
                let path_line = format!(
                    "  {}",
                    truncate_to_width(&format!("  {path}"), w.saturating_sub(2), "…", true)
                );
                if is_selected {
                    lines.push(
                        self.theme
                            .paint_selected_row(&self.theme.paint_muted(&path_line), w),
                    );
                } else {
                    lines.push(self.theme.paint_muted(&path_line));
                }
            }
        }

        if total > max_vis {
            lines.push(
                self.theme
                    .paint_muted(&format!(" ({}/{})", self.selected + 1, total)),
            );
        }
        lines
    }

    pub fn handle_input(&mut self, event: InputEvent) -> SessionResumeAction {
        if self.loading {
            return SessionResumeAction::None;
        }

        let InputEvent::Key(ref key) = event else {
            return SessionResumeAction::None;
        };

        if let Some((id, input)) = &mut self.rename {
            if matches_binding(key, "tui.select.confirm") {
                let id = id.clone();
                let name = input.value().trim().to_string();
                self.rename = None;
                if name.is_empty() {
                    self.status_line = Some("Name cannot be empty".into());
                    return SessionResumeAction::None;
                }
                return SessionResumeAction::Rename { id, name };
            }
            if matches_binding(key, "app.interrupt") {
                self.rename = None;
                return SessionResumeAction::None;
            }
            input.handle_input(event);
            return SessionResumeAction::None;
        }

        if self.confirming_delete.is_some() {
            if matches_binding(key, "tui.select.confirm") {
                let id = self.confirming_delete.take().unwrap();
                if self.current_session_id.as_deref() == Some(id.as_str()) {
                    self.status_line = Some("Cannot delete the active session".into());
                    return SessionResumeAction::None;
                }
                return SessionResumeAction::Delete(id);
            }
            if matches_binding(key, "app.interrupt") {
                self.confirming_delete = None;
                return SessionResumeAction::None;
            }
            return SessionResumeAction::None;
        }

        if matches_binding(key, "tui.input.tab") {
            self.scope = self.scope.toggle();
            self.clamp_selection();
            return SessionResumeAction::None;
        }
        if matches_binding(key, "app.session.toggleSort") {
            self.sort = self.sort.cycle();
            if self.sort != SortMode::Threaded {
                self.folded_parents.clear();
            }
            self.clamp_selection();
            return SessionResumeAction::None;
        }
        if matches_binding(key, "app.session.toggleNamedFilter") {
            self.name_filter = self.name_filter.toggle();
            self.clamp_selection();
            return SessionResumeAction::None;
        }
        if matches_binding(key, "app.session.togglePath") {
            self.show_path = !self.show_path;
            return SessionResumeAction::None;
        }
        if matches_binding(key, "app.session.rename") {
            if let Some(entry) = self.selected_entry() {
                let mut input = Input::new();
                if let Some(name) = &entry.name {
                    input.set_value(name.clone());
                }
                self.rename = Some((entry.id, input));
            }
            return SessionResumeAction::None;
        }
        if matches_binding(key, "app.session.delete") {
            if let Some(entry) = self.selected_entry() {
                if self.current_session_id.as_deref() == Some(entry.id.as_str()) {
                    self.status_line = Some("Cannot delete the active session".into());
                } else {
                    self.confirming_delete = Some(entry.id);
                }
            }
            return SessionResumeAction::None;
        }

        if self.sort == SortMode::Threaded && matches_tree_fold_up(key) {
            if let Some(entry) = self.selected_entry()
                && self.has_children(&entry.id)
            {
                self.folded_parents.insert(entry.id);
                self.clamp_selection();
            }
            return SessionResumeAction::None;
        }
        if self.sort == SortMode::Threaded && matches_tree_unfold_down(key) {
            if let Some(entry) = self.selected_entry() {
                self.folded_parents.remove(&entry.id);
                self.clamp_selection();
            }
            return SessionResumeAction::None;
        }

        if matches_binding(key, "tui.select.confirm") {
            if let Some(entry) = self.selected_entry() {
                return SessionResumeAction::Switch(entry.id);
            }
            return SessionResumeAction::None;
        }

        if matches_binding(key, "tui.select.up") {
            if self.selected > 0 {
                self.selected -= 1;
            }
            return SessionResumeAction::None;
        }
        if matches_binding(key, "tui.select.down") {
            let n = self.visible_rows().len();
            if n > 0 && self.selected + 1 < n {
                self.selected += 1;
            }
            return SessionResumeAction::None;
        }
        if matches_binding(key, "tui.select.pageUp") {
            self.selected = self.selected.saturating_sub(MAX_VISIBLE_SESSIONS);
            return SessionResumeAction::None;
        }
        if matches_binding(key, "tui.select.pageDown") {
            let n = self.visible_rows().len();
            if n > 0 {
                self.selected = (self.selected + MAX_VISIBLE_SESSIONS).min(n - 1);
            }
            return SessionResumeAction::None;
        }

        if matches_key_event(key, "backspace") {
            let mut v = self.filter.value().to_string();
            v.pop();
            self.filter.set_value(v);
            self.clamp_selection();
            return SessionResumeAction::None;
        }
        if let Some(text) = printable_from_key_event(key) {
            let mut v = self.filter.value().to_string();
            v.push_str(&text);
            self.filter.set_value(v);
            self.clamp_selection();
            return SessionResumeAction::None;
        }

        SessionResumeAction::None
    }
}

/// Build the row body after the `› `/`  ` prefix: capped preview · full id · count/age.
///
/// Session id is **never** truncated. When the terminal is narrower than
/// `prefix + id + meta`, preview shrinks first (possibly to empty); the id column
/// still renders in full even if the line exceeds `width`.
fn format_session_row_body(
    label: &str,
    id: &str,
    right: &str,
    width: usize,
    id_col_w: usize,
) -> String {
    const PREFIX_W: usize = 2;
    let right_w = visible_width(right);
    let id_w = visible_width(id);
    let id_col = id_col_w.max(id_w);
    let fixed = PREFIX_W + COL_GAP + id_col + COL_GAP + right_w;
    let preview_budget = width.saturating_sub(fixed).min(MAX_PREVIEW_COLS);
    let preview = if preview_budget == 0 {
        String::new()
    } else {
        truncate_to_width(label, preview_budget, "…", true)
    };
    let preview_pad = preview_budget.saturating_sub(visible_width(&preview));
    let id_pad = id_col.saturating_sub(id_w);

    let mut body = String::with_capacity(width.saturating_sub(PREFIX_W) + 8);
    body.push_str(&preview);
    body.push_str(&" ".repeat(preview_pad + COL_GAP));
    body.push_str(id);
    body.push_str(&" ".repeat(id_pad + COL_GAP));
    body.push_str(right);

    let content_w = PREFIX_W + visible_width(&body);
    if content_w < width {
        body.push_str(&" ".repeat(width - content_w));
    }
    body
}

fn is_hidden_by_fold(
    entry: &SessionListEntry,
    folded: &HashSet<String>,
    by_id: &HashMap<String, SessionListEntry>,
) -> bool {
    let mut cur = entry.parent_session_id.as_deref();
    while let Some(pid) = cur {
        if folded.contains(pid) {
            return true;
        }
        cur = by_id.get(pid).and_then(|e| e.parent_session_id.as_deref());
    }
    false
}

fn matches_tree_fold_up(key: &KeyEvent) -> bool {
    with_keybindings(|kb| kb.matches_event(key, "tui.tree.foldOrUp"))
}

fn matches_tree_unfold_down(key: &KeyEvent) -> bool {
    with_keybindings(|kb| kb.matches_event(key, "tui.tree.unfoldOrDown"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                continue;
            }
            out.push(c);
        }
        out
    }

    fn entry(id: &str, n: usize) -> SessionListEntry {
        SessionListEntry {
            id: id.into(),
            name: Some(format!("session-{id}")),
            first_message: None,
            message_count: n,
            modified_unix: Some(1_700_000_000 + n as u64),
            parent_session_id: None,
            tree_prefix: String::new(),
            cwd: Some(".".into()),
            path: None,
        }
    }

    #[test]
    fn resume_panel_viewport_caps_rows_and_shows_scroll_info() {
        let mut panel = SessionResumePanel::new(LayoutTheme::product_dark());
        panel.set_current_cwd(".");
        let entries: Vec<_> = (0..30).map(|i| entry(&format!("s{i}"), i)).collect();
        panel.load_entries(entries, None);
        panel.scope = SessionScope::All;

        let text = panel.render(80).join("\n");
        let plain = strip_ansi(&text);
        let body_hits = (0..30)
            .filter(|i| plain.contains(&format!("session-s{i}")))
            .count();
        assert!(
            body_hits <= MAX_VISIBLE_SESSIONS,
            "viewport must cap visible sessions; got {body_hits}:\n{plain}"
        );
        assert!(
            plain.contains("(1/30)") || plain.contains("/30)"),
            "expected scroll indicator: {plain}"
        );
        assert!(
            plain.contains('›') || text.contains('›'),
            "selected row should show › cursor"
        );
        assert!(
            !text.contains("\x1b[7m"),
            "selection must not use reverse video"
        );
    }

    #[test]
    fn resume_panel_selection_moves_viewport_window() {
        let mut panel = SessionResumePanel::new(LayoutTheme::product_dark());
        panel.set_current_cwd(".");
        let entries: Vec<_> = (0..25).map(|i| entry(&format!("s{i}"), i)).collect();
        panel.load_entries(entries, None);
        panel.scope = SessionScope::All;
        panel.selected = 20;
        let plain = strip_ansi(&panel.render(80).join("\n"));
        assert!(
            plain.contains("session-s20"),
            "selected entry must stay in viewport: {plain}"
        );
        assert!(
            plain.contains("(21/25)"),
            "scroll info follows selection: {plain}"
        );
        assert!(
            !plain.contains("session-s0"),
            "early rows should scroll off: {plain}"
        );
    }

    #[test]
    fn resume_row_keeps_full_session_id_and_caps_preview() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let long_title = "请给我一个表格markdown的示例以及更多说明文字用于撑满预览列";
        let body = format_session_row_body(long_title, uuid, "2  9m", 100, visible_width(uuid));
        assert!(
            body.contains(uuid),
            "full session id must appear untruncated: {body}"
        );
        assert!(
            !body.contains(long_title),
            "long preview must be capped: {body}"
        );
        assert!(
            body.contains('…'),
            "capped preview should use ellipsis: {body}"
        );
        let preview_part = body.split(uuid).next().unwrap_or("");
        assert!(
            visible_width(preview_part) <= MAX_PREVIEW_COLS + COL_GAP,
            "preview column budget exceeded: width={} body={body}",
            visible_width(preview_part)
        );
    }

    #[test]
    fn resume_row_never_truncates_id_when_narrow() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        // Too narrow for preview + id + meta; id must still be intact.
        let body = format_session_row_body(
            "very-long-preview-title",
            uuid,
            "99  13h",
            48,
            visible_width(uuid),
        );
        assert!(
            body.contains(uuid),
            "id must remain complete under narrow width: {body}"
        );
        assert!(
            !body.contains("very-long-preview-title"),
            "preview yields first under pressure: {body}"
        );
    }

    #[test]
    fn resume_panel_render_includes_full_ids() {
        let mut panel = SessionResumePanel::new(LayoutTheme::product_dark());
        panel.set_current_cwd(".");
        let uuid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        let mut e = entry(uuid, 3);
        e.name = Some("短标题".into());
        panel.load_entries(vec![e], None);
        panel.scope = SessionScope::All;
        let plain = strip_ansi(&panel.render(100).join("\n"));
        assert!(
            plain.contains(uuid),
            "rendered panel must show full session id: {plain}"
        );
        assert!(
            plain.contains("短标题"),
            "preview title still shown: {plain}"
        );
    }
}
