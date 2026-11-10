use std::time::Instant;

use xylitol_tui::{
    Component, InputEvent, PanelBounds, PanelOverflow, PanelPlacement, ScreenRect, SizeValue,
    truncate_to_width,
};

use super::super::slots::EditorSlot;
use super::UiRoot;
use crate::widgets::{render_queue_strip, render_scrollback};

fn wrap_marker_row(row: &mut String, left: &str, marker: &str) {
    if row.starts_with(' ') {
        row.replace_range(0..1, left);
    } else {
        row.insert_str(0, left);
    }
    if row.ends_with("  ") {
        let end = row.len();
        row.replace_range(end - 2..end, &format!(" {marker}"));
    } else if row.ends_with(' ') {
        let end = row.len();
        row.replace_range(end - 1..end, marker);
    } else {
        row.push_str(marker);
    }
}

fn wrap_content_row(row: &mut String, left: &str, right: &str) {
    if row.starts_with(' ') {
        row.replace_range(0..1, left);
    } else {
        row.insert_str(0, left);
    }
    if row.ends_with(' ') {
        let end = row.len();
        row.replace_range(end - 1..end, right);
    } else {
        row.push_str(right);
    }
}

impl UiRoot {
    fn paint_editor_chips(&mut self, editor: &mut [String], width: usize) -> bool {
        let top_base = editor.first().cloned().unwrap_or_default();
        let (border, chip_hits) = if self.slot.is_editor() {
            self.append_border_chips(&top_base, width)
        } else {
            (String::new(), Vec::new())
        };
        let chip_line_width = xylitol_tui::visible_width(&border);
        let has_top = !border.is_empty();
        if has_top && let Some(first) = editor.first_mut() {
            *first = border;
        }
        let mut chip_hits = chip_hits;
        let bottom_base = editor.last().cloned().unwrap_or_default();
        if self.slot.is_editor()
            && editor.len() > 1
            && !self.editor_autocomplete_open()
            && let Some(last) = editor.last_mut()
        {
            let (bottom, bottom_hits) = self.paint_bottom_chrome(&bottom_base, width);
            if !bottom.is_empty() {
                *last = bottom;
                chip_hits.extend(bottom_hits);
            }
        }
        if self.slot.is_editor() && editor.len() > 2 && !self.editor_autocomplete_open() {
            let border = self.operation_zone_paint();
            let buttons = self.chrome_button_paint();
            let left = border("│");
            let right = border("│");
            let prompt = self.theme.paint_user(self.glyphs.user());
            let marker = if self.session_sidebar_available() {
                self.theme.paint_chip("◨", true)
            } else {
                buttons("◨")
            };
            let last_content = editor.len() - 2;
            for row in editor.iter_mut().take(last_content).skip(1) {
                wrap_content_row(row, &left, &right);
            }
            if let Some(row) = editor.get_mut(last_content) {
                if width >= 80 {
                    wrap_marker_row(row, &prompt, &marker);
                } else {
                    wrap_content_row(row, &prompt, &right);
                }
            }
        }
        self.last_slot_close_hit = None;
        if !self.slot.is_editor() {
            self.paint_slot_close_chip(editor, width);
        }
        self.last_chip_width = chip_line_width;
        self.last_footer_hits = chip_hits;
        has_top
    }

    fn paint_slot_close_chip(&mut self, editor: &mut [String], width: usize) {
        if width <= 12 {
            return;
        }
        let Some(first) = editor.first_mut() else {
            return;
        };
        let chip = self.theme.paint_chip("[关闭]", false);
        let chip_w = xylitol_tui::visible_width("[关闭]");
        let base = truncate_to_width(
            &xylitol_tui::utils::strip_ansi_codes(first),
            width.saturating_sub(chip_w + 1),
            "…",
            false,
        );
        let pad = width
            .saturating_sub(chip_w)
            .saturating_sub(xylitol_tui::visible_width(&base));
        *first = format!("{base}{}{chip}", " ".repeat(pad));
        self.last_slot_close_hit = Some((width - chip_w, width));
    }

    fn editor_top_row(&self, dock_prefix: usize, editor_rows: usize) -> usize {
        let dock_total = dock_prefix.saturating_add(editor_rows).saturating_add(1);
        self.term_rows
            .saturating_sub(dock_total)
            .saturating_add(dock_prefix)
    }

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
                expanded: self.todo_expanded,
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

    pub(super) fn render_tray_slot(&mut self, width: usize) -> Vec<String> {
        if self.mcp_tray.is_empty() {
            self.last_tray_hits.clear();
            self.last_tray_rows = 0;
            return Vec::new();
        }
        let lead = " attach";
        let mut line = self.theme.paint_muted(lead).to_string();
        let mut col = xylitol_tui::visible_width(lead);
        let mut hits = Vec::with_capacity(self.mcp_tray.len());
        let mut truncated = false;
        for (index, r) in self.mcp_tray.refs().iter().enumerate() {
            let chip = format!("[{}/{} ×]", r.server, r.uri);
            let chip_w = xylitol_tui::visible_width(&chip);
            if col + 1 + chip_w > width {
                truncated = true;
                break;
            }
            if index > 0 {
                line.push(' ');
                col += 1;
            }
            line.push_str(&self.theme.paint_chip(&chip, false));
            hits.push(super::TrayChipHit {
                col_start: col + chip_w.saturating_sub(3),
                col_end: col + chip_w,
                index,
            });
            col += chip_w;
        }
        let mut lines = Vec::new();
        if truncated {
            line.push_str(&self.theme.paint_muted(" …"));
            line = truncate_to_width(&line, width.max(1), "…", false);
        }
        lines.push(line);
        self.last_tray_hits = hits;
        self.last_tray_rows = lines.len();
        lines
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

        if width == 0 {
            return Vec::new();
        }
        if !self.status_busy {
            let cue = self
                .status_next_turn_cue
                .as_deref()
                .map(|cue| paint_cue_line(&self.theme, cue, width))
                .unwrap_or_default();
            return vec![cue];
        }
        let mut lines = self.status_loader.render(width);
        if lines.first().is_some_and(|line| line.is_empty()) {
            lines.remove(0);
        }
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

    pub(super) fn render_editor_slot(&mut self, width: usize) -> Vec<String> {
        match &mut self.slot {
            EditorSlot::Editor => self.editor.render(width.max(1)),
            EditorSlot::Tree(tree) => tree.render(width, self.theme),
            slot @ (EditorSlot::Plate | EditorSlot::Settings) => vec![
                format!(" {}", slot.title()),
                " (stub) Esc close".to_string(),
            ],
            EditorSlot::Choice(ask) => ask.render(width),
            EditorSlot::Models(models) => models.render(width, self.theme),
            EditorSlot::Scenes(scenes) => scenes.render(width, self.theme),
            EditorSlot::Themes(themes) => themes.render(width, self.theme),
            EditorSlot::ImportConfirm(imp) => imp.render(width, self.theme),
            EditorSlot::SessionResume(panel) => panel.render(width.max(1)),
            EditorSlot::Mcp(mcp) => mcp.render(width, self.theme),
            EditorSlot::McpPrompts(prompts) => prompts.render(width, self.theme),
            EditorSlot::McpResources(resources) => resources.render(width, self.theme),
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

impl UiRoot {
    pub(crate) fn handle_sidebar_drag_pointer(
        &mut self,
        mouse: &crossterm::event::MouseEvent,
    ) -> bool {
        use crossterm::event::{MouseButton, MouseEventKind};

        let (placement, panel_w, content_w, panel_h) = self.session_sidebar_layout(self.term_cols);
        let Some(placement) = placement else {
            return false;
        };
        let (separator, separator_height) = match placement {
            PanelPlacement::DockRight => (content_w, self.transcript_rows()),
            PanelPlacement::FloatRight => (self.term_cols.saturating_sub(panel_w), panel_h),
            _ => return false,
        };
        let in_separator =
            mouse.column as usize == separator && (mouse.row as usize) < separator_height;
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) if in_separator => {
                self.session_sidebar_dragging = true;
                self.session_sidebar_drag_moved = false;
                self.session_sidebar_drag_start_col = mouse.column;
                self.session_sidebar_drag_start_width = self.session_sidebar_width;
                true
            }
            MouseEventKind::Drag(MouseButton::Left) if self.session_sidebar_dragging => {
                let delta = self.session_sidebar_drag_start_col as isize - mouse.column as isize;
                let width =
                    (self.session_sidebar_drag_start_width as isize + delta).max(24) as usize;
                self.session_sidebar_width = width.min(120);
                self.session_sidebar_layout_dirty = true;
                self.session_sidebar_drag_moved = true;
                true
            }
            MouseEventKind::Up(MouseButton::Left) if self.session_sidebar_dragging => {
                self.session_sidebar_dragging = false;
                if !self.session_sidebar_drag_moved {
                    self.session_sidebar_width_cycle();
                    self.session_sidebar_layout_dirty = true;
                }
                true
            }
            _ => false,
        }
    }
}

impl Component for UiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        let term_cols = self.term_cols.max(width).max(1);
        let (placement, panel_w, content_w, panel_h) = self.session_sidebar_layout(term_cols);
        let upper_w = match placement {
            Some(PanelPlacement::DockRight) => content_w,
            _ => term_cols,
        };

        let mut lines = Vec::new();
        if self.upper_cache_width == upper_w && self.upper_cache_gen == self.upper_gen {
            lines.extend(self.upper_cache_lines.iter().cloned());
        } else {
            let mut upper = Vec::new();
            let loaded = self.render_loaded_resources_slot(upper_w);
            let loaded_rows = loaded.len();
            upper.extend(loaded);
            upper.extend(self.render_scrollback_slot(upper_w, loaded_rows));
            self.upper_cache_width = upper_w;
            self.upper_cache_gen = self.upper_gen;
            self.upper_cache_lines = upper.clone();
            #[cfg(test)]
            {
                self.upper_rebuild_count = self.upper_rebuild_count.saturating_add(1);
            }
            lines.extend(upper);
        }

        if let Some(placement) = placement {
            let panel_content_w = panel_w.saturating_sub(1).max(1);
            let panel_col = match placement {
                PanelPlacement::DockRight => content_w,
                _ => term_cols.saturating_sub(panel_w),
            };
            self.session_sidebar_panel.set_bounds(PanelBounds {
                width: SizeValue::Absolute(panel_content_w),
                min_width: panel_content_w.min(Self::SESSION_SIDEBAR_WIDTHS[0]).max(1),
                max_width: Some(panel_content_w),
                min_height: Self::SESSION_SIDEBAR_CONTENT_ROWS,
                max_height: SizeValue::Absolute(panel_h.max(Self::SESSION_SIDEBAR_CONTENT_ROWS)),
                overflow: PanelOverflow::Scroll,
            });
            self.session_sidebar_panel.set_available(ScreenRect {
                row: 0,
                col: panel_col.min(u16::MAX as usize) as u16,
                height: panel_h.min(u16::MAX as usize) as u16,
                width: panel_content_w.min(u16::MAX as usize) as u16,
            });
            self.session_sidebar_panel
                .set_scroll(self.session_sidebar_scroll);
            let panel_lines = self.session_sidebar_panel.render(panel_content_w);
            let actual_h = panel_lines.len().min(panel_h);
            self.session_sidebar_panel_lines = panel_lines.clone();
            self.session_sidebar_panel_rect = Some(ScreenRect {
                row: 0,
                col: panel_col.min(u16::MAX as usize) as u16,
                height: actual_h.min(u16::MAX as usize) as u16,
                width: panel_w.min(u16::MAX as usize) as u16,
            });
        } else {
            self.session_sidebar_panel_lines.clear();
            self.session_sidebar_panel_rect = None;
        }

        let queue = self.render_queue_slot(term_cols);
        let todo = self.render_todo_bar_slot(term_cols);
        let toast = self.render_toast_notice_slot(term_cols);
        let status = self.render_status_slot(term_cols);
        let tray = self.render_tray_slot(term_cols);
        self.apply_fixed_zone_footprint(todo.len());
        let mut editor = self.render_editor_slot(term_cols);
        let has_chips = self.paint_editor_chips(&mut editor, term_cols);
        let dock_prefix = queue
            .len()
            .saturating_add(todo.len())
            .saturating_add(toast.len())
            .saturating_add(status.len())
            .saturating_add(tray.len());
        let editor_top = self.editor_top_row(dock_prefix, editor.len());
        self.last_slot_top_row = Some(editor_top);
        self.last_chip_row = has_chips.then_some(editor_top);
        let footer = if term_cols == 0 {
            self.footer.text().to_string()
        } else {
            truncate_to_width(self.footer.text(), term_cols, "...", true)
        };
        self.last_queue_rows = queue.len();
        self.last_todo_rows = todo.len();
        self.last_toast_rows = toast.len();
        self.last_status_rows = status.len();
        self.last_tray_rows = tray.len();
        self.last_editor_rows = editor.len();
        self.last_dock_rows = queue
            .len()
            .saturating_add(todo.len())
            .saturating_add(toast.len())
            .saturating_add(status.len())
            .saturating_add(tray.len())
            .saturating_add(editor.len())
            .saturating_add(1);
        self.sync_editor_screen_origin();
        self.queue_strip_row = lines.len();
        lines.extend(queue);
        lines.extend(todo);
        lines.extend(toast);
        lines.extend(status);
        self.tray_strip_row = lines.len();
        lines.extend(tray);
        lines.extend(editor);
        lines.push(footer);
        lines
    }

    fn overlay_viewport(&self, lines: &mut [String], width: usize, transcript_rows: usize) {
        let term_cols = self.term_cols.max(width).max(1);
        let (placement, panel_w, content_w, panel_h) = self.session_sidebar_layout(term_cols);
        let eff = match placement {
            Some(PanelPlacement::DockRight) => content_w,
            _ => term_cols,
        };

        self.overlay_client_notices(lines, eff, transcript_rows);
        let sep = self.theme.paint_muted("│");
        let panel_content_w = panel_w.saturating_sub(1).max(1);
        let actual_h = self.session_sidebar_panel_lines.len().min(panel_h);
        match placement {
            Some(PanelPlacement::DockRight) => {
                let rows = transcript_rows.min(lines.len());
                for (i, line) in lines.iter_mut().enumerate().take(rows) {
                    let content = truncate_to_width(line, content_w, "", false);
                    let cw = xylitol_tui::visible_width(&content);
                    let pad = " ".repeat(content_w.saturating_sub(cw));
                    if i < actual_h {
                        let cell = self
                            .session_sidebar_panel_lines
                            .get(i)
                            .cloned()
                            .unwrap_or_default();
                        let cell = truncate_to_width(&cell, panel_content_w, "", false);
                        let cw2 = xylitol_tui::visible_width(&cell);
                        let cpad = " ".repeat(panel_content_w.saturating_sub(cw2));
                        *line = format!("{content}{pad}{sep}{cell}{cpad}");
                    } else {
                        *line = format!("{content}{pad}{sep}");
                    }
                }
            }
            Some(PanelPlacement::FloatRight) => {
                let panel_col = term_cols.saturating_sub(panel_w);
                let rows = actual_h.min(transcript_rows).min(lines.len());
                for (i, line) in lines.iter_mut().enumerate().take(rows) {
                    let content = truncate_to_width(line, panel_col, "", false);
                    let cw = xylitol_tui::visible_width(&content);
                    let pad = " ".repeat(panel_col.saturating_sub(cw));
                    let cell = self
                        .session_sidebar_panel_lines
                        .get(i)
                        .cloned()
                        .unwrap_or_default();
                    let cell = truncate_to_width(&cell, panel_content_w, "", false);
                    let cw2 = xylitol_tui::visible_width(&cell);
                    let cpad = " ".repeat(panel_content_w.saturating_sub(cw2));
                    *line = format!("{content}{pad}{sep}{cell}{cpad}");
                }
            }
            _ => {}
        }
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.slot_pointer_dirty = false;
        self.handle_slot_input(event);
    }

    fn offer_pointer(&mut self, event: &InputEvent) -> bool {
        let InputEvent::Mouse(mouse) = event else {
            return false;
        };
        self.handle_sidebar_drag_pointer(mouse) || self.handle_slot_detail_pointer(mouse)
    }

    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        if matches!(event, InputEvent::Mouse(_)) {
            self.slot_pointer_dirty
        } else {
            true
        }
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
