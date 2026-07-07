//! Multi-line editor component with kill-ring, undo, history, and paste
//! tracking. Ported from pi's `components/editor.ts`.

#[allow(clippy::type_complexity, clippy::needless_range_loop)]
use crate::keybindings::with_keybindings;
use crate::keys::{decode_printable_key, matches_key};
use crate::kill_ring::{KillRing, KillRingOptions};
use crate::tui::{Component, CURSOR_MARKER, Focusable};
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
        Self { lines: vec![String::new()], cursor_line: 0, cursor_col: 0 }
    }
}

struct LayoutLine { text: String, has_cursor: bool, cursor_pos: Option<usize> }

pub struct EditorTheme { pub border_color: Box<dyn Fn(&str) -> String> }

pub struct EditorOptions { pub padding_x: usize }

impl Default for EditorOptions { fn default() -> Self { Self { padding_x: 0 } } }

// ── word-wrap ───────────────────────────────────────────────────────────────

struct TextChunk { text: String, start_index: usize, end_index: usize }

fn word_wrap_line(line: &str, max_width: usize) -> Vec<TextChunk> {
    let vis = visible_width(line);
    if vis <= max_width {
        return vec![TextChunk { text: line.to_string(), start_index: 0, end_index: line.len() }];
    }
    let gs: Vec<(usize, &str, usize)> = UnicodeSegmentation::grapheme_indices(line, true)
        .map(|(i, g)| (i, g, visible_width(g))).collect();
    let mut chunks = Vec::new();
    let (mut cur_w, mut start, mut w_i, mut w_w) = (0usize, 0usize, -1isize, 0usize);

    for (gi, &(idx, grav, gw)) in gs.iter().enumerate() {
        let first_ch = grav.chars().next().unwrap_or(' ');
        let is_ws = is_whitespace_char(first_ch);

        if cur_w + gw > max_width {
            if w_i >= 0 && cur_w - w_w + gw <= max_width {
                chunks.push(TextChunk { text: line[start..w_i as usize].to_string(), start_index: start, end_index: w_i as usize });
                start = w_i as usize; cur_w -= w_w;
            } else if start < idx {
                chunks.push(TextChunk { text: line[start..idx].to_string(), start_index: start, end_index: idx });
                start = idx; cur_w = 0;
            }
            w_i = -1;
        }

        if gw > max_width {
            let sub = word_wrap_line(grav, max_width);
            for sc in sub.iter().take(sub.len().saturating_sub(1)) {
                chunks.push(TextChunk { text: sc.text.clone(), start_index: idx + sc.start_index, end_index: idx + sc.end_index });
            }
            if let Some(last) = sub.last() { start = idx + last.start_index; cur_w = visible_width(&last.text); }
            w_i = -1; continue;
        }

        cur_w += gw;
        let next_ch = gs.get(gi + 1).and_then(|(_, g, _)| g.chars().next());
        if is_ws && next_ch.is_some_and(|c| !is_whitespace_char(c)) {
            w_i = gs.get(gi + 1).map(|(i, _, _)| *i).unwrap_or(0) as isize; w_w = cur_w;
        } else if !is_ws && next_ch.is_some_and(|c| !is_whitespace_char(c)) {
            if grav.len() > 1 || gs.get(gi + 1).is_some_and(|(_, g, _)| g.len() > 1) {
                w_i = gs.get(gi + 1).map(|(i, _, _)| *i).unwrap_or(0) as isize; w_w = cur_w;
            }
        }
    }
    chunks.push(TextChunk { text: line[start..].to_string(), start_index: start, end_index: line.len() });
    chunks
}

// ── Editor ──────────────────────────────────────────────────────────────────

pub struct Editor {
    state: EditorState, focused: bool, theme: EditorTheme, padding_x: usize,
    last_width: usize, scroll_offset: usize,
    history: Vec<String>, history_index: isize, history_draft: Option<EditorState>,
    kill_ring: KillRing, last_action: Option<String>, undo_stack: UndoStack<EditorState>,
    jump_mode: Option<bool>, pastes: HashMap<usize, String>, paste_counter: usize,
    paste_buffer: String, is_in_paste: bool,
    pub on_submit: Option<Box<dyn FnMut(String)>>,
    pub on_change: Option<Box<dyn FnMut(&str)>>,
    pub disable_submit: bool,
}

impl Editor {
    pub fn new(theme: EditorTheme, opts: EditorOptions) -> Self {
        Self { state: EditorState::default(), focused: false, theme, padding_x: opts.padding_x,
            last_width: 80, scroll_offset: 0, history: Vec::new(), history_index: -1,
            history_draft: None, kill_ring: KillRing::new(), last_action: None,
            undo_stack: UndoStack::new(), jump_mode: None, pastes: HashMap::new(),
            paste_counter: 0, paste_buffer: String::new(), is_in_paste: false,
            on_submit: None, on_change: None, disable_submit: false }
    }

    pub fn get_text(&self) -> String { self.state.lines.join("\n") }
    pub fn set_text(&mut self, text: String) { self.last_action = None; self.history_index = -1;
        self.history_draft = None; self.pastes.clear(); self.paste_counter = 0;
        let n = text.replace('\t', "    ").replace("\r\n", "\n").replace('\r', "\n");
        self.push_undo(); self.state.lines = if n.is_empty() { vec![String::new()] }
            else { n.split('\n').map(String::from).collect() };
        self.state.cursor_line = 0; self.state.cursor_col = 0; self.scroll_offset = 0; self.on_changed(); }
    pub fn add_to_history(&mut self, text: String) { let t = text.trim().to_string();
        if t.is_empty() || self.history.first() == Some(&t) { return; }
        self.history.insert(0, t); if self.history.len() > 100 { self.history.pop(); } }
    pub fn insert_text_at_cursor(&mut self, t: &str) { if t.is_empty() { return; }
        self.history_index = -1; self.last_action = None; self.push_undo(); self.insert_inner(t); }
    pub fn get_expanded_text(&self) -> String { let mut r = self.state.lines.join("\n");
        for (&id, c) in &self.pastes { r = r.replace(&format!("[paste #{}", id), c); } r }

    fn on_changed(&mut self) { if let Some(ref mut cb) = self.on_change { let t = self.state.lines.join("\n"); cb(&t); } }
    fn push_undo(&mut self) { self.undo_stack.push(self.state.clone()); }
    fn set_cursor_col(&mut self, c: usize) { self.state.cursor_col = c; }

    fn undo(&mut self) { self.history_index = -1;
        if let Some(s) = self.undo_stack.pop() { self.state = s; self.last_action = None; self.on_changed(); } }
    fn insert_inner(&mut self, text: &str) { let n = text.replace('\t', "    ").replace("\r\n", "\n").replace('\r', "\n");
        let il: Vec<&str> = n.split('\n').collect(); let cur = &self.state.lines[self.state.cursor_line];
        let before = &cur[..self.state.cursor_col]; let after = &cur[self.state.cursor_col..];
        if il.len() == 1 { self.state.lines[self.state.cursor_line] = format!("{before}{n}{after}");
            self.set_cursor_col(self.state.cursor_col + n.len()); } else { let first = format!("{before}{}", il[0]);
            let mid: Vec<String> = il[1..il.len()-1].iter().map(|s| s.to_string()).collect();
            let last = format!("{}{after}", il[il.len()-1]); let tail = self.state.lines.split_off(self.state.cursor_line + 1);
            self.state.lines[self.state.cursor_line] = first; self.state.lines.extend(mid);
            self.state.lines.push(last); self.state.lines.extend(tail);
            self.state.cursor_line += il.len() - 1; self.set_cursor_col(il.last().map_or(0, |l| l.len())); } self.on_changed(); }
    fn insert_ch(&mut self, ch: &str) { self.history_index = -1;
        let first = ch.chars().next().unwrap_or(' ');
        if is_whitespace_char(first) || self.last_action.as_deref() != Some("type-word") { self.push_undo(); }
        self.last_action = Some("type-word".into()); let line = self.state.lines[self.state.cursor_line].clone();
        let b = &line[..self.state.cursor_col]; let a = &line[self.state.cursor_col..];
        self.state.lines[self.state.cursor_line] = format!("{b}{ch}{a}");
        self.set_cursor_col(self.state.cursor_col + ch.len()); self.on_changed(); }
    fn backspace(&mut self) { self.history_index = -1; self.last_action = None;
        if self.state.cursor_col > 0 { self.push_undo();
            let line = self.state.lines[self.state.cursor_line].clone();
            let before = &line[..self.state.cursor_col]; let gs: Vec<&str> = UnicodeSegmentation::graphemes(before, true).collect();
            let len = gs.last().map_or(1, |g| g.len()); let col = self.state.cursor_col - len;
            self.state.lines[self.state.cursor_line] = format!("{}{}", &line[..col], &line[self.state.cursor_col..]);
            self.set_cursor_col(col); } else if self.state.cursor_line > 0 { self.push_undo();
            let cur = self.state.lines.remove(self.state.cursor_line);
            let pl = self.state.lines[self.state.cursor_line - 1].len();
            self.state.lines[self.state.cursor_line - 1].push_str(&cur);
            self.state.cursor_line -= 1; self.set_cursor_col(pl); } self.on_changed(); }
    fn fwd_delete(&mut self) { self.history_index = -1; self.last_action = None;
        let line = self.state.lines[self.state.cursor_line].clone();
        if self.state.cursor_col < line.len() { self.push_undo();
            let a = &line[self.state.cursor_col..]; let gs: Vec<&str> = UnicodeSegmentation::graphemes(a, true).collect();
            let len = gs.first().map_or(1, |g| g.len()); let col = self.state.cursor_col;
            self.state.lines[self.state.cursor_line] = format!("{}{}", &line[..col], &line[col+len..]); }
        else if self.state.cursor_line + 1 < self.state.lines.len() { self.push_undo();
            let n = self.state.lines.remove(self.state.cursor_line + 1);
            self.state.lines[self.state.cursor_line].push_str(&n); } self.on_changed(); }
    fn newline(&mut self) { self.history_index = -1; self.last_action = None; self.push_undo();
        let line = self.state.lines[self.state.cursor_line].clone();
        let b = line[..self.state.cursor_col].to_string(); let a = line[self.state.cursor_col..].to_string();
        self.state.lines[self.state.cursor_line] = b;
        self.state.lines.insert(self.state.cursor_line + 1, a); self.state.cursor_line += 1; self.set_cursor_col(0); self.on_changed(); }
    fn line_start(&mut self) { self.last_action = None; self.set_cursor_col(0); }
    fn line_end(&mut self) { self.last_action = None; self.set_cursor_col(self.state.lines[self.state.cursor_line].len()); }
    fn del_to_start(&mut self) { self.history_index = -1;
        let line = self.state.lines[self.state.cursor_line].clone(); let was = self.last_action.as_deref() == Some("kill");
        if self.state.cursor_col > 0 { self.push_undo(); self.kill_ring.push(line[..self.state.cursor_col].to_string(), KillRingOptions { prepend: true, accumulate: was });
            self.last_action = Some("kill".into()); self.state.lines[self.state.cursor_line] = line[self.state.cursor_col..].to_string(); self.set_cursor_col(0); }
        else if self.state.cursor_line > 0 { self.push_undo(); self.kill_ring.push("\n".to_string(), KillRingOptions { prepend: true, accumulate: was });
            self.last_action = Some("kill".into()); let cur = self.state.lines.remove(self.state.cursor_line);
            let pl = self.state.lines[self.state.cursor_line - 1].len();
            self.state.lines[self.state.cursor_line - 1].push_str(&cur); self.state.cursor_line -= 1; self.set_cursor_col(pl); } self.on_changed(); }
    fn del_to_end(&mut self) { self.history_index = -1;
        let line = self.state.lines[self.state.cursor_line].clone(); let was = self.last_action.as_deref() == Some("kill");
        if self.state.cursor_col < line.len() { self.push_undo(); self.kill_ring.push(line[self.state.cursor_col..].to_string(), KillRingOptions { prepend: false, accumulate: was });
            self.last_action = Some("kill".into()); self.state.lines[self.state.cursor_line] = line[..self.state.cursor_col].to_string(); }
        else if self.state.cursor_line + 1 < self.state.lines.len() { self.push_undo(); self.kill_ring.push("\n".to_string(), KillRingOptions { prepend: false, accumulate: was });
            self.last_action = Some("kill".into()); let n = self.state.lines.remove(self.state.cursor_line + 1);
            self.state.lines[self.state.cursor_line].push_str(&n); } self.on_changed(); }
    fn del_word_back(&mut self) { self.history_index = -1;
        let line = self.state.lines[self.state.cursor_line].clone(); let was = self.last_action.as_deref() == Some("kill");
        if self.state.cursor_col == 0 { if self.state.cursor_line > 0 { self.push_undo();
            self.kill_ring.push("\n".to_string(), KillRingOptions { prepend: true, accumulate: was }); self.last_action = Some("kill".into());
            let cur = self.state.lines.remove(self.state.cursor_line); let pl = self.state.lines[self.state.cursor_line - 1].len();
            self.state.lines[self.state.cursor_line - 1].push_str(&cur); self.state.cursor_line -= 1; self.set_cursor_col(pl); } }
        else { self.push_undo(); let old = self.state.cursor_col; let back = find_word_backward(&line, old);
            let del = line[back..old].to_string(); self.kill_ring.push(del, KillRingOptions { prepend: true, accumulate: was });
            self.last_action = Some("kill".into()); self.state.lines[self.state.cursor_line] = format!("{}{}", &line[..back], &line[old..]);
            self.set_cursor_col(back); } self.on_changed(); }
    fn del_word_fwd(&mut self) { self.history_index = -1;
        let line = self.state.lines[self.state.cursor_line].clone(); let was = self.last_action.as_deref() == Some("kill");
        if self.state.cursor_col >= line.len() { if self.state.cursor_line + 1 < self.state.lines.len() { self.push_undo();
            self.kill_ring.push("\n".to_string(), KillRingOptions { prepend: false, accumulate: was }); self.last_action = Some("kill".into());
            let n = self.state.lines.remove(self.state.cursor_line + 1); self.state.lines[self.state.cursor_line].push_str(&n); } }
        else { self.push_undo(); let old = self.state.cursor_col; let fwd = find_word_forward(&line, old);
            let del = line[old..fwd].to_string(); self.kill_ring.push(del, KillRingOptions { prepend: false, accumulate: was });
            self.last_action = Some("kill".into()); self.state.lines[self.state.cursor_line] = format!("{}{}", &line[..old], &line[fwd..]); } self.on_changed(); }
    fn yank(&mut self) { if self.kill_ring.is_empty() { return; } self.push_undo();
        let t = self.kill_ring.peek().unwrap().to_string(); self.insert_inner(&t); self.last_action = Some("yank".into()); }
    fn yank_pop(&mut self) { if self.last_action.as_deref() != Some("yank") || self.kill_ring.len() <= 1 { return; } self.push_undo();
        let prev = self.kill_ring.peek().unwrap().to_string(); let yl: Vec<&str> = prev.split('\n').collect();
        if yl.len() == 1 { let cur = self.state.lines[self.state.cursor_line].clone(); let dl = prev.len();
            let col = self.state.cursor_col.saturating_sub(dl);
            self.state.lines[self.state.cursor_line] = format!("{}{}", &cur[..col], &cur[self.state.cursor_col..]); self.set_cursor_col(col); }
        else { let sl = self.state.cursor_line.saturating_sub(yl.len() - 1);
            let sc = self.state.lines[sl].len().saturating_sub(yl[0].len());
            let a = self.state.lines[self.state.cursor_line][self.state.cursor_col..].to_string();
            let b = self.state.lines[sl][..sc].to_string(); self.state.lines.drain(sl..sl+yl.len());
            self.state.lines.insert(sl, format!("{b}{a}")); self.state.cursor_line = sl; self.set_cursor_col(sc); }
        self.kill_ring.rotate(); let nt = self.kill_ring.peek().unwrap().to_string(); self.insert_inner(&nt); self.last_action = Some("yank".into()); }
    fn hist_nav(&mut self, dir: isize) { self.last_action = None; if self.history.is_empty() { return; }
        let new = self.history_index - dir; if new < -1 || new >= self.history.len() as isize { return; }
        if self.history_index == -1 && new >= 0 { self.push_undo(); self.history_draft = Some(self.state.clone()); }
        self.history_index = new;
        if self.history_index == -1 { if let Some(d) = self.history_draft.take() { self.state = d; } else { self.set_text(String::new()); } }
        else { self.set_text(self.history[self.history_index as usize].clone()); } self.on_changed(); }
    fn submit(&mut self) { let text = self.state.lines.join("\n").trim().to_string();
        self.state = EditorState::default(); self.pastes.clear(); self.paste_counter = 0; self.history_index = -1;
        self.history_draft = None; self.scroll_offset = 0; self.undo_stack.clear(); self.last_action = None;
        if let Some(ref mut cb) = self.on_change { cb(""); }
        if let Some(ref mut s) = self.on_submit { s(text); } }
    fn move_cursor(&mut self, dl: isize, dc: isize) { self.last_action = None;
        if dl < 0 && self.state.cursor_line > 0 { self.state.cursor_line -= 1;
            let l = self.state.lines[self.state.cursor_line].len(); self.set_cursor_col(self.state.cursor_col.min(l)); }
        else if dl > 0 && self.state.cursor_line + 1 < self.state.lines.len() { self.state.cursor_line += 1;
            let l = self.state.lines[self.state.cursor_line].len(); self.set_cursor_col(self.state.cursor_col.min(l)); }
        if dc < 0 && self.state.cursor_col > 0 { let before = &self.state.lines[self.state.cursor_line][..self.state.cursor_col];
            let gs: Vec<&str> = UnicodeSegmentation::graphemes(before, true).collect();
            let len = gs.last().map_or(1, |g| g.len()); self.set_cursor_col(self.state.cursor_col.saturating_sub(len)); }
        else if dc > 0 { let line = &self.state.lines[self.state.cursor_line];
            if self.state.cursor_col < line.len() { let a = &line[self.state.cursor_col..];
                let gs: Vec<&str> = UnicodeSegmentation::graphemes(a, true).collect();
                let len = gs.first().map_or(1, |g| g.len()); self.set_cursor_col(self.state.cursor_col + len); } } }
    fn word_left(&mut self) { self.last_action = None;
        let line = &self.state.lines[self.state.cursor_line].clone();
        if self.state.cursor_col == 0 { if self.state.cursor_line > 0 { self.state.cursor_line -= 1;
            self.set_cursor_col(self.state.lines[self.state.cursor_line].len()); } return; }
        self.set_cursor_col(find_word_backward(line, self.state.cursor_col)); }
    fn word_right(&mut self) { self.last_action = None;
        let line = &self.state.lines[self.state.cursor_line].clone();
        if self.state.cursor_col >= line.len() { if self.state.cursor_line + 1 < self.state.lines.len() {
            self.state.cursor_line += 1; self.set_cursor_col(0); } return; }
        self.set_cursor_col(find_word_forward(line, self.state.cursor_col)); }
    fn jump_to(&mut self, ch: char, forward: bool) { self.last_action = None;
        let lines = &self.state.lines;
        if forward { for (li, line) in lines.iter().enumerate().skip(self.state.cursor_line) {
            let start = if li == self.state.cursor_line { self.state.cursor_col + 1 } else { 0 };
            if let Some(idx) = line[start..].find(ch) { self.state.cursor_line = li; self.set_cursor_col(start + idx); return; } } }
        else { for li in (0..=self.state.cursor_line).rev() { let line = &lines[li];
            let end = if li == self.state.cursor_line { self.state.cursor_col } else { line.len() };
            if let Some(idx) = line[..end].rfind(ch) { self.state.cursor_line = li; self.set_cursor_col(idx); return; } } } }
    fn paste(&mut self, content: &str) { self.history_index = -1; self.last_action = None; self.push_undo();
        let n = content.replace('\t', "    ").replace("\r\n", "\n").replace('\r', "\n");
        let cl: String = n.chars().filter(|&c| c == '\n' || (c as u32) >= 32).collect();
        let lc = cl.lines().count(); let clen = cl.len();
        if lc > 10 || clen > 1000 { self.paste_counter += 1; let pid = self.paste_counter;
            let marker = if lc > 10 { format!("[paste #{} +{} lines]", pid, lc) } else { format!("[paste #{} {} chars]", pid, clen) };
            self.pastes.insert(pid, cl); self.insert_inner(&marker); } else { self.insert_inner(&cl); } }

    fn layout_text(&self, content_width: usize) -> Vec<LayoutLine> {
        let mut layout = Vec::new();
        if self.state.lines.iter().all(|l| l.is_empty()) { layout.push(LayoutLine { text: String::new(), has_cursor: true, cursor_pos: Some(0) }); return layout; }
        for (i, line) in self.state.lines.iter().enumerate() {
            let is_cur = i == self.state.cursor_line; let lv = visible_width(line);
            if lv <= content_width { layout.push(LayoutLine { text: line.clone(), has_cursor: is_cur, cursor_pos: is_cur.then_some(self.state.cursor_col) }); }
            else { for (ci, chunk) in word_wrap_line(line, content_width).iter().enumerate() {
                let is_last = ci == word_wrap_line(line, content_width).len() - 1;
                let has = is_cur && self.state.cursor_col >= chunk.start_index && (is_last || self.state.cursor_col < chunk.end_index);
                layout.push(LayoutLine { text: chunk.text.clone(), has_cursor: has,
                    cursor_pos: if has { let cp = self.state.cursor_col.saturating_sub(chunk.start_index); Some(cp.min(chunk.text.len())) } else { None } }); } } }
        layout }
}

// ── Component impl ──────────────────────────────────────────────────────────

impl Component for Editor {
    fn render(&mut self, width: usize) -> Vec<String> {
        let max_pad = width.saturating_sub(1) / 2; let px = self.padding_x.min(max_pad);
        let cw = width.saturating_sub(px * 2).max(1); let lw = if px > 0 { cw } else { cw.saturating_sub(1).max(1) };
        self.last_width = lw;
        let layout = self.layout_text(lw); let max_vis = 5.max(self.state.lines.len().min(10));
        let cur_idx = layout.iter().position(|l| l.has_cursor).unwrap_or(0);
        if cur_idx < self.scroll_offset { self.scroll_offset = cur_idx; }
        else if cur_idx >= self.scroll_offset + max_vis { self.scroll_offset = cur_idx.saturating_sub(max_vis - 1); }
        let ms = layout.len().saturating_sub(max_vis); self.scroll_offset = self.scroll_offset.min(ms);
        let visible = &layout[self.scroll_offset..(self.scroll_offset + max_vis).min(layout.len())];
        let h = (self.theme.border_color)("─"); let lp = " ".repeat(px); let rp = " ".repeat(px);
        let mut result = Vec::new();
        let marker = if self.focused { CURSOR_MARKER } else { "" };

        // Top border
        if self.scroll_offset > 0 { let ind = format!("─── ↑ {} more ", self.scroll_offset); let iw = visible_width(&ind);
            result.push(if width >= iw { (self.theme.border_color)(&format!("{ind}{}", "─".repeat(width - iw))) } else { (self.theme.border_color)(&truncate_to_width(&ind, width, "", false)) }); }
        else { result.push((self.theme.border_color)(&h.repeat(width))); }

        for ll in visible { let mut display = ll.text.clone(); let mut lv = visible_width(&display);
            if ll.has_cursor && let Some(cp) = ll.cursor_pos { let before = &display[..cp.min(display.len())];
                let after = &display[cp.min(display.len())..];
                if !after.is_empty() { let gs: Vec<&str> = UnicodeSegmentation::graphemes(after, true).collect();
                    let first = gs.first().copied().unwrap_or(" "); let rest = &after[first.len()..];
                    display = format!("{before}{marker}\x1b[7m{first}\x1b[0m{rest}"); }
                else { display = format!("{before}{marker}\x1b[7m \x1b[0m"); lv += 1; } }
            let pad = cw.saturating_sub(lv); result.push(format!("{lp}{display}{}{rp}", " ".repeat(pad))); }

        let below = layout.len().saturating_sub(self.scroll_offset + visible.len());
        if below > 0 { let ind = format!("─── ↓ {} more ", below); let iw = visible_width(&ind);
            result.push(if width >= iw { (self.theme.border_color)(&format!("{ind}{}", "─".repeat(width - iw))) } else { (self.theme.border_color)(&truncate_to_width(&ind, width, "", false)) }); }
        else { result.push((self.theme.border_color)(&h.repeat(width))); }
        result
    }

    fn handle_input(&mut self, data: &str) {
        if let Some(dir) = self.jump_mode.take() { if let Some(s) = decode_printable_key(data).and_then(|s| s.chars().next()) { self.jump_to(s, dir); } return; }
        if data.contains("\x1b[200~") { self.is_in_paste = true; self.paste_buffer = data.replace("\x1b[200~", ""); return; }
        if self.is_in_paste { self.paste_buffer.push_str(data);
            if let Some(end) = self.paste_buffer.find("\x1b[201~") { let c = self.paste_buffer[..end].to_string();
                let r = self.paste_buffer[end + 6..].to_string(); self.paste_buffer.clear(); self.is_in_paste = false;
                if !c.is_empty() { self.paste(&c); } if !r.is_empty() { self.handle_input(&r); } } return; }

        macro_rules! k { ($n:expr) => { with_keybindings(|kb| kb.matches(data, $n)) }; }
        if k!("tui.editor.undo") { self.undo(); }
        else if k!("tui.editor.yankPop") { self.yank_pop(); }
        else if k!("tui.editor.yank") { self.yank(); }
        else if k!("tui.editor.deleteToLineStart") { self.del_to_start(); }
        else if k!("tui.editor.deleteToLineEnd") { self.del_to_end(); }
        else if k!("tui.editor.deleteWordBackward") { self.del_word_back(); }
        else if k!("tui.editor.deleteWordForward") { self.del_word_fwd(); }
        else if k!("tui.editor.deleteCharBackward") || matches_key(data, "shift+backspace") { self.backspace(); }
        else if k!("tui.editor.deleteCharForward") || matches_key(data, "shift+delete") { self.fwd_delete(); }
        else if k!("tui.editor.cursorLineStart") { self.line_start(); }
        else if k!("tui.editor.cursorLineEnd") { self.line_end(); }
        else if k!("tui.editor.cursorWordLeft") { self.word_left(); }
        else if k!("tui.editor.cursorWordRight") { self.word_right(); }
        else if k!("tui.editor.jumpForward") { self.jump_mode = Some(true); }
        else if k!("tui.editor.jumpBackward") { self.jump_mode = Some(false); }
        else if k!("tui.editor.pageUp") { self.move_cursor(-5, 0); }
        else if k!("tui.editor.pageDown") { self.move_cursor(5, 0); }
        else if with_keybindings(|kb| kb.matches(data, "tui.input.newLine")) || data == "\n" { self.newline(); }
        else if with_keybindings(|kb| kb.matches(data, "tui.input.submit")) {
            if self.disable_submit { return; } let line = &self.state.lines[self.state.cursor_line];
            if self.state.cursor_col > 0 && line.as_bytes().get(self.state.cursor_col - 1) == Some(&b'\\') { self.backspace(); self.newline(); } else { self.submit(); } }
        else if k!("tui.editor.cursorUp") { if self.state.cursor_line == 0 && self.state.cursor_col == 0 { self.hist_nav(-1); }
            else if self.state.cursor_line == 0 { self.line_start(); } else { self.move_cursor(-1, 0); } }
        else if k!("tui.editor.cursorDown") { if self.history_index > -1 && self.state.cursor_line == self.state.lines.len() - 1
            && self.state.cursor_col >= self.state.lines.last().map_or(0, |l| l.len()) { self.hist_nav(1); }
            else if self.state.cursor_line == self.state.lines.len() - 1 { self.line_end(); } else { self.move_cursor(1, 0); } }
        else if k!("tui.editor.cursorLeft") { self.move_cursor(0, -1); }
        else if k!("tui.editor.cursorRight") { self.move_cursor(0, 1); }
        else if matches_key(data, "shift+space") { self.insert_ch(" "); }
        else if let Some(p) = decode_printable_key(data) { self.insert_ch(&p); }
        else if let Some(c) = data.chars().next().filter(|c| (*c as u32) >= 32) { self.insert_ch(&c.to_string()); }
    }

    fn invalidate(&mut self) {}
}

impl Focusable for Editor {
    fn set_focused(&mut self, f: bool) { self.focused = f; }
    fn is_focused(&self) -> bool { self.focused }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn t() -> EditorTheme { EditorTheme { border_color: Box::new(|s| s.to_string()) } }

    #[test]
    fn empty() { let mut e = Editor::new(t(), EditorOptions::default()); assert!(e.render(30).iter().any(|l| l.contains('─'))); }

    #[test]
    fn insert_get() { let mut e = Editor::new(t(), EditorOptions::default()); e.insert_ch("h"); e.insert_ch("i"); assert_eq!(e.get_text(), "hi"); }

    #[test]
    fn backspace_del() { let mut e = Editor::new(t(), EditorOptions::default()); e.insert_ch("x"); e.backspace(); assert_eq!(e.get_text(), ""); }

    #[test]
    fn newline_split() { let mut e = Editor::new(t(), EditorOptions::default()); e.insert_ch("a"); e.newline(); e.insert_ch("b");
        assert_eq!(e.state.lines.len(), 2); assert_eq!(e.get_text(), "a\nb"); }

    #[test]
    fn undo_test() { let mut e = Editor::new(t(), EditorOptions::default()); e.insert_ch("a"); assert_eq!(e.get_text(), "a"); e.undo(); assert_eq!(e.get_text(), ""); }

    #[test]
    fn word_lr() { let mut e = Editor::new(t(), EditorOptions::default()); e.insert_ch("a"); e.insert_ch("b"); e.insert_ch(" "); e.insert_ch("c");
        e.word_left(); assert_eq!(e.state.cursor_col, 3); e.word_right(); assert_eq!(e.state.cursor_col, 4); }

    #[test]
    fn submit_cb() { let s = Rc::new(RefCell::new(String::new())); let sc = s.clone();
        let mut e = Editor::new(t(), EditorOptions::default()); e.on_submit = Some(Box::new(move |v| *sc.borrow_mut() = v));
        e.insert_ch("test"); e.submit(); assert_eq!(*s.borrow(), "test"); assert!(e.get_text().is_empty()); }
}
