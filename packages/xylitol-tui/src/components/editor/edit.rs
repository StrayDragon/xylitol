use super::types::{CursorPlacement, EditorState};
use crate::kill_ring::KillRingOptions;
use crate::utils::is_whitespace_char;
use crate::word_navigation::{find_word_backward, find_word_forward};
use unicode_segmentation::UnicodeSegmentation;

impl super::Editor {
    pub fn insert_text_at_cursor(&mut self, t: &str) {
        if t.is_empty() {
            return;
        }
        self.exit_history_browsing();
        self.last_action = None;
        self.push_undo();
        self.insert_inner(t);
    }

    pub(super) fn on_changed(&mut self) {
        if let Some(ref mut cb) = self.on_change {
            let t = self.state.lines.join("\n");
            cb(&t);
        }
    }
    pub(super) fn push_undo(&mut self) {
        self.undo_stack.push(self.state.clone());
    }
    pub(super) fn set_cursor_col(&mut self, c: usize) {
        self.state.cursor_col = c;
        self.preferred_visual_col = None;
        self.snapped_from_cursor_col = None;
    }

    // ── history navigation (c425 ed05) ─────────────────────────────────────

    pub(super) fn navigate_history(&mut self, direction: isize) {
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

    pub(super) fn exit_history_browsing(&mut self) {
        self.history_index = -1;
        self.history_draft = None;
    }

    pub(super) fn set_text_internal(&mut self, text: &str, cursor_placement: CursorPlacement) {
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

    // ── basic edit ops (unchanged logic, add exit_history + paste_burst) ───

    pub(super) fn undo(&mut self) {
        self.exit_history_browsing();
        if let Some(s) = self.undo_stack.pop() {
            self.state = s;
            self.last_action = None;
            self.preferred_visual_col = None;
            self.on_changed();
        }
    }
    pub(super) fn insert_inner(&mut self, text: &str) {
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
    pub(super) fn insert_ch(&mut self, ch: &str) {
        self.exit_history_browsing();
        let first = ch.chars().next().unwrap_or(' ');
        let now = self.clock.now();
        self.paste_burst.on_plain_char(now);
        // Suppress per-char paints during a recognized non-bracketed burst so
        // the UI does not look like a sped-up typewriter. Model still updates
        // every char (harness / get_text stay correct); idle tick paints once.
        if self.paste_burst.consecutive_plain_chars() >= crate::paste_burst::PASTE_BURST_MIN_CHARS {
            self.paste_burst_needs_paint = true;
        }

        if is_whitespace_char(first) || self.last_action.as_deref() != Some("type-word") {
            self.push_undo();
        }
        self.last_action = Some("type-word".into());
        let line = self.state.lines[self.state.cursor_line].clone();
        let b = &line[..self.state.cursor_col];
        let a = &line[self.state.cursor_col..];
        self.state.lines[self.state.cursor_line] = format!("{b}{ch}{a}");
        self.set_cursor_col(self.state.cursor_col + ch.len());
        self.on_changed();
        // Always refresh autocomplete from the live buffer. Paste-burst only
        // suppresses *paints* (`paste_burst_needs_paint`); skipping this refresh
        // left stale/`None` popups (c545 `$` → missed open or Tab `$$demo`).
        self.handle_autocomplete_on_edit();
    }

    pub(super) fn paste_burst_paint_suppressed(&self) -> bool {
        let now = self.clock.now();
        self.paste_burst.consecutive_plain_chars() >= crate::paste_burst::PASTE_BURST_MIN_CHARS
            && self.paste_burst.is_coalescing(now)
    }
    pub(super) fn backspace(&mut self) {
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
    pub(super) fn fwd_delete(&mut self) {
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
    pub(super) fn newline(&mut self) {
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
    pub(super) fn line_start(&mut self) {
        self.last_action = None;
        self.set_cursor_col(0);
    }
    pub(super) fn line_end(&mut self) {
        self.last_action = None;
        self.set_cursor_col(self.state.lines[self.state.cursor_line].len());
    }
    pub(super) fn del_to_start(&mut self) {
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
    pub(super) fn del_to_end(&mut self) {
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
    pub(super) fn del_word_back(&mut self) {
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
    pub(super) fn del_word_fwd(&mut self) {
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
    pub(super) fn yank(&mut self) {
        if self.kill_ring.is_empty() {
            return;
        }
        self.exit_history_browsing();
        self.push_undo();
        let t = self.kill_ring.peek().unwrap().to_string();
        self.insert_inner(&t);
        self.last_action = Some("yank".into());
    }
    pub(super) fn yank_pop(&mut self) {
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
    pub(super) fn submit(&mut self) {
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

    pub(super) fn move_cursor(&mut self, dl: isize, dc: isize) {
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

    pub(super) fn word_left(&mut self) {
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
    pub(super) fn word_right(&mut self) {
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
    pub(super) fn jump_to(&mut self, ch: char, forward: bool) {
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
    pub(super) fn paste(&mut self, content: &str) {
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
}
