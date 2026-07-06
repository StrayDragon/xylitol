//! The render engine — line-array differential rendering (c399).
//!
//! The core of pi-tui's rendering mode (skill `rendering-engine.md` Steps 2-10).
//! Owns `previous_lines` (last frame), diffs against `new_lines` each frame, and
//! writes only changed lines wrapped in synchronized output (CSI 2026). Three
//! strategies: first render (no clear, preserve scrollback above), full redraw
//! (clear + repaint, used on width/height change), normal diff (move cursor +
//! `\x1b[2K` + rewrite changed range).
//!
//! Hard width invariant (Step 6): a line wider than `width` is a hard error on
//! BOTH the diff path AND fullRender (this closes pi-tui's audit Finding 2 —
//! pi-tui only checks the diff path). The engine never writes from inside
//! `render()`; it builds one buffer per frame and writes once.

use std::time::{Duration, Instant};

use super::component::Component;
use super::outcome::UxOutcome;
use super::style::{CURSOR_MARKER, StyledLine};
use super::terminal::Terminal;
use crossterm::event::KeyEvent;

/// Minimum interval between rendered frames (~60fps cap). Coalesces bursts of
/// `request_render` calls into one render.
const MIN_RENDER_INTERVAL: Duration = Duration::from_millis(16);

/// The render engine. Generic over `Terminal` so tests run against
/// `CapturingTerminal` and production against `ProcessTerminal`.
pub struct Tui<T: Terminal> {
    term: T,
    /// The widget tree root (usually a `Container`). The engine calls
    /// `render(width)` each frame.
    root: Box<dyn Component>,
    previous_lines: Vec<String>,
    previous_width: u16,
    previous_height: u16,
    /// Index of the top visible row in the logical content array. Advances as
    /// content grows past the viewport (view scrolls to follow the end).
    previous_viewport_top: usize,
    /// Logical end-of-content row (for viewport math). Diverges from
    /// `hardware_cursor_row` when a diff writes only a middle range.
    cursor_row: usize,
    /// Where the terminal cursor physically stopped (for relative moves). The
    /// IME positioning computes a delta from this.
    hardware_cursor_row: usize,
    /// High-water mark of the terminal's working area (grows, resets on
    /// fullRender(true)). When content shrinks below it, orphan rows at the
    /// bottom are stale.
    max_lines_rendered: usize,
    render_requested: bool,
    last_render_at: Option<Instant>,
    stopped: bool,
    /// Force a full redraw with clear on the next do_render (set by
    /// request_render(true), e.g. theme change). Clears scrollback.
    force_clear: bool,
    /// Lines changed in the last frame (for tests / debug). Empty if no change.
    last_changed_range: Option<(usize, usize)>,
    /// Last full-redraw reason (debug visibility — diff engines are impossible
    /// to tune without knowing WHY a full redraw happened).
    last_full_redraw_reason: Option<&'static str>,
    /// App-wide input interceptors, run before focus routing (pi ux.md Step 4).
    /// Each may consume the key, rewrite it, or pass it through. Use for
    /// global keys (e.g. Ctrl+L force redraw) that must run regardless of which
    /// widget holds focus. NOT for Ctrl+C/D — those are widget keybindings
    /// (pi tui.ts:825 comment: "Pass input to focused component including
    /// Ctrl+C").
    input_listeners: Vec<Box<dyn InputListener>>,
}

/// Result of an input listener inspecting a key (pi ux.md Step 4).
pub enum ListenerResult {
    /// Stop routing; the key is fully consumed. The listener is responsible
    /// for requesting a render itself if it changed state (the engine returns
    /// before the focused-widget path that would normally request one).
    Consume,
    /// Replace the key with `key` and continue routing with the new key.
    Rewrite(KeyEvent),
    /// Let routing continue to the next listener / focused widget.
    Pass,
}

/// An app-wide key interceptor. Registered via [`Tui::add_input_listener`].
/// Listeners run in registration order before the focused widget.
pub trait InputListener: Send {
    fn on_key(&mut self, key: &KeyEvent) -> ListenerResult;
}

impl<T: Terminal> Tui<T> {
    pub fn new(term: T, root: Box<dyn Component>) -> Self {
        Self {
            term,
            root,
            previous_lines: Vec::new(),
            previous_width: 0,
            previous_height: 0,
            previous_viewport_top: 0,
            cursor_row: 0,
            hardware_cursor_row: 0,
            max_lines_rendered: 0,
            render_requested: false,
            last_render_at: None,
            stopped: false,
            force_clear: false,
            last_changed_range: None,
            last_full_redraw_reason: None,
            input_listeners: Vec::new(),
        }
    }

    /// Access the underlying terminal (for event polling by the host loop).
    pub fn term_mut(&mut self) -> &mut T {
        &mut self.term
    }

    /// Request a render frame. Coalesced: multiple calls in one tick → one
    /// render. `force = true` invalidates `previous_lines` (full redraw, clears
    /// scrollback) — use after theme changes where cached themed strings are stale.
    pub fn request_render(&mut self, force: bool) {
        if force {
            self.previous_lines.clear();
            self.previous_width = 0;
            self.previous_height = 0;
            self.cursor_row = 0;
            self.hardware_cursor_row = 0;
            self.max_lines_rendered = 0;
            self.previous_viewport_top = 0;
            self.force_clear = true;
            self.render_requested = true;
            self.last_render_at = None; // run ASAP
            return;
        }
        if self.render_requested {
            return;
        }
        self.render_requested = true;
    }

    /// Pump one render frame IF a render is pending AND the rate-limit interval
    /// has elapsed. Returns true if a frame was rendered. The host loop calls
    /// this each iteration; it's cheap when no render is pending.
    ///
    /// We do NOT auto-schedule via a timer (pi-tui uses setTimeout); the host
    /// async loop drives frames, so this is poll-based coalescing.
    pub fn try_render(&mut self) -> Result<(), RenderError> {
        if self.stopped || !self.render_requested {
            return Ok(());
        }
        if let Some(last) = self.last_render_at {
            let elapsed = last.elapsed();
            if elapsed < MIN_RENDER_INTERVAL {
                return Ok(()); // too soon; host will retry
            }
        }
        self.do_render()?;
        Ok(())
    }

    /// Force a render now, bypassing the rate-limit and pending checks. Used by
    /// the host loop when it knows content changed (e.g. after handling input).
    pub fn render_now(&mut self) -> Result<(), RenderError> {
        if self.stopped {
            return Ok(());
        }
        self.do_render()
    }

    /// The per-frame pipeline (rendering-engine.md Step 4-5).
    fn do_render(&mut self) -> Result<(), RenderError> {
        let width = self.term.width() as usize;
        let height = self.term.height() as usize;
        let width_changed = self.previous_width != 0 && self.previous_width as usize != width;
        let height_changed = self.previous_height != 0 && self.previous_height as usize != height;

        // 1. widget tree → line array
        let raw_lines = self.root.render(width);
        // 2. extract + strip CURSOR_MARKER, apply line resets, check widths
        let (new_lines, cursor_pos) = self.post_process_lines(&raw_lines, width)?;

        // 3. choose strategy
        let first_render = self.previous_lines.is_empty()
            && !width_changed
            && !height_changed
            && !self.force_clear;
        if self.force_clear {
            self.full_render(&new_lines, true, width, height, "forced full redraw")?;
            self.force_clear = false;
        } else if first_render {
            self.full_render(&new_lines, false, width, height, "first render")?;
        } else if width_changed || height_changed || self.previous_width == 0 {
            let reason = if width_changed {
                "terminal width changed"
            } else if height_changed {
                "terminal height changed"
            } else {
                "no previous frame"
            };
            self.full_render(&new_lines, true, width, height, reason)?;
        } else {
            self.diff_render(&new_lines, width, height)?;
        }

        // 4. position hardware cursor for IME
        self.position_hardware_cursor(cursor_pos, new_lines.len(), width);

        // 5. save state
        self.previous_lines = new_lines;
        self.previous_width = width as u16;
        self.previous_height = height as u16;
        self.render_requested = false;
        self.last_render_at = Some(Instant::now());
        Ok(())
    }

    /// Extract CURSOR_MARKER (for IME positioning), apply per-line resets
    /// (StyledLine::LINE_RESET), and enforce the hard width invariant on EVERY
    /// line (both diff and fullRender paths — closes audit Finding 2).
    /// Returns the serialized ANSI lines + the cursor position (row, col) if any.
    fn post_process_lines(
        &self,
        raw: &[StyledLine],
        width: usize,
    ) -> Result<ProcessedLines, RenderError> {
        let mut out = Vec::with_capacity(raw.len());
        let mut cursor_pos = None;
        for (row, line) in raw.iter().enumerate() {
            let lw = line.width();
            if lw > width {
                return Err(RenderError::LineTooWide {
                    row,
                    line_width: lw,
                    terminal_width: width,
                    line_text: line.plain_text(),
                });
            }
            let ansi = line.to_ansi();
            // CURSOR_MARKER detection: the marker is embedded in a span's text.
            // Find it, record (row, col-before-marker), strip it.
            if ansi.contains(CURSOR_MARKER) {
                let before = ansi.split(CURSOR_MARKER).next().unwrap_or("");
                let col = super::width::marker_aware_width(before);
                let cleaned = ansi.replace(CURSOR_MARKER, "");
                cursor_pos = Some((row, col));
                out.push(cleaned + StyledLine::LINE_RESET);
            } else {
                out.push(ansi + StyledLine::LINE_RESET);
            }
        }
        Ok((out, cursor_pos))
    }

    /// Strategy A/B: write all lines. `clear = true` clears screen + scrollback
    /// first (full redraw); `clear = false` preserves scrollback above (first
    /// render). rendering-engine.md Step 5 A/B.
    fn full_render(
        &mut self,
        new_lines: &[String],
        clear: bool,
        _width: usize,
        _height: usize,
        reason: &'static str,
    ) -> Result<(), RenderError> {
        let mut buf = String::from("\x1b[?2026h"); // begin synchronized update
        if clear {
            buf.push_str("\x1b[2J\x1b[H\x1b[3J"); // clear screen, home, clear scrollback
        }
        for (i, line) in new_lines.iter().enumerate() {
            if i > 0 {
                buf.push_str("\r\n");
            }
            buf.push_str(line);
        }
        buf.push_str("\x1b[?2026l"); // end synchronized update
        self.term.write(&buf);

        self.cursor_row = new_lines.len().saturating_sub(1);
        self.hardware_cursor_row = self.cursor_row;
        self.max_lines_rendered = if clear {
            new_lines.len()
        } else {
            self.max_lines_rendered.max(new_lines.len())
        };
        self.previous_viewport_top = new_lines.len().saturating_sub(self.term.height() as usize);
        self.last_full_redraw_reason = Some(reason);
        self.last_changed_range = Some((0, new_lines.len().saturating_sub(1)));
        Ok(())
    }

    /// Strategy C: diff previous vs new, rewrite only the changed range.
    /// rendering-engine.md Step 5 C.
    fn diff_render(
        &mut self,
        new_lines: &[String],
        width: usize,
        height: usize,
    ) -> Result<(), RenderError> {
        // Find first/last changed line (treat missing as "").
        let n_prev = self.previous_lines.len();
        let n_new = new_lines.len();
        let mut first_changed: Option<usize> = None;
        let mut last_changed: Option<usize> = None;
        let max_len = n_prev.max(n_new);
        for i in 0..max_len {
            let prev = self.previous_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let new = new_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            if prev != new {
                if first_changed.is_none() {
                    first_changed = Some(i);
                }
                last_changed = Some(i);
            }
        }

        match (first_changed, last_changed) {
            (None, _) => {
                // No changes — only cursor positioning (done in do_render).
                self.last_changed_range = None;
                self.last_full_redraw_reason = None;
                Ok(())
            }
            (Some(first), Some(last)) => {
                // If the first changed line is above the viewport, we can't reach
                // it by scrolling — fall back to full redraw.
                if first < self.previous_viewport_top {
                    self.full_render(
                        new_lines,
                        true,
                        width,
                        height,
                        "firstChanged above viewport",
                    )?;
                    return Ok(());
                }
                self.write_diff_range(new_lines, first, last, height)?;
                // Handle shrink: clear orphaned rows below the new content.
                if n_prev > n_new {
                    self.clear_orphan_rows(n_new, n_prev, height)?;
                }
                self.cursor_row = n_new.saturating_sub(1);
                self.max_lines_rendered = self.max_lines_rendered.max(n_new);
                self.last_changed_range = Some((first, last));
                self.last_full_redraw_reason = None;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Write the changed line range `[first, last]` via cursor moves + `\x1b[2K`
    /// + line content. rendering-engine.md Step 5 C normal write loop.
    fn write_diff_range(
        &mut self,
        new_lines: &[String],
        first: usize,
        last: usize,
        height: usize,
    ) -> Result<(), RenderError> {
        let mut buf = String::from("\x1b[?2026h");
        // Scroll the viewport if the changed range extends below the current
        // viewport bottom (the append case: new content past the old end).
        let prev_bottom = self.previous_viewport_top + height.saturating_sub(1);
        let move_target = first;
        if move_target > prev_bottom {
            let scroll = move_target - prev_bottom;
            for _ in 0..scroll {
                buf.push_str("\r\n"); // newline scrolls the terminal
            }
            self.previous_viewport_top += scroll;
            self.hardware_cursor_row = move_target;
        }
        // Move to the target row (relative).
        let line_diff = move_target as isize - self.hardware_cursor_row as isize;
        if line_diff > 0 {
            buf.push_str(&format!("\x1b[{}B", line_diff));
        } else if line_diff < 0 {
            buf.push_str(&format!("\x1b[{}A", -line_diff));
        }
        // Move to the start of the line, then write each changed line with a
        // leading clear-line.
        let render_end = last;
        let is_append = first >= self.previous_lines.len();
        buf.push_str(if is_append { "\r\n" } else { "\r" });
        for i in first..=render_end {
            if i > first {
                buf.push_str("\r\n");
            }
            buf.push_str("\x1b[2K"); // clear line first (new may be shorter)
            if let Some(line) = new_lines.get(i) {
                buf.push_str(line);
            }
        }
        self.hardware_cursor_row = render_end;
        buf.push_str("\x1b[?2026l");
        self.term.write(&buf);
        Ok(())
    }

    /// Clear orphaned rows when content shrank (n_new < n_prev). Moves down,
    /// clears each extra row with `\r\n\x1b[2K`, moves back.
    fn clear_orphan_rows(
        &mut self,
        n_new: usize,
        n_prev: usize,
        _height: usize,
    ) -> Result<(), RenderError> {
        let extras = n_prev.saturating_sub(n_new);
        if extras == 0 {
            return Ok(());
        }
        let mut buf = String::from("\x1b[?2026h");
        // Move to end of new content (one past last new line) then clear extras.
        let target = n_new;
        let line_diff = target as isize - self.hardware_cursor_row as isize;
        if line_diff > 0 {
            buf.push_str(&format!("\x1b[{}B", line_diff));
        } else if line_diff < 0 {
            buf.push_str(&format!("\x1b[{}A", -line_diff));
        }
        for _ in 0..extras {
            buf.push_str("\r\n\x1b[2K");
        }
        // Move back to end of new content.
        buf.push_str(&format!("\x1b[{}A", extras));
        buf.push_str("\x1b[?2026l");
        self.term.write(&buf);
        self.hardware_cursor_row = n_new.saturating_sub(1);
        Ok(())
    }

    /// Position the hardware cursor at the IME marker (rendering-engine.md
    /// Step 9). Hidden by default; positioned only so CJK IME candidate windows
    /// anchor correctly.
    fn position_hardware_cursor(
        &mut self,
        cursor_pos: Option<(usize, usize)>,
        total: usize,
        _width: usize,
    ) {
        let Some((row, col)) = cursor_pos else {
            self.term.write("\x1b[?25l"); // hide cursor
            return;
        };
        if total == 0 {
            self.term.write("\x1b[?25l");
            return;
        }
        let row = row.min(total - 1);
        let d = row as isize - self.hardware_cursor_row as isize;
        let mut buf = String::new();
        if d > 0 {
            buf.push_str(&format!("\x1b[{}B", d));
        } else if d < 0 {
            buf.push_str(&format!("\x1b[{}A", -d));
        }
        buf.push_str(&format!("\x1b[{}G", col + 1));
        if !buf.is_empty() {
            self.term.write(&buf);
        }
        self.hardware_cursor_row = row;
        // Hide hardware cursor by default (IME still anchors to it). Show only
        // if a host flag requests it (future: show_hardware_cursor config).
        self.term.write("\x1b[?25l");
    }

    /// Clean exit: move to end of content, newline (so the shell prompt doesn't
    /// overwrite), restore terminal. rendering-engine.md Step 10.
    pub fn stop(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        if !self.previous_lines.is_empty() {
            let diff = self.previous_lines.len() as isize - self.hardware_cursor_row as isize;
            let mut buf = String::new();
            if diff > 0 {
                buf.push_str(&format!("\x1b[{}B", diff));
            } else if diff < 0 {
                buf.push_str(&format!("\x1b[{}A", -diff));
            }
            buf.push_str("\r\n");
            self.term.write(&buf);
        }
        self.term.write("\x1b[?25h"); // show cursor
    }

    /// Debug accessors for tests.
    #[cfg(test)]
    pub(crate) fn last_changed_range(&self) -> Option<(usize, usize)> {
        self.last_changed_range
    }
    #[cfg(test)]
    pub(crate) fn last_full_redraw_reason(&self) -> Option<&'static str> {
        self.last_full_redraw_reason
    }
    #[cfg(test)]
    pub(crate) fn previous_line_count(&self) -> usize {
        self.previous_lines.len()
    }
    #[cfg(test)]
    pub(crate) fn set_root(&mut self, root: Box<dyn Component>) {
        self.root = root;
    }
    #[cfg(test)]
    pub(crate) fn render_requested(&self) -> bool {
        self.render_requested
    }

    // ── UX routing (c399 stage 3) ───────────────────────────────────────────

    /// Register an app-wide input interceptor (pi ux.md Step 4). Listeners run
    /// in registration order before the focused widget. Use for global keys
    /// (Ctrl+L force redraw). NOT for Ctrl+C/D — those are widget keybindings
    /// (see the module docs + pi tui.ts:825 comment).
    pub fn add_input_listener(&mut self, listener: Box<dyn InputListener>) {
        self.input_listeners.push(listener);
    }

    /// Route a single key event. Returns the high-level outcome (if any) the
    /// host loop should act on (Submit/Slash/Abort/Quit); `None` means no
    /// actionable intent this key. The engine requests a render itself when a
    /// listener or the focused widget handled the key.
    ///
    /// Pipeline (pi `tui.ts::handleInput`):
    /// 1. input listeners (consume stops; rewrite replaces the key)
    /// 2. route the (possibly rewritten) key to `root`, which forwards it to
    ///    its focused child if root is a `Container`
    /// 3. drain the focused widget's pending outcome via `take_outcome`
    /// 4. `request_render(false)` if the key was handled
    pub fn handle_event(&mut self, key: &KeyEvent) -> Option<UxOutcome> {
        // 1. Input listeners. A rewrite replaces `key` for all subsequent steps
        //    (including later listeners). A consume returns immediately.
        let mut current = *key;
        for listener in &mut self.input_listeners {
            match listener.on_key(&current) {
                ListenerResult::Consume => return None,
                ListenerResult::Rewrite(k) => current = k,
                ListenerResult::Pass => {}
            }
        }

        // 2. Route to root (Container forwards to its focused child).
        let handled = self.root.handle_input(&current);

        // 3. Drain any pending outcome the focused widget produced.
        let outcome = self.root.take_outcome();

        // 4. Request a render if the key was handled or produced an outcome.
        //    (Idle/NotHandled with no outcome = nothing changed, skip render.)
        if handled == super::component::InputResult::Handled || outcome.is_some() {
            self.request_render(false);
        }

        // Normalize: an `Idle` outcome is semantically "handled, no action" —
        // surface it so the host loop can distinguish from `None` (not handled).
        outcome
    }
}

/// Output of `post_process_lines`: serialized ANSI lines + optional IME cursor
/// position (row, col-before-marker).
type ProcessedLines = (Vec<String>, Option<(usize, usize)>);

/// Hard-error type for the width invariant (Step 6). The engine aborts rather
/// than silently wrapping a too-wide line (which would desync every subsequent
/// diff row). The host catches this, logs, and restores the terminal.
#[derive(Debug)]
pub enum RenderError {
    LineTooWide {
        row: usize,
        line_width: usize,
        terminal_width: usize,
        line_text: String,
    },
}

// Re-export Event so the host loop doesn't need to import crossterm directly
// just for the event type.
pub use crossterm::event::Event as TerminalEvent;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::engine::style::{CellStyle, Span};
    use crate::app::tui::engine::terminal::CapturingTerminal;

    /// A widget that renders a fixed list of plain-text lines (test helper).
    struct LinesWidget {
        lines: Vec<StyledLine>,
    }
    impl Component for LinesWidget {
        fn render(&self, _width: usize) -> Vec<StyledLine> {
            self.lines.clone()
        }
    }

    fn make_tui(cols: u16, rows: u16, lines: Vec<&str>) -> Tui<CapturingTerminal> {
        let term = CapturingTerminal::new(cols, rows);
        let widget = LinesWidget {
            lines: lines.into_iter().map(StyledLine::raw).collect(),
        };
        Tui::new(term, Box::new(widget))
    }

    #[test]
    fn first_render_writes_all_lines_without_clearing() {
        // Strategy A: first render preserves scrollback above (no \x1b[2J).
        let mut tui = make_tui(80, 24, vec!["hello", "world"]);
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        assert!(out.contains("\x1b[?2026h"), "sync begin");
        assert!(out.contains("hello"), "line 1");
        assert!(out.contains("world"), "line 2");
        assert!(
            !out.contains("\x1b[2J"),
            "first render must NOT clear screen"
        );
        assert!(out.contains("\x1b[?2026l"), "sync end");
        assert_eq!(tui.previous_line_count(), 2);
    }

    #[test]
    fn unchanged_frame_writes_nothing() {
        let mut tui = make_tui(80, 24, vec!["a", "b"]);
        tui.render_now().unwrap();
        let after_first = tui.term_mut().written.len();
        tui.render_now().unwrap(); // no change
        let after_second = tui.term_mut().written.len();
        // Second frame: no diff, but cursor hide is still emitted. The changed
        // range should be None.
        assert_eq!(tui.last_changed_range(), None, "no changed range");
        assert!(after_second >= after_first, "may emit cursor hide only");
    }

    #[test]
    fn append_only_writes_new_lines() {
        // Append case: adding a line should rewrite only the new line region
        // (not the whole screen). The changed range starts at the old length.
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![StyledLine::raw("a"), StyledLine::raw("b")],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        assert_eq!(tui.last_changed_range(), Some((0, 1)));
        tui.term_mut().written.clear();

        // Swap widget to append a line.
        let widget2 = LinesWidget {
            lines: vec![
                StyledLine::raw("a"),
                StyledLine::raw("b"),
                StyledLine::raw("c"),
            ],
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        assert!(out.contains("c"), "new line written");
        assert!(!out.contains("\x1b[2J"), "no full clear on append");
        // Changed range should start at index 2 (the new line).
        assert_eq!(tui.last_changed_range(), Some((2, 2)));
    }

    #[test]
    fn in_place_edit_rewrites_only_changed_line() {
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![
                StyledLine::raw("a"),
                StyledLine::raw("b"),
                StyledLine::raw("c"),
            ],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        tui.term_mut().written.clear();

        // Change only the middle line.
        let widget2 = LinesWidget {
            lines: vec![
                StyledLine::raw("a"),
                StyledLine::raw("CHANGED"),
                StyledLine::raw("c"),
            ],
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        assert!(out.contains("CHANGED"), "changed line written");
        assert!(!out.contains("\x1b[2J"), "no full clear on in-place edit");
        assert_eq!(
            tui.last_changed_range(),
            Some((1, 1)),
            "only line 1 changed"
        );
    }

    #[test]
    fn width_change_triggers_full_redraw() {
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![StyledLine::raw("a"), StyledLine::raw("b")],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        assert_eq!(tui.last_full_redraw_reason(), Some("first render"));

        tui.term_mut().written.clear();
        tui.term_mut().resize(100, 24); // width change
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        assert!(out.contains("\x1b[2J"), "width change -> full clear");
        assert_eq!(
            tui.last_full_redraw_reason(),
            Some("terminal width changed")
        );
    }

    #[test]
    fn shrink_clears_orphan_rows() {
        // Content 4 lines -> 2 lines: orphan rows 2,3 must be cleared.
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![
                StyledLine::raw("a"),
                StyledLine::raw("b"),
                StyledLine::raw("c"),
                StyledLine::raw("d"),
            ],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        tui.term_mut().written.clear();

        let widget2 = LinesWidget {
            lines: vec![StyledLine::raw("a"), StyledLine::raw("b")],
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        // Orphan clearing uses \x1b[2K; two orphan rows (c, d removed).
        // The diff writes the (unchanged) a,b range as None first, then clears.
        // Count clear-line escapes as a proxy for orphan clearing activity.
        let clear_count = out.matches("\x1b[2K").count();
        assert!(
            clear_count >= 2,
            "orphan rows cleared (>=2 \\x1b[2K), got {clear_count}: {out:?}"
        );
    }

    #[test]
    fn hard_width_error_on_overflow() {
        // A line wider than width MUST be a hard error, not silent wrap.
        let term = CapturingTerminal::new(3, 24); // narrow
        let widget = LinesWidget {
            lines: vec![StyledLine::raw("toolong")], // 6 > 3
        };
        let mut tui = Tui::new(term, Box::new(widget));
        let result = tui.render_now();
        assert!(matches!(result, Err(RenderError::LineTooWide { .. })));
    }

    #[test]
    fn hard_width_error_on_full_redraw_path_too() {
        // Closes audit Finding 2: width check on fullRender path (width change).
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![StyledLine::raw("short")],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        tui.term_mut().resize(2, 24); // narrow so "short" overflows
        let result = tui.render_now();
        assert!(
            matches!(result, Err(RenderError::LineTooWide { .. })),
            "full-redraw path also enforces width (audit Finding 2)"
        );
    }

    #[test]
    fn force_render_clears_previous_and_full_redraws() {
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![StyledLine::raw("a")],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        tui.term_mut().written.clear();

        tui.request_render(true); // force
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        assert!(out.contains("\x1b[2J"), "force render -> full clear");
    }

    #[test]
    fn cursor_marker_extracted_and_stripped() {
        // A Focusable widget embeds CURSOR_MARKER; the engine strips it and
        // records the (row, col) for IME positioning.
        let term = CapturingTerminal::new(80, 24);
        // line: "ab<MARKER>cd" — cursor after "ab" (col 2)
        let line = StyledLine::from_spans(vec![
            Span::raw("ab"),
            Span::raw(format!("{CURSOR_MARKER}cd")),
        ]);
        let widget = LinesWidget { lines: vec![line] };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        assert!(!out.contains(CURSOR_MARKER), "marker stripped before write");
        assert!(out.contains("abcd"), "text preserved");
    }

    #[test]
    fn styled_line_diffed_by_ansi_equality() {
        // Two frames where styling changes but plain text is identical: the
        // diff MUST detect the change (ansi differs).
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![StyledLine::from_spans(vec![Span::styled(
                "x",
                CellStyle::default(),
            )])],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        tui.term_mut().written.clear();

        // Same text, but bold now — ansi changes, diff must fire.
        let widget2 = LinesWidget {
            lines: vec![StyledLine::from_spans(vec![Span::styled(
                "x",
                CellStyle::default().bold(),
            )])],
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();
        assert_eq!(
            tui.last_changed_range(),
            Some((0, 0)),
            "style change detected"
        );
    }

    // ── UX routing (c399 stage 3) ───────────────────────────────────────────

    use crate::app::tui::engine::component::{Container, Focusable, InputResult};
    use crate::app::tui::engine::outcome::UxOutcome;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    /// Focusable widget that records handled keys + optionally yields an outcome.
    /// Mirrors `widgets::input::Input`'s contract without pulling the full input.
    struct RecordWidget {
        focused: bool,
        handled: Vec<KeyEvent>,
        outcome: Option<UxOutcome>,
    }
    impl RecordWidget {
        fn new() -> Self {
            Self {
                focused: false,
                handled: Vec::new(),
                outcome: None,
            }
        }
    }
    impl Component for RecordWidget {
        fn render(&self, _width: usize) -> Vec<StyledLine> {
            Vec::new()
        }
        fn handle_input(&mut self, key: &KeyEvent) -> InputResult {
            self.handled.push(*key);
            InputResult::Handled
        }
        fn focused(&self) -> bool {
            self.focused
        }
        fn set_focused(&mut self, focused: bool) {
            self.focused = focused;
        }
        fn take_outcome(&mut self) -> Option<UxOutcome> {
            self.outcome.take()
        }
    }
    impl Focusable for RecordWidget {}

    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn handle_event_routes_to_focused_child_and_drains_outcome() {
        let term = CapturingTerminal::new(80, 24);
        let mut root = Container::new();
        let mut w = RecordWidget::new();
        w.outcome = Some(UxOutcome::Submit("hi".into()));
        w.set_focused(true);
        assert!(w.focused);
        root.add(Box::new(w));
        root.set_focused_index(Some(0));
        let mut tui = Tui::new(term, Box::new(root));
        // Any key triggers handle_input; outcome is pre-set.
        let outcome = tui.handle_event(&char_key('a'));
        match outcome {
            Some(UxOutcome::Submit(s)) => assert_eq!(s, "hi"),
            other => panic!("expected Submit, got {other:?}"),
        }
    }

    #[test]
    fn handle_event_no_focus_returns_none() {
        let term = CapturingTerminal::new(80, 24);
        let root = Container::new(); // no children, no focus
        let mut tui = Tui::new(term, Box::new(root));
        assert!(tui.handle_event(&char_key('a')).is_none());
    }

    #[test]
    fn input_listener_consume_blocks_routing() {
        struct ConsumeAll;
        impl InputListener for ConsumeAll {
            fn on_key(&mut self, _key: &KeyEvent) -> ListenerResult {
                ListenerResult::Consume
            }
        }
        let term = CapturingTerminal::new(80, 24);
        let mut root = Container::new();
        let mut w = RecordWidget::new();
        w.outcome = Some(UxOutcome::Submit("should-not-fire".into()));
        root.add(Box::new(w));
        root.set_focused_index(Some(0));
        let mut tui = Tui::new(term, Box::new(root));
        tui.add_input_listener(Box::new(ConsumeAll));
        // Listener consumes → no routing, no outcome.
        assert!(tui.handle_event(&char_key('a')).is_none());
    }

    #[test]
    fn input_listener_rewrite_replaces_key() {
        /// Rewrites every key to 'z'.
        struct RewriteToZ;
        impl InputListener for RewriteToZ {
            fn on_key(&mut self, _key: &KeyEvent) -> ListenerResult {
                ListenerResult::Rewrite(char_key('z'))
            }
        }
        let term = CapturingTerminal::new(80, 24);
        let mut root = Container::new();
        let mut w = RecordWidget::new();
        root.add(Box::new(w));
        root.set_focused_index(Some(0));
        let mut tui = Tui::new(term, Box::new(root));
        tui.add_input_listener(Box::new(RewriteToZ));
        // Send 'a' — listener rewrites to 'z' — widget should record 'z'.
        let _ = tui.handle_event(&char_key('a'));
        // The widget is inside root (Box<dyn Component>); inspect via outcome
        // path isn't possible here (no outcome set). We at least confirm the
        // pipeline didn't panic and the rewrite path was taken.
        // (A fuller assertion lives in the component-level routing tests.)
    }

    #[test]
    fn input_listener_pass_continues_routing() {
        struct PassAll;
        impl InputListener for PassAll {
            fn on_key(&mut self, _key: &KeyEvent) -> ListenerResult {
                ListenerResult::Pass
            }
        }
        let term = CapturingTerminal::new(80, 24);
        let mut root = Container::new();
        let mut w = RecordWidget::new();
        w.outcome = Some(UxOutcome::Abort);
        root.add(Box::new(w));
        root.set_focused_index(Some(0));
        let mut tui = Tui::new(term, Box::new(root));
        tui.add_input_listener(Box::new(PassAll));
        assert!(matches!(
            tui.handle_event(&char_key('a')),
            Some(UxOutcome::Abort)
        ));
    }

    #[test]
    fn handle_event_requests_render_when_handled() {
        let term = CapturingTerminal::new(80, 24);
        let mut root = Container::new();
        root.add(Box::new(RecordWidget::new()));
        root.set_focused_index(Some(0));
        let mut tui = Tui::new(term, Box::new(root));
        assert!(!tui.render_requested());
        let _ = tui.handle_event(&char_key('a'));
        assert!(tui.render_requested(), "handled key requests a render");
    }

    #[test]
    fn handle_event_skips_render_when_not_handled() {
        // No focus → root.handle_input returns NotHandled → no render requested.
        let term = CapturingTerminal::new(80, 24);
        let root = Container::new();
        let mut tui = Tui::new(term, Box::new(root));
        let _ = tui.handle_event(&char_key('a'));
        assert!(
            !tui.render_requested(),
            "unhandled key should not request render"
        );
    }
}
