use xylitol_tui::components::select_list::{SelectItem, SelectList};
use xylitol_tui::{Component, InputEvent, truncate_to_width};

use super::super::theme::LayoutTheme;
use crate::driver::{LoadedResourcesSnapshot, McpServerPhase};
use crate::keybindings::matches_binding;
use crate::layout::DEFAULT_MAX_VISIBLE;

pub enum McpAction {
    None,
    Close,
    Authorize(String),
}

pub struct McpSlot {
    list: SelectList,
    summary_line: String,
    diag_lines: Vec<String>,
}

impl McpSlot {
    pub fn mount(theme: LayoutTheme, snap: &LoadedResourcesSnapshot) -> Self {
        let connected = snap
            .mcp_servers
            .iter()
            .filter(|s| s.phase == McpServerPhase::Connected)
            .count();
        let armed = snap.mcp_servers.iter().filter(|s| s.tools_armed).count();
        let summary_line = format!(
            "configured {} · connected {} · armed {}",
            snap.mcp_configured, connected, armed
        );
        let diag_lines = snap
            .mcp_diag_short
            .iter()
            .map(|d| format!(" diag: {d}"))
            .chain(snap.mcp_details.iter().map(|d| format!(" detail: {d}")))
            .collect();

        let items: Vec<SelectItem> = if snap.mcp_servers.is_empty() {
            vec![SelectItem::new("", "(no MCP servers configured)")]
        } else {
            snap.mcp_servers
                .iter()
                .map(|s| {
                    let phase = match s.phase {
                        McpServerPhase::Connecting => "connecting",
                        McpServerPhase::Connected => "connected",
                        McpServerPhase::Failed => "failed",
                    };
                    let armed = if s.tools_armed { "armed" } else { "not armed" };
                    let label = format!("{}  {}  {}  tools={}", s.id, phase, armed, s.tool_count);
                    SelectItem::new(s.id.clone(), label)
                })
                .collect()
        };

        Self {
            list: theme.select_list(items, DEFAULT_MAX_VISIBLE, (24, 72)),
            summary_line,
            diag_lines,
        }
    }

    pub fn diag_len(&self) -> usize {
        self.diag_lines.len()
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.list.max_visible = max_visible;
    }

    pub fn invalidate(&mut self) {
        self.list.invalidate();
    }

    pub fn retheme(&mut self, theme: LayoutTheme) {
        self.list = empty_mcp_list(theme);
    }

    pub fn handle_input(&mut self, event: InputEvent) -> McpAction {
        let InputEvent::Key(ref key) = event else {
            return McpAction::None;
        };
        if matches_binding(key, "tui.select.confirm") {
            return McpAction::Close;
        }
        if key.code == crossterm::event::KeyCode::Char('a')
            && let Some(item) = self.list.get_selected_item()
        {
            return McpAction::Authorize(item.value.clone());
        }
        if super::is_select_nav_key(key) {
            self.list.handle_input(event);
        }
        McpAction::None
    }

    pub fn select_row_at(&mut self, row: u16) -> Option<&str> {
        let index = self.list.item_index_at_screen_row(row)?;
        self.list.set_selected_index(index);
        self.list
            .filtered_items
            .get(index)
            .map(|i| i.value.as_str())
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let width = width.max(1);
        let mut lines = Vec::new();
        lines.push(truncate_to_width(
            &theme.paint_muted(&format!(" MCP · {}", self.summary_line)),
            width,
            "…",
            false,
        ));
        lines.extend(self.list.render(width));
        for diag in &self.diag_lines {
            lines.push(truncate_to_width(
                &theme.paint_muted(diag),
                width,
                "…",
                false,
            ));
        }
        lines.push(truncate_to_width(
            &theme.paint_muted(" Esc · Enter closes"),
            width,
            "…",
            false,
        ));
        lines
    }

    pub fn panel_text(&self) -> String {
        let mut lines = vec![self.summary_line.clone()];
        for item in &self.list.filtered_items {
            lines.push(format!(" {}", item.label));
        }
        lines.extend(self.diag_lines.iter().cloned());
        lines.push(" Esc · Enter closes".into());
        lines.join("\n")
    }

    pub fn selected_id(&self) -> Option<String> {
        self.list
            .get_selected_item()
            .map(|i| i.value.clone())
            .filter(|v| !v.is_empty())
    }

    pub fn selected_index(&self) -> usize {
        self.list.selected_index
    }
}

pub enum McpPromptAction {
    None,
    Close,
    Select(usize),
}

pub struct McpPromptSlot {
    entries: Vec<xylitol_plugin_sdk::mcp::McpCatalogEntry>,
    list: SelectList,
}

impl McpPromptSlot {
    pub fn mount(
        theme: LayoutTheme,
        entries: Vec<xylitol_plugin_sdk::mcp::McpCatalogEntry>,
    ) -> Self {
        let items: Vec<SelectItem> = if entries.is_empty() {
            vec![SelectItem::new("", "(no MCP prompts)")]
        } else {
            entries
                .iter()
                .map(|e| {
                    let description = e.description.trim();
                    let label = if description.is_empty() {
                        format!("{}  {}", e.server, e.name)
                    } else {
                        format!("{}  {}  {}", e.server, e.name, description)
                    };
                    SelectItem::new(format!("{}\u{0}{}", e.server, e.name), label)
                })
                .collect()
        };
        Self {
            entries,
            list: theme.select_list(items, DEFAULT_MAX_VISIBLE, (24, 72)),
        }
    }

    pub fn entry(&self, index: usize) -> Option<&xylitol_plugin_sdk::mcp::McpCatalogEntry> {
        self.entries.get(index)
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.list.max_visible = max_visible;
    }

    pub fn invalidate(&mut self) {
        self.list.invalidate();
    }

    pub fn retheme(&mut self, theme: LayoutTheme) {
        let entries = self.entries.clone();
        *self = Self::mount(theme, entries);
    }

    pub fn handle_input(&mut self, event: InputEvent) -> McpPromptAction {
        let InputEvent::Key(ref key) = event else {
            return McpPromptAction::None;
        };
        if matches_binding(key, "tui.select.confirm") {
            let index = self.list.selected_index;
            return match self.entries.get(index) {
                Some(_) => McpPromptAction::Select(index),
                None => McpPromptAction::Close,
            };
        }
        if super::is_select_nav_key(key) {
            self.list.handle_input(event);
        }
        McpPromptAction::None
    }

    pub fn select_row_at(&mut self, row: u16) -> Option<usize> {
        let index = self.list.item_index_at_screen_row(row)?;
        self.list.set_selected_index(index);
        self.entries.get(index).map(|_| index)
    }

    pub fn selected_index(&self) -> usize {
        self.list.selected_index
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(theme.paint_muted(&format!(" MCP prompts · {}", self.entries.len())));
        lines.extend(self.list.render(width.max(1)));
        lines.push(theme.paint_muted(" Enter selects · Esc closes"));
        lines
    }

    pub fn panel_text(&self) -> String {
        let mut lines = vec![format!("MCP prompts · {}", self.entries.len())];
        for item in &self.list.filtered_items {
            lines.push(format!(" {}", item.label));
        }
        lines.push(" Enter selects · Esc closes".into());
        lines.join("\n")
    }
}

pub enum McpResourceAction {
    None,
    Close,
    Add(usize),
}

pub struct McpResourceSlot {
    entries: Vec<xylitol_plugin_sdk::mcp::McpCatalogEntry>,
    list: SelectList,
}

impl McpResourceSlot {
    pub fn mount(
        theme: LayoutTheme,
        entries: Vec<xylitol_plugin_sdk::mcp::McpCatalogEntry>,
    ) -> Self {
        let items: Vec<SelectItem> = if entries.is_empty() {
            vec![SelectItem::new("", "(no MCP resources)")]
        } else {
            entries
                .iter()
                .map(|e| {
                    let uri = e.uri.as_deref().unwrap_or("");
                    let name = e.name.trim();
                    let label = if name.is_empty() {
                        format!("{}  {}", e.server, uri)
                    } else {
                        format!("{}  {}  {}", e.server, uri, name)
                    };
                    SelectItem::new(format!("{}\u{0}{}", e.server, uri), label)
                })
                .collect()
        };
        Self {
            entries,
            list: theme.select_list(items, DEFAULT_MAX_VISIBLE, (24, 72)),
        }
    }

    pub fn entry(&self, index: usize) -> Option<&xylitol_plugin_sdk::mcp::McpCatalogEntry> {
        self.entries.get(index)
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.list.max_visible = max_visible;
    }

    pub fn invalidate(&mut self) {
        self.list.invalidate();
    }

    pub fn retheme(&mut self, theme: LayoutTheme) {
        let entries = self.entries.clone();
        *self = Self::mount(theme, entries);
    }

    pub fn handle_input(&mut self, event: InputEvent) -> McpResourceAction {
        let InputEvent::Key(ref key) = event else {
            return McpResourceAction::None;
        };
        if matches_binding(key, "tui.select.confirm") {
            let index = self.list.selected_index;
            return match self.entries.get(index) {
                Some(_) => McpResourceAction::Add(index),
                None => McpResourceAction::Close,
            };
        }
        if super::is_select_nav_key(key) {
            self.list.handle_input(event);
        }
        McpResourceAction::None
    }

    pub fn select_row_at(&mut self, row: u16) -> Option<usize> {
        let index = self.list.item_index_at_screen_row(row)?;
        self.list.set_selected_index(index);
        self.entries.get(index).map(|_| index)
    }

    pub fn selected_index(&self) -> usize {
        self.list.selected_index
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(theme.paint_muted(&format!(
            " MCP resources · {} · selection attaches",
            self.entries.len()
        )));
        lines.extend(self.list.render(width.max(1)));
        lines.push(theme.paint_muted(" Enter attaches · Esc closes"));
        lines
    }

    pub fn panel_text(&self) -> String {
        let mut lines = vec![format!("MCP resources · {}", self.entries.len())];
        for item in &self.list.filtered_items {
            lines.push(format!(" {}", item.label));
        }
        lines.push(" Enter attaches · Esc closes".into());
        lines.join("\n")
    }
}

fn empty_mcp_list(theme: LayoutTheme) -> SelectList {
    theme.select_list(Vec::new(), DEFAULT_MAX_VISIBLE, (24, 72))
}
