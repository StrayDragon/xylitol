use super::types::{LayoutLine, TextChunk, VisualLine};
use crate::selection::ClipboardSink;
use crate::tui::{CURSOR_MARKER, Component, InputEvent};
use crate::utils::{is_whitespace_char, truncate_to_width, visible_width};
use crossterm::event::MouseEvent;
use unicode_segmentation::UnicodeSegmentation;

// ── word-wrap ───────────────────────────────────────────────────────────────

fn word_wrap_line(line: &str, max_width: usize) -> Vec<TextChunk> {
    let vis = visible_width(line);
    if vis <= max_width {
        return vec![TextChunk {
            text: line.to_string(),
            start_index: 0,
            end_index: line.len(),
        }];
    }
    let gs: Vec<(usize, &str, usize)> = UnicodeSegmentation::grapheme_indices(line, true)
        .map(|(i, g)| (i, g, visible_width(g)))
        .collect();
    let mut chunks = Vec::new();
    let (mut cur_w, mut start, mut w_i, mut w_w) = (0usize, 0usize, -1isize, 0usize);

    for (gi, &(idx, grav, gw)) in gs.iter().enumerate() {
        let first_ch = grav.chars().next().unwrap_or(' ');
        let is_ws = is_whitespace_char(first_ch);

        if cur_w + gw > max_width {
            if w_i >= 0 && cur_w - w_w + gw <= max_width {
                chunks.push(TextChunk {
                    text: line[start..w_i as usize].to_string(),
                    start_index: start,
                    end_index: w_i as usize,
                });
                start = w_i as usize;
                cur_w -= w_w;
            } else if start < idx {
                chunks.push(TextChunk {
                    text: line[start..idx].to_string(),
                    start_index: start,
                    end_index: idx,
                });
                start = idx;
                cur_w = 0;
            }
            w_i = -1;
        }

        if gw > max_width {
            let sub = word_wrap_line(grav, max_width);
            for sc in sub.iter().take(sub.len().saturating_sub(1)) {
                chunks.push(TextChunk {
                    text: sc.text.clone(),
                    start_index: idx + sc.start_index,
                    end_index: idx + sc.end_index,
                });
            }
            if let Some(last) = sub.last() {
                start = idx + last.start_index;
                cur_w = visible_width(&last.text);
            }
            w_i = -1;
            continue;
        }

        cur_w += gw;
        let next_ch = gs.get(gi + 1).and_then(|(_, g, _)| g.chars().next());
        if is_ws && next_ch.is_some_and(|c| !is_whitespace_char(c)) {
            w_i = gs.get(gi + 1).map(|(i, _, _)| *i).unwrap_or(0) as isize;
            w_w = cur_w;
        } else if !is_ws
            && next_ch.is_some_and(|c| !is_whitespace_char(c))
            && (grav.len() > 1 || gs.get(gi + 1).is_some_and(|(_, g, _)| g.len() > 1))
        {
            w_i = gs.get(gi + 1).map(|(i, _, _)| *i).unwrap_or(0) as isize;
            w_w = cur_w;
        }
    }
    chunks.push(TextChunk {
        text: line[start..].to_string(),
        start_index: start,
        end_index: line.len(),
    });
    chunks
}

impl super::Editor {
    // ── VisualLine system (c425 ed01) ──────────────────────────────────────

    pub fn build_visual_line_map(&self, width: usize) -> Vec<VisualLine> {
        let mut vls = Vec::new();
        for (i, line) in self.state.lines.iter().enumerate() {
            if line.is_empty() {
                vls.push(VisualLine {
                    logical_line: i,
                    start_col: 0,
                    len: 0,
                });
                continue;
            }
            let lv = visible_width(line);
            if lv <= width {
                vls.push(VisualLine {
                    logical_line: i,
                    start_col: 0,
                    len: line.len(),
                });
            } else {
                let chunks = word_wrap_line(line, width);
                for ch in &chunks {
                    vls.push(VisualLine {
                        logical_line: i,
                        start_col: ch.start_index,
                        len: ch.end_index - ch.start_index,
                    });
                }
            }
        }
        vls
    }

    pub(super) fn find_visual_line_at(&self, vls: &[VisualLine], line: usize, col: usize) -> usize {
        for (idx, vl) in vls.iter().enumerate() {
            if vl.logical_line != line {
                continue;
            }
            let offset = col.wrapping_sub(vl.start_col);
            let is_last = idx + 1 >= vls.len() || vls[idx + 1].logical_line != vl.logical_line;
            if offset < vl.len || (is_last && offset == vl.len) {
                return idx;
            }
        }
        vls.len().saturating_sub(1)
    }

    pub(super) fn find_current_visual_line(&self, vls: &[VisualLine]) -> usize {
        self.find_visual_line_at(vls, self.state.cursor_line, self.state.cursor_col)
    }

    // ── sticky column + vertical movement (c425 ed02) ─────────────────────

    pub(super) fn compute_vertical_move_column(
        &mut self,
        current_visual_col: usize,
        src_max: usize,
        tgt_max: usize,
    ) -> usize {
        let has_pref = self.preferred_visual_col.is_some(); // P
        let in_middle = current_visual_col < src_max; // S
        let tgt_too_short = tgt_max < current_visual_col; // T

        if !has_pref || in_middle {
            if tgt_too_short {
                // Cases 2 and 7: target shorter than current → remember current as preferred
                self.preferred_visual_col = Some(current_visual_col);
                return tgt_max;
            }
            // Cases 1 and 6: target fits → clear preferred
            self.preferred_visual_col = None;
            return current_visual_col;
        }

        let pref = self.preferred_visual_col.unwrap();
        let tgt_cant_fit_pref = tgt_max < pref; // U
        if tgt_too_short || tgt_cant_fit_pref {
            // Cases 4 and 5: keep preferred, go to end of target
            return tgt_max;
        }
        // Case 3: target fits preferred → use it and clear
        self.preferred_visual_col = None;
        pref
    }

    pub(super) fn move_to_visual_line(
        &mut self,
        vls: &[VisualLine],
        current_vl: usize,
        target_vl: usize,
    ) {
        let cur = &vls[current_vl];
        let tgt = &vls[target_vl];
        if cur.logical_line >= self.state.lines.len() || tgt.logical_line >= self.state.lines.len()
        {
            return;
        }

        let current_visual_col = if let Some(snapped) = self.snapped_from_cursor_col {
            let vi = self.find_visual_line_at(vls, cur.logical_line, snapped);
            snapped.wrapping_sub(vls[vi].start_col)
        } else {
            self.state.cursor_col.wrapping_sub(cur.start_col)
        };

        let is_last_src = target_vl as isize == -1 // never true in practice, placeholder for correct bound check
            || current_vl + 1 >= vls.len()
            || vls[current_vl + 1].logical_line != cur.logical_line;
        let src_max = if is_last_src {
            cur.len
        } else {
            cur.len.saturating_sub(1)
        };

        let is_last_tgt =
            target_vl + 1 >= vls.len() || vls[target_vl + 1].logical_line != tgt.logical_line;
        let tgt_max = if is_last_tgt {
            tgt.len
        } else {
            tgt.len.saturating_sub(1)
        };

        let move_to = self.compute_vertical_move_column(current_visual_col, src_max, tgt_max);

        self.state.cursor_line = tgt.logical_line;
        let target_col = tgt.start_col + move_to;
        let logical = &self.state.lines[tgt.logical_line];
        self.state.cursor_col = target_col.min(logical.len());

        // Snap to segment boundary for multi-grapheme units
        // Single-grapheme segments pass through
        let gs: Vec<(usize, &str)> =
            UnicodeSegmentation::grapheme_indices(logical.as_str(), true).collect();
        for (idx, grav) in &gs {
            if *idx > self.state.cursor_col {
                break;
            }
            if grav.len() <= 1 {
                continue;
            }
            let seg_end = idx + grav.len();
            if self.state.cursor_col < seg_end && *idx >= tgt.start_col {
                // Inside an atomic multi-grapheme segment → snap to its start
                self.snapped_from_cursor_col = Some(self.state.cursor_col);
                self.state.cursor_col = *idx;
                return;
            }
        }
        self.snapped_from_cursor_col = None;
    }

    // ── page scroll (c425 ed03) ────────────────────────────────────────────

    pub(super) fn page_scroll(&mut self, direction: isize) {
        self.last_action = None;
        let page_size = (self.terminal_rows * 30 / 100).max(5) as isize;
        let vls = self.build_visual_line_map(self.last_width);
        let current = self.find_current_visual_line(&vls) as isize;
        let target = (current + direction * page_size)
            .clamp(0, vls.len() as isize - 1)
            .max(0) as usize;
        if current as usize != target {
            self.move_to_visual_line(&vls, current as usize, target);
        }
    }

    // ── layout / render ────────────────────────────────────────────────────

    pub(super) fn layout_text(&self, content_width: usize) -> Vec<LayoutLine> {
        let mut layout = Vec::new();
        if self.state.lines.iter().all(|l| l.is_empty()) {
            layout.push(LayoutLine {
                text: String::new(),
                has_cursor: true,
                cursor_pos: Some(0),
                logical_line: 0,
                start_index: 0,
            });
            return layout;
        }
        for (i, line) in self.state.lines.iter().enumerate() {
            let is_cur = i == self.state.cursor_line;
            let lv = visible_width(line);
            if lv <= content_width {
                layout.push(LayoutLine {
                    text: line.clone(),
                    has_cursor: is_cur,
                    cursor_pos: is_cur.then_some(self.state.cursor_col),
                    logical_line: i,
                    start_index: 0,
                });
            } else {
                for (ci, chunk) in word_wrap_line(line, content_width).iter().enumerate() {
                    let is_last = ci == word_wrap_line(line, content_width).len() - 1;
                    let has = is_cur
                        && self.state.cursor_col >= chunk.start_index
                        && (is_last || self.state.cursor_col < chunk.end_index);
                    layout.push(LayoutLine {
                        text: chunk.text.clone(),
                        has_cursor: has,
                        cursor_pos: if has {
                            let cp = self.state.cursor_col.saturating_sub(chunk.start_index);
                            Some(cp.min(chunk.text.len()))
                        } else {
                            None
                        },
                        logical_line: i,
                        start_index: chunk.start_index,
                    });
                }
            }
        }
        layout
    }

    pub(super) fn is_editor_empty(&self) -> bool {
        self.state.lines.len() == 1 && self.state.lines[0].is_empty()
    }

    pub(super) fn is_on_first_visual_line(&self) -> bool {
        self.find_current_visual_line(&self.build_visual_line_map(self.last_width)) == 0
    }

    pub(super) fn is_on_last_visual_line(&self) -> bool {
        let vls = self.build_visual_line_map(self.last_width);
        self.find_current_visual_line(&vls) == vls.len().saturating_sub(1)
    }

    pub(super) fn content_max_visible(&self) -> usize {
        let mut max_vis = (self.terminal_rows * 30 / 100).max(5);
        if self.is_editor_empty() {
            max_vis = 1;
        }
        max_vis
    }

    /// Paint ↑/↓ more chrome; prefer the cue text over decorative dashes when narrow.
    pub(super) fn render_more_border(&self, width: usize, up: bool, count: usize) -> String {
        let arrow = if up { '↑' } else { '↓' };
        let core = format!(" {arrow} {count} more ");
        let core_w = visible_width(&core);
        if width == 0 {
            return String::new();
        }
        if width <= core_w {
            return (self.theme.border_color)(&truncate_to_width(&core, width, "…", false));
        }
        let lead = "───".to_string();
        let lead_w = visible_width(&lead);
        if lead_w + core_w <= width {
            let rest = width - lead_w - core_w;
            return (self.theme.border_color)(&format!("{lead}{core}{}", "─".repeat(rest)));
        }
        (self.theme.border_color)(&truncate_to_width(&format!("─{core}"), width, "…", false))
    }
}

// ── Component impl ──────────────────────────────────────────────────────────

impl Component for super::Editor {
    fn render(&mut self, width: usize) -> Vec<String> {
        let max_pad = width.saturating_sub(1) / 2;
        let px = self.padding_x.min(max_pad);
        let cw = width.saturating_sub(px * 2).max(1);
        let lw = if px > 0 {
            cw
        } else {
            cw.saturating_sub(1).max(1)
        };
        self.last_width = lw;
        self.last_paint_width = width;
        let layout = self.layout_text(lw);
        // Empty draft: keep a single content row (cursor) so the operation zone
        // stays compact — product chrome / agent_demo idle parity (c477).
        let max_vis = self.content_max_visible();
        let cur_idx = layout.iter().position(|l| l.has_cursor).unwrap_or(0);
        if cur_idx < self.scroll_offset {
            self.scroll_offset = cur_idx;
        } else if cur_idx >= self.scroll_offset + max_vis {
            self.scroll_offset = cur_idx.saturating_sub(max_vis - 1);
        }
        let ms = layout.len().saturating_sub(max_vis);
        self.scroll_offset = self.scroll_offset.min(ms);
        let visible = &layout[self.scroll_offset..(self.scroll_offset + max_vis).min(layout.len())];
        self.last_content_rows = visible.len();
        let lp = " ".repeat(px);
        let rp = " ".repeat(px);
        let mut result = Vec::new();
        let marker = if self.focused { CURSOR_MARKER } else { "" };

        // Top border — keep the "↑ N more" cue readable even on narrow widths.
        if self.scroll_offset > 0 {
            result.push(self.render_more_border(width, true, self.scroll_offset));
        } else {
            result.push((self.theme.border_color)(&"─".repeat(width.max(1))));
        }

        for ll in visible {
            let mut display = self.apply_selection_highlight(ll, &ll.text);
            let mut lv = visible_width(&ll.text);
            if ll.has_cursor
                && let Some(cp) = ll.cursor_pos
            {
                // Cursor paint uses the pre-highlight plain offsets on `ll.text`.
                let plain = &ll.text;
                let before = &plain[..cp.min(plain.len())];
                let after = &plain[cp.min(plain.len())..];
                // Re-apply selection on before/after pieces then insert cursor.
                let before_h = self.apply_selection_highlight(
                    &LayoutLine {
                        text: before.to_string(),
                        has_cursor: false,
                        cursor_pos: None,
                        logical_line: ll.logical_line,
                        start_index: ll.start_index,
                    },
                    before,
                );
                let after_start = ll.start_index + before.len();
                if !after.is_empty() {
                    let gs: Vec<&str> = UnicodeSegmentation::graphemes(after, true).collect();
                    let first = gs.first().copied().unwrap_or(" ");
                    let rest = &after[first.len()..];
                    let rest_h = self.apply_selection_highlight(
                        &LayoutLine {
                            text: rest.to_string(),
                            has_cursor: false,
                            cursor_pos: None,
                            logical_line: ll.logical_line,
                            start_index: after_start + first.len(),
                        },
                        rest,
                    );
                    display = format!("{before_h}{marker}\x1b[7m{first}\x1b[0m{rest_h}");
                } else {
                    display = format!("{before_h}{marker}\x1b[7m \x1b[0m");
                    lv += 1;
                }
            }
            let pad = cw.saturating_sub(lv);
            result.push(format!("{lp}{display}{}{rp}", " ".repeat(pad)));
        }

        let below = layout
            .len()
            .saturating_sub(self.scroll_offset + visible.len());
        if below > 0 {
            result.push(self.render_more_border(width, false, below));
        } else {
            result.push((self.theme.border_color)(&"─".repeat(width.max(1))));
        }

        // c430/c545: append autocomplete popup below border; clamp to content width.
        // While open, reserve a fixed band (max_visible + optional pagination row)
        // so filtering `/e` → `/ex` → `/exi` does not resize the dock / jump the
        // transcript. Cancel / clear releases the band (height restores).
        if let Some(ref mut ac_list) = self.autocomplete_list
            && self.autocomplete_state.is_some()
        {
            let reserved = self.autocomplete_max_visible.saturating_add(1);
            let ac_lines = ac_list.render(cw);
            let mut painted = 0usize;
            for line in &ac_lines {
                if painted >= reserved {
                    break;
                }
                let clipped = if visible_width(line) <= cw {
                    line.clone()
                } else {
                    truncate_to_width(line, cw, "", false)
                };
                let lw = visible_width(&clipped);
                let pad = cw.saturating_sub(lw);
                result.push(format!("{lp}{clipped}{}{rp}", " ".repeat(pad)));
                painted += 1;
            }
            while painted < reserved {
                result.push(format!("{lp}{}{rp}", " ".repeat(cw)));
                painted += 1;
            }
        }

        result
    }

    fn handle_input(&mut self, event: InputEvent) {
        match event {
            InputEvent::Paste(content) => {
                self.paste_burst.reset();
                self.paste_burst_needs_paint = false;
                self.selection.clear();
                if !content.is_empty() {
                    self.paste(&content);
                }
            }
            InputEvent::Key(key) => {
                self.selection.clear();
                self.handle_key(&key);
            }
            InputEvent::Mouse(mouse) => {
                // Absolute screen → editor-local (top-left of last paint).
                let local = MouseEvent {
                    kind: mouse.kind,
                    column: mouse.column.saturating_sub(self.screen_origin_col),
                    row: mouse.row.saturating_sub(self.screen_origin_row),
                    modifiers: mouse.modifiers,
                };
                let mut sink = DiscardClipboardSink;
                self.mouse_dirty = self.handle_mouse_local(&local, &mut sink);
            }
        }
    }

    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        match event {
            InputEvent::Mouse(_) => {
                self.mouse_dirty || self.selection.dragging || self.selection.has_selection()
            }
            InputEvent::Key(_) if self.paste_burst_paint_suppressed() => false,
            _ => true,
        }
    }

    fn take_pending_clipboard(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_clipboard)
    }

    fn wants_pointer_motion(&self) -> bool {
        self.selection.dragging
    }

    fn clear_pointer_selection(&mut self) -> bool {
        if !self.selection.has_selection() && !self.selection.dragging {
            return false;
        }
        self.selection.clear();
        self.mouse_dirty = true;
        true
    }

    fn tick(&mut self) -> bool {
        // Paste-burst catch-up only. Editor selection does not auto-scroll the
        // viewport while dragging (edge-scroll removed — caused click ghosts).
        if self.paste_burst_needs_paint && !self.paste_burst_paint_suppressed() {
            self.paste_burst_needs_paint = false;
            return true;
        }
        false
    }

    fn invalidate(&mut self) {}
}

/// Sink used when [`Editor::handle_input`] receives mouse without an external sink.
/// OSC52 still accumulates on [`Editor::pending_clipboard`] via [`Editor::maybe_copy_selection`].
struct DiscardClipboardSink;

impl ClipboardSink for DiscardClipboardSink {
    fn copy_text(&mut self, _text: &str) {}
}
