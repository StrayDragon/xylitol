//! Built-in theme picker payload for [`super::EditorSlot::Themes`].

use xylitol_tui::components::select_list::{SelectItem, SelectList};
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
    /// Enter-confirmed theme name (c2790 quick, same show-confirm-on-pending
    /// contract as the Models picker).
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

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        let mut head = " themes".to_string();
        // c2790 quick: immediate Enter feedback while a bang keeps the confirm
        // pending (same contract as the Models picker).
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
