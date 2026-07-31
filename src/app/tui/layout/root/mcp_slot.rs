//! `/mcp` SelectList slot methods for [`UiRoot`] (c1215).

use xylitol_tui::components::select_list::{SelectItem, SelectList, SelectListLayoutOptions};

use super::super::slots::EditorSlot;
use super::UiRoot;
use crate::app::core::driver::{LoadedResourcesSnapshot, McpServerPhase};

impl UiRoot {
    pub fn mcp_open(&self) -> bool {
        self.slot == EditorSlot::Mcp
    }

    /// Whether `/mcp` can sync-mount from the in-root loaded-resources cache (c1215).
    pub fn mcp_cache_usable_for_open(&self) -> bool {
        mcp_cache_usable(&self.loaded_resources)
    }

    #[cfg(test)]
    pub fn status_next_turn_cue_for_test(&self) -> Option<String> {
        self.status_next_turn_cue.clone()
    }

    /// Mount `/mcp` SelectList from a loaded-resources snapshot (c1215).
    pub fn mount_mcp_panel(&mut self, snap: &LoadedResourcesSnapshot) {
        let connected = snap
            .mcp_servers
            .iter()
            .filter(|s| s.phase == McpServerPhase::Connected)
            .count();
        let armed = snap.mcp_servers.iter().filter(|s| s.tools_armed).count();
        self.mcp_summary_line = format!(
            "configured {} · connected {} · armed {}",
            snap.mcp_configured, connected, armed
        );
        self.mcp_diag_lines = snap
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

        self.mcp_list = SelectList::new(
            items,
            10,
            self.theme.select_list_theme(),
            SelectListLayoutOptions {
                min_primary_column_width: Some(24),
                max_primary_column_width: Some(72),
                truncate_primary: None,
            },
        );
        self.slot = EditorSlot::Mcp;
    }

    /// Sync-mount `/mcp` from the UiRoot loaded-resources cache (c1215).
    pub fn mount_mcp_from_loaded_resources(&mut self) {
        let snap = self.loaded_resources.clone();
        self.mount_mcp_panel(&snap);
    }

    #[cfg(test)]
    pub fn mcp_panel_text_for_test(&self) -> String {
        let mut lines = vec![self.mcp_summary_line.clone()];
        for item in &self.mcp_list.filtered_items {
            lines.push(format!(" {}", item.label));
        }
        lines.extend(self.mcp_diag_lines.iter().cloned());
        lines.push(" Esc · Enter closes".into());
        lines.join("\n")
    }

    #[cfg(test)]
    pub fn mcp_selected_id_for_test(&self) -> Option<String> {
        self.mcp_list
            .get_selected_item()
            .map(|i| i.value.clone())
            .filter(|v| !v.is_empty())
    }

    #[cfg(test)]
    pub fn mcp_selected_index_for_test(&self) -> usize {
        self.mcp_list.selected_index
    }
}

/// Cache is usable when it already carries MCP discovery signal (c1215).
pub(crate) fn mcp_cache_usable(snap: &LoadedResourcesSnapshot) -> bool {
    snap.mcp_configured > 0 || !snap.mcp_servers.is_empty() || snap.mcp_connecting_label.is_some()
}
