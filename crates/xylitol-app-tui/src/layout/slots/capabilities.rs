use xylitol_plugin_sdk::session::{CapabilityEntry, CapabilityKind, CapabilitySnapshot};
use xylitol_tui::components::select_list::{SelectItem, SelectList};
use xylitol_tui::{Component, InputEvent};

use super::super::theme::LayoutTheme;
use crate::keybindings::matches_binding;
use crate::layout::DEFAULT_MAX_VISIBLE;

pub enum CapabilitiesAction {
    None,
    Toggle { id: String, enabled: bool },
    Close,
}

pub struct CapabilitiesSlot {
    theme: LayoutTheme,
    list: SelectList,
    entries: Vec<CapabilityEntry>,
    summary: String,
    pending: bool,
}

impl CapabilitiesSlot {
    pub fn mount(theme: LayoutTheme, snap: &CapabilitySnapshot) -> Self {
        let mut slot = Self {
            theme,
            list: theme.select_list(Vec::new(), DEFAULT_MAX_VISIBLE, (24, 72)),
            entries: Vec::new(),
            summary: String::new(),
            pending: false,
        };
        slot.apply_snapshot(snap);
        slot
    }

    pub fn apply_snapshot(&mut self, snap: &CapabilitySnapshot) {
        self.entries = snap.entries.clone();
        self.pending = snap.pending;
        let enabled = self
            .entries
            .iter()
            .filter(|entry| entry.effective_enabled)
            .count();
        let mut summary = format!("{enabled}/{} enabled", self.entries.len());
        if self.pending {
            summary.push_str(" · pending next turn");
        }
        self.summary = summary;

        let items: Vec<SelectItem> = if self.entries.is_empty() {
            vec![SelectItem::new("", "(no capabilities)")]
        } else {
            self.entries
                .iter()
                .map(|entry| {
                    let kind = match entry.kind {
                        CapabilityKind::McpServer => "mcp",
                        CapabilityKind::Tool => "tool",
                    };
                    let state = if entry.effective_enabled { "on" } else { "off" };
                    let pending = if entry.pending { "  pending" } else { "" };
                    SelectItem::new(
                        entry.id.clone(),
                        format!("{}  {}  {}{}", entry.id, kind, state, pending),
                    )
                })
                .collect()
        };
        self.list = self.theme.select_list(items, DEFAULT_MAX_VISIBLE, (24, 72));
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.list.max_visible = max_visible;
    }

    pub fn invalidate(&mut self) {
        self.list.invalidate();
    }

    pub fn retheme(&mut self, theme: LayoutTheme) {
        self.theme = theme;
        let snap = CapabilitySnapshot {
            entries: self.entries.clone(),
            pending: self.pending,
        };
        self.apply_snapshot(&snap);
    }

    pub fn handle_input(&mut self, event: InputEvent) -> CapabilitiesAction {
        let InputEvent::Key(ref key) = event else {
            return CapabilitiesAction::None;
        };
        if matches_binding(key, "app.interrupt") {
            return CapabilitiesAction::Close;
        }
        if matches_binding(key, "tui.select.confirm") {
            let Some(id) = self.selected_id() else {
                return CapabilitiesAction::None;
            };
            let Some(entry) = self.entries.iter().find(|entry| entry.id == id) else {
                return CapabilitiesAction::None;
            };
            return CapabilitiesAction::Toggle {
                id,
                enabled: !entry.effective_enabled,
            };
        }
        if super::is_select_nav_key(key) {
            self.list.handle_input(event);
        }
        CapabilitiesAction::None
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(theme.paint_muted(&format!(" Capabilities · {}", self.summary)));
        lines.extend(self.list.render(width.max(1)));
        lines.push(theme.paint_muted(" Enter toggle · Esc close"));
        lines
    }

    pub fn panel_text(&self) -> String {
        let mut lines = vec![self.summary.clone()];
        for item in &self.list.filtered_items {
            lines.push(format!(" {}", item.label));
        }
        lines.push(" Enter toggle · Esc close".into());
        lines.join("\n")
    }

    pub fn selected_id(&self) -> Option<String> {
        self.list
            .get_selected_item()
            .map(|item| item.value.clone())
            .filter(|value| !value.is_empty())
    }

    pub fn selected_index(&self) -> usize {
        self.list.selected_index
    }
}
