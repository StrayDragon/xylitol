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

    pub(super) fn render_chrome_toast_slot(&mut self, width: usize) -> Vec<String> {
        let Some((body, _)) = self.chrome_toast.as_ref() else {
            return Vec::new();
        };
        let line = format!(
            "{}{body}",
            crate::app::tui::commands::CHROME_TOAST_ERROR_PREFIX
        );
        let painted = self.theme.paint_warning(&line);
        if width == 0 {
            return vec![painted];
        }
        vec![truncate_to_width(&painted, width, "…", false)]
    }

    pub(super) fn render_status_slot(&mut self, width: usize) -> Vec<String> {
        let paint_cue_line =
            |theme: &crate::app::tui::layout::LayoutTheme, cue: &str, width: usize| {
                let cue_paint = theme.paint_muted(cue);
                let cue_w = xylitol_tui::visible_width(&cue_paint);
                if cue_w >= width {
                    return truncate_to_width(&cue_paint, width, "…", false);
                }
                let pad = width.saturating_sub(cue_w);
                format!("{}{cue_paint}", " ".repeat(pad))
            };

        if !self.status_busy {
            // Idle: optional MCP short cue (c1210), right-aligned; otherwise breathing room.
            if let Some(cue) = self.status_next_turn_cue.as_deref() {
                return vec![paint_cue_line(&self.theme, cue, width)];
            }
            return vec![String::new()];
        }
        // Keep Loader leading blank + spinner row (do not strip empties).
        let mut lines = self.status_loader.render(width);
        if let Some(cue) = self.status_next_turn_cue.as_deref() {
            if let Some(content) = lines.last_mut() {
                let cue_paint = self.theme.paint_muted(cue);
                // Loader → Text pads each line to full `width` with trailing spaces.
                // Measuring that padded line makes lead_w == width and silently drops the
                // right-aligned cue (Assembling / Working + mcp pending / Next turn).
                let lead = content.trim_end_matches(' ');
                let lead_w = xylitol_tui::visible_width(lead);
                let cue_w = xylitol_tui::visible_width(&cue_paint);
                if lead_w + 1 + cue_w <= width {
                    let pad = width.saturating_sub(lead_w + cue_w);
                    *content = format!("{lead}{}{cue_paint}", " ".repeat(pad));
                } else if cue_w < width {
                    let budget = width.saturating_sub(lead_w.saturating_add(1));
                    if budget > 3 {
                        let truncated = truncate_to_width(&cue_paint, budget, "…", false);
                        let pad =
                            width.saturating_sub(lead_w + xylitol_tui::visible_width(&truncated));
                        *content = format!("{lead}{}{truncated}", " ".repeat(pad.max(1)));
                    }
                }
            } else {
                lines.push(paint_cue_line(&self.theme, cue, width));
            }
        }
        lines
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
            EditorSlot::Choice => {
                // Brand lives on scrollback header (`Ask · …`); no redundant slot caption.
                let mut lines = Vec::new();
                if let Some(ref mut prompt) = self.choice_prompt {
                    lines.extend(prompt.render(width.max(1)));
                }
                lines
            }
            EditorSlot::Models => {
                let w = width.max(1);
                if self.models_last_width != w {
                    self.models_last_width = w;
                    self.rebuild_models_items_keep_selection();
                }
                let mut lines = Vec::new();
                lines.push(self.models_filter_line());
                lines.extend(self.models_list.render(w));
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
            EditorSlot::Mcp => {
                let mut lines = Vec::new();
                lines.push(
                    self.theme
                        .paint_muted(&format!(" MCP · {}", self.mcp_summary_line)),
                );
                lines.extend(self.mcp_list.render(width.max(1)));
                for diag in &self.mcp_diag_lines {
                    lines.push(self.theme.paint_muted(diag));
                }
                lines.push(self.theme.paint_muted(" Esc · Enter closes"));
                lines
            }
        }
    }

    pub(super) fn render_scrollback_slot(&mut self, width: usize) -> Vec<String> {
        // Idle empty: 0 rows (DESIGN editor.md — no loud placeholder wall).
        render_scrollback(
            &self.ui_model,
            self.glyphs,
            self.theme,
            self.fold,
            width,
            &mut self.scrollback_paint,
        )
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
        let toast = self.render_chrome_toast_slot(width);
        let status = self.render_status_slot(width);
        // Editor owns the operation-zone ─ borders (DESIGN editor.md / agent_demo).
        // Do NOT wrap with a second outer border pair.
        self.apply_chrome_footprint();
        let editor = self.render_editor_slot(width);
        let footer = if width == 0 {
            self.footer.text().to_string()
        } else {
            truncate_to_width(self.footer.text(), width, "...", true)
        };
        // Mode B dock = everything below loaded+scrollback+queue (ath30 / ptim06).
        self.last_mode_b_dock_rows = toast
            .len()
            .saturating_add(status.len())
            .saturating_add(editor.len())
            .saturating_add(1);
        lines.extend(toast);
        lines.extend(status);
        lines.extend(editor);
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
        dirty = self.clear_chrome_toast_if_expired() || dirty;
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
