use super::super::slots::{EditorSlot, ScenesSlot};
use super::UiRoot;

impl UiRoot {
    pub fn set_designing(&mut self, on: bool) {
        if self.designing == on {
            return;
        }
        self.designing = on;
        self.install_completion_sources();
    }

    pub fn mount_scenes_picker(&mut self) {
        self.slot = EditorSlot::Scenes(ScenesSlot::mount(self.theme));
    }

    pub fn scenes_open(&self) -> bool {
        matches!(&self.slot, EditorSlot::Scenes(_))
    }
}
