use super::super::slots::{CapabilitiesSlot, EditorSlot};
use super::UiRoot;
use xylitol_plugin_sdk::session::CapabilitySnapshot;

impl UiRoot {
    pub fn capabilities_open(&self) -> bool {
        matches!(&self.slot, EditorSlot::Capabilities(_))
    }

    pub fn mount_capabilities_panel(&mut self, snap: &CapabilitySnapshot) {
        self.slot = EditorSlot::Capabilities(CapabilitiesSlot::mount(self.theme, snap));
    }

    pub fn apply_capabilities_snapshot(&mut self, snap: &CapabilitySnapshot) {
        if let EditorSlot::Capabilities(caps) = &mut self.slot {
            caps.apply_snapshot(snap);
        }
    }

    pub fn close_capabilities_slot(&mut self) {
        if self.capabilities_open() {
            self.close_slot();
        }
    }

    pub fn take_pending_capability_toggle(&mut self) -> Option<(String, bool)> {
        self.pending.capability_toggle.take()
    }

    #[cfg(test)]
    pub fn capabilities_panel_text_for_test(&self) -> String {
        match &self.slot {
            EditorSlot::Capabilities(caps) => caps.panel_text(),
            _ => String::new(),
        }
    }

    #[cfg(test)]
    pub fn capabilities_selected_id_for_test(&self) -> Option<String> {
        match &self.slot {
            EditorSlot::Capabilities(caps) => caps.selected_id(),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn capabilities_selected_index_for_test(&self) -> usize {
        match &self.slot {
            EditorSlot::Capabilities(caps) => caps.selected_index(),
            _ => 0,
        }
    }
}
