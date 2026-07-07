//! In-memory virtual terminal — the test oracle for the engine (c399).
//!
//! Corresponds to pi-tui's `@xterm/headless` `VirtualTerminal` (skill
//! `terminal-foundations.md` testing strategy). Feeds the engine's `write` ANSI
//! stream into a cell grid, so tests can assert on the resulting cells (cursor
//! position, content, styles per row) rather than on raw escape strings.
//!
//! This is the round-trip partner of `style.rs`'s serializer: widget →
//! `StyledLine` → `to_ansi()` → engine diff → `terminal.write(bytes)` → here we
//! parse back to cells. If the round-trip is correct, what the user sees is what
//! the widget produced.
//!
//! Scope: a minimal SGR + cursor-move + clear parser. Not a full terminal
//! emulator (no scrollback history, no alternate screen, no Kitty images) —
//! enough to assert the engine's visible output.

use super::style::Color;

/// One cell of the virtual terminal grid.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct VCell {
    pub ch: char,
    pub style: VStyle,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct VStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
}

/// A virtual terminal: a cell grid that parses an ANSI stream. Cursor moves,
/// clears, and SGR updates mutate the grid; test helpers read it back.
pub struct VirtualTerminal {
    pub width: u16,
    pub height: u16,
    /// Row-major grid: `grid[y][x]`. Grows vertically as content arrives.
    pub grid: Vec<Vec<VCell>>,
    cursor_x: u16,
    cursor_y: u16,
    current_style: VStyle,
}

impl VirtualTerminal {
    pub fn new(width: u16, height: u16) -> Self {
        let grid = (0..height)
            .map(|_| vec![VCell::default(); width as usize])
            .collect();
        Self {
            width,
            height,
            grid,
            cursor_x: 0,
            cursor_y: 0,
            current_style: VStyle::default(),
        }
    }

    /// Feed an ANSI byte stream (what the engine wrote). Parses SGR, cursor
    /// moves (A/B/C/D/G/H), line clear (2K), screen clear (2J/3J), CR, LF.
    pub fn feed(&mut self, input: &str) {
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\x1b' => self.handle_escape(&mut chars),
                '\r' => self.cursor_x = 0,
                '\n' => {
                    // LF: move down AND carriage-return (be robust to bare LF;
                    // engine writes "\r\n" pairs anyway).
                    self.cursor_y = self.cursor_y.saturating_add(1);
                    self.cursor_x = 0;
                    self.ensure_row(self.cursor_y);
                }
                _ => self.put_char(ch),
            }
        }
    }

    fn handle_escape(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        match chars.next() {
            Some('[') => self.handle_csi(chars),
            Some(']') => self.handle_osc(chars),
            Some('_') => self.skip_until_st_or_bel(chars),
            Some(_) => { /* other 2-char ESC seqs: consume one */ }
            None => {}
        }
    }

    fn handle_csi(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        // Collect params until a final byte (0x40-0x7E).
        let mut params = String::new();
        let mut final_byte = '\0';
        for c in chars.by_ref() {
            if c.is_ascii() && (0x40..=0x7E).contains(&(c as u32)) {
                final_byte = c;
                break;
            }
            params.push(c);
        }
        let nums: Vec<i64> = params.split(';').filter_map(|s| s.parse().ok()).collect();
        match final_byte {
            'A' => self.cursor_y = self.cursor_y.saturating_sub(nums.first().copied().unwrap_or(1) as u16),
            'B' => {
                self.cursor_y += nums.first().copied().unwrap_or(1) as u16;
                self.ensure_row(self.cursor_y);
            }
            'C' => self.cursor_x += nums.first().copied().unwrap_or(1) as u16,
            'D' => self.cursor_x = self.cursor_x.saturating_sub(nums.first().copied().unwrap_or(1) as u16),
            'G' => self.cursor_x = nums.first().copied().unwrap_or(1).saturating_sub(1) as u16,
            'H' | 'f' => {
                self.cursor_y = nums.first().copied().unwrap_or(1).saturating_sub(1) as u16;
                self.cursor_x = nums.get(1).copied().unwrap_or(1).saturating_sub(1) as u16;
                self.ensure_row(self.cursor_y);
            }
            'J'
                // 2J = clear screen + home; 3J = clear scrollback (no-op here)
                if nums.first().copied().unwrap_or(0) >= 2 => {
                    for row in &mut self.grid {
                        for cell in row.iter_mut() {
                            *cell = VCell::default();
                        }
                    }
                    self.cursor_x = 0;
                    self.cursor_y = 0;
                }
            'K' => {
                // 2K = clear entire line
                let y = self.cursor_y as usize;
                if y < self.grid.len() {
                    for cell in self.grid[y].iter_mut() {
                        *cell = VCell::default();
                    }
                }
            }
            'm' => self.apply_sgr(&nums),
            // h/l (mode set/reset, e.g. ?2004h bracketed paste, ?2026h sync) — ignore
            'h' | 'l' => {}
            _ => {}
        }
    }

    fn handle_osc(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        // OSC terminated by BEL or ST (ESC \). Skip the payload.
        for c in chars.by_ref() {
            if c == '\x07' {
                return;
            }
            if c == '\x1b' {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                }
                return;
            }
        }
    }

    fn skip_until_st_or_bel(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        // APC/DCS/PM: skip until BEL or ST.
        for c in chars.by_ref() {
            if c == '\x07' {
                return;
            }
            if c == '\x1b' {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                }
                return;
            }
        }
    }

    fn apply_sgr(&mut self, nums: &[i64]) {
        if nums.is_empty() {
            self.current_style = VStyle::default();
            return;
        }
        let mut i = 0;
        while i < nums.len() {
            let n = nums[i];
            match n {
                0 => self.current_style = VStyle::default(),
                1 => self.current_style.bold = true,
                2 => self.current_style.dim = true,
                3 => self.current_style.italic = true,
                4 => self.current_style.underline = true,
                22 => {
                    self.current_style.bold = false;
                    self.current_style.dim = false;
                }
                23 => self.current_style.italic = false,
                24 => self.current_style.underline = false,
                30..=37 | 90..=97 => self.current_style.fg = Some(sgr_to_color(n as u8)),
                38 if i + 2 < nums.len() && nums[i + 1] == 5 => {
                    self.current_style.fg = Some(Color::Indexed(nums[i + 2] as u8));
                    i += 2;
                }
                38 if i + 4 < nums.len() && nums[i + 1] == 2 => {
                    self.current_style.fg = Some(Color::Rgb(
                        nums[i + 2] as u8,
                        nums[i + 3] as u8,
                        nums[i + 4] as u8,
                    ));
                    i += 4;
                }
                39 => self.current_style.fg = None,
                40..=47 | 100..=107 => self.current_style.bg = Some(sgr_to_color(n as u8)),
                48 if i + 2 < nums.len() && nums[i + 1] == 5 => {
                    self.current_style.bg = Some(Color::Indexed(nums[i + 2] as u8));
                    i += 2;
                }
                48 if i + 4 < nums.len() && nums[i + 1] == 2 => {
                    self.current_style.bg = Some(Color::Rgb(
                        nums[i + 2] as u8,
                        nums[i + 3] as u8,
                        nums[i + 4] as u8,
                    ));
                    i += 4;
                }
                49 => self.current_style.bg = None,
                _ => {}
            }
            i += 1;
        }
    }

    fn put_char(&mut self, ch: char) {
        let x = self.cursor_x as usize;
        let y = self.cursor_y as usize;
        self.ensure_row(self.cursor_y);
        if y < self.grid.len() && x < self.grid[y].len() {
            self.grid[y][x] = VCell {
                ch,
                style: self.current_style,
            };
        }
        self.cursor_x = self.cursor_x.saturating_add(1);
    }

    fn ensure_row(&mut self, y: u16) {
        while self.grid.len() <= y as usize {
            self.grid.push(vec![VCell::default(); self.width as usize]);
        }
    }

    // ── test helpers ──────────────────────────────────────────────────────

    /// Plain text of row `y` (trimmed of trailing blanks).
    pub fn row_text(&self, y: usize) -> String {
        let Some(row) = self.grid.get(y) else {
            return String::new();
        };
        let s: String = row
            .iter()
            .take_while(|c| c.ch != '\0')
            .map(|c| c.ch)
            .collect();
        s.trim_end().to_string()
    }

    /// Cell at (x, y), or default if out of bounds.
    pub fn cell_at(&self, x: usize, y: usize) -> VCell {
        self.grid
            .get(y)
            .and_then(|r| r.get(x))
            .copied()
            .unwrap_or_default()
    }

    /// Cursor position after the last feed.
    pub fn cursor(&self) -> (u16, u16) {
        (self.cursor_x, self.cursor_y)
    }

    /// Find the first row whose text contains `needle`. Useful when content
    /// scrolled and the exact row index isn't known.
    pub fn find_row_containing(&self, needle: &str) -> Option<usize> {
        self.grid
            .iter()
            .position(|_| true)
            .and_then(|_| (0..self.grid.len()).find(|&y| self.row_text(y).contains(needle)))
    }
}

fn sgr_to_color(code: u8) -> Color {
    match code {
        30 | 40 => Color::Black,
        31 | 41 => Color::Red,
        32 | 42 => Color::Green,
        33 | 43 => Color::Yellow,
        34 | 44 => Color::Blue,
        35 | 45 => Color::Magenta,
        36 | 46 => Color::Cyan,
        37 | 47 => Color::Gray,
        90 | 100 => Color::DarkGray,
        91 | 101 => Color::LightRed,
        92 | 102 => Color::LightGreen,
        93 | 103 => Color::LightYellow,
        94 | 104 => Color::LightBlue,
        95 | 105 => Color::LightMagenta,
        96 | 106 => Color::LightCyan,
        97 | 107 => Color::White,
        _ => Color::Reset,
    }
}

// ── c400: ScrollbackTerminal — terminal emulator with scrollback buffer ───

/// A terminal emulator that simulates real terminal scrollback: when `\r\n`
/// is written while the cursor is on the bottom screen row, the top screen row
/// is pushed into a scrollback buffer and a new blank row appears at the
/// bottom. This lets tests verify that "bottom-anchored" content (input,
/// loader) stays in the last visible rows even as the conversation grows.
///
/// Unlike `VirtualTerminal` (which grows upward infinitely), `ScrollbackTerminal`
/// has a fixed-height screen just like a physical terminal, so `\r\n` at the
/// bottom row triggers real scroll behavior.
pub struct ScrollbackTerminal {
    pub width: u16,
    pub height: u16,
    /// Rows currently visible on screen. `screen[0]` = top row.
    pub screen: Vec<Vec<VCell>>,
    /// Rows pushed out of the screen by \r\n scrolling (in order, oldest first).
    pub scrollback: Vec<Vec<VCell>>,
    cursor_x: u16,
    cursor_y: u16,
    current_style: VStyle,
}

impl ScrollbackTerminal {
    pub fn new(width: u16, height: u16) -> Self {
        let screen = (0..height)
            .map(|_| vec![VCell::default(); width as usize])
            .collect();
        Self {
            width,
            height,
            screen,
            scrollback: Vec::new(),
            cursor_x: 0,
            cursor_y: 0,
            current_style: VStyle::default(),
        }
    }

    /// Feed an ANSI byte stream. Parses SGR, cursor moves, clears,
    /// and simulates `\r\n` scrollback.
    pub fn feed(&mut self, input: &str) {
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\x1b' => self.handle_escape(&mut chars),
                '\r' => self.cursor_x = 0,
                '\n' => {
                    // LF: carriage-return + move down. At bottom row, scroll.
                    self.cursor_x = 0;
                    if self.cursor_y >= self.height.saturating_sub(1) {
                        self.scroll_up();
                    } else {
                        self.cursor_y += 1;
                    }
                }
                _ => self.put_char(ch),
            }
        }
    }

    /// Push the top screen row into scrollback, shift all rows up,
    /// add a blank row at the bottom (simulating terminal scroll).
    fn scroll_up(&mut self) {
        if self.screen.is_empty() {
            return;
        }
        if let Some(top_row) = self.screen.first() {
            self.scrollback.push(top_row.clone());
        }
        self.screen.remove(0);
        self.screen
            .push(vec![VCell::default(); self.width as usize]);
        // Cursor stays at the bottom row.
    }

    // ── helpers (mirror VirtualTerminal) ──────────────────────────────────

    fn put_char(&mut self, ch: char) {
        let x = self.cursor_x as usize;
        let y = self.cursor_y as usize;
        if y < self.screen.len() && x < self.screen[y].len() {
            self.screen[y][x] = VCell {
                ch,
                style: self.current_style,
            };
        }
        self.cursor_x = self.cursor_x.saturating_add(1);
    }

    fn handle_escape(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        match chars.next() {
            Some('[') => self.handle_csi(chars),
            Some(']') => self.handle_osc(chars),
            Some('_') => self.skip_aps(chars),
            Some(_) => {}
            None => {}
        }
    }

    fn handle_csi(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        let mut params = String::new();
        let mut final_byte = '\0';
        for c in chars.by_ref() {
            if c.is_ascii() && (0x40..=0x7E).contains(&(c as u32)) {
                final_byte = c;
                break;
            }
            params.push(c);
        }
        let nums: Vec<i64> = params.split(';').filter_map(|s| s.parse().ok()).collect();
        match final_byte {
            'A' => {
                let n = nums.first().copied().unwrap_or(1) as u16;
                self.cursor_y = self.cursor_y.saturating_sub(n);
            }
            'B' => {
                let n = nums.first().copied().unwrap_or(1) as u16;
                self.cursor_y = (self.cursor_y + n).min(self.height.saturating_sub(1));
            }
            'C' => {
                let n = nums.first().copied().unwrap_or(1) as u16;
                self.cursor_x = (self.cursor_x + n).min(self.width.saturating_sub(1));
            }
            'D' => {
                let n = nums.first().copied().unwrap_or(1) as u16;
                self.cursor_x = self.cursor_x.saturating_sub(n);
            }
            'G' => {
                let col = nums.first().copied().unwrap_or(1).saturating_sub(1) as u16;
                self.cursor_x = col.min(self.width.saturating_sub(1));
            }
            'H' | 'f' => {
                let row = nums.first().copied().unwrap_or(1).saturating_sub(1) as u16;
                let col = nums.get(1).copied().unwrap_or(1).saturating_sub(1) as u16;
                self.cursor_y = row.min(self.height.saturating_sub(1));
                self.cursor_x = col.min(self.width.saturating_sub(1));
            }
            'J' if nums.first().copied().unwrap_or(0) >= 2 => {
                // 2J = clear screen + home. Also clear scrollback.
                for row in &mut self.screen {
                    for cell in row.iter_mut() {
                        *cell = VCell::default();
                    }
                }
                self.scrollback.clear();
                self.cursor_x = 0;
                self.cursor_y = 0;
            }
            'K' => {
                let y = self.cursor_y as usize;
                if y < self.screen.len() {
                    for cell in self.screen[y].iter_mut() {
                        *cell = VCell::default();
                    }
                }
            }
            'm' => self.apply_sgr(&nums),
            'h' | 'l' => {} // mode set/reset — ignore
            _ => {}
        }
    }

    fn apply_sgr(&mut self, nums: &[i64]) {
        if nums.is_empty() {
            self.current_style = VStyle::default();
            return;
        }
        let mut i = 0;
        while i < nums.len() {
            let n = nums[i];
            match n {
                0 => self.current_style = VStyle::default(),
                1 => self.current_style.bold = true,
                2 => self.current_style.dim = true,
                3 => self.current_style.italic = true,
                4 => self.current_style.underline = true,
                22 => {
                    self.current_style.bold = false;
                    self.current_style.dim = false;
                }
                23 => self.current_style.italic = false,
                24 => self.current_style.underline = false,
                30..=37 | 90..=97 => self.current_style.fg = Some(sgr_to_color(n as u8)),
                38 if i + 2 < nums.len() && nums[i + 1] == 5 => {
                    self.current_style.fg = Some(Color::Indexed(nums[i + 2] as u8));
                    i += 2;
                }
                38 if i + 4 < nums.len() && nums[i + 1] == 2 => {
                    self.current_style.fg = Some(Color::Rgb(
                        nums[i + 2] as u8,
                        nums[i + 3] as u8,
                        nums[i + 4] as u8,
                    ));
                    i += 4;
                }
                39 => self.current_style.fg = None,
                40..=47 | 100..=107 => self.current_style.bg = Some(sgr_to_color(n as u8)),
                48 if i + 2 < nums.len() && nums[i + 1] == 5 => {
                    self.current_style.bg = Some(Color::Indexed(nums[i + 2] as u8));
                    i += 2;
                }
                48 if i + 4 < nums.len() && nums[i + 1] == 2 => {
                    self.current_style.bg = Some(Color::Rgb(
                        nums[i + 2] as u8,
                        nums[i + 3] as u8,
                        nums[i + 4] as u8,
                    ));
                    i += 4;
                }
                49 => self.current_style.bg = None,
                _ => {}
            }
            i += 1;
        }
    }

    fn handle_osc(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        for c in chars.by_ref() {
            if c == '\x07' {
                return;
            }
            if c == '\x1b' {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                }
                return;
            }
        }
    }

    fn skip_aps(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        for c in chars.by_ref() {
            if c == '\x07' {
                return;
            }
            if c == '\x1b' {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                }
                return;
            }
        }
    }

    // ── test helpers ──────────────────────────────────────────────────────

    /// Plain text of screen row `y` (0 = top, trimmed trailing blanks).
    pub fn screen_text(&self, y: usize) -> String {
        let Some(row) = self.screen.get(y) else {
            return String::new();
        };
        let s: String = row
            .iter()
            .take_while(|c| c.ch != '\0')
            .map(|c| c.ch)
            .collect();
        s.trim_end().to_string()
    }

    /// Whether screen row `y` contains `needle`.
    pub fn screen_contains(&self, y: usize, needle: &str) -> bool {
        self.screen_text(y).contains(needle)
    }

    /// Check that the last `line_count` screen rows are all non-empty
    /// (at least one non-`\0` cell each), verifying that bottom-anchored
    /// content is present in the bottom screen positions.
    pub fn footer_visible(&self, line_count: usize) -> bool {
        let start = self.screen.len().saturating_sub(line_count);
        for row in &self.screen[start..] {
            if row.iter().all(|c| c.ch == '\0') {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::engine::component::Component;
    use crate::app::tui::engine::style::{CellStyle, Span, StyledLine};
    use crate::app::tui::engine::terminal::CapturingTerminal;
    use crate::app::tui::engine::tui::Tui;

    /// Widget emitting fixed lines — shared with the tui.rs tests' helper shape.
    struct LinesWidget {
        lines: Vec<StyledLine>,
    }
    impl Component for LinesWidget {
        fn render(&self, _width: usize) -> Vec<StyledLine> {
            self.lines.clone()
        }
    }

    /// End-to-end: widget → engine → ANSI → VirtualTerminal cell grid.
    /// Asserts the round-trip (widget output == what the user sees).
    fn render_e2e(lines: Vec<StyledLine>, width: u16, height: u16) -> VirtualTerminal {
        let term = CapturingTerminal::new(width, height);
        let widget = LinesWidget { lines };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        let written = tui.term_mut().written.clone();
        let mut vt = VirtualTerminal::new(width, height);
        vt.feed(&written);
        vt
    }

    #[test]
    fn plain_text_round_trips_to_grid() {
        let vt = render_e2e(
            vec![StyledLine::raw("hello"), StyledLine::raw("world")],
            80,
            24,
        );
        assert_eq!(vt.row_text(0), "hello");
        assert_eq!(vt.row_text(1), "world");
    }

    #[test]
    fn styled_text_carries_color_to_grid() {
        let line = StyledLine::from_spans(vec![Span::styled(
            "red",
            CellStyle::default().fg(Color::Red),
        )]);
        let vt = render_e2e(vec![line], 80, 24);
        let cell = vt.cell_at(0, 0);
        assert_eq!(cell.ch, 'r');
        assert_eq!(cell.style.fg, Some(Color::Red));
    }

    #[test]
    fn bold_round_trips() {
        let line = StyledLine::from_spans(vec![Span::styled("b", CellStyle::default().bold())]);
        let vt = render_e2e(vec![line], 80, 24);
        assert!(vt.cell_at(0, 0).style.bold);
    }

    #[test]
    fn diff_update_reflects_in_grid() {
        // First frame: "a". Second frame: "a" + "b". Grid should show both
        // somewhere (diff appends "b"; the cursor may land on a later row).
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![StyledLine::raw("a")],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();

        let widget2 = LinesWidget {
            lines: vec![StyledLine::raw("a"), StyledLine::raw("b")],
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();

        let mut vt = VirtualTerminal::new(80, 24);
        vt.feed(&tui.term_mut().written);
        // "a" from the first render is in the grid.
        assert!(
            vt.find_row_containing("a").is_some(),
            "first-frame content present"
        );
        // "b" was appended via diff and must appear somewhere in the grid.
        assert!(
            vt.find_row_containing("b").is_some(),
            "appended line appears in grid: rows={:?}",
            (0..3).map(|y| vt.row_text(y)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn in_place_edit_updates_grid() {
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![StyledLine::raw("old")],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();

        let widget2 = LinesWidget {
            lines: vec![StyledLine::raw("NEW")],
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();

        let mut vt = VirtualTerminal::new(80, 24);
        vt.feed(&tui.term_mut().written);
        assert_eq!(
            vt.row_text(0),
            "NEW",
            "in-place edit reflected; clear-line + rewrite"
        );
    }

    #[test]
    fn cursor_marker_does_not_appear_in_grid() {
        // The engine strips CURSOR_MARKER before writing, so the grid never
        // contains the APC bytes (they'd be zero-width anyway, but the strip
        // ensures clean text).
        use crate::app::tui::engine::style::CURSOR_MARKER;
        let line = StyledLine::from_spans(vec![
            Span::raw("ab"),
            Span::raw(format!("{CURSOR_MARKER}cd")),
        ]);
        let vt = render_e2e(vec![line], 80, 24);
        assert_eq!(vt.row_text(0), "abcd");
        // No stray ESC bytes in any cell.
        for row in &vt.grid {
            for cell in row {
                assert!(cell.ch != '\x1b', "no ESC leak: {:?}", cell);
            }
        }
    }

    // ── unit-level VirtualTerminal parsing ────────────────────────────────

    #[test]
    fn vt_parses_simple_text() {
        let mut vt = VirtualTerminal::new(10, 3);
        vt.feed("hi");
        assert_eq!(vt.row_text(0), "hi");
        assert_eq!(vt.cursor(), (2, 0));
    }

    #[test]
    fn vt_parses_newline_and_cr() {
        let mut vt = VirtualTerminal::new(10, 3);
        vt.feed("a\nb\r");
        assert_eq!(vt.row_text(0), "a");
        assert_eq!(vt.row_text(1), "b");
        assert_eq!(vt.cursor(), (0, 1));
    }

    #[test]
    fn vt_parses_sgr_reset() {
        let mut vt = VirtualTerminal::new(10, 3);
        vt.feed("\x1b[31mr\x1b[0mg");
        assert_eq!(vt.cell_at(0, 0).style.fg, Some(Color::Red));
        assert_eq!(vt.cell_at(1, 0).style.fg, None, "reset clears fg");
    }

    #[test]
    fn vt_parses_clear_line() {
        let mut vt = VirtualTerminal::new(10, 3);
        vt.feed("hello");
        vt.feed("\r\x1b[2K");
        assert_eq!(vt.row_text(0), "", "2K clears the line");
    }

    #[test]
    fn vt_parses_clear_screen() {
        let mut vt = VirtualTerminal::new(10, 3);
        vt.feed("hello\nworld");
        vt.feed("\x1b[2J\x1b[H");
        assert_eq!(vt.row_text(0), "");
        assert_eq!(vt.row_text(1), "");
        assert_eq!(vt.cursor(), (0, 0));
    }

    #[test]
    fn vt_parses_256_color() {
        let mut vt = VirtualTerminal::new(10, 3);
        vt.feed("\x1b[38;5;202mx");
        assert_eq!(vt.cell_at(0, 0).style.fg, Some(Color::Indexed(202)));
    }

    #[test]
    fn vt_parses_truecolor() {
        let mut vt = VirtualTerminal::new(10, 3);
        vt.feed("\x1b[38;2;1;2;3mx");
        assert_eq!(vt.cell_at(0, 0).style.fg, Some(Color::Rgb(1, 2, 3)));
    }

    // ── c400: ScrollbackTerminal unit tests ───────────────────────────

    #[test]
    fn scrollback_terminal_scrolls_on_newline_at_bottom() {
        let mut st = ScrollbackTerminal::new(10, 3);
        st.feed("row0\r\nrow1\r\nrow2\r\nrow3");
        // height=3: row0 scrolled out, row1-row3 on screen.
        // No trailing \r\n after row3 (real terminal: last line visible).
        assert_eq!(st.screen_text(0), "row1");
        assert_eq!(st.screen_text(1), "row2");
        assert_eq!(st.screen_text(2), "row3");
    }

    #[test]
    fn scrollback_terminal_accumulates_scrollback() {
        let mut st = ScrollbackTerminal::new(10, 2);
        for i in 0..5 {
            let line = format!("line{i}");
            if i < 4 {
                st.feed(&format!("{line}\r\n"));
            } else {
                st.feed(&line);
            }
        }
        // height=2: 5 lines written, 3 scrolled into buffer, last 2 on screen.
        assert_eq!(st.screen_text(0), "line3");
        assert_eq!(st.screen_text(1), "line4");
        assert_eq!(st.scrollback.len(), 3);
    }

    #[test]
    fn scrollback_terminal_clear_screen_clears_scrollback() {
        let mut st = ScrollbackTerminal::new(10, 2);
        st.feed("a\r\nb\r\nc\r\nd");
        assert_eq!(st.scrollback.len(), 2, "2 lines in scrollback");
        st.feed("\x1b[2J\x1b[H");
        assert_eq!(
            st.scrollback.len(),
            0,
            "clear screen also clears scrollback"
        );
        assert!(st.screen_text(0).is_empty());
    }

    #[test]
    fn scrollback_terminal_footer_visible() {
        let mut st = ScrollbackTerminal::new(10, 4);
        // Write 4 lines — fill screen, bottom 2 occupied.
        st.feed("a\r\nb\r\nc\r\nd");
        assert!(st.footer_visible(2), "last 2 rows present");
        // Write 4 more — push old content into scrollback.
        st.feed("\r\ne\r\nf\r\ng\r\nh");
        assert_eq!(st.screen.len(), 4, "fixed screen height");
        assert!(st.scrollback.len() >= 4, "old content scrolled");
    }

    // ── c400: engine-to-ScrollbackTerminal integration tests ──────────

    #[test]
    fn footer_stays_visible_as_content_grows_past_height() {
        let height: u16 = 8;
        let width: u16 = 80;
        let term = CapturingTerminal::new(width, height);
        let mut tui = Tui::new(
            term,
            Box::new(LinesWidget {
                lines: vec![StyledLine::raw("line0")],
            }),
        );
        let mut st = ScrollbackTerminal::new(width, height);

        // Render frames with growing content.
        for n in 2..=12 {
            let lines: Vec<StyledLine> = (0..n)
                .map(|i| StyledLine::raw(&format!("line{i}")))
                .collect();
            tui.set_root(Box::new(LinesWidget { lines }));
            tui.render_now().unwrap();
            st.feed(&tui.term_mut().written);
            tui.term_mut().written.clear();
        }

        // After all frames: verify scrollback captured lines that scrolled
        // out, and the screen shows the tail at the bottom.
        assert!(
            !st.scrollback.is_empty(),
            "content past height should scroll lines into scrollback"
        );
        // Last screen row should contain "line11" (the last line rendered).
        let last_screen = st.screen_text((height - 1) as usize);
        assert!(
            last_screen.contains("line11"),
            "last screen row should show newest line, got: {last_screen:?}"
        );
    }

    #[test]
    fn footer_content_never_overlaps_with_history() {
        let height: u16 = 8;
        let width: u16 = 80;
        let term = CapturingTerminal::new(width, height);
        let mut tui = Tui::new(
            term,
            Box::new(LinesWidget {
                lines: vec![StyledLine::raw("history 0")],
            }),
        );
        let mut st = ScrollbackTerminal::new(width, height);

        // 15 lines of history + 2 fixed footer lines.
        let labels: Vec<String> = (0..15).map(|i| format!("history {i}")).collect();
        let footer_label = "<input>";
        let loader_label = "Ready";

        // First frame with footer lines.
        let lines: Vec<StyledLine> = labels
            .iter()
            .map(|l| StyledLine::raw(l))
            .chain(std::iter::once(StyledLine::raw(loader_label)))
            .chain(std::iter::once(StyledLine::raw(footer_label)))
            .collect();
        let widget = LinesWidget { lines };
        tui.set_root(Box::new(widget));
        tui.render_now().unwrap();
        st.feed(&tui.term_mut().written);
        tui.term_mut().written.clear();

        // After first render (17 lines, height=8): footer must be on
        // the last 2 screen rows.
        assert!(
            st.screen_contains((height - 1) as usize, footer_label),
            "footer line on last screen row"
        );
        assert!(
            st.screen_contains((height - 2) as usize, loader_label),
            "loader line on second-to-last screen row"
        );
        // History should be in scrollback, not overlaying footer.
        assert!(
            st.scrollback.len() >= 7,
            "enough history scrolled out (9 visible - 8 height = 9 in scrollback? check)"
        );
    }
}
