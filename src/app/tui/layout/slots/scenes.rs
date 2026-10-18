//! Designing scene catalog payload for [`super::EditorSlot::Scenes`].

use xylitol_tui::components::select_list::{SelectItem, SelectList};
use xylitol_tui::{Component, InputEvent};

use super::super::theme::LayoutTheme;
use crate::app::tui::keybindings::matches_binding;
use crate::app::tui::layout::DEFAULT_MAX_VISIBLE;

pub enum ScenesAction {
    None,
    Select(String),
}

pub struct ScenesSlot {
    list: SelectList,
}

impl ScenesSlot {
    pub fn mount(theme: LayoutTheme) -> Self {
        let items: Vec<SelectItem> = crate::app::debug_fixtures::scene_catalog()
            .into_iter()
            .map(|(id, description)| {
                let label = format!("{id} — {description}");
                SelectItem::new(id, label)
            })
            .collect();
        Self {
            list: theme.select_list(items, DEFAULT_MAX_VISIBLE, (12, 48)),
        }
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.list.max_visible = max_visible;
    }

    pub fn invalidate(&mut self) {
        self.list.invalidate();
    }

    pub fn retheme(&mut self, theme: LayoutTheme) {
        *self = Self::mount(theme);
    }

    pub fn handle_input(&mut self, event: InputEvent) -> ScenesAction {
        let InputEvent::Key(ref key) = event else {
            return ScenesAction::None;
        };
        if matches_binding(key, "tui.select.confirm") {
            if let Some(item) = self.list.get_selected_item() {
                return ScenesAction::Select(item.value.clone());
            }
            return ScenesAction::None;
        }
        if super::is_select_nav_key(key) {
            self.list.handle_input(event);
        }
        ScenesAction::None
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(theme.paint_muted(&xylitol_tui::truncate_to_width(
            " scenes — use /designing to reopen",
            width.max(1),
            "",
            false,
        )));
        lines.extend(self.list.render(width.max(1)));
        lines
    }
}
