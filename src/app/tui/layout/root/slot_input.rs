//! UiRoot slot input routing (c1170 / ath12).

use xylitol_tui::{Component, Input, InputEvent, matches_key_event, printable_from_key_event};

use super::super::session_tree::FilterMode;
use super::super::slots::EditorSlot;
use super::ImportConfirmDecision;
use super::UiRoot;
use crate::app::tui::keybindings::matches_binding;
use crate::app::tui::session_resume::SessionResumeAction;

impl UiRoot {
    pub(super) fn handle_slot_input(&mut self, event: InputEvent) {
        match self.slot {
            EditorSlot::Tree => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if let Some((_, ref mut input)) = self.tree_label_edit {
                    if matches_binding(key, "tui.select.confirm") {
                        if let Some((id, input)) = self.tree_label_edit.take() {
                            let text = input.value().trim().to_string();
                            let ann = if text.is_empty() { None } else { Some(text) };
                            self.pending_tree_label = Some((id, ann));
                        }
                        return;
                    }
                    input.handle_input(event);
                    return;
                }
                if matches_binding(key, "app.tree.filter.default") {
                    self.apply_tree_filter(FilterMode::Default);
                    return;
                }
                if matches_binding(key, "app.tree.filter.noTools") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::NoTools));
                    return;
                }
                if matches_binding(key, "app.tree.filter.userOnly") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::UserOnly));
                    return;
                }
                if matches_binding(key, "app.tree.filter.labeledOnly") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::LabeledOnly));
                    return;
                }
                if matches_binding(key, "app.tree.filter.all") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::All));
                    return;
                }
                if matches_binding(key, "app.tree.filter.cycleBackward") {
                    self.apply_tree_filter(self.tree_filter.cycle_backward());
                    return;
                }
                if matches_binding(key, "app.tree.filter.cycleForward") {
                    self.apply_tree_filter(self.tree_filter.cycle());
                    return;
                }
                if matches_binding(key, "tui.select.confirm") {
                    let id = self.tree.selected_id().unwrap_or("?").to_string();
                    self.pending_tree_travel = Some(id);
                    return;
                }
                if matches_binding(key, "app.session.fork") {
                    let id = self.tree.selected_id().unwrap_or("?").to_string();
                    self.pending_tree_fork = Some(id);
                    return;
                }
                if matches_binding(key, "app.tree.editLabel") {
                    let Some(id) = self.tree.selected_id().map(str::to_string) else {
                        return;
                    };
                    let current = self.tree.annotation_of(&id).unwrap_or("").to_string();
                    let mut input = Input::new();
                    input.set_value(current);
                    self.tree_label_edit = Some((id, input));
                    return;
                }
                if matches_binding(key, "app.tree.toggleLabelTimestamp") {
                    self.tree.toggle_annotation_timestamps();
                    return;
                }
                if matches_binding(key, "tui.select.up")
                    || matches_binding(key, "tui.select.down")
                    || matches_binding(key, "tui.select.pageUp")
                    || matches_binding(key, "tui.select.pageDown")
                    || matches_binding(key, "tui.tree.foldOrUp")
                    || matches_binding(key, "tui.tree.unfoldOrDown")
                    || matches_key_event(key, "backspace")
                    || printable_from_key_event(key).is_some()
                {
                    self.tree.handle_input(event);
                }
                return;
            }
            EditorSlot::Plate | EditorSlot::Settings | EditorSlot::Choice => {
                // Empty shells: Esc is handled by InputListener; ignore other keys.
                return;
            }
            EditorSlot::Models => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if matches_binding(key, "tui.select.confirm") {
                    self.confirm_models_selection();
                    return;
                }
                // ←→ / Shift+Tab: cycle provisional thinking on focused model (c1470).
                // `tui.select.pageUp|pageDown` stay unbound so ←→ are not stolen.
                if matches_key_event(key, "left") {
                    self.cycle_focused_model_level(false);
                    return;
                }
                if matches_key_event(key, "right") || matches_key_event(key, "shift+tab") {
                    self.cycle_focused_model_level(true);
                    return;
                }
                if matches_binding(key, "tui.select.up")
                    || matches_binding(key, "tui.select.down")
                    || matches_binding(key, "tui.select.pageUp")
                    || matches_binding(key, "tui.select.pageDown")
                {
                    self.models_list.handle_input(event);
                    self.rebuild_models_items_keep_selection();
                    return;
                }
                if matches_key_event(key, "backspace") {
                    self.models_filter.pop();
                    self.apply_models_filter();
                    return;
                }
                if let Some(text) = printable_from_key_event(key) {
                    self.models_filter.push_str(&text);
                    self.apply_models_filter();
                }
                return;
            }
            EditorSlot::Themes => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if matches_binding(key, "tui.select.confirm") {
                    if let Some(item) = self.themes_list.get_selected_item() {
                        self.pending_theme_select = Some(item.value.clone());
                    }
                    return;
                }
                if matches_binding(key, "tui.select.up")
                    || matches_binding(key, "tui.select.down")
                    || matches_binding(key, "tui.select.pageUp")
                    || matches_binding(key, "tui.select.pageDown")
                {
                    self.themes_list.handle_input(event);
                }
                return;
            }
            EditorSlot::ImportConfirm => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if matches_binding(key, "tui.select.confirm") {
                    let Some(path) = self.import_confirm_path.clone() else {
                        return;
                    };
                    let accepted = self
                        .import_confirm_list
                        .get_selected_item()
                        .is_some_and(|item| item.value == "yes");
                    self.pending_import_decision = Some(if accepted {
                        ImportConfirmDecision::Accepted { path }
                    } else {
                        ImportConfirmDecision::Rejected
                    });
                    return;
                }
                if matches_binding(key, "tui.select.up")
                    || matches_binding(key, "tui.select.down")
                    || matches_binding(key, "tui.select.pageUp")
                    || matches_binding(key, "tui.select.pageDown")
                {
                    self.import_confirm_list.handle_input(event);
                }
                return;
            }
            EditorSlot::SessionResume => {
                let action = self.session_resume.handle_input(event);
                match action {
                    SessionResumeAction::Switch(id) => {
                        self.pending_session_resume_select = Some(id);
                    }
                    SessionResumeAction::Rename { id, name } => {
                        self.pending_session_resume_rename = Some((id, name));
                    }
                    SessionResumeAction::Delete(id) => {
                        self.pending_session_resume_delete = Some(id);
                    }
                    SessionResumeAction::None => {}
                }
                return;
            }
            EditorSlot::Editor => {}
        }

        if let InputEvent::Key(ref key) = event {
            if matches_binding(key, "app.thinking.toggle") {
                self.fold.thinking_expanded = !self.fold.thinking_expanded;
                self.scrollback_paint.invalidate();
                self.bump_upper_gen();
                return;
            }
            if matches_binding(key, "app.tools.blocks") {
                self.fold.tools_expanded = !self.fold.tools_expanded;
                self.scrollback_paint.invalidate();
                self.bump_upper_gen();
                return;
            }
            if matches_binding(key, "app.tools.expand") {
                self.fold.tools_output_expanded = !self.fold.tools_output_expanded;
                self.scrollback_paint.invalidate();
                self.bump_upper_gen();
                return;
            }
            // Product MUST NOT open Command Plate (DESIGN 明确不做；Ctrl+P 留给
            // session-resume path toggle 等已接线绑定，勿再抢占).
        }

        self.editor.handle_input(event);
        self.sync_editor_border();
    }
}
