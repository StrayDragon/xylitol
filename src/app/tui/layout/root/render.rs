//! Slot render helpers for UiRoot (c1170).

use std::time::Instant;

use xylitol_tui::{Component, InputEvent, truncate_to_width};

use super::super::session_tree::{tree_help_line, tree_search_line, wrap_help_line};
use super::super::slots::EditorSlot;
use super::UiRoot;
use crate::app::tui::widgets::{render_queue_strip, render_scrollback};

impl UiRoot {
    /// Codex-style startup card (brand + skills/MCP) above scrollback (c1135).
    pub(super) fn render_loaded_resources_slot(&mut self, width: usize) -> Vec<String> {
        crate::app::tui::widgets::render_loaded_resources(
            self.theme,
            &self.loaded_resources,
            &self.cwd,
            &self.model,
            width,
        )
    }

    pub(super) fn render_queue_slot(&mut self, width: usize) -> Vec<String> {
        render_queue_strip(
            self.theme,
            &self.ui_model.pending_steer,
            &self.ui_model.pending_follow_up,
            width,
        )
    }

    pub(super) fn render_status_slot(&mut self, width: usize) -> Vec<String> {
        if !self.status_busy {
            // Idle breathing room above editor (status.md / agent_demo status_lines).
            return vec![String::new()];
        }
        // Keep Loader leading blank + spinner row (do not strip empties).
        self.status_loader.render(width)
    }

    pub(super) fn render_editor_slot(&mut self, width: usize) -> Vec<String> {
        match self.slot {
            EditorSlot::Editor => self.editor.render(width.max(1)),
            EditorSlot::Tree => {
                let mut lines = Vec::new();
                lines.push(" Session tree".to_string());
                if let Some((_, ref mut input)) = self.tree_label_edit {
                    lines.push(
                        self.theme
                            .paint_muted(" Label edit · Enter save · Esc cancel"),
                    );
                    lines.extend(input.render(width.max(1)));
                    return lines;
                }
                // pi order: TreeHelp then SearchLine.
                for help in wrap_help_line(&tree_help_line(), width.max(1)) {
                    lines.push(self.theme.paint_muted(&help));
                }
                lines.push(
                    self.theme
                        .paint_muted(&tree_search_line(self.tree.search_query())),
                );
                lines.extend(self.tree.render(width.max(1)));
                lines
            }
            EditorSlot::Plate => vec![
                " Command Plate".to_string(),
                " (stub) Esc close".to_string(),
            ],
            EditorSlot::Settings => vec![" Settings".to_string(), " (stub) Esc close".to_string()],
            EditorSlot::Choice => vec![" Choice".to_string(), " (stub) Esc close".to_string()],
            EditorSlot::Models => {
                let mut lines = Vec::new();
                lines.push(self.models_filter_line());
                lines.extend(self.models_list.render(width.max(1)));
                lines
            }
            EditorSlot::Themes => {
                let mut lines = Vec::new();
                lines.push(self.theme.paint_muted(" themes"));
                lines.extend(self.themes_list.render(width.max(1)));
                lines
            }
            EditorSlot::ImportConfirm => {
                let mut lines = Vec::new();
                let path = self.import_confirm_path.as_deref().unwrap_or("?");
                lines.push(
                    self.theme
                        .paint_muted(&format!(" Replace current session with {path}?")),
                );
                lines.extend(self.import_confirm_list.render(width.max(1)));
                lines
            }
            EditorSlot::SessionResume => self.session_resume.render(width.max(1)),
        }
    }

    pub(super) fn render_scrollback_slot(&mut self, width: usize) -> Vec<String> {
        // Idle empty: 0 rows (DESIGN editor.md — no loud placeholder wall).
        render_scrollback(&self.ui_model, self.glyphs, self.theme, self.fold, width)
    }
}

impl Component for UiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if self.upper_cache_width == width && self.upper_cache_gen == self.upper_gen {
            lines.extend(self.upper_cache_lines.iter().cloned());
        } else {
            let mut upper = Vec::new();
            upper.extend(self.render_loaded_resources_slot(width));
            upper.extend(self.render_scrollback_slot(width));
            // Queue strip sits between transcript and status (pi morphology).
            upper.extend(self.render_queue_slot(width));
            self.upper_cache_width = width;
            self.upper_cache_gen = self.upper_gen;
            self.upper_cache_lines = upper.clone();
            #[cfg(test)]
            {
                self.upper_rebuild_count = self.upper_rebuild_count.saturating_add(1);
            }
            lines.extend(upper);
        }
        lines.extend(self.render_status_slot(width));
        // Editor owns the operation-zone ─ borders (DESIGN editor.md / agent_demo).
        // Do NOT wrap with a second outer border pair.
        lines.extend(self.render_editor_slot(width));
        let footer = if width == 0 {
            self.footer.text().to_string()
        } else {
            truncate_to_width(self.footer.text(), width, "...", true)
        };
        lines.push(footer);
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.handle_slot_input(event);
    }

    fn invalidate(&mut self) {
        self.status_loader.invalidate();
        self.editor.invalidate();
        self.footer.invalidate();
        self.tree.invalidate();
        self.models_list.invalidate();
        self.import_confirm_list.invalidate();
        self.session_resume.invalidate();
    }

    fn tick(&mut self) -> bool {
        let mut dirty = self.editor.tick();
        if self.status_busy {
            let interval = self.status_loader.interval_ms() as u128;
            if self.loader_last_tick.elapsed().as_millis() >= interval {
                dirty = Component::tick(&mut self.status_loader) || dirty;
                self.loader_last_tick = Instant::now();
            }
        }
        dirty
    }
}
