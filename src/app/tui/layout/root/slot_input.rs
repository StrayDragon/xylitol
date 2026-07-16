//! UiRoot slot input routing (c1170 / ath12).

use xylitol_tui::{Component, Input, InputEvent, matches_key_event, printable_from_key_event};

use super::super::session_tree::FilterMode;
use super::super::slots::EditorSlot;
use super::ImportConfirmDecision;
use super::UiRoot;
use crate::app::tui::session_resume::SessionResumeAction;

impl UiRoot {
    pub(super) fn handle_slot_input(&mut self, event: InputEvent) {
        match self.slot {
            EditorSlot::Tree => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if let Some((_, ref mut input)) = self.tree_label_edit {
                    if matches_key_event(key, "enter") {
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
                if matches_key_event(key, "ctrl+d") {
                    self.apply_tree_filter(FilterMode::Default);
                    return;
                }
                if matches_key_event(key, "ctrl+t") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::NoTools));
                    return;
                }
                if matches_key_event(key, "ctrl+u") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::UserOnly));
                    return;
                }
                if matches_key_event(key, "ctrl+l") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::LabeledOnly));
                    return;
                }
                if matches_key_event(key, "ctrl+a") {
                    self.apply_tree_filter(self.tree_filter.toggle(FilterMode::All));
                    return;
                }
                if matches_key_event(key, "ctrl+shift+o") {
                    self.apply_tree_filter(self.tree_filter.cycle_backward());
                    return;
                }
                if matches_key_event(key, "ctrl+o") {
                    self.apply_tree_filter(self.tree_filter.cycle());
                    return;
                }
                if matches_key_event(key, "enter") {
                    let id = self.tree.selected_id().unwrap_or("?").to_string();
                    self.pending_tree_travel = Some(id);
                    return;
                }
                if matches_key_event(key, "shift+f") {
                    let id = self.tree.selected_id().unwrap_or("?").to_string();
                    self.pending_tree_fork = Some(id);
                    return;
                }
                if matches_key_event(key, "shift+l") {
                    let Some(id) = self.tree.selected_id().map(str::to_string) else {
                        return;
                    };
                    let current = self.tree.annotation_of(&id).unwrap_or("").to_string();
                    let mut input = Input::new();
                    input.set_value(current);
                    self.tree_label_edit = Some((id, input));
                    return;
                }
                if matches_key_event(key, "shift+t") {
                    self.tree.toggle_annotation_timestamps();
                    return;
                }
                if matches_key_event(key, "up")
                    || matches_key_event(key, "down")
                    || matches_key_event(key, "pageUp")
                    || matches_key_event(key, "pageDown")
                    || matches_key_event(key, "left")
                    || matches_key_event(key, "right")
                    || matches_key_event(key, "ctrl+left")
                    || matches_key_event(key, "alt+left")
                    || matches_key_event(key, "ctrl+right")
                    || matches_key_event(key, "alt+right")
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
                if matches_key_event(key, "enter") {
                    if let Some(item) = self.models_list.get_selected_item() {
                        self.pending_model_select = Some(item.value.clone());
                    }
                    return;
                }
                if matches_key_event(key, "up")
                    || matches_key_event(key, "down")
                    || matches_key_event(key, "pageUp")
                    || matches_key_event(key, "pageDown")
                {
                    self.models_list.handle_input(event);
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
            EditorSlot::ImportConfirm => {
                let InputEvent::Key(ref key) = event else {
                    return;
                };
                if matches_key_event(key, "enter") {
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
                if matches_key_event(key, "up")
                    || matches_key_event(key, "down")
                    || matches_key_event(key, "pageUp")
                    || matches_key_event(key, "pageDown")
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
            if matches_key_event(key, "ctrl+t") {
                self.fold.thinking_expanded = !self.fold.thinking_expanded;
                return;
            }
            if matches_key_event(key, "alt+e") {
                self.fold.tools_expanded = !self.fold.tools_expanded;
                return;
            }
            if matches_key_event(key, "ctrl+o") {
                self.fold.tools_output_expanded = !self.fold.tools_output_expanded;
                return;
            }
            // MAY: Ctrl+P opens Plate stub (Esc closes).
            if matches_key_event(key, "ctrl+p") {
                self.open_slot(EditorSlot::Plate);
                return;
            }
        }

        self.editor.handle_input(event);
        self.sync_editor_border();
    }
}
