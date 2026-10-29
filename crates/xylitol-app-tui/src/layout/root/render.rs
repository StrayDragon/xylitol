use std::time::Instant;

use xylitol_tui::{Component, InputEvent, truncate_to_width};

use super::super::slots::EditorSlot;
use super::UiRoot;
use crate::widgets::{render_queue_strip, render_scrollback};

impl UiRoot {
    pub(super) fn render_loaded_resources_slot(&mut self, width: usize) -> Vec<String> {
        crate::widgets::render_loaded_resources(
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
        let max_rows = crate::widgets::todo_bar_max_rows(self.term_rows);
        let mut frame = crate::widgets::render_todo_bar(
            self.theme,
            self.glyphs,
            crate::widgets::TodoBarParams {
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

    pub(super) fn render_mode_strip_slot(&mut self, width: usize) -> Vec<String> {
        let Some(state) = self.ui_model.mode_strip.as_ref() else {
            return Vec::new();
        };
        vec![crate::widgets::render_mode_strip(
            state,
            self.glyphs,
            width,
            &self.theme,
        )]
    }

    pub(super) fn render_toast_notice_slot(&mut self, width: usize) -> Vec<String> {
        let Some((body, _, _)) = self.toast_notice.as_ref() else {
            return Vec::new();
        };
        let line = format!("{}{body}", crate::commands::TOAST_NOTICE_ERROR_PREFIX);
        let painted = self.theme.paint_warning(&line);
        if width == 0 {
            return vec![painted];
        }
        vec![truncate_to_width(&painted, width, "…", false)]
    }

    pub(super) fn overlay_session_popup(
        &self,
        lines: &mut [String],
        width: usize,
        transcript_rows: usize,
    ) {
        if !self.session_popup_open || width == 0 {
            return;
        }
        let box_w = Self::SESSION_POPUP_W.min(width.saturating_sub(2));
        if box_w == 0 {
            return;
        }
        let cap = transcript_rows.min(lines.len());
        let title = "┌ 会话详情";
        let title_w = xylitol_tui::visible_width(title);
        let title_paint = self.theme.paint_muted(&format!(
            "{title} {}",
            "─".repeat(box_w.saturating_sub(title_w + 1))
        ));
        let fields: [(&str, &String); 4] = [
            ("会话", &self.session_popup_id),
            ("模型", &self.model),
            ("思考", &self.thinking_level),
            ("目录", &self.cwd),
        ];
        let mut body: Vec<String> = vec![title_paint];
        for (label, value) in fields {
            body.push(format!("{} {}", self.theme.paint_muted(label), value));
        }
        for (i, row) in body.iter().enumerate() {
            if i >= cap {
                break;
            }
            let block = truncate_to_width(row, box_w, "…", false);
            let bw = xylitol_tui::visible_width(&block);
            let budget = width.saturating_sub(box_w);
            let left = truncate_to_width(&lines[i], budget, "…", false);
            let lw = xylitol_tui::visible_width(&left);
            let pad = " ".repeat(budget.saturating_sub(lw));
            let right_pad = " ".repeat(box_w.saturating_sub(bw));
            lines[i] = format!("{left}{pad}{block}{right_pad}");
        }
    }

    pub(super) fn overlay_client_notices(
        &self,
        lines: &mut [String],
        width: usize,
        transcript_rows: usize,
    ) {
        if width == 0 {
            return;
        }
        let cap = transcript_rows.min(lines.len());
        let now = Instant::now();
        for (i, notice) in self
            .client_notices
            .iter()
            .filter(|n| now < n.until)
            .enumerate()
        {
            if i >= cap {
                break;
            }
            let raw = match notice.kind {
                super::ClientNoticeKind::Success => format!("✓ {}", notice.body),
                super::ClientNoticeKind::Info => format!("◆ {}", notice.body),
            };
            let painted = match notice.kind {
                super::ClientNoticeKind::Success => self.theme.paint_success(&raw),
                super::ClientNoticeKind::Info => self.theme.paint_muted(&raw),
            };
            let overlay = truncate_to_width(&painted, width, "…", false);
            let ow = xylitol_tui::visible_width(&overlay);
            let budget = width.saturating_sub(ow);
            let left = if budget == 0 {
                String::new()
            } else {
                let truncated = truncate_to_width(&lines[i], budget, "…", false);
                let lw = xylitol_tui::visible_width(&truncated);
                format!("{truncated}{}", " ".repeat(budget.saturating_sub(lw)))
            };
            lines[i] = format!("{left}{overlay}");
        }
    }

    pub(super) fn render_status_slot(&mut self, width: usize) -> Vec<String> {
        let paint_cue_line = |theme: &crate::layout::LayoutTheme, cue: &str, width: usize| {
            let cue_paint = theme.paint_muted(cue);
            let cue_w = xylitol_tui::visible_width(&cue_paint);
            if cue_w >= width {
                return truncate_to_width(&cue_paint, width, "…", false);
            }
            let pad = width.saturating_sub(cue_w);
            format!("{}{cue_paint}", " ".repeat(pad))
        };

        if !self.status_busy {
            if let Some(cue) = self.status_next_turn_cue.as_deref() {
                return vec![paint_cue_line(&self.theme, cue, width)];
            }
            return vec![String::new()];
        }
        let mut lines = self.status_loader.render(width);
        if let Some(cue) = self.status_next_turn_cue.as_deref() {
            if let Some(content) = lines.last_mut() {
                let cue_paint = self.theme.paint_muted(cue);
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

    pub(super) fn session_sidebar_cell(&self, row: usize, w: usize) -> String {
        let row = row.saturating_add(self.session_sidebar_scroll);
        if row == 0 {
            let title = "┌ 会话侧栏";
            let tw = xylitol_tui::visible_width(title);
            return self
                .theme
                .paint_muted(&format!("{title} {}", "─".repeat(w.saturating_sub(tw + 1))));
        }
        match row {
            1 => self.theme.paint_muted("实验特性 · 施工中"),
            2 => self.theme.paint_muted("正式业务后启用"),
            _ => String::new(),
        }
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
            EditorSlot::Scenes(scenes) => scenes.render(width, self.theme),
            EditorSlot::Themes(themes) => themes.render(width, self.theme),
            EditorSlot::ImportConfirm(imp) => imp.render(width, self.theme),
            EditorSlot::SessionResume(panel) => panel.render(width.max(1)),
            EditorSlot::Mcp(mcp) => mcp.render(width, self.theme),
            EditorSlot::Capabilities(caps) => caps.render(width, self.theme),
        }
    }

    pub(super) fn render_scrollback_slot(
        &mut self,
        width: usize,
        content_row_offset: usize,
    ) -> Vec<String> {
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
        let (sidebar_on, _sidebar_w, content_w) = self.session_sidebar_layout(width);
        let width = if sidebar_on { content_w } else { width };
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
        let queue = self.render_queue_slot(width);
        let todo = self.render_todo_bar_slot(width);
        let mode = self.render_mode_strip_slot(width);
        let toast = self.render_toast_notice_slot(width);
        let status = self.render_status_slot(width);
        self.apply_fixed_zone_footprint(todo.len());
        let editor = self.render_editor_slot(width);
        let footer = if width == 0 {
            self.footer.text().to_string()
        } else {
            truncate_to_width(self.footer.text(), width, "...", true)
        };
        self.last_queue_rows = queue.len();
        self.last_todo_rows = todo.len();
        self.last_toast_rows = toast.len();
        self.last_status_rows = status.len();
        self.last_editor_rows = editor.len();
        self.last_dock_rows = queue
            .len()
            .saturating_add(todo.len())
            .saturating_add(mode.len())
            .saturating_add(toast.len())
            .saturating_add(status.len())
            .saturating_add(editor.len())
            .saturating_add(1);
        self.sync_editor_screen_origin();
        lines.extend(queue);
        lines.extend(todo);
        lines.extend(mode);
        lines.extend(toast);
        lines.extend(status);
        lines.extend(editor);
        lines.push(footer);
        lines
    }

    fn overlay_viewport(&self, lines: &mut [String], width: usize, transcript_rows: usize) {
        let (sidebar_on, sidebar_w, content_w) = self.session_sidebar_layout(width);
        let eff = if sidebar_on { content_w } else { width };
        self.overlay_session_popup(lines, eff, transcript_rows);
        self.overlay_client_notices(lines, eff, transcript_rows);
        if sidebar_on {
            let sep = self.theme.paint_muted("│");
            for (i, line) in lines.iter_mut().enumerate() {
                let content = truncate_to_width(line, eff, "", false);
                let cw = xylitol_tui::visible_width(&content);
                let pad = " ".repeat(eff.saturating_sub(cw));
                let cell = self.session_sidebar_cell(i, sidebar_w.saturating_sub(1));
                let cell = truncate_to_width(&cell, sidebar_w.saturating_sub(1), "", false);
                let cw2 = xylitol_tui::visible_width(&cell);
                let cpad = " ".repeat(sidebar_w.saturating_sub(1).saturating_sub(cw2));
                *line = format!("{content}{pad}{sep}{cell}{cpad}");
            }
        }
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
