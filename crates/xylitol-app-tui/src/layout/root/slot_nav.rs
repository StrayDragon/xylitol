use std::time::{Duration, Instant};

use super::super::slots::EditorSlot;
use super::{ImportConfirmDecision, UiRoot};
use crate::bridge::UiPhase;

#[derive(Debug, Default)]
pub struct EscWindow {
    pub last_at: Option<Instant>,
    pub suppress_idle: bool,
}

impl UiRoot {
    pub fn on_escape(&mut self, esc: &mut EscWindow) -> bool {
        if matches!(&self.slot, EditorSlot::Tree(_)) {
            let stay = if let EditorSlot::Tree(tree) = &mut self.slot {
                tree.cancel_label_edit() || tree.clear_search_if_any()
            } else {
                false
            };
            if !stay {
                self.close_slot();
            }
            return true;
        }
        if matches!(&self.slot, EditorSlot::ImportConfirm(_)) {
            self.pending.import_decision = Some(ImportConfirmDecision::Rejected);
            self.close_import_confirm();
            return true;
        }
        if matches!(&self.slot, EditorSlot::Choice(_)) {
            let handled = if let EditorSlot::Choice(ask) = &mut self.slot {
                ask.handle_esc()
            } else {
                false
            };
            if !handled {
                self.close_slot();
            }
            return true;
        }
        if matches!(&self.slot, EditorSlot::SessionResume(_)) {
            let stay = if let EditorSlot::SessionResume(panel) = &mut self.slot {
                panel.cancel_substate()
            } else {
                false
            };
            if !stay {
                self.close_session_resume();
            }
            return true;
        }
        if self.slot.is_overlay() {
            self.close_slot();
            return true;
        }
        if self.ui_model.phase == UiPhase::Busy {
            esc.last_at = None;
            return false;
        }
        if self.editor.get_text().is_empty() {
            let now = Instant::now();
            if let Some(prev) = esc.last_at
                && now.duration_since(prev) < Duration::from_millis(500)
            {
                esc.last_at = None;
                self.pending.tree_open = true;
                return true;
            }
            esc.last_at = Some(now);
            return false;
        }
        esc.last_at = None;
        false
    }

    pub fn close_slot(&mut self) {
        if let EditorSlot::Choice(ask) = &mut self.slot {
            ask.abort();
        }
        self.slot = EditorSlot::Editor;
        self.pending.clear_cancelled_on_close();
    }

    pub fn close_session_tree(&mut self) {
        if self.slot.is_tree() {
            self.close_slot();
        }
    }

    pub fn open_slot(&mut self, slot: EditorSlot) {
        match slot {
            EditorSlot::Editor => self.close_slot(),
            EditorSlot::Tree(_) => self.pending.tree_open = true,
            EditorSlot::Plate | EditorSlot::Settings | EditorSlot::Choice(_) => {
                self.slot = slot;
            }
            EditorSlot::Models(_)
            | EditorSlot::Scenes(_)
            | EditorSlot::Themes(_)
            | EditorSlot::ImportConfirm(_)
            | EditorSlot::SessionResume(_)
            | EditorSlot::Mcp(_)
            | EditorSlot::McpPrompts(_)
            | EditorSlot::McpResources(_)
            | EditorSlot::Capabilities(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_esc_window_is_caller_owned() {
        let mut root = UiRoot::new();
        let mut window = EscWindow::default();
        assert!(!root.on_escape(&mut window), "first Esc arms the window");
        assert!(window.last_at.is_some());
        assert!(root.on_escape(&mut window), "second Esc opens the tree");
        assert!(window.last_at.is_none(), "window resets after opening");
    }
}
