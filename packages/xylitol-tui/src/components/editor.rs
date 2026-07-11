//! Multi-line editor component with kill-ring, undo, history, VisualLine
//! vertical cursor movement, PasteBurst integration, and autocomplete popup.
//! Ported from pi's `components/editor.ts` (c425 VL+sticky+PasteBurst + c430 autocomplete).

#![allow(
    clippy::type_complexity,
    clippy::needless_range_loop,
    clippy::module_inception
)]
use crate::autocomplete::{
    AutocompleteItem, AutocompleteSuggestions, CombinedAutocompleteProvider,
};
use crate::clock::Clock;
use crate::completion::{
    AtPathSource, CompletionContext, CompletionRegistry, CompletionSource, SlashCommandSource,
};
use crate::components::select_list::{
    SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme,
};
use crate::keybindings::with_keybindings;
use crate::keys::{matches_key_event, printable_from_key_event};
use crate::kill_ring::{KillRing, KillRingOptions};
use crate::paste_burst::PasteBurst;
use crate::tui::{CURSOR_MARKER, Component, Focusable, InputEvent};
use crate::undo_stack::UndoStack;
use crate::utils::{is_whitespace_char, truncate_to_width, visible_width};
use crate::word_navigation::{find_word_backward, find_word_forward};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

// ── types ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct EditorState {
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
        }
    }
}

/// A visual line — one row on screen after word-wrap.
pub struct VisualLine {
    pub logical_line: usize,
    pub start_col: usize,
    pub len: usize,
}

/// Where to place cursor after set_text_internal.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CursorPlacement {
    Start,
    End,
}

/// Autocomplete popup mode.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AutocompleteMode {
    Regular,
    Force,
}

struct LayoutLine {
    text: String,
    has_cursor: bool,
    cursor_pos: Option<usize>,
}

pub struct EditorTheme {
    pub border_color: Box<dyn Fn(&str) -> String>,
    pub select_list_theme: SelectListTheme,
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self {
            border_color: Box::new(|s| s.to_string()),
            select_list_theme: SelectListTheme::default(),
        }
    }
}

pub struct EditorOptions {
    pub padding_x: usize,
    pub terminal_rows: usize,
}

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            padding_x: 0,
            terminal_rows: 24,
        }
    }
}

// ── word-wrap ───────────────────────────────────────────────────────────────

struct TextChunk {
    text: String,
    start_index: usize,
    end_index: usize,
}

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

// ── Editor ──────────────────────────────────────────────────────────────────

pub struct Editor {
    state: EditorState,
    focused: bool,
    theme: EditorTheme,
    padding_x: usize,
    terminal_rows: usize,
    last_width: usize,
    scroll_offset: usize,
    history: Vec<String>,
    history_index: isize,
    history_draft: Option<EditorState>,
    kill_ring: KillRing,
    last_action: Option<String>,
    undo_stack: UndoStack<EditorState>,
    jump_mode: Option<bool>,
    pastes: HashMap<usize, String>,
    paste_counter: usize,
    // c425 new fields
    preferred_visual_col: Option<usize>,
    snapped_from_cursor_col: Option<usize>,
    paste_burst: PasteBurst,
    clock: Box<dyn Clock>,
    // c430 autocomplete integration (CompletionSource registry)
    completion: CompletionRegistry,
    autocomplete_list: Option<SelectList>,
    autocomplete_state: Option<AutocompleteMode>,
    autocomplete_prefix: String,
    autocomplete_max_visible: usize,
    autocomplete_start_token: usize,
    pub on_submit: Option<Box<dyn FnMut(String)>>,
    pub on_change: Option<Box<dyn FnMut(&str)>>,
    pub disable_submit: bool,
}

impl Editor {
    pub fn new(theme: EditorTheme, opts: EditorOptions, clock: Box<dyn Clock>) -> Self {
        Self {
            state: EditorState::default(),
            focused: false,
            theme,
            padding_x: opts.padding_x,
            terminal_rows: opts.terminal_rows,
            last_width: 80,
            scroll_offset: 0,
            history: Vec::new(),
            history_index: -1,
            history_draft: None,
            kill_ring: KillRing::new(),
            last_action: None,
            undo_stack: UndoStack::new(),
            jump_mode: None,
            pastes: HashMap::new(),
            paste_counter: 0,
            preferred_visual_col: None,
            snapped_from_cursor_col: None,
            paste_burst: PasteBurst::new(),
            clock,
            completion: CompletionRegistry::new(),
            autocomplete_list: None,
            autocomplete_state: None,
            autocomplete_prefix: String::new(),
            autocomplete_max_visible: 5,
            autocomplete_start_token: 0,
            on_submit: None,
            on_change: None,
            disable_submit: false,
        }
    }

    pub fn get_text(&self) -> String {
        self.state.lines.join("\n")
    }

    /// Replace the border ANSI wrapper used when painting editor chrome.
    pub fn set_border_color(&mut self, border_color: Box<dyn Fn(&str) -> String>) {
        self.theme.border_color = border_color;
    }

    pub fn set_text(&mut self, text: String) {
        self.exit_history_browsing();
        self.last_action = None;
        self.pastes.clear();
        self.paste_counter = 0;
        let n = text
            .replace('\t', "    ")
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        self.push_undo();
        self.state.lines = if n.is_empty() {
            vec![String::new()]
        } else {
            n.split('\n').map(String::from).collect()
        };
        self.state.cursor_line = 0;
        self.state.cursor_col = 0;
        self.scroll_offset = 0;
        self.on_changed();
    }
    pub fn add_to_history(&mut self, text: String) {
        let t = text.trim().to_string();
        if t.is_empty() || self.history.first() == Some(&t) {
            return;
        }
        self.history.insert(0, t);
        if self.history.len() > 100 {
            self.history.pop();
        }
    }
    pub fn insert_text_at_cursor(&mut self, t: &str) {
        if t.is_empty() {
            return;
        }
        self.exit_history_browsing();
        self.last_action = None;
        self.push_undo();
        self.insert_inner(t);
    }
    pub fn get_expanded_text(&self) -> String {
        let mut r = self.state.lines.join("\n");
        for (&id, c) in &self.pastes {
            r = r.replace(&format!("[paste #{}", id), c);
        }
        r
    }

    fn on_changed(&mut self) {
        if let Some(ref mut cb) = self.on_change {
            let t = self.state.lines.join("\n");
            cb(&t);
        }
    }
    fn push_undo(&mut self) {
        self.undo_stack.push(self.state.clone());
    }
    fn set_cursor_col(&mut self, c: usize) {
        self.state.cursor_col = c;
        self.preferred_visual_col = None;
        self.snapped_from_cursor_col = None;
    }

    // ── history navigation (c425 ed05) ─────────────────────────────────────

    fn navigate_history(&mut self, direction: isize) {
        self.last_action = None;
        if self.history.is_empty() {
            return;
        }
        let new = self.history_index - direction;
        if new < -1 || new >= self.history.len() as isize {
            return;
        }

        if self.history_index == -1 && new >= 0 {
            self.push_undo();
            self.history_draft = Some(self.state.clone());
        }
        self.history_index = new;

        if self.history_index == -1 {
            if let Some(d) = self.history_draft.take() {
                self.state = d;
                self.preferred_visual_col = None;
                self.snapped_from_cursor_col = None;
                self.scroll_offset = 0;
                self.on_changed();
            } else {
                self.set_text_internal("", CursorPlacement::End);
            }
        } else {
            let placement = if direction < 0 {
                CursorPlacement::End
            } else {
                CursorPlacement::Start
            };
            self.set_text_internal(
                &self.history[self.history_index as usize].clone(),
                placement,
            );
        }
    }

    fn exit_history_browsing(&mut self) {
        self.history_index = -1;
        self.history_draft = None;
    }

    fn set_text_internal(&mut self, text: &str, cursor_placement: CursorPlacement) {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(String::from).collect()
        };
        self.state.lines = lines;
        match cursor_placement {
            CursorPlacement::Start => {
                self.state.cursor_line = 0;
                self.state.cursor_col = 0;
            }
            CursorPlacement::End => {
                let last = self.state.lines.len().saturating_sub(1);
                self.state.cursor_line = last;
                self.state.cursor_col = self.state.lines.get(last).map_or(0, |l| l.len());
            }
        }
        self.scroll_offset = 0;
        self.preferred_visual_col = None;
        self.snapped_from_cursor_col = None;
        self.on_changed();
    }

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

    fn find_visual_line_at(&self, vls: &[VisualLine], line: usize, col: usize) -> usize {
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

    fn find_current_visual_line(&self, vls: &[VisualLine]) -> usize {
        self.find_visual_line_at(vls, self.state.cursor_line, self.state.cursor_col)
    }

    // ── sticky column + vertical movement (c425 ed02) ─────────────────────

    fn compute_vertical_move_column(
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

    fn move_to_visual_line(&mut self, vls: &[VisualLine], current_vl: usize, target_vl: usize) {
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

    fn page_scroll(&mut self, direction: isize) {
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

    // ── basic edit ops (unchanged logic, add exit_history + paste_burst) ───

    fn undo(&mut self) {
        self.exit_history_browsing();
        if let Some(s) = self.undo_stack.pop() {
            self.state = s;
            self.last_action = None;
            self.preferred_visual_col = None;
            self.on_changed();
        }
    }
    fn insert_inner(&mut self, text: &str) {
        self.exit_history_browsing();
        let n = text
            .replace('\t', "    ")
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        let il: Vec<&str> = n.split('\n').collect();
        let cur = &self.state.lines[self.state.cursor_line];
        let before = &cur[..self.state.cursor_col];
        let after = &cur[self.state.cursor_col..];
        if il.len() == 1 {
            self.state.lines[self.state.cursor_line] = format!("{before}{n}{after}");
            self.set_cursor_col(self.state.cursor_col + n.len());
        } else {
            let first = format!("{before}{}", il[0]);
            let mid: Vec<String> = il[1..il.len() - 1].iter().map(|s| s.to_string()).collect();
            let last = format!("{}{after}", il[il.len() - 1]);
            let tail = self.state.lines.split_off(self.state.cursor_line + 1);
            self.state.lines[self.state.cursor_line] = first;
            self.state.lines.extend(mid);
            self.state.lines.push(last);
            self.state.lines.extend(tail);
            self.state.cursor_line += il.len() - 1;
            self.set_cursor_col(il.last().map_or(0, |l| l.len()));
        }
        self.on_changed();
    }
    fn insert_ch(&mut self, ch: &str) {
        self.exit_history_browsing();
        let first = ch.chars().next().unwrap_or(' ');
        let now = self.clock.now();
        if is_whitespace_char(first) || self.last_action.as_deref() != Some("type-word") {
            self.push_undo();
        }
        self.last_action = Some("type-word".into());
        let line = self.state.lines[self.state.cursor_line].clone();
        let b = &line[..self.state.cursor_col];
        let a = &line[self.state.cursor_col..];
        self.state.lines[self.state.cursor_line] = format!("{b}{ch}{a}");
        self.set_cursor_col(self.state.cursor_col + ch.len());
        self.paste_burst.on_plain_char(now);
        self.on_changed();
        // CompletionSource registry: refresh open popup, or probe for a new match.
        self.handle_autocomplete_on_edit();
    }
    fn backspace(&mut self) {
        self.exit_history_browsing();
        self.last_action = None;
        if self.state.cursor_col > 0 {
            self.push_undo();
            let line = self.state.lines[self.state.cursor_line].clone();
            let before = &line[..self.state.cursor_col];
            let gs: Vec<&str> = UnicodeSegmentation::graphemes(before, true).collect();
            let len = gs.last().map_or(1, |g| g.len());
            let col = self.state.cursor_col - len;
            self.state.lines[self.state.cursor_line] =
                format!("{}{}", &line[..col], &line[self.state.cursor_col..]);
            self.set_cursor_col(col);
        } else if self.state.cursor_line > 0 {
            self.push_undo();
            let cur = self.state.lines.remove(self.state.cursor_line);
            let pl = self.state.lines[self.state.cursor_line - 1].len();
            self.state.lines[self.state.cursor_line - 1].push_str(&cur);
            self.state.cursor_line -= 1;
            self.set_cursor_col(pl);
        }
        self.on_changed();
    }
    fn fwd_delete(&mut self) {
        self.exit_history_browsing();
        self.last_action = None;
        let line = self.state.lines[self.state.cursor_line].clone();
        if self.state.cursor_col < line.len() {
            self.push_undo();
            let a = &line[self.state.cursor_col..];
            let gs: Vec<&str> = UnicodeSegmentation::graphemes(a, true).collect();
            let len = gs.first().map_or(1, |g| g.len());
            let col = self.state.cursor_col;
            self.state.lines[self.state.cursor_line] =
                format!("{}{}", &line[..col], &line[col + len..]);
        } else if self.state.cursor_line + 1 < self.state.lines.len() {
            self.push_undo();
            let n = self.state.lines.remove(self.state.cursor_line + 1);
            self.state.lines[self.state.cursor_line].push_str(&n);
        }
        self.on_changed();
    }
    fn newline(&mut self) {
        self.exit_history_browsing();
        self.last_action = None;
        self.push_undo();
        let line = self.state.lines[self.state.cursor_line].clone();
        let b = line[..self.state.cursor_col].to_string();
        let a = line[self.state.cursor_col..].to_string();
        self.state.lines[self.state.cursor_line] = b;
        self.state.lines.insert(self.state.cursor_line + 1, a);
        self.state.cursor_line += 1;
        self.set_cursor_col(0);
        self.on_changed();
    }
    fn line_start(&mut self) {
        self.last_action = None;
        self.set_cursor_col(0);
    }
    fn line_end(&mut self) {
        self.last_action = None;
        self.set_cursor_col(self.state.lines[self.state.cursor_line].len());
    }
    fn del_to_start(&mut self) {
        self.exit_history_browsing();
        let line = self.state.lines[self.state.cursor_line].clone();
        let was = self.last_action.as_deref() == Some("kill");
        if self.state.cursor_col > 0 {
            self.push_undo();
            self.kill_ring.push(
                line[..self.state.cursor_col].to_string(),
                KillRingOptions {
                    prepend: true,
                    accumulate: was,
                },
            );
            self.last_action = Some("kill".into());
            self.state.lines[self.state.cursor_line] = line[self.state.cursor_col..].to_string();
            self.set_cursor_col(0);
        } else if self.state.cursor_line > 0 {
            self.push_undo();
            self.kill_ring.push(
                "\n".to_string(),
                KillRingOptions {
                    prepend: true,
                    accumulate: was,
                },
            );
            self.last_action = Some("kill".into());
            let cur = self.state.lines.remove(self.state.cursor_line);
            let pl = self.state.lines[self.state.cursor_line - 1].len();
            self.state.lines[self.state.cursor_line - 1].push_str(&cur);
            self.state.cursor_line -= 1;
            self.set_cursor_col(pl);
        }
        self.on_changed();
    }
    fn del_to_end(&mut self) {
        self.exit_history_browsing();
        let line = self.state.lines[self.state.cursor_line].clone();
        let was = self.last_action.as_deref() == Some("kill");
        if self.state.cursor_col < line.len() {
            self.push_undo();
            self.kill_ring.push(
                line[self.state.cursor_col..].to_string(),
                KillRingOptions {
                    prepend: false,
                    accumulate: was,
                },
            );
            self.last_action = Some("kill".into());
            self.state.lines[self.state.cursor_line] = line[..self.state.cursor_col].to_string();
        } else if self.state.cursor_line + 1 < self.state.lines.len() {
            self.push_undo();
            self.kill_ring.push(
                "\n".to_string(),
                KillRingOptions {
                    prepend: false,
                    accumulate: was,
                },
            );
            self.last_action = Some("kill".into());
            let n = self.state.lines.remove(self.state.cursor_line + 1);
            self.state.lines[self.state.cursor_line].push_str(&n);
        }
        self.on_changed();
    }
    fn del_word_back(&mut self) {
        self.exit_history_browsing();
        let line = self.state.lines[self.state.cursor_line].clone();
        let was = self.last_action.as_deref() == Some("kill");
        if self.state.cursor_col == 0 {
            if self.state.cursor_line > 0 {
                self.push_undo();
                self.kill_ring.push(
                    "\n".to_string(),
                    KillRingOptions {
                        prepend: true,
                        accumulate: was,
                    },
                );
                self.last_action = Some("kill".into());
                let cur = self.state.lines.remove(self.state.cursor_line);
                let pl = self.state.lines[self.state.cursor_line - 1].len();
                self.state.lines[self.state.cursor_line - 1].push_str(&cur);
                self.state.cursor_line -= 1;
                self.set_cursor_col(pl);
            }
        } else {
            self.push_undo();
            let old = self.state.cursor_col;
            let back = find_word_backward(&line, old);
            let del = line[back..old].to_string();
            self.kill_ring.push(
                del,
                KillRingOptions {
                    prepend: true,
                    accumulate: was,
                },
            );
            self.last_action = Some("kill".into());
            self.state.lines[self.state.cursor_line] = format!("{}{}", &line[..back], &line[old..]);
            self.set_cursor_col(back);
        }
        self.on_changed();
    }
    fn del_word_fwd(&mut self) {
        self.exit_history_browsing();
        let line = self.state.lines[self.state.cursor_line].clone();
        let was = self.last_action.as_deref() == Some("kill");
        if self.state.cursor_col >= line.len() {
            if self.state.cursor_line + 1 < self.state.lines.len() {
                self.push_undo();
                self.kill_ring.push(
                    "\n".to_string(),
                    KillRingOptions {
                        prepend: false,
                        accumulate: was,
                    },
                );
                self.last_action = Some("kill".into());
                let n = self.state.lines.remove(self.state.cursor_line + 1);
                self.state.lines[self.state.cursor_line].push_str(&n);
            }
        } else {
            self.push_undo();
            let old = self.state.cursor_col;
            let fwd = find_word_forward(&line, old);
            let del = line[old..fwd].to_string();
            self.kill_ring.push(
                del,
                KillRingOptions {
                    prepend: false,
                    accumulate: was,
                },
            );
            self.last_action = Some("kill".into());
            self.state.lines[self.state.cursor_line] = format!("{}{}", &line[..old], &line[fwd..]);
        }
        self.on_changed();
    }
    fn yank(&mut self) {
        if self.kill_ring.is_empty() {
            return;
        }
        self.exit_history_browsing();
        self.push_undo();
        let t = self.kill_ring.peek().unwrap().to_string();
        self.insert_inner(&t);
        self.last_action = Some("yank".into());
    }
    fn yank_pop(&mut self) {
        if self.last_action.as_deref() != Some("yank") || self.kill_ring.len() <= 1 {
            return;
        }
        self.exit_history_browsing();
        self.push_undo();
        let prev = self.kill_ring.peek().unwrap().to_string();
        let yl: Vec<&str> = prev.split('\n').collect();
        if yl.len() == 1 {
            let cur = self.state.lines[self.state.cursor_line].clone();
            let dl = prev.len();
            let col = self.state.cursor_col.saturating_sub(dl);
            self.state.lines[self.state.cursor_line] =
                format!("{}{}", &cur[..col], &cur[self.state.cursor_col..]);
            self.set_cursor_col(col);
        } else {
            let sl = self.state.cursor_line.saturating_sub(yl.len() - 1);
            let sc = self.state.lines[sl].len().saturating_sub(yl[0].len());
            let a = self.state.lines[self.state.cursor_line][self.state.cursor_col..].to_string();
            let b = self.state.lines[sl][..sc].to_string();
            self.state.lines.drain(sl..sl + yl.len());
            self.state.lines.insert(sl, format!("{b}{a}"));
            self.state.cursor_line = sl;
            self.set_cursor_col(sc);
        }
        self.kill_ring.rotate();
        let nt = self.kill_ring.peek().unwrap().to_string();
        self.insert_inner(&nt);
        self.last_action = Some("yank".into());
    }
    fn submit(&mut self) {
        self.exit_history_browsing();
        let text = self.state.lines.join("\n").trim().to_string();
        self.state = EditorState::default();
        self.pastes.clear();
        self.paste_counter = 0;
        self.scroll_offset = 0;
        self.undo_stack.clear();
        self.last_action = None;
        self.preferred_visual_col = None;
        self.snapped_from_cursor_col = None;
        self.paste_burst.reset();
        if let Some(ref mut cb) = self.on_change {
            cb("");
        }
        if let Some(ref mut s) = self.on_submit {
            s(text);
        }
    }

    // ── move_cursor (c425 ed02 — rewritten with VisualLine) ──────────────

    fn move_cursor(&mut self, dl: isize, dc: isize) {
        self.last_action = None;
        if dl != 0 {
            let vls = self.build_visual_line_map(self.last_width);
            let cur = self.find_current_visual_line(&vls) as isize;
            let target = cur + dl;
            if target >= 0 && (target as usize) < vls.len() {
                self.move_to_visual_line(&vls, cur as usize, target as usize);
                // After vertical move, clear preferred_visual_col if we also did a
                // horizontal move (dc != 0 handled below), otherwise keep sticky col.
            } else if dl < 0 {
                // Already at top visual line → jump to start of logical line
                self.set_cursor_col(0);
            } else {
                // Already at bottom → jump to end
                let last = self.state.lines[self.state.cursor_line].len();
                self.set_cursor_col(last);
            }
        }

        if dc < 0 && self.state.cursor_col > 0 {
            let before = &self.state.lines[self.state.cursor_line][..self.state.cursor_col];
            let gs: Vec<&str> = UnicodeSegmentation::graphemes(before, true).collect();
            let len = gs.last().map_or(1, |g| g.len());
            self.set_cursor_col(self.state.cursor_col.saturating_sub(len));
        } else if dc > 0 {
            let line = &self.state.lines[self.state.cursor_line];
            if self.state.cursor_col < line.len() {
                let a = &line[self.state.cursor_col..];
                let gs: Vec<&str> = UnicodeSegmentation::graphemes(a, true).collect();
                let len = gs.first().map_or(1, |g| g.len());
                self.set_cursor_col(self.state.cursor_col + len);
            } else if self.state.cursor_line + 1 < self.state.lines.len() {
                self.state.cursor_line += 1;
                self.set_cursor_col(0);
            }
        }
    }

    fn word_left(&mut self) {
        self.last_action = None;
        let line = &self.state.lines[self.state.cursor_line].clone();
        if self.state.cursor_col == 0 {
            if self.state.cursor_line > 0 {
                self.state.cursor_line -= 1;
                self.set_cursor_col(self.state.lines[self.state.cursor_line].len());
            }
            return;
        }
        self.set_cursor_col(find_word_backward(line, self.state.cursor_col));
    }
    fn word_right(&mut self) {
        self.last_action = None;
        let line = &self.state.lines[self.state.cursor_line].clone();
        if self.state.cursor_col >= line.len() {
            if self.state.cursor_line + 1 < self.state.lines.len() {
                self.state.cursor_line += 1;
                self.set_cursor_col(0);
            }
            return;
        }
        self.set_cursor_col(find_word_forward(line, self.state.cursor_col));
    }
    fn jump_to(&mut self, ch: char, forward: bool) {
        self.last_action = None;
        let lines = &self.state.lines;
        if forward {
            for (li, line) in lines.iter().enumerate().skip(self.state.cursor_line) {
                let start = if li == self.state.cursor_line {
                    self.state.cursor_col + 1
                } else {
                    0
                };
                if let Some(idx) = line[start..].find(ch) {
                    self.state.cursor_line = li;
                    self.set_cursor_col(start + idx);
                    return;
                }
            }
        } else {
            for li in (0..=self.state.cursor_line).rev() {
                let line = &lines[li];
                let end = if li == self.state.cursor_line {
                    self.state.cursor_col
                } else {
                    line.len()
                };
                if let Some(idx) = line[..end].rfind(ch) {
                    self.state.cursor_line = li;
                    self.set_cursor_col(idx);
                    return;
                }
            }
        }
    }
    fn paste(&mut self, content: &str) {
        self.exit_history_browsing();
        self.last_action = None;
        self.push_undo();
        let n = content
            .replace('\t', "    ")
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        let cl: String = n
            .chars()
            .filter(|&c| c == '\n' || (c as u32) >= 32)
            .collect();
        let lc = cl.lines().count();
        let clen = cl.len();
        if lc > 10 || clen > 1000 {
            self.paste_counter += 1;
            let pid = self.paste_counter;
            let marker = if lc > 10 {
                format!("[paste #{} +{} lines]", pid, lc)
            } else {
                format!("[paste #{} {} chars]", pid, clen)
            };
            self.pastes.insert(pid, cl);
            self.insert_inner(&marker);
        } else {
            self.insert_inner(&cl);
        }
    }

    // ── layout / render ────────────────────────────────────────────────────

    fn layout_text(&self, content_width: usize) -> Vec<LayoutLine> {
        let mut layout = Vec::new();
        if self.state.lines.iter().all(|l| l.is_empty()) {
            layout.push(LayoutLine {
                text: String::new(),
                has_cursor: true,
                cursor_pos: Some(0),
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
                    });
                }
            }
        }
        layout
    }

    fn is_editor_empty(&self) -> bool {
        self.state.lines.len() == 1 && self.state.lines[0].is_empty()
    }

    fn is_on_first_visual_line(&self) -> bool {
        self.find_current_visual_line(&self.build_visual_line_map(self.last_width)) == 0
    }

    fn is_on_last_visual_line(&self) -> bool {
        let vls = self.build_visual_line_map(self.last_width);
        self.find_current_visual_line(&vls) == vls.len().saturating_sub(1)
    }

    // ── c430: autocomplete via CompletionSource registry ───────────────

    fn completion_ctx(&self) -> CompletionContext<'_> {
        CompletionContext {
            lines: &self.state.lines,
            cursor_line: self.state.cursor_line,
            cursor_col: self.state.cursor_col,
        }
    }

    /// Primary API: register pluggable completion sources (`/`, `@`, future `$`/`^`).
    pub fn set_completion_sources(&mut self, sources: Vec<Box<dyn CompletionSource>>) {
        self.cancel_autocomplete();
        self.completion.set_sources(sources);
    }

    /// Compatibility shim: split Combined into Slash + AtPath sources.
    pub fn set_autocomplete_provider(&mut self, provider: Option<CombinedAutocompleteProvider>) {
        self.cancel_autocomplete();
        match provider {
            None => self.completion.clear(),
            Some(p) => {
                let (commands, base, fd) = p.into_parts();
                let at: Box<dyn CompletionSource> = match fd {
                    Some(fd_path) => Box::new(AtPathSource::new_with_fd(base, fd_path)),
                    None => Box::new(AtPathSource::new(base)),
                };
                self.completion
                    .set_sources(vec![Box::new(SlashCommandSource::new(commands)), at]);
            }
        }
    }

    fn is_showing_autocomplete(&self) -> bool {
        self.autocomplete_state.is_some()
    }

    /// After text/cursor edits: dismiss, refresh, or open a matching source.
    fn handle_autocomplete_on_edit(&mut self) {
        if self.completion.is_empty() {
            return;
        }
        if self.is_showing_autocomplete() {
            let ctx = self.completion_ctx();
            if self.completion.should_dismiss_active(&ctx) {
                self.cancel_autocomplete();
                return;
            }
            self.request_autocomplete(false, false);
        } else {
            let ctx = self.completion_ctx();
            if self.completion.probe_first(&ctx).is_some() {
                self.request_autocomplete(false, false);
            }
        }
    }

    fn handle_tab_completion(&mut self) {
        if self.completion.is_empty() {
            return;
        }
        // Force=true for non-slash probes (file path Tab); slash stays regular.
        let ctx = self.completion_ctx();
        let force = match self.completion.probe_first(&ctx) {
            Some((_, m)) => !m.prefix.starts_with('/'),
            None => true,
        };
        self.request_autocomplete(force, true);
    }

    fn request_autocomplete(&mut self, force: bool, explicit_tab: bool) {
        if self.completion.is_empty() {
            return;
        }
        self.autocomplete_start_token = self.autocomplete_start_token.wrapping_add(1);
        let start_token = self.autocomplete_start_token;
        self.start_autocomplete_request(start_token, force, explicit_tab);
    }

    fn start_autocomplete_request(&mut self, start_token: usize, force: bool, explicit_tab: bool) {
        if start_token != self.autocomplete_start_token {
            return;
        }

        let ctx = CompletionContext {
            lines: &self.state.lines,
            cursor_line: self.state.cursor_line,
            cursor_col: self.state.cursor_col,
        };
        let suggestions = self.completion.open_or_refresh(&ctx);

        if start_token != self.autocomplete_start_token {
            return;
        }

        match suggestions {
            Some(s) if !s.items.is_empty() => {
                if force && explicit_tab && s.items.len() == 1 {
                    let item = s.items[0].clone();
                    let prefix = s.prefix.clone();
                    let source_index = self.completion.active_index().unwrap_or(0);
                    self.push_undo();
                    self.last_action = None;
                    let (new_lines, nl, nc) = self.completion.apply_probed(
                        source_index,
                        &self.state.lines,
                        self.state.cursor_line,
                        self.state.cursor_col,
                        &item,
                        &prefix,
                    );
                    self.state.lines = new_lines;
                    self.state.cursor_line = nl;
                    self.set_cursor_col(nc);
                    self.cancel_autocomplete();
                    self.on_changed();
                } else {
                    self.apply_autocomplete_suggestions(
                        s,
                        if force {
                            AutocompleteMode::Force
                        } else {
                            AutocompleteMode::Regular
                        },
                    );
                }
            }
            _ => {
                self.cancel_autocomplete();
            }
        }
    }

    fn apply_autocomplete_suggestions(
        &mut self,
        suggestions: AutocompleteSuggestions,
        mode: AutocompleteMode,
    ) {
        self.autocomplete_prefix = suggestions.prefix.clone();
        let items: Vec<SelectItem> = suggestions
            .items
            .into_iter()
            .map(|i| SelectItem {
                value: i.value,
                label: i.label,
                description: i.description,
            })
            .collect();

        let best_idx = self.get_best_autocomplete_match_index(&items, &self.autocomplete_prefix);
        let layout = if self.autocomplete_prefix.starts_with('/') {
            SelectListLayoutOptions {
                min_primary_column_width: Some(12),
                max_primary_column_width: Some(32),
                truncate_primary: None,
            }
        } else {
            SelectListLayoutOptions {
                min_primary_column_width: None,
                max_primary_column_width: None,
                truncate_primary: None,
            }
        };
        let mut sl = SelectList::new(
            items,
            self.autocomplete_max_visible,
            SelectListTheme::default(),
            layout,
        );
        if best_idx < sl.filtered_items.len() {
            sl.set_selected_index(best_idx);
        }
        self.autocomplete_list = Some(sl);
        self.autocomplete_state = Some(mode);
    }

    fn get_best_autocomplete_match_index(&self, items: &[SelectItem], prefix: &str) -> usize {
        if prefix.is_empty() {
            return 0;
        }
        // Strip leading trigger for value compare (slash items store bare names).
        let needle = prefix
            .strip_prefix('/')
            .or_else(|| prefix.strip_prefix('@'))
            .unwrap_or(prefix);
        let mut first_prefix = items.len();
        for (i, item) in items.iter().enumerate() {
            if item.value == needle || item.value == prefix {
                return i;
            }
            if first_prefix == items.len()
                && (item.value.starts_with(needle) || item.value.starts_with(prefix))
            {
                first_prefix = i;
            }
        }
        if first_prefix < items.len() {
            first_prefix
        } else {
            0
        }
    }

    fn cancel_autocomplete_request(&mut self) {
        self.autocomplete_start_token = self.autocomplete_start_token.wrapping_add(1);
    }

    fn clear_autocomplete_ui(&mut self) {
        self.autocomplete_state = None;
        self.autocomplete_list = None;
        self.autocomplete_prefix.clear();
        self.completion.clear_active();
    }

    fn cancel_autocomplete(&mut self) {
        self.cancel_autocomplete_request();
        self.clear_autocomplete_ui();
    }

    fn handle_key(&mut self, key: &crossterm::event::KeyEvent) {
        if let Some(dir) = self.jump_mode.take() {
            if let Some(s) = printable_from_key_event(key).and_then(|s| s.chars().next()) {
                self.jump_to(s, dir);
            }
            return;
        }

        macro_rules! k {
            ($n:expr) => {
                with_keybindings(|kb| kb.matches_event(key, $n))
            };
        }

        // Undo
        if k!("tui.editor.undo") {
            self.undo();
            return;
        }

        // ── c430: autocomplete active routing ────────────────────────
        if self.autocomplete_state.is_some() && self.autocomplete_list.is_some() {
            if k!("tui.select.cancel") {
                self.cancel_autocomplete();
                return;
            }
            if k!("tui.select.up") || k!("tui.select.down") {
                if let Some(ref mut list) = self.autocomplete_list {
                    list.handle_input(InputEvent::Key(*key));
                }
                return;
            }
            if k!("tui.input.tab") || k!("tui.select.confirm") {
                // Extract apply data before the mutable self borrow
                let apply_data = if let Some(ref list) = self.autocomplete_list {
                    list.get_selected_item().map(|i| {
                        (
                            i.value.clone(),
                            i.label.clone(),
                            i.description.clone(),
                            self.autocomplete_prefix.clone(),
                        )
                    })
                } else {
                    None
                };
                if let Some((val, lbl, desc, prefix)) = apply_data {
                    let ai = AutocompleteItem {
                        value: val.clone(),
                        label: lbl.clone(),
                        description: desc.clone(),
                    };
                    if let Some((new_lines, nl, nc)) = self.completion.apply_active(
                        &self.state.lines,
                        self.state.cursor_line,
                        self.state.cursor_col,
                        &ai,
                        &prefix,
                    ) {
                        self.push_undo();
                        self.last_action = None;
                        self.state.lines = new_lines;
                        self.state.cursor_line = nl;
                        self.set_cursor_col(nc);
                        self.cancel_autocomplete();
                        self.on_changed();
                    } else {
                        self.cancel_autocomplete();
                    }
                } else {
                    self.cancel_autocomplete();
                }
                return;
            }
        }
        // ── end c430 autocomplete routing ────────────────────────────

        // Deletion / kill ring
        if k!("tui.editor.yankPop") {
            self.yank_pop();
        } else if k!("tui.editor.yank") {
            self.yank();
        } else if k!("tui.editor.deleteToLineStart") {
            self.del_to_start();
        } else if k!("tui.editor.deleteToLineEnd") {
            self.del_to_end();
        } else if k!("tui.editor.deleteWordBackward") {
            self.del_word_back();
        } else if k!("tui.editor.deleteWordForward") {
            self.del_word_fwd();
        } else if k!("tui.editor.deleteCharBackward") || matches_key_event(key, "shift+backspace") {
            self.backspace();
            self.handle_autocomplete_on_edit();
        } else if k!("tui.editor.deleteCharForward") || matches_key_event(key, "shift+delete") {
            self.fwd_delete();
            self.handle_autocomplete_on_edit();
        }
        // Cursor movement
        else if k!("tui.editor.cursorLineStart") {
            self.line_start();
        } else if k!("tui.editor.cursorLineEnd") {
            self.line_end();
        } else if k!("tui.editor.cursorWordLeft") {
            self.word_left();
            self.handle_autocomplete_on_edit();
        } else if k!("tui.editor.cursorWordRight") {
            self.word_right();
            self.handle_autocomplete_on_edit();
        } else if k!("tui.editor.jumpForward") {
            self.jump_mode = Some(true);
        } else if k!("tui.editor.jumpBackward") {
            self.jump_mode = Some(false);
        } else if k!("tui.editor.pageUp") {
            self.page_scroll(-1);
        } else if k!("tui.editor.pageDown") {
            self.page_scroll(1);
        }
        // Tab — trigger completion (c430)
        else if k!("tui.input.tab") && self.autocomplete_state.is_none() {
            self.handle_tab_completion();
        }
        // Newline / Submit with PasteBurst
        else if k!("tui.input.newLine") {
            let now = self.clock.now();
            if self
                .paste_burst
                .should_insert_newline_instead_of_submit(now)
            {
                self.paste_burst.extend_window(now);
                self.newline();
            } else {
                self.newline();
            }
        } else if k!("tui.input.submit") {
            if self.disable_submit {
                return;
            }
            let now = self.clock.now();
            let line = &self.state.lines[self.state.cursor_line];
            // Backslash-Escape: insert newline instead of submit
            if self.state.cursor_col > 0
                && line.as_bytes().get(self.state.cursor_col - 1) == Some(&b'\\')
            {
                self.backspace();
                self.newline();
            }
            // PasteBurst check: rapid paste → insert newline instead of submit
            else if self
                .paste_burst
                .should_insert_newline_instead_of_submit(now)
            {
                self.paste_burst.extend_window(now);
                self.newline();
            } else {
                self.submit();
            }
        }
        // Arrow navigation with VisualLine+history
        else if k!("tui.editor.cursorUp") {
            if (self.is_on_first_visual_line() || self.state.cursor_col == 0)
                && (self.is_editor_empty() || self.history_index > -1)
            {
                self.navigate_history(-1);
            } else if self.is_on_first_visual_line() {
                self.line_start();
            } else {
                self.move_cursor(-1, 0);
                self.handle_autocomplete_on_edit();
            }
        } else if k!("tui.editor.cursorDown") {
            if self.history_index > -1 && self.is_on_last_visual_line() {
                self.navigate_history(1);
            } else if self.is_on_last_visual_line() {
                self.line_end();
            } else {
                self.move_cursor(1, 0);
                self.handle_autocomplete_on_edit();
            }
        } else if k!("tui.editor.cursorLeft") {
            self.move_cursor(0, -1);
            self.handle_autocomplete_on_edit();
        } else if k!("tui.editor.cursorRight") {
            self.move_cursor(0, 1);
            self.handle_autocomplete_on_edit();
        }
        // Printable chars
        else if matches_key_event(key, "shift+space") {
            self.insert_ch(" ");
        } else if let Some(p) = printable_from_key_event(key) {
            self.insert_ch(&p);
        }
        // Everything else → reset paste burst
        else {
            self.paste_burst.reset();
        }
    }
}

// ── Component impl ──────────────────────────────────────────────────────────

impl Component for Editor {
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
        let layout = self.layout_text(lw);
        let max_vis = (self.terminal_rows * 30 / 100).max(5);
        let cur_idx = layout.iter().position(|l| l.has_cursor).unwrap_or(0);
        if cur_idx < self.scroll_offset {
            self.scroll_offset = cur_idx;
        } else if cur_idx >= self.scroll_offset + max_vis {
            self.scroll_offset = cur_idx.saturating_sub(max_vis - 1);
        }
        let ms = layout.len().saturating_sub(max_vis);
        self.scroll_offset = self.scroll_offset.min(ms);
        let visible = &layout[self.scroll_offset..(self.scroll_offset + max_vis).min(layout.len())];
        let h = (self.theme.border_color)("─");
        let lp = " ".repeat(px);
        let rp = " ".repeat(px);
        let mut result = Vec::new();
        let marker = if self.focused { CURSOR_MARKER } else { "" };

        // Top border
        if self.scroll_offset > 0 {
            let ind = format!("─── ↑ {} more ", self.scroll_offset);
            let iw = visible_width(&ind);
            result.push(if width >= iw {
                (self.theme.border_color)(&format!("{ind}{}", "─".repeat(width - iw)))
            } else {
                (self.theme.border_color)(&truncate_to_width(&ind, width, "", false))
            });
        } else {
            result.push((self.theme.border_color)(&h.repeat(width)));
        }

        for ll in visible {
            let mut display = ll.text.clone();
            let mut lv = visible_width(&display);
            if ll.has_cursor
                && let Some(cp) = ll.cursor_pos
            {
                let before = &display[..cp.min(display.len())];
                let after = &display[cp.min(display.len())..];
                if !after.is_empty() {
                    let gs: Vec<&str> = UnicodeSegmentation::graphemes(after, true).collect();
                    let first = gs.first().copied().unwrap_or(" ");
                    let rest = &after[first.len()..];
                    display = format!("{before}{marker}\x1b[7m{first}\x1b[0m{rest}");
                } else {
                    display = format!("{before}{marker}\x1b[7m \x1b[0m");
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
            let ind = format!("─── ↓ {} more ", below);
            let iw = visible_width(&ind);
            result.push(if width >= iw {
                (self.theme.border_color)(&format!("{ind}{}", "─".repeat(width - iw)))
            } else {
                (self.theme.border_color)(&truncate_to_width(&ind, width, "", false))
            });
        } else {
            result.push((self.theme.border_color)(&h.repeat(width)));
        }

        // c430/c545: append autocomplete popup below border; clamp to content width.
        if let Some(ref mut ac_list) = self.autocomplete_list
            && self.autocomplete_state.is_some()
        {
            let ac_lines = ac_list.render(cw);
            for line in &ac_lines {
                let clipped = if visible_width(line) <= cw {
                    line.clone()
                } else {
                    truncate_to_width(line, cw, "", false)
                };
                let lw = visible_width(&clipped);
                let pad = cw.saturating_sub(lw);
                result.push(format!("{lp}{clipped}{}{rp}", " ".repeat(pad)));
            }
        }

        result
    }

    fn handle_input(&mut self, event: InputEvent) {
        match event {
            InputEvent::Paste(content) => {
                self.paste_burst.reset();
                if !content.is_empty() {
                    self.paste(&content);
                }
            }
            InputEvent::Key(key) => self.handle_key(&key),
        }
    }

    fn invalidate(&mut self) {}
}

impl Focusable for Editor {
    fn set_focused(&mut self, f: bool) {
        self.focused = f;
    }
    fn is_focused(&self) -> bool {
        self.focused
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::SystemClock;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn t() -> EditorTheme {
        EditorTheme {
            border_color: Box::new(|s| s.to_string()),
            select_list_theme: SelectListTheme::default(),
        }
    }
    fn clk() -> Box<dyn Clock> {
        Box::new(SystemClock)
    }

    #[test]
    fn empty() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        assert!(e.render(30).iter().any(|l| l.contains('─')));
    }

    #[test]
    fn insert_get() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.insert_ch("h");
        e.insert_ch("i");
        assert_eq!(e.get_text(), "hi");
    }

    #[test]
    fn backspace_del() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.insert_ch("x");
        e.backspace();
        assert_eq!(e.get_text(), "");
    }

    #[test]
    fn newline_split() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.insert_ch("a");
        e.newline();
        e.insert_ch("b");
        assert_eq!(e.state.lines.len(), 2);
        assert_eq!(e.get_text(), "a\nb");
    }

    #[test]
    fn undo_test() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.insert_ch("a");
        assert_eq!(e.get_text(), "a");
        e.undo();
        assert_eq!(e.get_text(), "");
    }

    #[test]
    fn word_lr() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.insert_ch("a");
        e.insert_ch("b");
        e.insert_ch(" ");
        e.insert_ch("c");
        e.word_left();
        assert_eq!(e.state.cursor_col, 3);
        e.word_right();
        assert_eq!(e.state.cursor_col, 4);
    }

    #[test]
    fn submit_cb() {
        let s = Rc::new(RefCell::new(String::new()));
        let sc = s.clone();
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.on_submit = Some(Box::new(move |v| *sc.borrow_mut() = v));
        e.insert_ch("test");
        e.submit();
        assert_eq!(*s.borrow(), "test");
        assert!(e.get_text().is_empty());
    }

    // c425 new tests

    #[test]
    fn visual_line_map_wraps_long_lines() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.insert_inner("a".repeat(100).as_str());
        let vls = e.build_visual_line_map(30);
        assert!(
            vls.len() >= 3,
            "100-char line in 30-col should wrap to >=3 VLs, got {}",
            vls.len()
        );
    }

    #[test]
    fn visual_line_map_empty_line_still_one_vl() {
        let e = Editor::new(t(), EditorOptions::default(), clk());
        let vls = e.build_visual_line_map(80);
        assert_eq!(vls.len(), 1);
        assert_eq!(vls[0].logical_line, 0);
    }

    #[test]
    fn move_cursor_up_down_preserves_visual_col() {
        let mut e = Editor::new(
            t(),
            EditorOptions {
                padding_x: 0,
                terminal_rows: 40,
            },
            clk(),
        );
        e.set_text("aaaa\nbbbb\ncccc".to_string());
        e.last_width = 80;
        e.state.cursor_line = 2;
        e.state.cursor_col = 3;
        e.move_cursor(-1, 0);
        assert_eq!(e.state.cursor_line, 1);
        // On a non-wrapped line, visual col = cursor col = 3
        assert_eq!(e.state.cursor_col, 3, "up-arrow should stay at same column");
    }

    #[test]
    fn preferred_visual_col_cleared_on_horizontal() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.set_text("aaaa\nbbbb\ncccc".to_string());
        e.last_width = 80;
        e.state.cursor_line = 2;
        e.state.cursor_col = 3;
        // Move up — this sets preferred_visual_col via compute_vertical_move_column
        e.move_cursor(-1, 0);
        // Now move left — preferred should be cleared by set_cursor_col
        e.move_cursor(0, -1);
        assert!(
            e.preferred_visual_col.is_none(),
            "horizontal move should clear preferred_visual_col"
        );
    }

    #[test]
    fn page_scroll_moves_by_page_size() {
        let mut e = Editor::new(
            t(),
            EditorOptions {
                padding_x: 0,
                terminal_rows: 40,
            },
            clk(),
        );
        // 50 single-char lines — page size = 12
        let lines: Vec<String> = (0..50).map(|i| format!("line {}", i)).collect();
        e.set_text(lines.join("\n"));
        e.state.cursor_line = 5;
        e.state.cursor_col = 0;
        e.last_width = 80;
        e.page_scroll(1);
        // Should have moved ~12 visual lines (all single-line lines → 12 logical lines)
        assert!(
            e.state.cursor_line >= 15,
            "page_down from line 5 should go to ~17 (5+12), got {}",
            e.state.cursor_line
        );
    }

    #[test]
    fn paste_burst_enter_inserts_newline_not_submit() {
        use crate::clock::MockClock;
        use std::time::Duration;
        let theme = t();
        let mut clock = MockClock::new();
        // Type 8 fast chars → burst detected
        let mut e = Editor::new(theme, EditorOptions::default(), Box::new(clock.clone()));
        for _ in 0..8 {
            e.insert_ch("x");
            clock.advance(Duration::from_millis(1));
        }
        // Now Enter should be suppressed
        let now = clock.now();
        assert!(
            e.paste_burst.should_insert_newline_instead_of_submit(now),
            "8 fast chars should trigger paste burst enter suppression"
        );
    }

    #[test]
    fn paste_burst_reset_on_nonprintable() {
        use crate::clock::MockClock;
        use std::time::Duration;
        let theme = t();
        let mut clock = MockClock::new();
        let mut e = Editor::new(theme, EditorOptions::default(), Box::new(clock.clone()));
        for _ in 0..8 {
            e.insert_ch("x");
            clock.advance(Duration::from_millis(1));
        }
        assert!(
            e.paste_burst
                .should_insert_newline_instead_of_submit(clock.now())
        );
        // Simulate a non-printable key (CursorLeft)
        e.paste_burst.reset();
        assert!(
            !e.paste_burst
                .should_insert_newline_instead_of_submit(clock.now()),
            "reset should clear burst state"
        );
    }

    #[test]
    fn history_draft_restored_on_back_past_first() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.add_to_history("line 1".to_string());
        e.add_to_history("line 2".to_string());
        e.set_text("current".to_string());
        // Navigate through history
        e.navigate_history(-1); // → line 2
        e.navigate_history(-1); // → line 1
        // Now go back past first
        e.navigate_history(1); // → line 2
        assert_eq!(e.get_text(), "line 2");
    }

    #[test]
    fn exit_history_on_edit() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.add_to_history("old".to_string());
        e.navigate_history(-1);
        assert_eq!(e.history_index, 0);
        // Typing a character should exit history browsing
        e.insert_ch("x");
        assert_eq!(e.history_index, -1, "edit should exit history browsing");
    }

    #[test]
    fn visual_line_map_multiple_logical_lines() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.set_text("short\nloooooooooooooooooooooooooooooong\nshort".to_string());
        let vls = e.build_visual_line_map(20);
        // Line 0: 1 VL, Line 1: wraps to ~2 VLs, Line 2: 1 VL → total ≥ 4
        assert!(vls.len() >= 4, "expected >=4 VLs, got {}", vls.len());
        // All VLs should have correct logical_line mapping
        assert_eq!(vls[0].logical_line, 0);
        assert_eq!(vls[vls.len() - 1].logical_line, 2);
    }

    #[test]
    fn find_current_visual_line_at_end_of_wrapped_line() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        // Long-ish line in 30-col editor → wraps to multiple VLs
        e.set_text(
            "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123".to_string(),
        );
        let vls = e.build_visual_line_map(30);
        assert!(
            vls.len() >= 2,
            "long line at width 30 should wrap to ≥2 VLs"
        );
        // Cursor at end of line should be on the last VL
        e.state.cursor_col = e.state.lines[0].len();
        let ci = e.find_current_visual_line(&vls);
        assert_eq!(
            ci,
            vls.len() - 1,
            "cursor at end should be on last VL ({}), got {}",
            vls.len() - 1,
            ci
        );
    }

    #[test]
    fn set_text_internal_start_placement() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.set_text_internal("hello\nworld", CursorPlacement::Start);
        assert_eq!(e.state.cursor_line, 0);
        assert_eq!(e.state.cursor_col, 0);
    }

    #[test]
    fn set_text_internal_end_placement() {
        let mut e = Editor::new(t(), EditorOptions::default(), clk());
        e.set_text_internal("hello\nworld", CursorPlacement::End);
        assert_eq!(e.state.cursor_line, 1);
        assert_eq!(e.state.cursor_col, 5); // "world".len()
    }
}
