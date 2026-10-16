use super::TUI;
use super::render::{BEGIN_RENDER_BATCH, END_RENDER_BATCH, SEGMENT_RESET};
use crate::terminal::Terminal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VerticalShift {
    Up(usize),
    Down(usize),
}

pub(super) fn previous_body(line: &str) -> &str {
    line.strip_suffix(SEGMENT_RESET).unwrap_or(line)
}

pub(super) fn shifted_previous_index(
    row: usize,
    total_rows: usize,
    dock_rows: usize,
    shift: VerticalShift,
) -> Option<usize> {
    let transcript_rows = total_rows.saturating_sub(dock_rows);
    match shift {
        VerticalShift::Up(amount) if row < transcript_rows.saturating_sub(amount) => {
            Some(row + amount)
        }
        VerticalShift::Down(amount) if row >= amount && row < transcript_rows => Some(row - amount),
        _ => None,
    }
}

fn append_rewrite_lines(buf: &mut String, lines: &[String], start: usize, end: usize) {
    let end = end.min(lines.len());
    if start >= end {
        return;
    }
    buf.push_str(&format!("\x1b[{};1H", start + 1));
    for (i, line) in lines.iter().enumerate().take(end).skip(start) {
        if i > start {
            buf.push_str("\r\n");
        }
        buf.push_str("\x1b[2K");
        buf.push_str(line);
    }
}

impl<T: Terminal> TUI<T> {
    /// Find the first/last changed line index comparing new_lines to
    /// previous_lines. Returns (first, last, appended) with -1 meaning "none".
    /// Mirrors pi's firstChanged/lastChanged + append detection.
    pub(super) fn compute_line_diff(&self, new_lines: &[String]) -> (isize, isize, bool) {
        let mut first_changed: isize = -1;
        let mut last_changed: isize = -1;
        let max_lines = new_lines.len().max(self.previous_lines.len());
        for i in 0..max_lines {
            let old = self.previous_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let new = new_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            if old != new {
                if first_changed == -1 {
                    first_changed = i as isize;
                }
                last_changed = i as isize;
            }
        }
        let appended = new_lines.len() > self.previous_lines.len();
        if appended {
            if first_changed == -1 {
                first_changed = self.previous_lines.len() as isize;
            }
            last_changed = (new_lines.len() - 1) as isize;
        }
        (first_changed, last_changed, appended)
    }

    pub(super) fn detect_application_owned_vertical_shift(
        &self,
        new_lines: &[String],
        dock_rows: usize,
    ) -> Option<VerticalShift> {
        if new_lines.len() != self.previous_lines.len() {
            return None;
        }
        let transcript_rows = new_lines.len().saturating_sub(dock_rows);
        if transcript_rows < 2
            || !(transcript_rows..new_lines.len())
                .all(|i| previous_body(&self.previous_lines[i]) == new_lines[i])
        {
            return None;
        }

        for amount in 1..transcript_rows {
            if (0..transcript_rows - amount)
                .all(|i| previous_body(&self.previous_lines[i + amount]) == new_lines[i])
            {
                return Some(VerticalShift::Up(amount));
            }
            if (amount..transcript_rows)
                .all(|i| previous_body(&self.previous_lines[i - amount]) == new_lines[i])
            {
                return Some(VerticalShift::Down(amount));
            }
        }
        None
    }

    /// Shift only terminal rows for an AO wheel frame, without DECSTBM or CSI
    /// S/T. IL/DL leave the hardware cursor at the explicit CUP origin; redraw
    /// the exposed transcript edge and dock in the same synchronized batch.
    pub(super) fn application_owned_vertical_shift_render(
        &mut self,
        new_lines: &[String],
        dock_rows: usize,
        shift: VerticalShift,
    ) {
        let transcript_rows = new_lines.len().saturating_sub(dock_rows);
        let mut buf = String::from(BEGIN_RENDER_BATCH);
        buf.push_str("\x1b[1;1H");

        match shift {
            VerticalShift::Up(amount) => {
                buf.push_str(&format!("\x1b[{amount}M"));
                append_rewrite_lines(
                    &mut buf,
                    new_lines,
                    transcript_rows.saturating_sub(amount),
                    new_lines.len(),
                );
            }
            VerticalShift::Down(amount) => {
                buf.push_str(&format!("\x1b[{amount}L"));
                append_rewrite_lines(&mut buf, new_lines, 0, amount);
                append_rewrite_lines(&mut buf, new_lines, transcript_rows, new_lines.len());
            }
        }

        let final_row = new_lines.len().saturating_sub(1);
        buf.push_str(&format!("\x1b[{};1H", final_row + 1));
        buf.push_str(END_RENDER_BATCH);
        self.terminal.write(&buf);

        self.cursor_row = final_row;
        self.hardware_cursor_row = final_row;
        self.previous_viewport_top = 0;
        self.max_lines_rendered = self.max_lines_rendered.max(new_lines.len());
    }

    pub(super) fn differential_render(
        &mut self,
        new_lines: &[String],
        _width: usize,
        height: usize,
        first_changed_in: isize,
        last_changed: isize,
        appended: bool,
    ) {
        let prev_viewport_top = self.previous_viewport_top;

        // All-deletions branch: content only shrank. Clear the surplus lines.
        // Full-clear guards already handled in `do_render` (`deletions_need_full`).
        if first_changed_in >= new_lines.len() as isize {
            if self.previous_lines.len() > new_lines.len() {
                let mut buf = String::from(BEGIN_RENDER_BATCH);
                let target = new_lines.len().saturating_sub(1);
                // Viewport-relative move (pi computeLineDiff).
                let current_screen = (self.hardware_cursor_row as isize
                    - prev_viewport_top as isize)
                    .clamp(0, height.saturating_sub(1) as isize);
                let target_screen = target as isize - prev_viewport_top as isize;
                let diff = target_screen - current_screen;
                if diff > 0 {
                    buf.push_str(&format!("\x1b[{}B", diff));
                } else if diff < 0 {
                    buf.push_str(&format!("\x1b[{}A", -diff));
                }
                buf.push('\r');
                let extra = self.previous_lines.len() - new_lines.len();
                let off = if new_lines.is_empty() { 0 } else { 1 };
                if extra > 0 && off > 0 {
                    buf.push_str(&format!("\x1b[{}B", off));
                }
                for i in 0..extra {
                    buf.push_str("\r\x1b[2K");
                    if i < extra - 1 {
                        buf.push_str("\x1b[1B");
                    }
                }
                let back = extra.saturating_sub(1) + off;
                if back > 0 {
                    buf.push_str(&format!("\x1b[{}A", back));
                }
                buf.push_str(END_RENDER_BATCH);
                self.terminal.write(&buf);
                self.cursor_row = target;
                self.hardware_cursor_row = target;
                self.previous_viewport_top = height.max(new_lines.len()).saturating_sub(height);
                self.max_lines_rendered = self.max_lines_rendered.max(new_lines.len());
            }
            return;
        }

        let first_changed = first_changed_in as usize;
        let append_start =
            appended && first_changed == self.previous_lines.len() && first_changed > 0;
        let move_target = if append_start {
            first_changed - 1
        } else {
            first_changed
        };

        let mut buf = String::from(BEGIN_RENDER_BATCH);
        let mut hardware_cursor_row = self.hardware_cursor_row;

        // Viewport scroll (pi Step 5C, tui.ts:1466-1478): if the changed region
        // falls below the previous viewport bottom, CUD to the screen's last row
        // then emit `\r\n` to scroll the terminal — lifting prevViewportTop so
        // the diff writes land on visible rows instead of past the screen edge.
        let mut prev_viewport_top = self.previous_viewport_top;
        let prev_viewport_bottom = prev_viewport_top + height.saturating_sub(1);
        if move_target > prev_viewport_bottom {
            let current_screen_row = (hardware_cursor_row as isize - prev_viewport_top as isize)
                .clamp(0, height.saturating_sub(1) as isize)
                as usize;
            let move_to_bottom = height - 1 - current_screen_row;
            if move_to_bottom > 0 {
                buf.push_str(&format!("\x1b[{}B", move_to_bottom));
            }
            let scroll = move_target - prev_viewport_bottom;
            if scroll > 0 {
                buf.push_str(&"\r\n".repeat(scroll));
            }
            prev_viewport_top += scroll;
            // pi: hardwareCursorRow = moveTargetRow after scroll.
            hardware_cursor_row = move_target;
        }

        // Move to the target row (relative to the current viewport top) and
        // start the changed region.
        let screen_row = move_target as isize - prev_viewport_top as isize;
        let cursor_diff = screen_row - (hardware_cursor_row as isize - prev_viewport_top as isize);
        if cursor_diff > 0 {
            buf.push_str(&format!("\x1b[{}B", cursor_diff));
        } else if cursor_diff < 0 {
            buf.push_str(&format!("\x1b[{}A", -cursor_diff));
        }
        buf.push_str(if append_start { "\r\n" } else { "\r" });

        let end = last_changed.min((new_lines.len() - 1) as isize) as usize;
        let start = first_changed;
        for (i, line) in new_lines.iter().enumerate().take(end + 1).skip(start) {
            if i > start {
                buf.push_str("\r\n");
            }
            buf.push_str("\x1b[2K");
            buf.push_str(line);
        }

        // Shrink cleanup in the same sync batch (pi tui.ts:1555-1568): move to
        // content end if we stopped early, clear surplus rows, step back.
        let mut final_cursor_row = end;
        if self.previous_lines.len() > new_lines.len() && !appended {
            let content_end = new_lines.len().saturating_sub(1);
            if end < content_end {
                let move_down = content_end - end;
                buf.push_str(&format!("\x1b[{}B", move_down));
                final_cursor_row = content_end;
            }
            let extra = self.previous_lines.len() - new_lines.len();
            for _ in 0..extra {
                buf.push_str("\r\n\x1b[2K");
            }
            if extra > 0 {
                buf.push_str(&format!("\x1b[{}A", extra));
            }
        }

        buf.push_str(END_RENDER_BATCH);
        self.terminal.write(&buf);

        self.cursor_row = new_lines.len().saturating_sub(1);
        self.hardware_cursor_row = final_cursor_row;
        self.previous_viewport_top =
            prev_viewport_top.max(final_cursor_row.saturating_sub(height.saturating_sub(1)));
        self.max_lines_rendered = self.max_lines_rendered.max(new_lines.len());
    }
}
