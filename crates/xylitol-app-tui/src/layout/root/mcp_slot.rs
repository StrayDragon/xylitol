use super::super::slots::{EditorSlot, McpPromptSlot, McpResourceSlot, McpSlot};
use super::UiRoot;
use crate::driver::LoadedResourcesSnapshot;

impl UiRoot {
    pub fn mcp_open(&self) -> bool {
        matches!(
            &self.slot,
            EditorSlot::Mcp(_) | EditorSlot::McpPrompts(_) | EditorSlot::McpResources(_)
        )
    }

    pub fn mcp_cache_usable_for_open(&self) -> bool {
        mcp_cache_usable(&self.loaded_resources)
    }

    #[cfg(test)]
    pub fn status_next_turn_cue_for_test(&self) -> Option<String> {
        self.status_next_turn_cue.clone()
    }

    pub fn mount_mcp_panel(&mut self, snap: &LoadedResourcesSnapshot) {
        self.slot = EditorSlot::Mcp(McpSlot::mount(self.theme, snap));
    }

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

    pub fn mount_mcp_prompt_picker(
        &mut self,
        entries: Vec<xylitol_plugin_sdk::mcp::McpCatalogEntry>,
    ) {
        self.slot = EditorSlot::McpPrompts(McpPromptSlot::mount(self.theme, entries));
    }

    pub fn mount_mcp_resource_picker(
        &mut self,
        entries: Vec<xylitol_plugin_sdk::mcp::McpCatalogEntry>,
    ) {
        self.slot = EditorSlot::McpResources(McpResourceSlot::mount(self.theme, entries));
    }

    pub fn close_mcp_slot(&mut self) {
        if self.mcp_open() {
            self.close_slot();
        }
    }

    #[cfg(test)]
    pub fn mcp_prompt_panel_text_for_test(&self) -> String {
        match &self.slot {
            EditorSlot::McpPrompts(prompts) => prompts.panel_text(),
            _ => String::new(),
        }
    }

    #[cfg(test)]
    pub fn mcp_resource_panel_text_for_test(&self) -> String {
        match &self.slot {
            EditorSlot::McpResources(resources) => resources.panel_text(),
            _ => String::new(),
        }
    }

    #[cfg(test)]
    pub fn mcp_prompts_open_for_test(&self) -> bool {
        matches!(&self.slot, EditorSlot::McpPrompts(_))
    }

    #[cfg(test)]
    pub fn mcp_resources_open_for_test(&self) -> bool {
        matches!(&self.slot, EditorSlot::McpResources(_))
    }
}

pub(crate) fn mcp_cache_usable(snap: &LoadedResourcesSnapshot) -> bool {
    snap.mcp_configured > 0
        || !snap.mcp_servers.is_empty()
        || snap.mcp_connecting_label.is_some()
        || !snap.mcp_details.is_empty()
}
