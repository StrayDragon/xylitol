use crate::terminal::Terminal;
use crate::utils::visible_width;
use crossterm::event::{KeyEvent, KeyEventKind};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub const CURSOR_MARKER: &str = "\x1b_pi:c\x07";

/// Error from a render pass. The engine hard-errors when a component emits a
/// line wider than the terminal `width` — a widget that overflows desyncs the
/// cursor and corrupts subsequent lines, so we surface it loudly instead of
/// silently truncating (mirrors pi-tui's crash guard in the diff path).
#[derive(Debug)]
pub struct RenderError {
    pub width: usize,
    pub line_width: usize,
    pub line_index: usize,
    pub line_preview: String,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "line {} is {} cols wide but terminal width is {}; use visible_width()/truncate_to_width(). preview: {:?}",
            self.line_index, self.line_width, self.width, self.line_preview
        )
    }
}

impl std::error::Error for RenderError {}

/// Heuristic for Kitty inline-image lines (APC `<_...ST`). Their visible width
/// is 0 (all escape bytes), so they're exempt from the width invariant.
fn is_image_line(line: &str) -> bool {
    line.contains("\x1b_G") || line.contains("\x1b]1337;File")
}

pub trait Component {
    fn render(&mut self, width: usize) -> Vec<String>;
    fn handle_input(&mut self, data: &str);
    fn invalidate(&mut self);
    fn wants_key_release(&self) -> bool {
        false
    }
    /// Advance any time-driven state (e.g. a spinner frame). Called by the
    /// event loop on its tick. Default no-op so non-animated components ignore it.
    /// Returning true hints that a re-render is wanted (host decides).
    fn tick(&mut self) -> bool {
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
    /// Row index (in the rendered line buffer) of the top of the visible
    /// viewport at the end of the previous frame. Differential rendering uses
    /// this to translate buffer rows to screen rows when content exceeds one
    /// screen. Mirrors pi's `previousViewportTop`.
    previous_viewport_top: usize,
    focused_index: Option<usize>,
    stopped: bool,
    /// Logical cursor row = content end (used for viewport math). Distinct from
    /// `hardware_cursor_row` (where the terminal's cursor physically stopped).
    cursor_row: usize,
    hardware_cursor_row: usize,
    show_hardware_cursor: bool,
    clear_on_shrink: bool,
    max_lines_rendered: usize,
    full_redraw_count: u64,
    focus_order_counter: u64,
    // ── render scheduling (pi's requestRender/scheduleRender) ──
    /// True when a render has been requested but not yet executed.
    render_requested: bool,
    /// Monotonic instant of the last actual render, for the 16ms throttle.
    last_render_at: Option<std::time::Instant>,
}

/// Minimum spacing between throttled frames (~60fps). Mirrors pi's
/// MIN_RENDER_INTERVAL_MS. `render_frame` bypasses this; `try_render` honors it.
const MIN_RENDER_INTERVAL_MS: u64 = 16;
const BEGIN_RENDER_BATCH: &str = "\x1b[?2026h\x1b[?7l";
const END_RENDER_BATCH: &str = "\x1b[?7h\x1b[?2026l";

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
            previous_viewport_top: 0,
            focused_index: None,
            stopped: false,
            cursor_row: 0,
            hardware_cursor_row: 0,
            show_hardware_cursor: false,
            clear_on_shrink: false,
            max_lines_rendered: 0,
            full_redraw_count: 0,
            focus_order_counter: 0,
            render_requested: false,
            last_render_at: None,
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

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start_impl(None)
    }

    /// Like `start()` but polls an external quit flag in addition to `stopped`.
    /// The TUI cleanly restores the terminal even if the flag is set externally.
    pub fn start_with_flag(
        &mut self,
        quit_flag: &Arc<AtomicBool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.start_impl(Some(quit_flag))
    }

    fn start_impl(
        &mut self,
        quit_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crossterm::event::{self, Event};

        self.stopped = false;
        // terminal.start() owns: raw mode + bracketed paste + keyboard
        // protocol negotiation (Kitty push / modifyOtherKeys fallback, c410).
        self.terminal.hide_cursor();
        self.terminal.start();
        self.do_render()?;

        while !self.stopped {
            if let Some(flag) = quit_flag
                && flag.load(std::sync::atomic::Ordering::SeqCst)
            {
                break;
            }
            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if !self.should_dispatch_key_event(&key_event) {
                            continue;
                        }
                        let data = self.key_event_to_string(&key_event);
                        if !data.is_empty() {
                            self.handle_input(&data);
                            self.do_render()?;
                        }
                    }
                    Event::Resize(_, _) => {
                        // Re-query size before rendering so we don't paint with
                        // stale columns/rows (the CrosstermTerminal caches them).
                        self.terminal.refresh_size();
                        self.do_render()?;
                    }
                    Event::Paste(data) => {
                        self.handle_input(&format!("\x1b[200~{}~\x1b[201~", data));
                        self.do_render()?;
                    }
                    _ => {}
                }
            } else {
                // Idle tick: advance animated or scripted components. If any
                // wants a repaint, drive a frame.
                let wants_render = self.idle_tick();
                if wants_render {
                    self.do_render()?;
                }
            }
        }

        // Move cursor past rendered content before tearing down (so the shell
        // prompt lands below the TUI output). Protocol/raw-mode cleanup is
        // delegated to terminal.stop() (c410: pops Kitty, disables
        // modifyOtherKeys, drains stdin, disables raw mode, shows cursor).
        if !self.previous_lines.is_empty() {
            let target_row = self.previous_lines.len();
            if target_row > self.hardware_cursor_row {
                self.terminal
                    .write(&format!("\x1b[{}B", target_row - self.hardware_cursor_row));
            }
            self.terminal.write("\r\n");
        }
        self.terminal.flush();
        self.terminal.stop();
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

    /// Advance time-driven component state once without reading input.
    ///
    /// Host-driven loops and test harnesses can call this to exercise the same
    /// idle path that `start()` uses for spinner/script progress.
    pub fn idle_tick(&mut self) -> bool {
        let mut changed = self.components.iter_mut().any(|c| c.tick());
        changed |= self
            .overlays
            .iter_mut()
            .any(|(component, _, _)| component.tick());
        changed
    }

    fn handle_input(&mut self, data: &str) {
        self.dispatch_input(data);
    }

    fn should_dispatch_key_event(&self, key: &KeyEvent) -> bool {
        match key.kind {
            KeyEventKind::Press | KeyEventKind::Repeat => true,
            KeyEventKind::Release => self
                .focused_index
                .and_then(|idx| self.components.get(idx))
                .is_some_and(|component| component.wants_key_release()),
        }
    }

    /// Run one render pass: composite children + overlays, diff against the
    /// previous frame, and write only the changed lines to the terminal.
    ///
    /// Public so host loops (and tests) can drive single frames instead of the
    /// blocking `start()` loop. In a host-driven setup the host calls this after
    /// state changes (input, async events, ticks). Bypasses the throttle — use
    /// `try_render` to honor it, or `request_render` to mark + let a host loop
    /// drive the actual frame.
    pub fn render_frame(&mut self) -> Result<(), RenderError> {
        self.do_render()
    }

    /// Mark a render as needed. The actual frame is driven by whoever calls
    /// `try_render` (a host loop) or by `run_event_loop`'s internal timer. If
    /// `force`, previous-frame state is reset so the next render is a full
    /// redraw — mirrors pi's `requestRender(true)`.
    pub fn request_render(&mut self, force: bool) {
        if force {
            self.previous_lines.clear();
            self.previous_width = 0;
            self.previous_height = 0;
            self.previous_viewport_top = 0;
            self.cursor_row = 0;
            self.hardware_cursor_row = 0;
            self.max_lines_rendered = 0;
        }
        self.render_requested = true;
    }

    /// Render only if one is pending AND the 16ms throttle has elapsed. Returns
    /// Ok(true) if a frame was actually rendered, Ok(false) if skipped, Err if
    /// the render hit the width invariant. Host loops call this on their tick.
    pub fn try_render(&mut self) -> Result<bool, RenderError> {
        if !self.render_requested {
            return Ok(false);
        }
        if let Some(last) = self.last_render_at {
            let elapsed = last.elapsed();
            if elapsed < std::time::Duration::from_millis(MIN_RENDER_INTERVAL_MS) {
                return Ok(false);
            }
        }
        self.render_requested = false;
        self.last_render_at = Some(std::time::Instant::now());
        self.do_render()?;
        Ok(true)
    }

    /// Render immediately, bypassing the throttle. Returns Ok(true) if a frame
    /// was rendered (i.e. not stopped), Err on width-invariant violation.
    pub fn render_now(&mut self) -> Result<bool, RenderError> {
        self.render_requested = false;
        self.last_render_at = Some(std::time::Instant::now());
        self.do_render()?;
        Ok(!self.stopped)
    }

    fn do_render(&mut self) -> Result<(), RenderError> {
        if self.stopped {
            return Ok(());
        }
        let width = self.terminal.columns() as usize;
        let height = self.terminal.rows() as usize;
        if width == 0 {
            return Ok(());
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

        // Hard width invariant (pi's crash guard): every rendered line must fit
        // the terminal width. An overflowing line desyncs the cursor and
        // corrupts the diff, so we stop loudly rather than paint garbage.
        // Image-bearing lines (Kitty APC) are exempt — their visible width is 0.
        for (i, line) in new_lines.iter().enumerate() {
            if line.contains(CURSOR_MARKER) || is_image_line(line) {
                continue;
            }
            let lw = visible_width(line);
            if lw > width {
                return Err(RenderError {
                    width,
                    line_width: lw,
                    line_index: i,
                    line_preview: line.chars().take(40).collect(),
                });
            }
        }

        let width_changed = self.previous_width != 0 && self.previous_width != width;
        let height_changed = self.previous_height != 0 && self.previous_height != height;

        // Compute the changed range up front so the full-vs-diff decision can
        // consider it (pi decides after computing firstChanged/lastChanged).
        let (first_changed, last_changed, appended) = self.compute_line_diff(&new_lines);

        // Full redraw triggers (order matches pi tui.ts:1335-1459):
        // 1. first frame / width change / height change (non-Termux)
        // 2. clearOnShrink (content shrank below the historical high-water mark)
        // 3. firstChanged < prevViewportTop — the change is above the visible
        //    viewport, the diff path can't reach it, so repaint everything.
        // 4. all-deletions with the new tail above the viewport — likewise can't
        //    be expressed as a diff, fullRedraw to resync.
        let do_full = self.previous_lines.is_empty()
            || width_changed
            || height_changed
            || (self.clear_on_shrink
                && new_lines.len() < self.max_lines_rendered
                && self.overlays.is_empty())
            || (first_changed >= 0 && (first_changed as usize) < self.previous_viewport_top)
            || (first_changed >= new_lines.len() as isize
                && !new_lines.is_empty()
                && new_lines.len() <= self.previous_viewport_top);

        if do_full {
            let clear = !(self.previous_lines.is_empty() && !width_changed && !height_changed);
            self.full_render(&new_lines, clear, height);
        } else if first_changed < 0 {
            // No change at all — just reposition the cursor (pi's no-op branch).
        } else {
            self.differential_render(
                &new_lines,
                width,
                height,
                first_changed,
                last_changed,
                appended,
            );
        }

        self.position_cursor(cursor_pos, new_lines.len());
        self.previous_lines = new_lines;
        self.previous_kitty_ids.clear();
        self.previous_width = width;
        self.previous_height = height;
        self.terminal.flush();
        Ok(())
    }

    fn full_render(&mut self, new_lines: &[String], clear: bool, height: usize) {
        self.full_redraw_count += 1;
        let mut buf = String::from(BEGIN_RENDER_BATCH);
        if clear {
            buf.push_str("\x1b[2J\x1b[H\x1b[3J");
        }
        for (i, line) in new_lines.iter().enumerate() {
            if i > 0 {
                buf.push_str("\r\n");
            }
            buf.push_str(line);
        }
        buf.push_str(END_RENDER_BATCH);
        self.terminal.write(&buf);
        let len = new_lines.len();
        self.cursor_row = len.saturating_sub(1);
        self.hardware_cursor_row = self.cursor_row;
        if clear {
            self.max_lines_rendered = len;
        } else {
            self.max_lines_rendered = self.max_lines_rendered.max(len);
        }
        // Content-end aligned viewport: the bottom of the content sticks to the
        // bottom of the screen when content exceeds one screen. Mirrors pi's
        // `previousViewportTop = max(0, max(height, len) - height)`.
        self.previous_viewport_top = height.max(len).saturating_sub(height);
    }

    /// Find the first/last changed line index comparing new_lines to
    /// previous_lines. Returns (first, last, appended) with -1 meaning "none".
    /// Mirrors pi's firstChanged/lastChanged + append detection.
    fn compute_line_diff(&self, new_lines: &[String]) -> (isize, isize, bool) {
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

    fn differential_render(
        &mut self,
        new_lines: &[String],
        _width: usize,
        height: usize,
        first_changed_in: isize,
        last_changed: isize,
        appended: bool,
    ) {
        // All-deletions branch: content only shrank. Clear the surplus lines.
        if first_changed_in >= new_lines.len() as isize {
            if self.previous_lines.len() > new_lines.len() {
                let mut buf = String::from(BEGIN_RENDER_BATCH);
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

        // Viewport scroll (pi Step 5C, tui.ts:1466-1478): if the changed region
        // falls below the previous viewport bottom, CUD to the screen's last row
        // then emit `\r\n` to scroll the terminal — lifting prevViewportTop so
        // the diff writes land on visible rows instead of past the screen edge.
        let mut prev_viewport_top = self.previous_viewport_top;
        let prev_viewport_bottom = prev_viewport_top + height.saturating_sub(1);
        if move_target > prev_viewport_bottom {
            let current_screen_row = (self.hardware_cursor_row as isize
                - prev_viewport_top as isize)
                .clamp(0, height as isize - 1) as usize;
            let move_to_bottom = height - 1 - current_screen_row;
            if move_to_bottom > 0 {
                buf.push_str(&format!("\x1b[{}B", move_to_bottom));
            }
            let scroll = move_target - prev_viewport_bottom;
            if scroll > 0 {
                buf.push_str(&"\r\n".repeat(scroll));
            }
            prev_viewport_top += scroll;
        }

        // Move to the target row (relative to the current viewport top) and
        // start the changed region.
        let screen_row = move_target as isize - prev_viewport_top as isize;
        let cursor_diff =
            screen_row - (self.hardware_cursor_row as isize - prev_viewport_top as isize);
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
        buf.push_str(END_RENDER_BATCH);
        self.terminal.write(&buf);

        // Shrink cleanup: if content got shorter, clear the surplus rows below
        // the new tail (pi tui.ts:1555-1568). finalCursorRow tracks where the
        // cursor physically ended so the cursor-positioning step stays accurate.
        let mut final_cursor_row = end;
        if self.previous_lines.len() > new_lines.len() && !appended {
            let extra = self.previous_lines.len() - new_lines.len();
            let mut tail = String::from(BEGIN_RENDER_BATCH);
            // Move to one past the last rendered line, clear each surplus row.
            for _ in 0..extra {
                tail.push_str("\r\n\x1b[2K");
            }
            // Step back up to the content end.
            if extra > 0 {
                tail.push_str(&format!("\x1b[{}A", extra));
            }
            tail.push_str(END_RENDER_BATCH);
            self.terminal.write(&tail);
            final_cursor_row = new_lines.len().saturating_sub(1);
        }

        self.cursor_row = new_lines.len().saturating_sub(1);
        self.hardware_cursor_row = final_cursor_row;
        self.previous_viewport_top =
            prev_viewport_top.max(final_cursor_row.saturating_sub(height.saturating_sub(1)));
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

        // workingHeight = max(content, screen, minLinesNeeded). Deliberately
        // excludes maxLinesRendered — including it historically caused
        // self-reinforcing scrollback growth on width changes (pi tui.ts:1067).
        // minLinesNeeded covers overlays anchored below the content end.
        let min_lines_needed = rendered
            .iter()
            .map(|(ov, row, _)| row + ov.len())
            .max()
            .unwrap_or(0);
        let working_height = lines.len().max(term_height).max(min_lines_needed);
        lines.resize(working_height, String::new());

        // Overlay screen coords are relative to the visible viewport top; map
        // them to absolute buffer rows via viewportStart (pi tui.ts:1074).
        let viewport_start = working_height.saturating_sub(term_height);

        for (ov_lines, row, col) in rendered {
            for (i, ov_line) in ov_lines.iter().enumerate() {
                let abs_row = viewport_start + row + i;
                if let Some(base) = lines.get_mut(abs_row) {
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
        use crate::utils::{extract_segments, slice_by_column, slice_with_width};

        let reset = "\x1b[0m\x1b]8;;\x07";
        let ow = visible_width(overlay);
        let after_start = col + ow;
        let after_len = total_width.saturating_sub(after_start);

        // Single pass: split the base into before[0,col) and after[afterStart,...).
        // after inherits the SGR active at col so an overlay laid mid-line doesn't
        // leave the trailing content unstyled (pi compositeLineAt via extractSegments).
        let seg = extract_segments(base, col, after_start, after_len, true);

        // Overlay slice (strict — reject wide chars crossing the overlay's right
        // edge). Truncate the overlay to its declared width if it overflows.
        let overlay_slice = if ow > 0 {
            slice_with_width(overlay, 0, ow, true).0
        } else {
            String::new()
        };

        // Pad before up to col, then reset, then overlay, then reset.
        let before_pad = " ".repeat(col.saturating_sub(seg.before_width));
        let before_part = format!("{}{}{}", seg.before, before_pad, reset);

        let overlay_part = if after_len == 0 {
            // No room for after; overlay (truncated) + reset is all that fits.
            format!("{}{}", overlay_slice, reset)
        } else {
            let after_pad = " ".repeat(after_len.saturating_sub(seg.after_width));
            format!(
                "{}{}{}{}{}",
                reset, overlay_slice, reset, seg.after, after_pad
            )
        };

        let mut result = format!("{}{}", before_part, overlay_part);

        // Final safety net (pi compositeLineAt tui.ts:1218): if anything still
        // overflowed (wide-char rounding, unexpected ANSI), hard-slice to width.
        if visible_width(&result) > total_width {
            result = slice_by_column(&result, 0, total_width);
        }
        result
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEventState, KeyModifiers};

    struct DummyTerminal;

    impl Terminal for DummyTerminal {
        fn write(&mut self, _data: &str) {}
        fn columns(&self) -> u16 {
            80
        }
        fn rows(&self) -> u16 {
            24
        }
        fn hide_cursor(&mut self) {}
        fn show_cursor(&mut self) {}
        fn clear_line(&mut self) {}
        fn clear_from_cursor(&mut self) {}
        fn clear_screen(&mut self) {}
        fn flush(&mut self) {}
    }

    struct DummyComponent {
        wants_release: bool,
    }

    impl Component for DummyComponent {
        fn render(&mut self, _width: usize) -> Vec<String> {
            Vec::new()
        }

        fn handle_input(&mut self, _data: &str) {}

        fn invalidate(&mut self) {}

        fn wants_key_release(&self) -> bool {
            self.wants_release
        }
    }

    fn key_with_kind(kind: KeyEventKind) -> KeyEvent {
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn dispatches_press_and_repeat() {
        let tui = TUI::new(DummyTerminal);

        assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Press)));
        assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Repeat)));
    }

    #[test]
    fn ignores_release_without_opt_in() {
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(DummyComponent {
            wants_release: false,
        }));
        tui.set_focus(Some(0));

        assert!(!tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Release)));
    }

    #[test]
    fn dispatches_release_when_focused_component_requests_it() {
        let mut tui = TUI::new(DummyTerminal);
        tui.add_child(Box::new(DummyComponent {
            wants_release: true,
        }));
        tui.set_focus(Some(0));

        assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Release)));
    }
}
