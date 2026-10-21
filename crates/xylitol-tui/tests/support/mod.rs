//! Not part of the public API; `#[cfg(test)]` / test-crates only.
//! `#[allow(dead_code)]` 说明：Cargo 自动发现 tests/ 下 12 个 integration
pub mod vt_feed;

use vte::{Params, Perform};

use xylitol_tui::terminal::Terminal;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Color {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

pub struct VirtualTerminal {
    cols: u16,
    rows: u16,
    scroll_top: usize,
    scroll_bottom: usize,
    grid: Vec<Vec<Cell>>,
    cursor_row: usize,
    cursor_col: usize,
    auto_wrap: bool,
    attrs: Cell,
    title: Option<String>,
    mouse_capture_desired: bool,
    mouse_capture_active: bool,
    alternate_screen_active: bool,
}

#[allow(dead_code)]
impl VirtualTerminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        let rows_us = rows as usize;
        let cols_us = cols as usize;
        Self {
            cols,
            rows,
            scroll_top: 0,
            scroll_bottom: rows_us.saturating_sub(1),
            grid: vec![vec![Cell::default(); cols_us]; rows_us],
            cursor_row: 0,
            cursor_col: 0,
            auto_wrap: true,
            attrs: Cell::default(),
            title: None,
            mouse_capture_desired: false,
            mouse_capture_active: false,
            alternate_screen_active: false,
        }
    }

    #[allow(dead_code)]
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let new_rows = rows as usize;
        let new_cols = cols as usize;
        self.grid.resize(new_rows, vec![Cell::default(); new_cols]);
        for row in &mut self.grid {
            row.resize(new_cols, Cell::default());
        }
        self.cols = cols;
        self.rows = rows;
        self.scroll_top = 0;
        self.scroll_bottom = new_rows.saturating_sub(1);
        self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
    }

    pub fn viewport(&self) -> Vec<String> {
        let top = self.viewport_top();
        let height = self.rows as usize;
        (top..top + height)
            .map(|r| {
                if r < self.grid.len() {
                    row_to_trimmed_string(&self.grid[r])
                } else {
                    String::new()
                }
            })
            .collect()
    }

    pub fn scroll_buffer(&self) -> Vec<String> {
        self.grid.iter().map(|r| row_to_trimmed_string(r)).collect()
    }

    fn viewport_top(&self) -> usize {
        let height = self.rows as usize;
        self.grid.len().saturating_sub(height)
    }

    pub fn cell(&self, row: usize, col: usize) -> Cell {
        self.grid
            .get(row)
            .and_then(|r| r.get(col))
            .cloned()
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn viewport_cell(&self, row: usize, col: usize) -> Cell {
        let abs = self.viewport_top() + row;
        self.cell(abs, col)
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        let top = self.viewport_top();
        (self.cursor_col, self.cursor_row.saturating_sub(top))
    }

    pub fn viewport_top_pub(&self) -> usize {
        self.viewport_top()
    }

    pub fn grid_len(&self) -> usize {
        self.grid.len()
    }

    pub fn grid_row(&self, row: usize) -> &[Cell] {
        self.grid.get(row).map(|r| r.as_slice()).unwrap_or(&[])
    }

    #[allow(dead_code)]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn ensure_row(&mut self, row: usize) {
        let cols = self.cols as usize;
        while self.grid.len() <= row {
            self.grid.push(vec![Cell::default(); cols]);
        }
    }

    fn print_at_cursor(&mut self, ch: char) {
        let row = self.cursor_row;
        let col = self.cursor_col;
        self.ensure_row(row);
        let cols = self.cols as usize;
        if col < cols {
            let cell = Cell { ch, ..self.attrs };
            self.grid[row][col] = cell;
        }
        self.cursor_col += 1;
        if self.cursor_col >= self.cols as usize {
            if self.auto_wrap {
                self.cursor_col = 0;
                self.cursor_row += 1;
                self.ensure_row(self.cursor_row);
            } else {
                self.cursor_col = self.cols as usize - 1;
            }
        }
    }

    fn line_feed(&mut self) {
        self.cursor_row += 1;
        self.ensure_row(self.cursor_row);
        let cols = self.cols as usize;
        if self.grid[self.cursor_row].len() < cols {
            self.grid[self.cursor_row].resize(cols, Cell::default());
        }
    }

    fn carriage_return(&mut self) {
        self.cursor_col = 0;
    }

    fn move_cursor_to(&mut self, row: usize, col: usize) {
        let cols = self.cols.max(1) as usize;
        let target_row = row.saturating_sub(1);
        let target_col = col.saturating_sub(1).min(cols.saturating_sub(1));
        self.ensure_row(target_row);
        self.cursor_row = target_row;
        self.cursor_col = target_col;
    }

    fn move_cursor_row(&mut self, delta: isize) {
        let target = self.cursor_row as isize + delta;
        self.cursor_row = target.max(0) as usize;
        self.ensure_row(self.cursor_row);
    }

    fn move_cursor_col(&mut self, delta: isize) {
        let cols = self.cols as usize;
        let target = self.cursor_row as isize;
        let _ = target;
        let new_col = (self.cursor_col as isize + delta).max(0) as usize;
        self.cursor_col = new_col.min(cols.saturating_sub(1));
    }

    fn erase_in_display(&mut self, mode: u16) {
        let cols = self.cols as usize;
        match mode {
            2 | 3 => {
                self.grid = vec![vec![Cell::default(); cols]; self.rows as usize];
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            0 => {
                self.clear_row_range(self.cursor_row, self.cursor_col, cols);
                for r in (self.cursor_row + 1)..self.grid.len() {
                    self.clear_row_range(r, 0, cols);
                }
            }
            1 => {
                for r in 0..self.cursor_row {
                    self.clear_row_range(r, 0, cols);
                }
                self.clear_row_range(self.cursor_row, 0, self.cursor_col + 1);
            }
            _ => {}
        }
    }

    fn erase_in_line(&mut self, mode: u16) {
        let cols = self.cols as usize;
        let end = match mode {
            0 => cols,
            1 => self.cursor_col + 1,
            2 => cols,
            _ => return,
        };
        let start = match mode {
            1 => 0,
            _ => self.cursor_col,
        };
        self.clear_row_range(self.cursor_row, start, end);
    }

    fn clear_row_range(&mut self, row: usize, start: usize, end: usize) {
        if let Some(r) = self.grid.get_mut(row) {
            for c in start..end.min(r.len()) {
                r[c] = Cell::default();
            }
        }
    }

    fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let last = self.rows.saturating_sub(1) as usize;
        self.scroll_top = top.min(last);
        self.scroll_bottom = bottom.min(last).max(self.scroll_top);
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    fn scroll_region(&mut self, count: usize, up: bool) {
        let count = count
            .max(1)
            .min(self.scroll_bottom.saturating_sub(self.scroll_top) + 1);
        let region_len = self.scroll_bottom - self.scroll_top + 1;
        let overlap = region_len - count;
        let blank = vec![Cell::default(); self.cols as usize];
        if up {
            for offset in 0..overlap {
                self.grid[self.scroll_top + offset] =
                    self.grid[self.scroll_top + offset + count].clone();
            }
            for row in self.scroll_bottom + 1 - count..=self.scroll_bottom {
                self.grid[row] = blank.clone();
            }
        } else {
            for offset in (0..overlap).rev() {
                self.grid[self.scroll_top + offset + count] =
                    self.grid[self.scroll_top + offset].clone();
            }
            for row in self.scroll_top..self.scroll_top + count {
                self.grid[row] = blank.clone();
            }
        }
    }

    fn insert_delete_lines(&mut self, count: usize, insert: bool) {
        if self.cursor_row < self.scroll_top || self.cursor_row > self.scroll_bottom {
            return;
        }
        let top = self.cursor_row;
        let count = count.max(1).min(self.scroll_bottom.saturating_sub(top) + 1);
        let overlap = self.scroll_bottom - top + 1 - count;
        let blank = vec![Cell::default(); self.cols as usize];
        if insert {
            for offset in (0..overlap).rev() {
                self.grid[top + offset + count] = self.grid[top + offset].clone();
            }
            self.grid[top..top + count].fill(blank.clone());
        } else {
            for offset in 0..overlap {
                self.grid[top + offset] = self.grid[top + offset + count].clone();
            }
            let start = self.scroll_bottom + 1 - count;
            self.grid[start..=self.scroll_bottom].fill(blank.clone());
        }
    }

    fn apply_sgr(&mut self, params: &Params) {
        let flat: Vec<u16> = params.iter().flat_map(|s| s.iter().copied()).collect();
        let mut i = 0;
        while i < flat.len() {
            let v = flat[i];
            match v {
                0 => self.attrs = Cell::default(),
                1 => self.attrs.bold = true,
                2 => self.attrs.dim = true,
                3 => self.attrs.italic = true,
                4 => self.attrs.underline = true,
                7 => self.attrs.reverse = true,
                22 => {
                    self.attrs.bold = false;
                    self.attrs.dim = false;
                }
                23 => self.attrs.italic = false,
                24 => self.attrs.underline = false,
                27 => self.attrs.reverse = false,
                30..=37 => self.attrs.fg = Color::Indexed((v - 30) as u8),
                38 => {
                    if let Some(c) = parse_extended_color(&flat, &mut i) {
                        self.attrs.fg = c;
                    }
                }
                39 => self.attrs.fg = Color::Default,
                40..=47 => self.attrs.bg = Color::Indexed((v - 40) as u8),
                48 => {
                    if let Some(c) = parse_extended_color(&flat, &mut i) {
                        self.attrs.bg = c;
                    }
                }
                49 => self.attrs.bg = Color::Default,
                90..=97 => self.attrs.fg = Color::Indexed((v - 90 + 8) as u8),
                100..=107 => self.attrs.bg = Color::Indexed((v - 100 + 8) as u8),
                _ => {}
            }
            i += 1;
        }
    }
}

fn parse_extended_color(flat: &[u16], i: &mut usize) -> Option<Color> {
    let mode = flat.get(*i + 1).copied()?;
    match mode {
        5 => {
            let idx = flat.get(*i + 2).copied()? as u8;
            *i += 2;
            Some(Color::Indexed(idx))
        }
        2 => {
            let r = flat.get(*i + 2).copied()? as u8;
            let g = flat.get(*i + 3).copied()? as u8;
            let b = flat.get(*i + 4).copied()? as u8;
            *i += 4;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

fn row_to_trimmed_string(row: &[Cell]) -> String {
    let s: String = row
        .iter()
        .map(|c| if c.ch == '\0' { ' ' } else { c.ch })
        .collect();
    s.trim_end().to_string()
}

fn nth_param(params: &Params, n: usize) -> Option<u16> {
    params
        .iter()
        .nth(n)
        .and_then(|slice| slice.first().copied())
}

impl Terminal for VirtualTerminal {
    fn write(&mut self, data: &str) {
        let mut parser = vte::Parser::new();
        let mut performer = VTPerformer { vt: self };
        parser.advance(&mut performer, data.as_bytes());
    }
    fn columns(&self) -> u16 {
        self.cols
    }
    fn rows(&self) -> u16 {
        self.rows
    }
    fn hide_cursor(&mut self) {}
    fn show_cursor(&mut self) {}
    fn clear_line(&mut self) {
        self.erase_in_line(2);
    }
    fn clear_from_cursor(&mut self) {
        self.erase_in_display(0);
    }
    fn clear_screen(&mut self) {
        self.erase_in_display(2);
    }
    fn flush(&mut self) {}

    fn set_size_hint(&mut self, cols: u16, rows: u16) {
        self.resize(cols, rows);
    }

    fn enable_mouse_capture(&mut self) {
        self.mouse_capture_desired = true;
        self.mouse_capture_active = true;
    }

    fn disable_mouse_capture(&mut self) {
        self.mouse_capture_desired = false;
        self.mouse_capture_active = false;
    }

    fn mouse_capture_active(&self) -> bool {
        self.mouse_capture_active
    }

    fn enter_alternate_screen(&mut self) {
        self.alternate_screen_active = true;
    }

    fn leave_alternate_screen(&mut self) {
        self.alternate_screen_active = false;
    }

    fn alternate_screen_active(&self) -> bool {
        self.alternate_screen_active
    }

    fn start(&mut self) {
        if self.mouse_capture_desired {
            self.mouse_capture_active = true;
        }
    }

    fn stop(&mut self) {
        self.mouse_capture_active = false;
        self.alternate_screen_active = false;
    }
}

struct VTPerformer<'a> {
    vt: &'a mut VirtualTerminal,
}

impl Perform for VTPerformer<'_> {
    fn print(&mut self, c: char) {
        self.vt.print_at_cursor(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0B | 0x0C => self.vt.line_feed(),
            b'\r' => self.vt.carriage_return(),
            b'\t' => {
                let next = ((self.vt.cursor_col / 8) + 1) * 8;
                self.vt.cursor_col = next.min(self.vt.cols as usize - 1);
            }
            0x08 => {
                self.vt.move_cursor_col(-1);
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, byte: char) {
        match byte {
            'H' | 'f' => {
                let row = nth_param(params, 0).unwrap_or(1);
                let col = nth_param(params, 1).unwrap_or(1);
                self.vt.move_cursor_to(row as usize, col as usize);
            }
            'A' => {
                let n = nth_param(params, 0).unwrap_or(1);
                self.vt.move_cursor_row(-(n.max(1) as isize));
            }
            'B' => {
                let n = nth_param(params, 0).unwrap_or(1);
                self.vt.move_cursor_row(n.max(1) as isize);
            }
            'C' => {
                let n = nth_param(params, 0).unwrap_or(1);
                self.vt.move_cursor_col(n.max(1) as isize);
            }
            'D' => {
                let n = nth_param(params, 0).unwrap_or(1);
                self.vt.move_cursor_col(-(n.max(1) as isize));
            }
            'G' => {
                let col = nth_param(params, 0).unwrap_or(1);
                self.vt.move_cursor_to(self.vt.cursor_row + 1, col as usize);
            }
            'd' => {
                let row = nth_param(params, 0).unwrap_or(1);
                self.vt.move_cursor_to(row as usize, self.vt.cursor_col + 1);
            }
            'J' => {
                let mode = nth_param(params, 0).unwrap_or(0);
                self.vt.erase_in_display(mode);
            }
            'K' => {
                let mode = nth_param(params, 0).unwrap_or(0);
                self.vt.erase_in_line(mode);
            }
            'm' => self.vt.apply_sgr(params),
            'r' => {
                let top = nth_param(params, 0).unwrap_or(1).max(1) as usize - 1;
                let bottom = nth_param(params, 1).unwrap_or(self.vt.rows).max(1) as usize - 1;
                self.vt.set_scroll_region(top, bottom);
            }
            'S' => {
                let count = nth_param(params, 0).unwrap_or(1) as usize;
                self.vt.scroll_region(count, true);
            }
            'T' => {
                let count = nth_param(params, 0).unwrap_or(1) as usize;
                self.vt.scroll_region(count, false);
            }
            'L' => {
                let count = nth_param(params, 0).unwrap_or(1) as usize;
                self.vt.insert_delete_lines(count, true);
            }
            'M' => {
                let count = nth_param(params, 0).unwrap_or(1) as usize;
                self.vt.insert_delete_lines(count, false);
            }
            'h' | 'l' if nth_param(params, 0) == Some(7) => {
                self.vt.auto_wrap = byte == 'h';
            }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if let Some(p) = params.first()
            && (p.starts_with(b"0;") || p.starts_with(b"2;"))
            && let Ok(s) = std::str::from_utf8(&p[2..])
        {
            self.vt.title = Some(s.to_string());
        }
    }
}

pub struct LoggingVirtualTerminal {
    inner: VirtualTerminal,
    writes: Vec<String>,
    cursor_visible: bool,
    show_cursor_calls: u32,
    hide_cursor_calls: u32,
    mouse_enable_calls: u32,
    mouse_disable_calls: u32,
    alt_enter_calls: u32,
    alt_leave_calls: u32,
}

#[allow(dead_code)]
impl LoggingVirtualTerminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            inner: VirtualTerminal::new(cols, rows),
            writes: Vec::new(),
            cursor_visible: true,
            show_cursor_calls: 0,
            hide_cursor_calls: 0,
            mouse_enable_calls: 0,
            mouse_disable_calls: 0,
            alt_enter_calls: 0,
            alt_leave_calls: 0,
        }
    }

    #[allow(dead_code)]
    pub fn raw_writes(&self) -> &[String] {
        &self.writes
    }

    pub fn all_writes(&self) -> String {
        self.writes.concat()
    }

    pub fn write_count(&self) -> usize {
        self.writes.len()
    }

    pub fn clear_writes(&mut self) {
        self.writes.clear();
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn show_cursor_calls(&self) -> u32 {
        self.show_cursor_calls
    }

    pub fn hide_cursor_calls(&self) -> u32 {
        self.hide_cursor_calls
    }

    pub fn mouse_enable_calls(&self) -> u32 {
        self.mouse_enable_calls
    }

    pub fn mouse_disable_calls(&self) -> u32 {
        self.mouse_disable_calls
    }

    pub fn alt_enter_calls(&self) -> u32 {
        self.alt_enter_calls
    }

    pub fn alt_leave_calls(&self) -> u32 {
        self.alt_leave_calls
    }

    #[allow(dead_code)]
    pub fn inner(&self) -> &VirtualTerminal {
        &self.inner
    }

    pub fn count_occurrences(&self, needle: &str) -> usize {
        self.writes.iter().map(|w| w.matches(needle).count()).sum()
    }
}

impl Terminal for LoggingVirtualTerminal {
    fn write(&mut self, data: &str) {
        self.writes.push(data.to_string());
        self.inner.write(data);
    }
    fn columns(&self) -> u16 {
        self.inner.columns()
    }
    fn rows(&self) -> u16 {
        self.inner.rows()
    }
    fn hide_cursor(&mut self) {
        self.hide_cursor_calls = self.hide_cursor_calls.saturating_add(1);
        self.cursor_visible = false;
        self.inner.hide_cursor();
    }
    fn show_cursor(&mut self) {
        self.show_cursor_calls = self.show_cursor_calls.saturating_add(1);
        self.cursor_visible = true;
        self.inner.show_cursor();
    }
    fn clear_line(&mut self) {
        self.inner.clear_line();
    }
    fn clear_from_cursor(&mut self) {
        self.inner.clear_from_cursor();
    }
    fn clear_screen(&mut self) {
        self.inner.clear_screen();
    }
    fn flush(&mut self) {
        self.inner.flush();
    }

    fn set_size_hint(&mut self, cols: u16, rows: u16) {
        self.inner.set_size_hint(cols, rows);
    }

    fn enable_mouse_capture(&mut self) {
        let was = self.inner.mouse_capture_active();
        self.inner.enable_mouse_capture();
        if !was && self.inner.mouse_capture_active() {
            self.mouse_enable_calls = self.mouse_enable_calls.saturating_add(1);
        }
    }

    fn disable_mouse_capture(&mut self) {
        let was = self.inner.mouse_capture_active();
        self.inner.disable_mouse_capture();
        if was && !self.inner.mouse_capture_active() {
            self.mouse_disable_calls = self.mouse_disable_calls.saturating_add(1);
        }
    }

    fn mouse_capture_active(&self) -> bool {
        self.inner.mouse_capture_active()
    }

    fn enter_alternate_screen(&mut self) {
        if !self.inner.alternate_screen_active() {
            self.writes.push("\x1b[?1049h".to_string());
            self.alt_enter_calls = self.alt_enter_calls.saturating_add(1);
        }
        self.inner.enter_alternate_screen();
    }

    fn leave_alternate_screen(&mut self) {
        if self.inner.alternate_screen_active() {
            self.writes.push("\x1b[?1049l".to_string());
            self.alt_leave_calls = self.alt_leave_calls.saturating_add(1);
        }
        self.inner.leave_alternate_screen();
    }

    fn alternate_screen_active(&self) -> bool {
        self.inner.alternate_screen_active()
    }

    fn start(&mut self) {
        let was = self.inner.mouse_capture_active();
        self.inner.start();
        if !was && self.inner.mouse_capture_active() {
            self.mouse_enable_calls = self.mouse_enable_calls.saturating_add(1);
        }
    }

    fn stop(&mut self) {
        let was_mouse = self.inner.mouse_capture_active();
        let was_alt = self.inner.alternate_screen_active();
        if was_alt {
            self.writes.push("\x1b[?1049l".to_string());
            self.alt_leave_calls = self.alt_leave_calls.saturating_add(1);
        }
        self.inner.stop();
        if was_mouse && !self.inner.mouse_capture_active() {
            self.mouse_disable_calls = self.mouse_disable_calls.saturating_add(1);
        }
    }
}

impl std::ops::Deref for LoggingVirtualTerminal {
    type Target = VirtualTerminal;
    fn deref(&self) -> &VirtualTerminal {
        &self.inner
    }
}

use std::cell::RefCell;
use std::rc::Rc;

use xylitol_tui::RenderError;
use xylitol_tui::TUI;
use xylitol_tui::tui::{Component, InputEvent};

pub struct MutableComponent {
    pub lines: Rc<RefCell<Vec<String>>>,
}

#[allow(dead_code)]
impl Component for MutableComponent {
    fn render(&mut self, _width: usize) -> Vec<String> {
        self.lines.borrow().clone()
    }
    fn handle_input(&mut self, _event: InputEvent) {}
    fn invalidate(&mut self) {}
}

pub struct TuiTestHarness {
    pub tui: TUI<LoggingVirtualTerminal>,
}

#[allow(dead_code)]
impl TuiTestHarness {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            tui: TUI::new(LoggingVirtualTerminal::new(cols, rows)),
        }
    }

    pub fn mount(&mut self, component: Box<dyn Component>) -> &mut Self {
        self.tui.add_child(component);
        self
    }

    #[allow(dead_code)]
    pub fn mount_shared(&mut self, lines: Rc<RefCell<Vec<String>>>) -> &mut Self {
        self.tui.add_child(Box::new(MutableComponent { lines }));
        self
    }

    pub fn keys(&mut self, seq: &str) -> &mut Self {
        for ev in vt_feed::parse_vt_to_input_events(seq) {
            self.tui.dispatch_event(ev);
        }
        self
    }

    pub fn focus(&mut self, index: Option<usize>) -> &mut Self {
        self.tui.set_focus(index);
        self
    }

    pub fn render(&mut self) -> &mut Self {
        self.render_result().expect("render_frame failed in test")
    }

    pub fn render_result(&mut self) -> Result<&mut Self, RenderError> {
        self.tui.render_frame()?;
        Ok(self)
    }

    pub fn tick(&mut self) -> &mut Self {
        self.tui.idle_tick();
        self
    }

    pub fn assert_text_contains(&self, needle: &str) -> &Self {
        let text = self.tui.terminal.viewport().join("\n");
        assert!(
            text.contains(needle),
            "viewport should contain {needle:?}, got:\n{text}"
        );
        self
    }

    #[allow(dead_code)]
    pub fn assert_cursor_at(&self, col: usize, row: usize) -> &Self {
        let (c, r) = self.tui.terminal.cursor_position();
        assert_eq!(
            (c, r),
            (col, row),
            "cursor should be at ({col},{row}), got ({c},{r})"
        );
        self
    }

    #[allow(dead_code)]
    pub fn assert_cell_text(&self, row: usize, col: usize, ch: char) -> &Self {
        let cell = self.tui.terminal.cell(row, col);
        assert_eq!(
            cell.ch, ch,
            "cell ({row},{col}) should be {ch:?}, got {cell:?}"
        );
        self
    }
}

// only snapshot_test references these, so they carry #[allow(dead_code)]

#[allow(dead_code)]
pub fn viewport_snapshot(harness: &TuiTestHarness) -> String {
    let term = &harness.tui.terminal;
    let top = term.viewport_top_pub();
    let height = term.rows() as usize;
    (top..top + height)
        .enumerate()
        .map(|(idx, r)| {
            let row_text = if r < term.grid_len() {
                render_row_annotated(term.grid_row(r))
            } else {
                String::new()
            };
            if row_text.is_empty() {
                format!("{idx}|")
            } else {
                format!("{idx}| {row_text}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[allow(dead_code)]
fn render_row_annotated(row: &[Cell]) -> String {
    let mut out = String::new();
    let mut cur_tag = String::new();
    for cell in row {
        let tag = style_tag(cell);
        if tag != cur_tag {
            if !cur_tag.is_empty() {
                out.push_str("[/]");
            }
            if !tag.is_empty() {
                out.push_str(&format!("[{tag}]"));
            }
            cur_tag = tag;
        }
        out.push(if cell.ch == '\0' { ' ' } else { cell.ch });
    }
    if !cur_tag.is_empty() {
        out.push_str("[/]");
    }
    out.trim_end().to_string()
}

#[allow(dead_code)]
fn style_tag(cell: &Cell) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if cell.bold {
        parts.push("bold");
    }
    if cell.dim {
        parts.push("dim");
    }
    if cell.italic {
        parts.push("italic");
    }
    if cell.underline {
        parts.push("underline");
    }
    if cell.reverse {
        parts.push("reverse");
    }
    let mut s = parts.join("+");
    if !matches!(cell.fg, Color::Default) {
        if !s.is_empty() {
            s.push('+');
        }
        s.push_str(&format!("fg:{:?}", cell.fg));
    }
    if !matches!(cell.bg, Color::Default) {
        if !s.is_empty() {
            s.push('+');
        }
        s.push_str(&format!("bg:{:?}", cell.bg));
    }
    s
}
