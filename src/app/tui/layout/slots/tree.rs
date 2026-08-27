//! MessageHistory tree payload for [`super::EditorSlot::Tree`].

use xylitol_tui::{Component, Input, InputEvent, TreeNode, TreeSelector, TreeSelectorOptions};

use super::super::session_tree::{FilterMode, tree_help_line, tree_search_line, wrap_help_line};
use super::super::theme::LayoutTheme;
use crate::app::tui::keybindings::matches_binding;
use xylitol_tui::{matches_key_event, printable_from_key_event};

pub enum TreeAction {
    None,
    Travel(String),
    Fork(String),
    Label {
        id: String,
        annotation: Option<String>,
    },
}

pub struct TreeSlot {
    tree: TreeSelector,
    filter: FilterMode,
    label_edit: Option<(String, Input)>,
}

impl TreeSlot {
    pub fn mount(theme: LayoutTheme, roots: Vec<TreeNode>, active_id: Option<&str>) -> Self {
        let mut slot = Self {
            tree: TreeSelector::new(
                roots,
                theme.tree_selector_theme(),
                TreeSelectorOptions {
                    max_visible: 10,
                    unicode_connectors: true,
                    include_node: Some(Box::new(|n| FilterMode::Default.include(n))),
                    active_id: active_id.map(str::to_string),
                    status_suffix: None,
                },
            ),
            filter: FilterMode::Default,
            label_edit: None,
        };
        slot.apply_filter(FilterMode::Default);
        slot
    }

    pub fn filter(&self) -> FilterMode {
        self.filter
    }

    /// Whether a node-label edit session is open (Shift+L; Esc cancels it).
    pub fn is_label_editing(&self) -> bool {
        self.label_edit.is_some()
    }

    pub fn search_query(&self) -> &str {
        self.tree.search_query()
    }

    pub fn is_folded(&self, id: &str) -> bool {
        self.tree.is_folded(id)
    }

    pub fn select_id(&mut self, id: &str) -> bool {
        self.tree.select_id(id)
    }

    pub fn set_max_visible(&mut self, max_visible: usize) {
        self.tree.set_max_visible(max_visible);
    }

    pub fn apply_label(&mut self, id: &str, label: Option<String>) {
        self.tree.set_annotation(id, label.clone());
        if label.is_some() {
            self.tree.set_annotation_at(id, Some("just now".into()));
        } else {
            self.tree.set_annotation_at(id, None);
        }
    }

    pub fn cancel_label_edit(&mut self) -> bool {
        self.label_edit.take().is_some()
    }

    pub fn clear_search_if_any(&mut self) -> bool {
        self.tree.clear_search_if_any()
    }

    pub fn invalidate(&mut self) {
        self.tree.invalidate();
    }

    pub fn wipe_themed(&mut self, theme: LayoutTheme) {
        let selected = self.tree.selected_id().map(str::to_string);
        self.tree = TreeSelector::new(
            Vec::new(),
            theme.tree_selector_theme(),
            TreeSelectorOptions {
                max_visible: crate::app::tui::layout::DEFAULT_MAX_VISIBLE,
                unicode_connectors: true,
                include_node: None,
                active_id: None,
                status_suffix: None,
            },
        );
        if let Some(id) = selected {
            let _ = self.tree.select_id(&id);
        }
    }

    pub fn apply_filter(&mut self, mode: FilterMode) {
        self.filter = mode;
        self.tree
            .set_include_node(Some(Box::new(move |n| mode.include(n))));
        self.tree
            .set_status_suffix(mode.status_suffix().map(str::to_string));
    }

    pub fn handle_input(&mut self, event: InputEvent) -> TreeAction {
        let InputEvent::Key(ref key) = event else {
            return TreeAction::None;
        };
        if let Some((_, ref mut input)) = self.label_edit {
            if matches_binding(key, "tui.select.confirm") {
                if let Some((id, input)) = self.label_edit.take() {
                    let text = input.value().trim().to_string();
                    let ann = if text.is_empty() { None } else { Some(text) };
                    return TreeAction::Label {
                        id,
                        annotation: ann,
                    };
                }
                return TreeAction::None;
            }
            input.handle_input(event);
            return TreeAction::None;
        }
        if matches_binding(key, "app.tree.filter.default") {
            self.apply_filter(FilterMode::Default);
            return TreeAction::None;
        }
        if matches_binding(key, "app.tree.filter.noTools") {
            self.apply_filter(self.filter.toggle(FilterMode::NoTools));
            return TreeAction::None;
        }
        if matches_binding(key, "app.tree.filter.userOnly") {
            self.apply_filter(self.filter.toggle(FilterMode::UserOnly));
            return TreeAction::None;
        }
        if matches_binding(key, "app.tree.filter.labeledOnly") {
            self.apply_filter(self.filter.toggle(FilterMode::LabeledOnly));
            return TreeAction::None;
        }
        if matches_binding(key, "app.tree.filter.all") {
            self.apply_filter(self.filter.toggle(FilterMode::All));
            return TreeAction::None;
        }
        if matches_binding(key, "app.tree.filter.cycleBackward") {
            self.apply_filter(self.filter.cycle_backward());
            return TreeAction::None;
        }
        if matches_binding(key, "app.tree.filter.cycleForward") {
            self.apply_filter(self.filter.cycle());
            return TreeAction::None;
        }
        if matches_binding(key, "tui.select.confirm") {
            let id = self.tree.selected_id().unwrap_or("?").to_string();
            return TreeAction::Travel(id);
        }
        if matches_binding(key, "app.session.fork") {
            let id = self.tree.selected_id().unwrap_or("?").to_string();
            return TreeAction::Fork(id);
        }
        if matches_binding(key, "app.tree.editLabel") {
            let Some(id) = self.tree.selected_id().map(str::to_string) else {
                return TreeAction::None;
            };
            let current = self.tree.annotation_of(&id).unwrap_or("").to_string();
            let mut input = Input::new();
            input.set_value(current);
            self.label_edit = Some((id, input));
            return TreeAction::None;
        }
        if matches_binding(key, "app.tree.toggleLabelTimestamp") {
            self.tree.toggle_annotation_timestamps();
            return TreeAction::None;
        }
        if super::is_select_nav_key(key)
            || matches_binding(key, "tui.tree.foldOrUp")
            || matches_binding(key, "tui.tree.unfoldOrDown")
            || matches_key_event(key, "backspace")
            || printable_from_key_event(key).is_some()
        {
            self.tree.handle_input(event);
        }
        TreeAction::None
    }

    pub fn render(&mut self, width: usize, theme: LayoutTheme) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(" Session tree".to_string());
        if let Some((_, ref mut input)) = self.label_edit {
            lines.push(theme.paint_muted(" Label edit · Enter save · Esc cancel"));
            lines.extend(input.render(width.max(1)));
            return lines;
        }
        for help in wrap_help_line(&tree_help_line(), width.max(1)) {
            lines.push(theme.paint_muted(&help));
        }
        lines.push(theme.paint_muted(&tree_search_line(self.tree.search_query())));
        lines.extend(self.tree.render(width.max(1)));
        lines
    }

    pub fn panel_text(&mut self, width: usize) -> String {
        self.tree.render(width).join("\n")
    }
}
