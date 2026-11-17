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
}
