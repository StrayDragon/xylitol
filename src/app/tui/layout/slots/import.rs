//! `/session-import` Yes/No confirm payload for [`super::EditorSlot::ImportConfirm`].

use xylitol_tui::components::select_list::{SelectItem, SelectList, SelectListLayoutOptions};
use xylitol_tui::{Component, InputEvent};

use super::super::theme::LayoutTheme;
use crate::app::tui::keybindings::matches_binding;
use crate::app::tui::layout::DEFAULT_MAX_VISIBLE;

/// User choice from `/session-import` confirm slot (c1010).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportConfirmDecision {
    Accepted { path: String },
    Rejected,
}

pub enum ImportAction {
    None,
    Decide(ImportConfirmDecision),
}

pub struct ImportSlot {
    list: SelectList,
    path: String,
}

impl ImportSlot {
    pub fn mount(theme: LayoutTheme, path: &str) -> Self {
        Self {
            path: path.to_string(),
            list: import_confirm_list(theme),
        }
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.list.max_visible = max_visible;
    }

    pub fn invalidate(&mut self) {
        self.list.invalidate();
    }

    pub fn retheme(&mut self, theme: LayoutTheme) {
        self.list = import_confirm_list(theme);
    }

    pub fn handle_input(&mut self, event: InputEvent) -> ImportAction {
        let InputEvent::Key(ref key) = event else {
            return ImportAction::None;
        };
        if matches_binding(key, "tui.select.confirm") {
            let accepted = self
                .list
                .get_selected_item()
                .is_some_and(|item| item.value == "yes");
            return ImportAction::Decide(if accepted {
                ImportConfirmDecision::Accepted {
                    path: self.path.clone(),
                }
            } else {
                ImportConfirmDecision::Rejected
            });
        }
        if matches_binding(key, "tui.select.up")
            || matches_binding(key, "tui.select.down")
            || matches_binding(key, "tui.select.pageUp")
            || matches_binding(key, "tui.select.pageDown")
        {
            self.list.handle_input(event);
        }
        ImportAction::None
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(theme.paint_muted(&format!(" Replace current session with {}?", self.path)));
        lines.extend(self.list.render(width.max(1)));
        lines
    }
}

fn import_confirm_list(theme: LayoutTheme) -> SelectList {
    let mut list = SelectList::new(
        vec![SelectItem::new("yes", "Yes"), SelectItem::new("no", "No")],
        DEFAULT_MAX_VISIBLE,
        theme.select_list_theme(),
        SelectListLayoutOptions {
            min_primary_column_width: Some(8),
            max_primary_column_width: Some(24),
            truncate_primary: None,
        },
    );
    list.selected_index = 0;
    list
}
