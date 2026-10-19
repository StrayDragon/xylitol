//! Models picker slot methods for [`UiRoot`] (c1470).

use super::super::models_picker::ModelPickerRow;
use super::super::slots::{EditorSlot, ModelsSlot};
use super::UiRoot;

impl UiRoot {
    /// Mount fuzzy model picker in the editor slot (c630 / c1470 levels).
    pub fn mount_models_picker(&mut self, rows: Vec<ModelPickerRow>) {
        self.slot = EditorSlot::Models(ModelsSlot::mount(self.theme, rows));
    }
}
