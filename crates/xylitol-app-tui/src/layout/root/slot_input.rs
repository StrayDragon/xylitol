use crossterm::event::KeyEvent;
use xylitol_tui::{Component, InputEvent};

use super::super::slots::{
    CapabilitiesAction, EditorSlot, ImportAction, McpAction, ModelsAction, ScenesAction,
    ThemesAction, TreeAction,
};
use super::UiRoot;
use crate::keybindings::matches_binding;
use crate::session_resume::SessionResumeAction;

impl UiRoot {
    fn handle_session_details_key(&mut self, key: &KeyEvent) -> bool {
        if matches_binding(key, "app.session.detailsCycle") {
            self.cycle_session_details();
            return true;
        }
        if matches_binding(key, "app.session.sidebarWidth") {
            self.session_sidebar_width_cycle();
            return true;
        }
        if matches_binding(key, "app.session.popup") {
            if self.session_popup_open {
                self.close_session_popup();
            } else {
                self.request_session_popup();
            }
            return true;
        }
        if matches_binding(key, "app.session.sidebar") {
            if self.session_sidebar_on {
                self.close_session_sidebar();
            } else {
                self.request_session_sidebar();
            }
            return true;
        }
        false
    }

    pub(super) fn handle_slot_input(&mut self, event: InputEvent) {
        if let InputEvent::Mouse(mouse) = &event
            && self.handle_pointer_input(mouse)
        {
            return;
        }
        if let InputEvent::Key(ref key) = event
            && self.handle_session_details_key(key)
        {
            return;
        }
        if matches!(&self.slot, EditorSlot::Editor) {
            self.handle_editor_keys(event);
            return;
        }
        if self.capabilities_open() {
            if self.handle_capabilities_input(event) {
                self.close_slot();
            }
            return;
        }
        let close_slot = match &mut self.slot {
            EditorSlot::Editor => false,
            EditorSlot::Tree(tree) => {
                match tree.handle_input(event) {
                    TreeAction::None => {}
                    TreeAction::Travel(id) => self.pending.tree_travel = Some(id),
                    TreeAction::Fork(id) => self.pending.tree_fork = Some(id),
                    TreeAction::Label { id, annotation } => {
                        self.pending.tree_label = Some((id, annotation));
                    }
                }
                false
            }
            EditorSlot::Plate | EditorSlot::Settings => false,
            EditorSlot::Choice(ask) => {
                ask.handle_input(event);
                false
            }
            EditorSlot::Models(models) => {
                if let ModelsAction::Select(choice) = models.handle_input(event) {
                    self.pending.model_select = Some(choice);
                }
                false
            }
            EditorSlot::Scenes(scenes) => {
                if let ScenesAction::Select(id) = scenes.handle_input(event) {
                    self.pending.scene_select = Some(id);
                }
                false
            }
            EditorSlot::Themes(themes) => {
                if let ThemesAction::Select(name) = themes.handle_input(event) {
                    self.pending.theme_select = Some(name);
                }
                false
            }
            EditorSlot::ImportConfirm(imp) => {
                if let ImportAction::Decide(decision) = imp.handle_input(event) {
                    self.pending.import_decision = Some(decision);
                }
                false
            }
            EditorSlot::SessionResume(panel) => {
                match panel.handle_input(event) {
                    SessionResumeAction::Switch(id) => {
                        self.pending.session_resume_select = Some(id);
                    }
                    SessionResumeAction::Rename { id, name } => {
                        self.pending.session_resume_rename = Some((id, name));
                    }
                    SessionResumeAction::Delete(id) => {
                        self.pending.session_resume_delete = Some(id);
                    }
                    SessionResumeAction::None => {}
                }
                false
            }
            EditorSlot::Mcp(mcp) => matches!(mcp.handle_input(event), McpAction::Close),
            EditorSlot::Capabilities(_) => false,
        };
        if close_slot {
            self.close_slot();
        }
    }

    fn handle_slot_row_click(&mut self, row: u16) -> bool {
        if !self.slot.is_overlay() {
            return false;
        }
        let Some(top) = self.last_slot_top_row else {
            return false;
        };
        let row = row as usize;
        if row < top {
            return false;
        }
        let local = row.saturating_sub(top);
        let header_rows = match &self.slot {
            EditorSlot::Models(_) | EditorSlot::Themes(_) | EditorSlot::Mcp(_) => 1,
            EditorSlot::Capabilities(_) => 1,
            EditorSlot::Tree(tree) => tree.header_rows(self.term_cols),
            EditorSlot::SessionResume(panel) => panel.header_rows(self.term_cols),
            _ => 0,
        };
        if local < header_rows {
            return false;
        }
        let list_row = (local - header_rows) as u16;
        if self.capabilities_open() {
            let action = match &mut self.slot {
                EditorSlot::Capabilities(caps) => caps.toggle_row_at(list_row),
                _ => None,
            };
            return match action {
                Some(action) => {
                    let _ = self.handle_capabilities_action(action);
                    true
                }
                None => false,
            };
        }
        if let EditorSlot::Tree(tree) = &mut self.slot {
            let previous = tree.selected_id();
            let Some(id) = tree.select_row_at(list_row) else {
                return false;
            };
            if previous.as_deref() == Some(id.as_str()) {
                self.pending.tree_travel = Some(id);
            }
            return true;
        }
        if let EditorSlot::SessionResume(panel) = &mut self.slot {
            let previous = panel.selected_session_id();
            let Some(id) = panel.select_row_at(list_row as usize) else {
                return false;
            };
            if previous.as_deref() == Some(id.as_str()) {
                self.pending.session_resume_select = Some(id);
            }
            return true;
        }
        match &mut self.slot {
            EditorSlot::Models(models) => {
                let previous = models.selected_id();
                let Some(id) = models.select_row_at(list_row) else {
                    return false;
                };
                if previous.as_deref() == Some(id)
                    && let Some((model_id, thinking)) = models.confirm_selected()
                {
                    self.pending.model_select =
                        Some(super::super::models_picker::PendingModelChoice {
                            model_id,
                            thinking,
                        });
                }
                true
            }
            EditorSlot::Themes(themes) => {
                let previous = themes.selected_id();
                let Some(name) = themes.select_row_at(list_row) else {
                    return false;
                };
                if previous.as_deref() == Some(name) {
                    self.pending.theme_select = themes.confirm_selected();
                }
                true
            }
            EditorSlot::Mcp(mcp) => mcp.select_row_at(list_row).is_some(),
            _ => false,
        }
    }

    #[cfg(test)]
    pub fn handle_slot_row_click_for_test(&mut self, row: u16) -> bool {
        self.handle_slot_row_click(row)
    }

    #[cfg(test)]
    pub fn handle_close_chip_for_test(&mut self, col: usize, row: usize) -> bool {
        if Some(row) != self.last_slot_top_row {
            return false;
        }
        let Some((start, end)) = self.last_slot_close_hit else {
            return false;
        };
        if col < start || col >= end {
            return false;
        }
        self.close_slot();
        true
    }

    fn handle_pointer_input(&mut self, mouse: &crossterm::event::MouseEvent) -> bool {
        if matches!(
            mouse.kind,
            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
        ) && let Some((start, end)) = self.last_slot_close_hit
            && Some(mouse.row as usize) == self.last_slot_top_row
            && (mouse.column as usize) >= start
            && (mouse.column as usize) < end
        {
            self.close_slot();
            return true;
        }
        if matches!(
            mouse.kind,
            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
        ) && self.handle_slot_row_click(mouse.row)
        {
            return true;
        }
        match mouse.kind {
            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
                if self.click_todo_bar_at(mouse.column, mouse.row) =>
            {
                self.todo_selection.clear();
                self.todo_pointer_dirty = true;
                true
            }
            crossterm::event::MouseEventKind::ScrollUp
                if self.scroll_todo_bar_at(mouse.row, -1) =>
            {
                true
            }
            crossterm::event::MouseEventKind::ScrollDown
                if self.scroll_todo_bar_at(mouse.row, 1) =>
            {
                true
            }
            _ => self.handle_todo_bar_selection(mouse),
        }
    }

    fn handle_capabilities_input(&mut self, event: InputEvent) -> bool {
        let EditorSlot::Capabilities(caps) = &mut self.slot else {
            return false;
        };
        match caps.handle_input(event) {
            CapabilitiesAction::Toggle { id, enabled } => {
                self.pending.capability_toggle = Some((id, enabled));
                false
            }
            CapabilitiesAction::Close => true,
            CapabilitiesAction::None => false,
        }
    }

    fn handle_capabilities_action(&mut self, action: CapabilitiesAction) -> bool {
        match action {
            CapabilitiesAction::Toggle { id, enabled } => {
                self.pending.capability_toggle = Some((id, enabled));
                false
            }
            CapabilitiesAction::Close => true,
            CapabilitiesAction::None => false,
        }
    }

    pub fn toggle_thinking_default(&mut self) {
        self.fold.thinking_expanded = !self.fold.thinking_expanded;
        self.fold.clear_thinking_overrides();
        self.scrollback_paint.invalidate();
        self.bump_upper_gen();
    }

    pub fn reset_tools_blocks(&mut self) {
        self.fold.tools_expanded = !self.fold.tools_expanded;
        self.fold.compaction_expanded = !self.fold.compaction_expanded;
        self.fold.clear_tools_overrides();
        self.scrollback_paint.invalidate();
        self.bump_upper_gen();
    }

    pub fn toggle_tools_output_default(&mut self) {
        self.fold.tools_output_expanded = !self.fold.tools_output_expanded;
        self.fold.clear_output_overrides();
        self.bump_upper_gen();
    }

    fn handle_editor_keys(&mut self, event: InputEvent) {
        if let InputEvent::Key(ref key) = event {
            if matches_binding(key, "app.thinking.toggle") {
                self.toggle_thinking_default();
                return;
            }
            if matches_binding(key, "app.tools.blocks") {
                self.reset_tools_blocks();
                return;
            }
            if matches_binding(key, "app.tools.expand") {
                self.toggle_tools_output_default();
                return;
            }
            if matches_binding(key, "app.activity.expandNearest") {
                let _ = self.expand_nearest_activity();
                return;
            }
            if matches_binding(key, "app.activity.collapseNearest") {
                let _ = self.collapse_nearest_activity();
                return;
            }
        }
        self.editor.handle_input(event);
        self.sync_editor_border();
    }

    fn handle_todo_bar_selection(&mut self, mouse: &crossterm::event::MouseEvent) -> bool {
        let Some(rect) = self.todo_bar_screen_rect() else {
            return false;
        };
        let in_bar = rect.contains(mouse.column, mouse.row);
        if !in_bar && !self.todo_selection.is_dragging() {
            return false;
        }
        let mut scroll = xylitol_tui::ScrollView::new(self.last_todo_plain.len().max(1));
        scroll.set_lines(self.last_todo_plain.clone());
        let dock = xylitol_tui::ScreenRect {
            row: 0,
            col: 0,
            height: 0,
            width: 0,
        };
        let mut copied = Vec::new();
        let dirty = {
            let mut sink = TodoCopySink { out: &mut copied };
            self.todo_selection
                .handle_mouse(mouse, &mut scroll, rect, dock, &mut sink)
        };
        self.todo_clipboard.append(&mut copied);
        if dirty {
            self.todo_pointer_dirty = true;
        }
        if self.todo_selection.last_event_copied() {
            self.arm_copy_notice();
        }
        in_bar || dirty
    }

    fn todo_bar_screen_rect(&self) -> Option<xylitol_tui::ScreenRect> {
        if self.last_todo_rows == 0 || self.last_todo_plain.is_empty() {
            return None;
        }
        let dock = self.last_dock_rows.max(1).min(self.term_rows);
        let dock_top = self.term_rows.saturating_sub(dock);
        let row = dock_top.saturating_add(self.last_queue_rows);
        let width = self
            .last_todo_plain
            .first()
            .map(|l| xylitol_tui::visible_width(l) as u16)
            .unwrap_or(1)
            .max(1);
        Some(xylitol_tui::ScreenRect {
            row: row as u16,
            col: 0,
            height: self.last_todo_rows as u16,
            width,
        })
    }
}

struct TodoCopySink<'a> {
    out: &'a mut Vec<String>,
}

impl xylitol_tui::ClipboardSink for TodoCopySink<'_> {
    fn copy_text(&mut self, text: &str) {
        if let Some(seq) = xylitol_tui::format_osc52(text) {
            self.out.push(seq);
        }
    }
}
