//! Slot render helpers for UiRoot (c1170).

use std::time::Instant;

use xylitol_tui::{Component, InputEvent, truncate_to_width};

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

    pub(super) fn render_todo_bar_slot(&mut self, width: usize) -> Vec<String> {
        if self.ui_model.todo.is_empty() {
            self.todo_scroll = 0;
            self.last_todo_hits.clear();
            self.last_todo_plain.clear();
            self.todo_selection.clear();
            return Vec::new();
        }
        let max_rows = crate::app::tui::widgets::todo_bar_max_rows(self.term_rows);
        let mut frame = crate::app::tui::widgets::render_todo_bar(
            self.theme,
            self.glyphs,
            crate::app::tui::widgets::TodoBarParams {
                list: &self.ui_model.todo,
                doing_open: self.todo_doing_open,
                past_open: self.todo_past_open,
                pending_open: self.todo_pending_open,
                scroll: self.todo_scroll,
                width,
                max_rows,
            },
        );
        self.last_todo_hits = frame.hits;
        self.last_todo_plain = frame.plain;
        if self.todo_selection.has_selection() || self.todo_selection.is_dragging() {
            self.todo_selection.apply_highlight(&mut frame.lines, 0);
        }
        frame.lines
    }

    pub(super) fn render_toast_notice_slot(&mut self, width: usize) -> Vec<String> {
        let Some((body, _, kind)) = self.toast_notice.as_ref() else {
            return Vec::new();
        };
        // resume 渲染修复: Info toasts (session-switch tips) render muted without the
        // `Error: ` prefix — they are transient hints, not failures.
        let line = match kind {
            crate::app::tui::layout::root::ToastKind::Error => format!(
                "{}{body}",
                crate::app::tui::commands::TOAST_NOTICE_ERROR_PREFIX
            ),
            crate::app::tui::layout::root::ToastKind::Info => body.clone(),
        };
        let painted = match kind {
            crate::app::tui::layout::root::ToastKind::Error => self.theme.paint_warning(&line),
            crate::app::tui::layout::root::ToastKind::Info => self.theme.paint_muted(&line),
        };
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
            // ApplicationOwned copy cue (ath31): reuse the blank status row when no next-turn cue
            // so dock height stays stable; never use Error: toast-notice.
            if let Some(cue) = self.status_next_turn_cue.as_deref() {
                let mut lines = vec![paint_cue_line(&self.theme, cue, width)];
                if self.copy_notice_visible() {
                    let painted = self.theme.paint_muted("Copied");
                    lines.push(truncate_to_width(&painted, width.max(1), "…", false));
                }
                return lines;
            }
            if self.copy_notice_visible() {
                let painted = self.theme.paint_muted("Copied");
                return vec![truncate_to_width(&painted, width.max(1), "…", false)];
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
        if self.copy_notice_visible() {
            let painted = self.theme.paint_muted("Copied");
            lines.push(truncate_to_width(&painted, width, "…", false));
        }
        lines
    }

    pub(super) fn render_editor_slot(&mut self, width: usize) -> Vec<String> {
        match &mut self.slot {
            EditorSlot::Editor => self.editor.render(width.max(1)),
            EditorSlot::Tree(tree) => tree.render(width, self.theme),
            EditorSlot::Plate => vec![
                " Command Plate".to_string(),
                " (stub) Esc close".to_string(),
            ],
            EditorSlot::Settings => vec![" Settings".to_string(), " (stub) Esc close".to_string()],
            EditorSlot::Choice(ask) => ask.render(width),
            EditorSlot::Models(models) => models.render(width, self.theme),
            EditorSlot::Themes(themes) => themes.render(width, self.theme),
            EditorSlot::ImportConfirm(imp) => imp.render(width, self.theme),
            EditorSlot::SessionResume(panel) => panel.render(width.max(1)),
            EditorSlot::Mcp(mcp) => mcp.render(width, self.theme),
        }
    }

    pub(super) fn render_scrollback_slot(
        &mut self,
        width: usize,
        content_row_offset: usize,
    ) -> Vec<String> {
        // Idle empty: 0 rows (DESIGN editor.md — no loud placeholder wall).
        let lines = render_scrollback(
            &self.ui_model,
            self.glyphs,
            self.theme,
            &self.fold,
            &mut self.activity,
            width,
            &mut self.scrollback_paint,
            &mut self.fold_hits,
        );
        if content_row_offset > 0 {
            for region in &mut self.fold_hits.regions {
                region.content_row = region.content_row.saturating_add(content_row_offset);
            }
        }
        lines
    }
}

impl Component for UiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        if self.upper_cache_width == width && self.upper_cache_gen == self.upper_gen {
            lines.extend(self.upper_cache_lines.iter().cloned());
        } else {
            let mut upper = Vec::new();
            let loaded = self.render_loaded_resources_slot(width);
            let loaded_rows = loaded.len();
            upper.extend(loaded);
            upper.extend(self.render_scrollback_slot(width, loaded_rows));
            self.upper_cache_width = width;
            self.upper_cache_gen = self.upper_gen;
            self.upper_cache_lines = upper.clone();
            #[cfg(test)]
            {
                self.upper_rebuild_count = self.upper_rebuild_count.saturating_add(1);
            }
            lines.extend(upper);
        }
        // Queue strip is part of the dock (between transcript and status) — must not
        // live in the ApplicationOwned ScrollView or short sessions pin it under
        // the startup card with a pad of empty rows (Inline stuck-to-bottom feel).
        let queue = self.render_queue_slot(width);
        let todo = self.render_todo_bar_slot(width);
        let toast = self.render_toast_notice_slot(width);
        let status = self.render_status_slot(width);
        // Editor owns the operation-zone ─ borders (DESIGN editor.md / agent_demo).
        // Do NOT wrap with a second outer border pair.
        self.apply_fixed_zone_footprint(todo.len());
        let editor = self.render_editor_slot(width);
        let footer = if width == 0 {
            self.footer.text().to_string()
        } else {
            truncate_to_width(self.footer.text(), width, "...", true)
        };
        // ApplicationOwned dock = queue + 待办栏 + toast + status + editor + footer.
        self.last_queue_rows = queue.len();
        self.last_todo_rows = todo.len();
        self.last_toast_rows = toast.len();
        self.last_status_rows = status.len();
        self.last_editor_rows = editor.len();
        self.last_dock_rows = queue
            .len()
            .saturating_add(todo.len())
            .saturating_add(toast.len())
            .saturating_add(status.len())
            .saturating_add(editor.len())
            .saturating_add(1);
        self.sync_editor_screen_origin();
        lines.extend(queue);
        lines.extend(todo);
        lines.extend(toast);
        lines.extend(status);
        lines.extend(editor);
        lines.push(footer);
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.handle_slot_input(event);
    }

    fn take_pending_clipboard(&mut self) -> Vec<String> {
        let mut seqs = self.editor.take_pending_clipboard();
        seqs.append(&mut self.todo_clipboard);
        seqs
    }

    fn invalidate(&mut self) {
        self.status_loader.invalidate();
        self.editor.invalidate();
        self.footer.invalidate();
        self.slot.invalidate();
    }

    fn tick(&mut self) -> bool {
        let mut dirty = self.editor.tick();
        dirty = self.clear_toast_notice_if_expired() || dirty;
        dirty = self.clear_copy_notice_if_expired() || dirty;
        if self.status_busy {
            let interval = self.status_loader.interval_ms() as u128;
            if self.loader_last_tick.elapsed().as_millis() >= interval {
                dirty = Component::tick(&mut self.status_loader) || dirty;
                self.loader_last_tick = Instant::now();
            }
        }
        dirty
    }

    fn clear_pointer_selection(&mut self) -> bool {
        let editor = Component::clear_pointer_selection(&mut self.editor);
        let todo = self.todo_selection.has_selection() || self.todo_selection.is_dragging();
        if todo {
            self.todo_selection.clear();
            self.todo_pointer_dirty = true;
        }
        editor || todo
    }
}
