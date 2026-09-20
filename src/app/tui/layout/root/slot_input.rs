//! UiRoot slot input routing (c1170 / ath12).

use xylitol_tui::{Component, InputEvent};

use super::super::slots::{
    EditorSlot, ImportAction, McpAction, ModelsAction, ThemesAction, TreeAction,
};
use super::UiRoot;
use crate::app::tui::keybindings::matches_binding;
use crate::app::tui::session_resume::SessionResumeAction;

impl UiRoot {
    pub(super) fn handle_slot_input(&mut self, event: InputEvent) {
        if let InputEvent::Mouse(mouse) = &event {
            match mouse.kind {
                crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
                    if self.click_todo_bar_at(mouse.column, mouse.row) =>
                {
                    self.todo_selection.clear();
                    self.todo_pointer_dirty = true;
                    return;
                }
                crossterm::event::MouseEventKind::ScrollUp
                    if self.scroll_todo_bar_at(mouse.row, -1) =>
                {
                    return;
                }
                crossterm::event::MouseEventKind::ScrollDown
                    if self.scroll_todo_bar_at(mouse.row, 1) =>
                {
                    return;
                }
                _ if self.handle_todo_bar_selection(mouse) => return,
                _ => {}
            }
        }
        if matches!(&self.slot, EditorSlot::Editor) {
            self.handle_editor_keys(event);
            return;
        }
        let close_mcp = match &mut self.slot {
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
        };
        if close_mcp {
            self.close_slot();
        }
    }

    fn handle_editor_keys(&mut self, event: InputEvent) {
        if let InputEvent::Key(ref key) = event {
            if matches_binding(key, "app.thinking.toggle") {
                self.fold.thinking_expanded = !self.fold.thinking_expanded;
                self.fold.clear_thinking_overrides();
                self.scrollback_paint.invalidate();
                self.bump_upper_gen();
                return;
            }
            if matches_binding(key, "app.tools.blocks") {
                self.fold.tools_expanded = !self.fold.tools_expanded;
                self.fold.compaction_expanded = !self.fold.compaction_expanded;
                self.fold.clear_tools_overrides();
                self.scrollback_paint.invalidate();
                self.bump_upper_gen();
                return;
            }
            if matches_binding(key, "app.tools.expand") {
                // Ctrl+O = global default flip + clear per-block overrides
                // (att30; same default+overrides model as Alt+E / Ctrl+T).
                // No full-cache clear (ath25): entry fingerprints carry
                // per-block `output_effective`, so only viewport blocks repaint.
                self.fold.tools_output_expanded = !self.fold.tools_output_expanded;
                self.fold.clear_output_overrides();
                self.bump_upper_gen();
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
