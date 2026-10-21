use super::super::models_picker::ModelPickerRow;
use super::super::slots::{EditorSlot, ModelsSlot};
use super::UiRoot;

impl UiRoot {
    pub fn mount_models_picker(&mut self, rows: Vec<ModelPickerRow>) {
        self.slot = EditorSlot::Models(ModelsSlot::mount(self.theme, rows));
    }
}
