//! Resume session panel (pi SessionSelectorComponent port; c1065).

use std::collections::{HashMap, HashSet};

use crossterm::event::KeyEvent;
use xylitol_tui::{
    Component, Input, InputEvent, matches_key_event, printable_from_key_event, truncate_to_width,
    visible_width, with_keybindings,
};

use crate::app::core::driver::SessionListEntry;
use crate::app::tui::layout::LayoutTheme;
use crate::runtime_protocol::format_session_age;

use super::search::{NameFilter, SessionScope, SortMode, filter_and_sort};

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

        for (i, entry) in rows.iter().enumerate() {
            let primary = entry
                .name
                .as_deref()
                .filter(|s| !s.is_empty())
                .or(entry.first_message.as_deref().filter(|s| !s.is_empty()))
                .unwrap_or(entry.id.as_str());
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
            let right_w = visible_width(&right);
            let left_w = w.saturating_sub(right_w + 1);
            let mut line = truncate_to_width(&label, left_w.max(1), "…", true);
            let pad = w.saturating_sub(visible_width(&line) + right_w);
            line.push_str(&" ".repeat(pad));
            line.push_str(&right);
            if i == self.selected {
                line = format!("\x1b[7m{line}\x1b[27m");
            }
            lines.push(line);
            if self.show_path {
                let path = entry
                    .path
                    .as_deref()
                    .or(entry.cwd.as_deref())
                    .unwrap_or("—");
                lines.push(self.theme.paint_muted(&truncate_to_width(
                    &format!("   {path}"),
                    w,
                    "…",
                    true,
                )));
            }
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
            if matches_key_event(key, "enter") {
                let id = id.clone();
                let name = input.value().trim().to_string();
                self.rename = None;
                if name.is_empty() {
                    self.status_line = Some("Name cannot be empty".into());
                    return SessionResumeAction::None;
                }
                return SessionResumeAction::Rename { id, name };
            }
            if matches_key_event(key, "escape") {
                self.rename = None;
                return SessionResumeAction::None;
            }
            input.handle_input(event);
            return SessionResumeAction::None;
        }

        if self.confirming_delete.is_some() {
            if matches_key_event(key, "enter") {
                let id = self.confirming_delete.take().unwrap();
                if self.current_session_id.as_deref() == Some(id.as_str()) {
                    self.status_line = Some("Cannot delete the active session".into());
                    return SessionResumeAction::None;
                }
                return SessionResumeAction::Delete(id);
            }
            if matches_key_event(key, "escape") {
                self.confirming_delete = None;
                return SessionResumeAction::None;
            }
            return SessionResumeAction::None;
        }

        if matches_key_event(key, "tab") {
            self.scope = self.scope.toggle();
            self.clamp_selection();
            return SessionResumeAction::None;
        }
        if matches_key_event(key, "ctrl+s") {
            self.sort = self.sort.cycle();
            if self.sort != SortMode::Threaded {
                self.folded_parents.clear();
            }
            self.clamp_selection();
            return SessionResumeAction::None;
        }
        if matches_key_event(key, "ctrl+n") {
            self.name_filter = self.name_filter.toggle();
            self.clamp_selection();
            return SessionResumeAction::None;
        }
        if matches_key_event(key, "ctrl+p") {
            self.show_path = !self.show_path;
            return SessionResumeAction::None;
        }
        if matches_key_event(key, "ctrl+r") {
            if let Some(entry) = self.selected_entry() {
                let mut input = Input::new();
                if let Some(name) = &entry.name {
                    input.set_value(name.clone());
                }
                self.rename = Some((entry.id, input));
            }
            return SessionResumeAction::None;
        }
        if matches_key_event(key, "ctrl+d") {
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

        if matches_key_event(key, "enter") {
            if let Some(entry) = self.selected_entry() {
                return SessionResumeAction::Switch(entry.id);
            }
            return SessionResumeAction::None;
        }

        if matches_key_event(key, "up") {
            if self.selected > 0 {
                self.selected -= 1;
            }
            return SessionResumeAction::None;
        }
        if matches_key_event(key, "down") {
            let n = self.visible_rows().len();
            if n > 0 && self.selected + 1 < n {
                self.selected += 1;
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
