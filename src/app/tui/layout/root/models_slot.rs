//! Models picker slot methods for [`UiRoot`] (c1470).

use xylitol_tui::{SelectItem, fuzzy_filter};

use super::super::models_picker::{ModelPickerRow, PendingModelChoice};
use super::super::slots::EditorSlot;
use super::UiRoot;
use super::empty_widgets::empty_models_list;
use crate::protocol::types::ThinkingLevel;

impl UiRoot {
    /// Mount fuzzy model picker in the editor slot (c630 / c1470 levels).
    pub fn mount_models_picker(&mut self, rows: Vec<ModelPickerRow>) {
        self.models_filter.clear();
        self.models_rows = rows;
        self.models_list = empty_models_list(self.theme);
        self.apply_models_filter();
        self.slot = EditorSlot::Models;
    }

    pub(super) fn apply_models_filter(&mut self) {
        let filter = self.models_filter.as_str();
        let width = self.models_last_width.max(20);
        let focused_id = self
            .models_list
            .get_selected_item()
            .map(|i| i.value.clone());
        let mut rows: Vec<&ModelPickerRow> = self.models_rows.iter().collect();
        if !filter.is_empty() {
            let items: Vec<SelectItem> = self
                .models_rows
                .iter()
                .map(|r| r.to_select_item(false, width))
                .collect();
            let filtered = fuzzy_filter(&items, filter, |item| item.value.as_str());
            let ids: std::collections::HashSet<_> =
                filtered.iter().map(|i| i.value.clone()).collect();
            rows.retain(|r| ids.contains(&r.id));
        }
        let focus = focused_id
            .as_deref()
            .or_else(|| rows.first().map(|r| r.id.as_str()));
        self.models_items = rows
            .iter()
            .map(|r| r.to_select_item(focus == Some(r.id.as_str()), width))
            .collect();
        self.models_list.filtered_items = self.models_items.clone();
        if focused_id
            .as_ref()
            .is_none_or(|id| !self.models_items.iter().any(|i| i.value == *id))
        {
            self.models_list.selected_index = 0;
        } else if let Some(id) = focused_id
            && let Some(idx) = self.models_items.iter().position(|i| i.value == id)
        {
            self.models_list.selected_index = idx;
        }
    }

    pub(super) fn rebuild_models_items_keep_selection(&mut self) {
        let selected = self
            .models_list
            .get_selected_item()
            .map(|i| i.value.clone());
        self.apply_models_filter();
        if let Some(id) = selected
            && let Some(idx) = self.models_items.iter().position(|i| i.value == id)
        {
            self.models_list.selected_index = idx;
        }
    }

    pub(super) fn cycle_focused_model_level(&mut self, forward: bool) {
        let Some(item) = self.models_list.get_selected_item() else {
            return;
        };
        let id = item.value.clone();
        if let Some(row) = self.models_rows.iter_mut().find(|r| r.id == id) {
            row.cycle_provisional(forward);
        }
        self.rebuild_models_items_keep_selection();
    }

    pub(super) fn confirm_models_selection(&mut self) {
        let Some(item) = self.models_list.get_selected_item() else {
            return;
        };
        let id = item.value.clone();
        let thinking = self
            .models_rows
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.provisional)
            .unwrap_or(ThinkingLevel::Off);
        self.pending_model_select = Some(PendingModelChoice {
            model_id: id,
            thinking,
        });
    }

    pub(super) fn models_filter_line(&self) -> String {
        if self.models_filter.is_empty() {
            self.theme.paint_muted(" models")
        } else {
            self.theme
                .paint_muted(&format!(" filter: {}", self.models_filter))
        }
    }
}
