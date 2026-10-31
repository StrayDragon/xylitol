use xylitol_tui::components::select_list::{SelectItem, SelectList};
use xylitol_tui::{Component, InputEvent};

use super::super::theme::LayoutTheme;
use crate::keybindings::matches_binding;
use crate::layout::DEFAULT_MAX_VISIBLE;

pub enum ThemesAction {
    None,
    Select(String),
}

pub struct ThemesSlot {
    list: SelectList,
    confirmed: Option<String>,
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
            list: theme.select_list(items, 4, (12, 24)),
            confirmed: None,
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
                self.confirmed = Some(item.value.clone());
                return ThemesAction::Select(item.value.clone());
            }
            return ThemesAction::None;
        }
        if super::is_select_nav_key(key) {
            self.list.handle_input(event);
        }
        ThemesAction::None
    }

    pub fn selected_id(&self) -> Option<String> {
        self.list.get_selected_item().map(|i| i.value.clone())
    }

    pub fn select_row_at(&mut self, row: u16) -> Option<&str> {
        let index = self.list.item_index_at_screen_row(row)?;
        self.list.set_selected_index(index);
        self.list
            .filtered_items
            .get(index)
            .map(|i| i.value.as_str())
    }

    pub fn confirm_selected(&mut self) -> Option<String> {
        let item = self.list.get_selected_item()?;
        self.confirmed = Some(item.value.clone());
        Some(item.value.clone())
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        let mut head = " themes".to_string();
        if let Some(name) = &self.confirmed
            && self
                .list
                .get_selected_item()
                .is_some_and(|i| &i.value == name)
        {
            head = format!("{head} ✓ 已选 {name}");
        }
        lines.push(theme.paint_muted(&xylitol_tui::truncate_to_width(
            &head,
            width.max(1),
            "",
            false,
        )));
        lines.extend(self.list.render(width.max(1)));
        lines
    }
}

fn empty_themes_list(theme: LayoutTheme) -> SelectList {
    theme.select_list(Vec::new(), DEFAULT_MAX_VISIBLE, (12, 24))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_select_row_at_maps_rendered_rows_to_items() {
        let theme = LayoutTheme::product_dark();
        let mut slot = ThemesSlot::mount(theme, None);
        assert_eq!(
            slot.select_row_at(0),
            None,
            "no screen window before the first render"
        );
        let _ = slot.render(80, theme);
        assert_eq!(slot.select_row_at(0), Some("dark"));
        assert_eq!(slot.select_row_at(1), Some("light"));
        assert_eq!(slot.selected_id(), Some("light".to_string()));
        assert_eq!(slot.select_row_at(2), None, "row past the visible window");
        assert_eq!(
            slot.selected_id(),
            Some("light".to_string()),
            "a miss must not move the selection"
        );
    }

    #[test]
    fn themes_confirm_selected_returns_and_records_the_item() {
        let theme = LayoutTheme::product_dark();
        let mut slot = ThemesSlot::mount(theme, Some("dark"));
        let _ = slot.render(80, theme);
        assert_eq!(slot.select_row_at(1), Some("light"));
        assert_eq!(slot.confirm_selected(), Some("light".to_string()));
        assert_eq!(slot.confirmed.as_deref(), Some("light"));
        let lines = slot.render(80, theme);
        assert!(
            lines.first().is_some_and(|l| l.contains("✓ 已选")),
            "confirmed pick shows in the header: {lines:?}"
        );
    }

    #[test]
    fn themes_confirm_selected_without_items_is_none() {
        let theme = LayoutTheme::product_dark();
        let mut slot = ThemesSlot::mount(theme, None);
        slot.retheme(theme);
        assert_eq!(slot.selected_id(), None);
        assert_eq!(slot.confirm_selected(), None);
        assert_eq!(slot.confirmed, None);
    }
}
