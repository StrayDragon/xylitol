//! `/mcp` SelectList payload for [`super::EditorSlot::Mcp`].

use xylitol_tui::components::select_list::{SelectItem, SelectList, SelectListLayoutOptions};
use xylitol_tui::{Component, InputEvent};

use super::super::theme::LayoutTheme;
use crate::app::core::driver::{LoadedResourcesSnapshot, McpServerPhase};
use crate::app::tui::keybindings::matches_binding;
use crate::app::tui::layout::DEFAULT_MAX_VISIBLE;

pub enum McpAction {
    None,
    Close,
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
            list: SelectList::new(
                items,
                DEFAULT_MAX_VISIBLE,
                theme.select_list_theme(),
                SelectListLayoutOptions {
                    min_primary_column_width: Some(24),
                    max_primary_column_width: Some(72),
                    truncate_primary: None,
                },
            ),
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
        if super::is_select_nav_key(key) {
            self.list.handle_input(event);
        }
        McpAction::None
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(theme.paint_muted(&format!(" MCP · {}", self.summary_line)));
        lines.extend(self.list.render(width.max(1)));
        for diag in &self.diag_lines {
            lines.push(theme.paint_muted(diag));
        }
        lines.push(theme.paint_muted(" Esc · Enter closes"));
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

fn empty_mcp_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        Vec::new(),
        DEFAULT_MAX_VISIBLE,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(24),
            max_primary_column_width: Some(72),
            truncate_primary: None,
        },
    )
}
