use crate::terminal::Terminal;
use crate::utils::visible_width;
use std::collections::HashSet;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub const CURSOR_MARKER: &str = "\x1b_pi:c\x07";

pub trait Component {
    fn render(&mut self, width: usize) -> Vec<String>;
    fn handle_input(&mut self, data: &str);
    fn invalidate(&mut self);
    fn wants_key_release(&self) -> bool {
        false
    }
}

pub trait Focusable: Component {
    fn set_focused(&mut self, focused: bool);
    fn is_focused(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayAnchor {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopCenter,
    BottomCenter,
    LeftCenter,
    RightCenter,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OverlayMargin {
    pub top: Option<usize>,
    pub right: Option<usize>,
    pub bottom: Option<usize>,
    pub left: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum SizeValue {
    Absolute(usize),
    Percent(f64),
}

impl SizeValue {
    pub fn resolve(&self, reference: usize) -> usize {
        match self {
            SizeValue::Absolute(n) => *n,
            SizeValue::Percent(p) => ((reference as f64) * p / 100.0).floor() as usize,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OverlayOptions {
    pub width: Option<SizeValue>,
    pub min_width: Option<usize>,
    pub max_height: Option<SizeValue>,
    pub anchor: Option<OverlayAnchor>,
    pub offset_x: Option<i32>,
    pub offset_y: Option<i32>,
    pub row: Option<SizeValue>,
    pub col: Option<SizeValue>,
    pub margin: Option<OverlayMargin>,
    pub non_capturing: bool,
}

#[derive(Debug, Clone, Copy)]
struct OverlayStackEntry {
    /// Focused index before the overlay opened; restored on overlay close.
    /// Unused until the overlay focus-restore state machine is wired (see pi
    /// tui.ts overlayFocusRestore); kept so the struct matches pi's shape.
    #[allow(dead_code)]
    pre_focus: Option<usize>,
    hidden: bool,
    focus_order: u64,
}

pub struct TUI<T: Terminal> {
    pub terminal: T,
    components: Vec<Box<dyn Component>>,
    overlays: Vec<(Box<dyn Component>, OverlayOptions, OverlayStackEntry)>,
    previous_lines: Vec<String>,
    previous_kitty_ids: HashSet<u32>,
    previous_width: usize,
    previous_height: usize,
    focused_index: Option<usize>,
    stopped: bool,
    cursor_row: usize,
    hardware_cursor_row: usize,
    show_hardware_cursor: bool,
    clear_on_shrink: bool,
    max_lines_rendered: usize,
    full_redraw_count: u64,
    focus_order_counter: u64,
}

impl<T: Terminal> TUI<T> {
    pub fn new(terminal: T) -> Self {
        Self {
            terminal,
            components: Vec::new(),
            overlays: Vec::new(),
            previous_lines: Vec::new(),
            previous_kitty_ids: HashSet::new(),
            previous_width: 0,
            previous_height: 0,
            focused_index: None,
            stopped: false,
            cursor_row: 0,
            hardware_cursor_row: 0,
            show_hardware_cursor: false,
            clear_on_shrink: true,
            max_lines_rendered: 0,
            full_redraw_count: 0,
            focus_order_counter: 0,
        }
    }

    pub fn full_redraws(&self) -> u64 {
        self.full_redraw_count
    }
    pub fn set_show_hardware_cursor(&mut self, enabled: bool) {
        if self.show_hardware_cursor == enabled {
            return;
        }
        self.show_hardware_cursor = enabled;
        if !enabled {
            self.terminal.hide_cursor();
        }
    }
    pub fn set_clear_on_shrink(&mut self, enabled: bool) {
        self.clear_on_shrink = enabled;
    }
    pub fn add_child(&mut self, component: Box<dyn Component>) {
        self.components.push(component);
    }
    pub fn remove_child(&mut self, index: usize) {
        if index < self.components.len() {
            self.components.remove(index);
        }
    }
    pub fn clear_children(&mut self) {
        self.components.clear();
    }
    pub fn set_focus(&mut self, index: Option<usize>) {
        self.focused_index = index;
    }

    pub fn show_overlay(&mut self, component: Box<dyn Component>, options: OverlayOptions) {
        self.focus_order_counter += 1;
        let entry = OverlayStackEntry {
            pre_focus: self.focused_index,
            hidden: false,
            focus_order: self.focus_order_counter,
        };
        self.overlays.push((component, options, entry));
        self.terminal.hide_cursor();
    }

    pub fn start(&mut self) -> io::Result<()> {
        self.start_impl(None)
    }

    /// Like `start()` but polls an external quit flag in addition to `stopped`.
    /// The TUI cleanly restores the terminal even if the flag is set externally.
    pub fn start_with_flag(&mut self, quit_flag: &Arc<AtomicBool>) -> io::Result<()> {
        self.start_impl(Some(quit_flag))
    }

    fn start_impl(&mut self, quit_flag: Option<&Arc<AtomicBool>>) -> io::Result<()> {
        use crossterm::event::{self, Event};
        use crossterm::terminal::enable_raw_mode;

        self.stopped = false;
        self.terminal.hide_cursor();
        enable_raw_mode()?;
        io::stdout().write_all(b"\x1b[?2004h")?;
        self.do_render();

        while !self.stopped {
            if let Some(flag) = quit_flag
                && flag.load(std::sync::atomic::Ordering::SeqCst)
            {
                break;
            }
            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        let data = self.key_event_to_string(&key_event);
                        if !data.is_empty() {
                            self.handle_input(&data);
                            self.do_render();
                        }
                    }
                    Event::Resize(_, _) => {
                        self.do_render();
                    }
                    Event::Paste(data) => {
                        self.handle_input(&format!("\x1b[200~{}~\x1b[201~", data));
                        self.do_render();
                    }
                    _ => {}
                }
            }
        }

        io::stdout().write_all(b"\x1b[?2004l")?;
        if !self.previous_lines.is_empty() {
            let target_row = self.previous_lines.len();
            if target_row > self.hardware_cursor_row {
                self.terminal
                    .write(&format!("\x1b[{}B", target_row - self.hardware_cursor_row));
            }
            self.terminal.write("\r\n");
        }
        self.terminal.flush();
        crossterm::terminal::disable_raw_mode()?;
        self.terminal.show_cursor();
        self.terminal.flush();
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stopped = true;
    }

    fn key_event_to_string(&self, key: &crossterm::event::KeyEvent) -> String {
        use crossterm::event::{KeyCode, KeyModifiers};
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Char(c) => {
                if ctrl && c.is_ascii_lowercase() {
                    std::char::from_u32(c as u32 & 0x1f)
                        .map(|c| c.to_string())
                        .unwrap_or_default()
                } else if ctrl && c == ' ' {
                    "\x00".to_string()
                } else if alt && !ctrl {
                    format!("\x1b{}", c)
                } else {
                    c.to_string()
                }
            }
            KeyCode::Enter => "\r".to_string(),
            KeyCode::Tab => {
                if shift {
                    "\x1b[Z".to_string()
                } else {
                    "\t".to_string()
                }
            }
            KeyCode::Backspace => "\x7f".to_string(),
            KeyCode::Esc => "\x1b".to_string(),
            KeyCode::Up => "\x1b[A".to_string(),
            KeyCode::Down => "\x1b[B".to_string(),
            KeyCode::Right => "\x1b[C".to_string(),
            KeyCode::Left => "\x1b[D".to_string(),
            KeyCode::Home => "\x1b[H".to_string(),
            KeyCode::End => "\x1b[F".to_string(),
            KeyCode::PageUp => "\x1b[5~".to_string(),
            KeyCode::PageDown => "\x1b[6~".to_string(),
            KeyCode::Delete => "\x1b[3~".to_string(),
            KeyCode::Insert => "\x1b[2~".to_string(),
            KeyCode::F(n) => match n {
                1 => "\x1bOP",
                2 => "\x1bOQ",
                3 => "\x1bOR",
                4 => "\x1bOS",
                5 => "\x1b[15~",
                6 => "\x1b[17~",
                7 => "\x1b[18~",
                8 => "\x1b[19~",
                9 => "\x1b[20~",
                10 => "\x1b[21~",
                11 => "\x1b[23~",
                12 => "\x1b[24~",
                _ => "",
            }
            .to_string(),
            _ => String::new(),
        }
    }

    /// Route one decoded key/escape sequence to the focused component.
    ///
    /// Public so host loops (and tests) can feed input without going through
    /// the blocking `start()` event loop. The data is the raw byte sequence
    /// (e.g. `"\r"` for Enter, `"\x1b[A"` for Up) — the same form `start_impl`
    /// produces from crossterm KeyEvents.
    pub fn dispatch_input(&mut self, data: &str) {
        if let Some(idx) = self.focused_index
            && idx < self.components.len()
        {
            let input = data.to_string();
            self.components[idx].handle_input(&input);
        }
    }

    fn handle_input(&mut self, data: &str) {
        self.dispatch_input(data);
    }

    /// Run one render pass: composite children + overlays, diff against the
    /// previous frame, and write only the changed lines to the terminal.
    ///
    /// Public so host loops (and tests) can drive single frames instead of the
    /// blocking `start()` loop. In a host-driven setup the host calls this after
    /// state changes (input, async events, ticks).
    pub fn render_frame(&mut self) {
        self.do_render();
    }

    fn do_render(&mut self) {
        if self.stopped {
            return;
        }
        let width = self.terminal.columns() as usize;
        let height = self.terminal.rows() as usize;
        if width == 0 {
            return;
        }

        let mut new_lines = Vec::new();
        for comp in &mut self.components {
            new_lines.extend(comp.render(width));
        }

        if !self.overlays.is_empty() {
            new_lines = self.composite_overlays(new_lines, width, height);
        }

        let reset = "\x1b[0m\x1b]8;;\x07";
        for line in &mut new_lines {
            if !line.is_empty() {
                line.push_str(reset);
            }
        }

        let cursor_pos = self.extract_cursor_position(&mut new_lines, height);

        let width_changed = self.previous_width != 0 && self.previous_width != width;
        let height_changed = self.previous_height != 0 && self.previous_height != height;

        let do_full = self.previous_lines.is_empty()
            || width_changed
            || height_changed
            || (self.clear_on_shrink
                && new_lines.len() < self.max_lines_rendered
                && self.overlays.is_empty());

        if do_full {
            let clear = !(self.previous_lines.is_empty() && !width_changed && !height_changed);
            self.full_render(&new_lines, clear);
        } else {
            self.differential_render(&new_lines);
        }

        self.position_cursor(cursor_pos, new_lines.len());
        self.previous_lines = new_lines;
        self.previous_kitty_ids.clear();
        self.previous_width = width;
        self.previous_height = height;
        self.terminal.flush();
    }

    fn full_render(&mut self, new_lines: &[String], clear: bool) {
        self.full_redraw_count += 1;
        let mut buf = String::from("\x1b[?2026h");
        if clear {
            buf.push_str("\x1b[2J\x1b[H\x1b[3J");
        }
        for (i, line) in new_lines.iter().enumerate() {
            if i > 0 {
                buf.push_str("\r\n");
            }
            buf.push_str(line);
        }
        buf.push_str("\x1b[?2026l");
        self.terminal.write(&buf);
        let len = new_lines.len();
        self.cursor_row = len.saturating_sub(1);
        self.hardware_cursor_row = self.cursor_row;
        self.max_lines_rendered = self.max_lines_rendered.max(len);
    }

    fn differential_render(&mut self, new_lines: &[String]) {
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
        if first_changed == -1 {
            return;
        }

        if first_changed >= new_lines.len() as isize {
            if self.previous_lines.len() > new_lines.len() {
                let mut buf = String::from("\x1b[?2026h");
                let target = new_lines.len().saturating_sub(1);
                let diff = target as isize - self.hardware_cursor_row as isize;
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
                buf.push_str("\x1b[?2026l");
                self.terminal.write(&buf);
                self.cursor_row = target;
                self.hardware_cursor_row = target;
            }
            return;
        }

        let mut buf = String::from("\x1b[?2026h");
        let append_start =
            appended && first_changed as usize == self.previous_lines.len() && first_changed > 0;
        let move_target = if append_start {
            first_changed as usize - 1
        } else {
            first_changed as usize
        };
        let diff = move_target as isize - self.hardware_cursor_row as isize;
        if diff > 0 {
            buf.push_str(&format!("\x1b[{}B", diff));
        } else if diff < 0 {
            buf.push_str(&format!("\x1b[{}A", -diff));
        }
        buf.push_str(if append_start { "\r\n" } else { "\r" });

        let end = last_changed.min((new_lines.len() - 1) as isize) as usize;
        let start = first_changed as usize;
        for (i, line) in new_lines.iter().enumerate().take(end + 1).skip(start) {
            if i > start {
                buf.push_str("\r\n");
            }
            buf.push_str("\x1b[2K");
            buf.push_str(line);
        }
        buf.push_str("\x1b[?2026l");
        self.terminal.write(&buf);
        self.cursor_row = end;
        self.hardware_cursor_row = end;
        self.max_lines_rendered = self.max_lines_rendered.max(new_lines.len());
    }

    fn composite_overlays(
        &mut self,
        mut lines: Vec<String>,
        term_width: usize,
        term_height: usize,
    ) -> Vec<String> {
        if self.overlays.is_empty() {
            return lines;
        }

        let mut indices: Vec<usize> = (0..self.overlays.len())
            .filter(|&i| !self.overlays[i].2.hidden)
            .collect();
        indices.sort_by_key(|&i| self.overlays[i].2.focus_order);

        // Collect layout info before rendering (avoid borrow conflicts)
        struct LayoutInfo {
            width: usize,
            max_height: Option<usize>,
            opts: OverlayOptions,
        }
        let mut layouts: Vec<LayoutInfo> = Vec::new();
        for &i in &indices {
            let opts = &self.overlays[i].1;
            let mut w = opts
                .width
                .as_ref()
                .map(|sv| sv.resolve(term_width))
                .unwrap_or(term_width.min(80));
            if let Some(mw) = opts.min_width {
                w = w.max(mw);
            }
            w = w.clamp(1, term_width);
            let mh = opts.max_height.as_ref().map(|h| h.resolve(term_height));
            layouts.push(LayoutInfo {
                width: w,
                max_height: mh,
                opts: opts.clone(),
            });
        }

        // Render each overlay
        let mut rendered: Vec<(Vec<String>, usize, usize)> = Vec::new();
        for (idx, layout) in indices.iter().zip(layouts.iter()) {
            let ov_lines = {
                let (comp, _opts, _) = &mut self.overlays[*idx];
                let mut ov = comp.render(layout.width);
                if let Some(mh) = layout.max_height {
                    ov.truncate(mh);
                }
                ov
            };
            let (row, col) = self.resolve_overlay_position(
                &layout.opts,
                ov_lines.len(),
                layout.width,
                term_width,
                term_height,
            );
            rendered.push((ov_lines, row, col));
        }

        let working_height = lines.len().max(term_height);
        lines.resize(working_height, String::new());

        for (ov_lines, row, col) in rendered {
            for (i, ov_line) in ov_lines.iter().enumerate() {
                if let Some(base) = lines.get_mut(row + i) {
                    *base = self.composite_line(base, ov_line, col, term_width);
                }
            }
        }
        lines
    }

    fn resolve_overlay_position(
        &self,
        _opts: &OverlayOptions,
        _h: usize,
        _w: usize,
        _tw: usize,
        _th: usize,
    ) -> (usize, usize) {
        // Simplified anchor-based positioning
        let anchor = _opts.anchor.unwrap_or(OverlayAnchor::Center);
        let mt = _opts.margin.unwrap_or_default().top.unwrap_or(0);
        let ml = _opts.margin.unwrap_or_default().left.unwrap_or(0);

        let row = match anchor {
            OverlayAnchor::Center
            | OverlayAnchor::TopCenter
            | OverlayAnchor::BottomCenter
            | OverlayAnchor::LeftCenter
            | OverlayAnchor::RightCenter => mt + (_th.saturating_sub(_h)) / 2,
            OverlayAnchor::TopLeft | OverlayAnchor::TopRight => mt,
            OverlayAnchor::BottomLeft | OverlayAnchor::BottomRight => mt + _th.saturating_sub(_h),
        };

        let col = match anchor {
            OverlayAnchor::Center | OverlayAnchor::LeftCenter | OverlayAnchor::RightCenter => {
                ml + (_tw.saturating_sub(_w)) / 2
            }
            OverlayAnchor::TopLeft | OverlayAnchor::BottomLeft => ml,
            OverlayAnchor::TopRight | OverlayAnchor::BottomRight => ml + _tw.saturating_sub(_w),
            OverlayAnchor::TopCenter | OverlayAnchor::BottomCenter => {
                ml + (_tw.saturating_sub(_w)) / 2
            }
        };

        (
            row + _opts.offset_y.unwrap_or(0).max(0) as usize,
            col + _opts.offset_x.unwrap_or(0).max(0) as usize,
        )
    }

    fn composite_line(&self, base: &str, overlay: &str, col: usize, total_width: usize) -> String {
        use crate::utils::slice_by_column;
        let reset = "\x1b[0m\x1b]8;;\x07";
        let before = slice_by_column(base, 0, col);
        let bw = visible_width(&before);
        let ow = visible_width(overlay);
        let after_start = col + ow;
        if after_start >= total_width {
            let pad = col.saturating_sub(bw);
            return format!("{}{}{}{}{}", before, " ".repeat(pad), reset, overlay, reset);
        }
        let after = slice_by_column(base, after_start, total_width - after_start);
        let aw = visible_width(&after);
        format!(
            "{}{}{}{}{}{}{}",
            before,
            " ".repeat(col.saturating_sub(bw)),
            reset,
            overlay,
            reset,
            after,
            " ".repeat((total_width - after_start).saturating_sub(aw))
        )
    }

    fn extract_cursor_position(
        &self,
        lines: &mut [String],
        height: usize,
    ) -> Option<(usize, usize)> {
        let vp = lines.len().saturating_sub(height);
        for row in (vp..lines.len()).rev() {
            if let Some(mi) = lines[row].find(CURSOR_MARKER) {
                let col = visible_width(&lines[row][..mi]);
                lines[row] = format!(
                    "{}{}",
                    &lines[row][..mi],
                    &lines[row][mi + CURSOR_MARKER.len()..]
                );
                return Some((row, col));
            }
        }
        None
    }

    fn position_cursor(&mut self, cursor_pos: Option<(usize, usize)>, total_lines: usize) {
        if let Some((row, col)) = cursor_pos {
            let vp = total_lines.saturating_sub(self.terminal.rows() as usize);
            let sr = row.saturating_sub(vp);
            if self.show_hardware_cursor {
                self.terminal
                    .write(&format!("\x1b[{};{}H", sr + 1, col + 1));
            }
            self.terminal.show_cursor();
        } else if !self.show_hardware_cursor {
            self.terminal.hide_cursor();
        }
    }

    pub fn invalidate(&mut self) {
        for c in &mut self.components {
            c.invalidate();
        }
        for (c, _, _) in &mut self.overlays {
            c.invalidate();
        }
    }
}
