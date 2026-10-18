//! Preview scene catalog slot methods for [`UiRoot`].

use super::super::slots::{EditorSlot, ScenesSlot};
use super::UiRoot;

impl UiRoot {
    pub fn mount_scenes_picker(&mut self) {
        self.slot = EditorSlot::Scenes(ScenesSlot::mount(self.theme));
    }

    pub fn scenes_open(&self) -> bool {
        matches!(&self.slot, EditorSlot::Scenes(_))
    }

    pub fn take_pending_scene_select(&mut self) -> Option<String> {
        self.pending.scene_select.take()
    }
}
