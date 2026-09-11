//! Fuzzy model picker payload for [`super::EditorSlot::Models`].

use xylitol_tui::components::select_list::{SelectItem, SelectList};
use xylitol_tui::{
    Component, InputEvent, fuzzy_filter, matches_key_event, printable_from_key_event,
};

use super::super::models_picker::{ModelPickerRow, PendingModelChoice};
use super::super::theme::LayoutTheme;
use crate::app::tui::keybindings::matches_binding;
use crate::app::tui::layout::DEFAULT_MAX_VISIBLE;
use crate::protocol::model::THINKING_OFF;

pub enum ModelsAction {
    None,
    Select(PendingModelChoice),
}

pub struct ModelsSlot {
    list: SelectList,
    items: Vec<SelectItem>,
    rows: Vec<ModelPickerRow>,
    filter: String,
    last_width: usize,
    /// Enter-confirmed model id (c2790 quick). The picker stays open while a
    /// bang keeps the main loop stopped; the confirm line renders only while
    /// the focused row still matches, so re-navigation simply drops the badge.
    confirmed_id: Option<String>,
}

impl ModelsSlot {
    pub fn mount(theme: LayoutTheme, rows: Vec<ModelPickerRow>) -> Self {
        let mut slot = Self {
            list: empty_models_list(theme),
            items: Vec::new(),
            rows,
            filter: String::new(),
            last_width: 80,
            confirmed_id: None,
        };
        slot.apply_filter();
        slot
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.list.max_visible = max_visible;
    }

    pub fn invalidate(&mut self) {
        self.list.invalidate();
    }

    pub fn retheme(&mut self, theme: LayoutTheme) {
        self.list = empty_models_list(theme);
        self.apply_filter();
    }

    pub fn apply_filter(&mut self) {
        let filter = self.filter.as_str();
        let width = self.last_width.max(20);
        let focused_id = self.list.get_selected_item().map(|i| i.value.clone());
        let mut rows: Vec<&ModelPickerRow> = self.rows.iter().collect();
        if !filter.is_empty() {
            let items: Vec<SelectItem> = self
                .rows
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
        self.items = rows
            .iter()
            .map(|r| r.to_select_item(focus == Some(r.id.as_str()), width))
            .collect();
        self.list.filtered_items = self.items.clone();
        if focused_id
            .as_ref()
            .is_none_or(|id| !self.items.iter().any(|i| i.value == *id))
        {
            self.list.selected_index = 0;
        } else if let Some(id) = focused_id
            && let Some(idx) = self.items.iter().position(|i| i.value == id)
        {
            self.list.selected_index = idx;
        }
    }

    fn rebuild_keep_selection(&mut self) {
        let selected = self.list.get_selected_item().map(|i| i.value.clone());
        self.apply_filter();
        if let Some(id) = selected
            && let Some(idx) = self.items.iter().position(|i| i.value == id)
        {
            self.list.selected_index = idx;
        }
    }

    fn cycle_focused_level(&mut self, forward: bool) {
        let Some(item) = self.list.get_selected_item() else {
            return;
        };
        let id = item.value.clone();
        if let Some(row) = self.rows.iter_mut().find(|r| r.id == id) {
            row.cycle_provisional(forward);
        }
        self.rebuild_keep_selection();
    }

    fn confirm(&self) -> Option<PendingModelChoice> {
        let item = self.list.get_selected_item()?;
        let id = item.value.clone();
        let thinking = self
            .rows
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.provisional.clone())
            .unwrap_or_else(|| THINKING_OFF.into());
        Some(PendingModelChoice {
            model_id: id,
            thinking,
        })
    }

    pub fn handle_input(&mut self, event: InputEvent) -> ModelsAction {
        let InputEvent::Key(ref key) = event else {
            return ModelsAction::None;
        };
        if matches_binding(key, "tui.select.confirm") {
            if let Some(choice) = self.confirm() {
                self.confirmed_id = Some(choice.model_id.clone());
                return ModelsAction::Select(choice);
            }
            return ModelsAction::None;
        }
        if matches_key_event(key, "left") {
            self.cycle_focused_level(false);
            return ModelsAction::None;
        }
        if matches_key_event(key, "right") || matches_key_event(key, "shift+tab") {
            self.cycle_focused_level(true);
            return ModelsAction::None;
        }
        if super::is_select_nav_key(key) {
            self.list.handle_input(event);
            self.rebuild_keep_selection();
            return ModelsAction::None;
        }
        if matches_key_event(key, "backspace") {
            self.filter.pop();
            self.apply_filter();
            return ModelsAction::None;
        }
        if let Some(text) = printable_from_key_event(key) {
            self.filter.push_str(&text);
            self.apply_filter();
        }
        ModelsAction::None
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let w = width.max(1);
        if self.last_width != w {
            self.last_width = w;
            self.rebuild_keep_selection();
        }
        let mut lines = Vec::new();
        lines.push(self.filter_line(theme, w));
        lines.extend(self.list.render(w));
        lines
    }

    fn filter_line(&self, theme: LayoutTheme, width: usize) -> String {
        let base = if self.filter.is_empty() {
            " models".to_string()
        } else {
            format!(" filter: {}", self.filter)
        };
        // c2790 quick: immediate Enter feedback while a bang keeps the confirm
        // pending — the picker stays open and the focused row is flagged.
        let confirmed = self
            .confirmed_id
            .as_ref()
            .and_then(|id| self.rows.iter().find(|r| &r.id == id))
            .map(|r| format!(" ✓ 已选 {}", r.label));
        let text = match confirmed {
            Some(mark) => format!("{base}{mark}"),
            None => base,
        };
        theme.paint_muted(&xylitol_tui::truncate_to_width(
            &text,
            width.max(1),
            "",
            false,
        ))
    }
}

fn empty_models_list(theme: LayoutTheme) -> SelectList {
    theme.select_list(Vec::new(), DEFAULT_MAX_VISIBLE, (24, 48))
}
