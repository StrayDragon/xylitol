//! Built-in theme picker payload for [`super::EditorSlot::Themes`].

use xylitol_tui::components::select_list::{SelectItem, SelectList, SelectListLayoutOptions};
use xylitol_tui::{Component, InputEvent};

use super::super::theme::LayoutTheme;
use crate::app::tui::keybindings::matches_binding;
use crate::app::tui::layout::DEFAULT_MAX_VISIBLE;

pub enum ThemesAction {
    None,
    Select(String),
}

pub struct ThemesSlot {
    list: SelectList,
}

impl ThemesSlot {
    pub fn mount(theme: LayoutTheme, current: Option<&str>) -> Self {
        let current = current.unwrap_or("dark");
        let items: Vec<SelectItem> = ["dark", "light"]
            .into_iter()
            .map(|name| {
                let label = if current.eq_ignore_ascii_case(name) {
                    format!("{name} *")
                } else {
                    name.to_string()
                };
                SelectItem::new(name, label)
            })
            .collect();
        Self {
            list: SelectList::new(
                items,
                4,
                theme.select_list_theme(),
                SelectListLayoutOptions {
                    min_primary_column_width: Some(12),
                    max_primary_column_width: Some(24),
                    truncate_primary: None,
                },
            ),
        }
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.list.max_visible = max_visible;
    }

    pub fn invalidate(&mut self) {
        self.list.invalidate();
    }

    pub fn retheme(&mut self, theme: LayoutTheme) {
        self.list = empty_themes_list(theme);
    }

    pub fn handle_input(&mut self, event: InputEvent) -> ThemesAction {
        let InputEvent::Key(ref key) = event else {
            return ThemesAction::None;
        };
        if matches_binding(key, "tui.select.confirm") {
            if let Some(item) = self.list.get_selected_item() {
                return ThemesAction::Select(item.value.clone());
            }
            return ThemesAction::None;
        }
        if matches_binding(key, "tui.select.up")
            || matches_binding(key, "tui.select.down")
            || matches_binding(key, "tui.select.pageUp")
            || matches_binding(key, "tui.select.pageDown")
        {
            self.list.handle_input(event);
        }
        ThemesAction::None
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(theme.paint_muted(" themes"));
        lines.extend(self.list.render(width.max(1)));
        lines
    }
}

fn empty_themes_list(theme: LayoutTheme) -> SelectList {
    SelectList::new(
        Vec::new(),
        DEFAULT_MAX_VISIBLE,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(12),
            max_primary_column_width: Some(24),
            truncate_primary: None,
        },
    )
}
