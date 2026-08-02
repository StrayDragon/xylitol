//! Escape / slot open-close navigation (c1170).

use std::time::{Duration, Instant};

use super::super::slots::EditorSlot;
use super::empty_widgets::{
    empty_mcp_list, empty_models_list, empty_session_resume_panel, empty_themes_list,
    import_confirm_list,
};
use super::{ImportConfirmDecision, UiRoot};
use crate::app::tui::bridge::UiPhase;
use crate::protocol::error::XyToolError;

impl UiRoot {
    pub fn on_escape(&mut self) -> bool {
        if self.slot.is_tree() {
            if self.tree_label_edit.take().is_some() {
                return true;
            }
            if self.tree.clear_search_if_any() {
                return true;
            }
            self.close_slot();
            return true;
        }
        if self.slot == EditorSlot::ImportConfirm {
            self.pending_import_decision = Some(ImportConfirmDecision::Rejected);
            self.close_import_confirm();
            return true;
        }
        if self.slot == EditorSlot::Choice {
            // Esc → ChoicePrompt skip success when mounted; bare Choice shell still closes.
            if let Some(ref mut prompt) = self.choice_prompt {
                use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
                use xylitol_tui::{Component, InputEvent};
                if !prompt.finished() {
                    prompt.handle_input(InputEvent::Key(KeyEvent::new(
                        KeyCode::Esc,
                        KeyModifiers::NONE,
                    )));
                }
            } else {
                self.close_slot();
            }
            return true;
        }
        if self.slot == EditorSlot::SessionResume {
            if self.session_resume.cancel_substate() {
                return true;
            }
            self.close_session_resume();
            return true;
        }
        if self.slot.is_overlay() {
            self.close_slot();
            return true;
        }
        if self.ui_model.phase == UiPhase::Busy {
            self.last_esc_at = None;
            return false;
        }
        if self.editor.get_text().is_empty() {
            let now = Instant::now();
            if let Some(prev) = self.last_esc_at
                && now.duration_since(prev) < Duration::from_millis(500)
            {
                self.last_esc_at = None;
                self.pending_tree_open = true;
                return true;
            }
            self.last_esc_at = Some(now);
            return false;
        }
        self.last_esc_at = None;
        false
    }

    pub fn close_slot(&mut self) {
        if self.slot == EditorSlot::Choice {
            // Silent close without ChoiceResult → abort oneshot (not skip JSON).
            if let Some(tx) = self.ask_reply.take() {
                let _ = tx.send(Err(XyToolError::Aborted));
            }
            self.choice_prompt = None;
            self.choice_pending = None;
        }
        self.slot = EditorSlot::Editor;
        self.models_filter.clear();
        self.models_items.clear();
        self.models_list = empty_models_list(self.theme);
        self.themes_list = empty_themes_list(self.theme);
        self.pending_theme_select = None;
        self.import_confirm_path = None;
        self.import_confirm_list = import_confirm_list(self.theme);
        self.session_resume = empty_session_resume_panel(self.theme);
        self.pending_session_resume_rename = None;
        self.pending_session_resume_delete = None;
        self.mcp_list = empty_mcp_list(self.theme);
        self.mcp_summary_line.clear();
        self.mcp_diag_lines.clear();
    }

    pub fn close_session_tree(&mut self) {
        if self.slot.is_tree() {
            self.close_slot();
        }
    }

    /// Open a non-Editor slot (replaces any current overlay).
    pub fn open_slot(&mut self, slot: EditorSlot) {
        match slot {
            EditorSlot::Editor => self.close_slot(),
            EditorSlot::Tree => self.pending_tree_open = true,
            EditorSlot::Plate | EditorSlot::Settings | EditorSlot::Choice => {
                self.slot = slot;
            }
            EditorSlot::Models
            | EditorSlot::Themes
            | EditorSlot::ImportConfirm
            | EditorSlot::SessionResume
            | EditorSlot::Mcp => {
                // Opened via mount_* after slash dispatch.
            }
        }
    }
}
