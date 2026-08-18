//! `/mcp` SelectList slot methods for [`UiRoot`] (c1215).

use super::super::slots::{EditorSlot, McpSlot};
use super::UiRoot;
use crate::app::core::driver::LoadedResourcesSnapshot;

impl UiRoot {
    pub fn mcp_open(&self) -> bool {
        matches!(&self.slot, EditorSlot::Mcp(_))
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
        self.slot = EditorSlot::Mcp(McpSlot::mount(self.theme, snap));
    }

    /// Sync-mount `/mcp` from the UiRoot loaded-resources cache (c1215).
    pub fn mount_mcp_from_loaded_resources(&mut self) {
        let snap = self.loaded_resources.clone();
        self.mount_mcp_panel(&snap);
    }

    #[cfg(test)]
    pub fn mcp_panel_text_for_test(&self) -> String {
        match &self.slot {
            EditorSlot::Mcp(mcp) => mcp.panel_text(),
            _ => String::new(),
        }
    }

    #[cfg(test)]
    pub fn mcp_selected_id_for_test(&self) -> Option<String> {
        match &self.slot {
            EditorSlot::Mcp(mcp) => mcp.selected_id(),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn mcp_selected_index_for_test(&self) -> usize {
        match &self.slot {
            EditorSlot::Mcp(mcp) => mcp.selected_index(),
            _ => 0,
        }
    }
}

/// Cache is usable when it already carries MCP discovery signal (c1215).
pub(crate) fn mcp_cache_usable(snap: &LoadedResourcesSnapshot) -> bool {
    snap.mcp_configured > 0 || !snap.mcp_servers.is_empty() || snap.mcp_connecting_label.is_some()
}
