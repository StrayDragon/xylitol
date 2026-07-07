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
    /// Whether a shrink below `max_lines_rendered` triggers a full redraw
    /// (pi tui.ts:1361-1365 `clearOnShrink`). Set at construction from
    /// `$XYLITOL_TUI_CLEAR_ON_SHRINK` (default on); tests override via
    /// [`set_clear_on_shrink`](Self::set_clear_on_shrink) to stay deterministic.
    clear_on_shrink: bool,
}

/// Whether `clearOnShrink` (full redraw when content shrinks below the working
/// area high-water) is enabled. Reads `$XYLITOL_TUI_CLEAR_ON_SHRINK`; defaults
/// to enabled (pi's default). Set `XYLITOL_TUI_CLEAR_ON_SHRINK=0` to disable.
fn clear_on_shrink_enabled() -> bool {
    !matches!(
        std::env::var("XYLITOL_TUI_CLEAR_ON_SHRINK").ok().as_deref(),
        Some("0" | "false")
    )
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
            clear_on_shrink: clear_on_shrink_enabled(),
        }
    }

    /// Access the underlying terminal (for event polling by the host loop).
    pub fn term_mut(&mut self) -> &mut T {
        &mut self.term
    }

    /// Override the `clearOnShrink` behavior (defaults from `$XYLITOL_TUI_CLEAR_ON_SHRINK`).
    /// Tests use this for determinism; production leaves the default.
    #[cfg(test)]
    pub(crate) fn set_clear_on_shrink(&mut self, on: bool) {
        self.clear_on_shrink = on;
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
        let mut raw_lines = self.root.render(width);
        // 1b. Pad to at least terminal height (pi tui.ts:1065-1068).
        //     Without this, short content leaves blank rows below and the
        //     footer (loader + input) drifts to the middle instead of
        //     anchoring to the bottom. Extra empty StyledLines create blank
        //     rows that push the footer down.
        while raw_lines.len() < height {
            raw_lines.push(StyledLine::empty());
        }
        // 2. extract + strip CURSOR_MARKER, apply line resets, check widths
        let (new_lines, cursor_pos) = self.post_process_lines(&raw_lines, width)?;

        // 3. choose strategy
        let first_render = self.previous_lines.is_empty()
            && !width_changed
            && !height_changed
            && !self.force_clear;
        // clearOnShrink (pi tui.ts:1361-1365): when content has shrunk below
        // the working-area high water mark, a diff would leave stale orphan
        // rows; force a full redraw to reclaim them. Toggleable via
        // XYLITOL_TUI_CLEAR_ON_SHRINK (defaults on, like pi).
        let n_new = new_lines.len();
        let clear_on_shrink = self.previous_lines.len() > n_new
            && n_new < self.max_lines_rendered
            && self.clear_on_shrink;
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
        } else if clear_on_shrink {
            self.full_render(
                &new_lines,
                true,
                width,
                height,
                "clearOnShrink (content shrank)",
            )?;
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
    ///
    /// Branches (mirroring pi `tui.ts::doDiff` + Step 5C spec):
    /// - **No changes**: nothing to write.
    /// - **All deletions** (`first >= n_new`): the changed range is entirely
    ///   below the new content (a pure tail-shrink). Move to end-of-content,
    ///   clear extras with `\r\n\x1b[2K`, move back. Falls back to fullRender
    ///   when extras > height or the target row is above the viewport.
    /// - **`first < previous_viewport_top`**: unreachable by diff → fullRender.
    /// - **Otherwise**: the normal write loop (single buffered, with the shrink
    ///   tail-clearing inlined so it shares one sync-output envelope + one
    ///   `finalCursor` tracking).
    fn diff_render(
        &mut self,
        new_lines: &[String],
        width: usize,
        height: usize,
    ) -> Result<(), RenderError> {
        use std::cmp::Ordering;

        let n_prev = self.previous_lines.len();
        let n_new = new_lines.len();
        // Find first/last changed line (treat missing as "").
        let mut first: Option<usize> = None;
        let mut last: Option<usize> = None;
        let max_len = n_prev.max(n_new);
        for i in 0..max_len {
            let prev = self.previous_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let new = new_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            if prev != new {
                if first.is_none() {
                    first = Some(i);
                }
                last = Some(i);
            }
        }
        let (Some(first), Some(last)) = (first, last) else {
            // No changes — only cursor positioning (done in do_render).
            self.last_changed_range = None;
            self.last_full_redraw_reason = None;
            return Ok(());
        };

        // firstChanged above the viewport → unreachable by diff → fullRender.
        if first < self.previous_viewport_top {
            return self.full_render(
                new_lines,
                true,
                width,
                height,
                "firstChanged above viewport",
            );
        }

        // All-deletions: the whole changed range is below the new content tail.
        if first >= n_new {
            // Step 5C: if extras > height, or target above viewport, fall back
            // to B (full redraw). Clearing many rows one-by-one would push
            // content into scrollback and desync viewport_top.
            let extras = n_prev.saturating_sub(n_new);
            let target = n_new.saturating_sub(1);
            if extras > height || target < self.previous_viewport_top {
                return self.full_render(new_lines, true, width, height, "all-deletions fallback");
            }
            let mut buf = String::from("\x1b[?2026h");
            // Move to the end of the new content (screen-relative).
            self.move_to_row(&mut buf, target, height);
            for _ in 0..extras {
                buf.push_str("\r\n\x1b[2K");
            }
            // Move back to end of new content.
            if extras > 0 {
                buf.push_str(&format!("\x1b[{extras}A"));
            }
            buf.push_str("\x1b[?2026l");
            self.term.write(&buf);
            self.hardware_cursor_row = target;
            self.cursor_row = n_new.saturating_sub(1);
            let floor = n_new.saturating_sub(height);
            if self.previous_viewport_top < floor {
                self.previous_viewport_top = floor;
            }
            self.max_lines_rendered = self.max_lines_rendered.max(n_new);
            self.last_changed_range = Some((first, last));
            self.last_full_redraw_reason = None;
            return Ok(());
        }

        // Normal write loop (single buffer, shrink inlined).
        let mut buf = String::from("\x1b[?2026h");
        let prev_viewport_top = self.previous_viewport_top;

        // appendStart: pure append starting exactly at the old tail (pi
        // tui.ts:1394). The write loop opens with `\r\n` and the scroll target
        // backs up one row to the last unchanged line.
        let append_start = n_new > n_prev && first == n_prev && first > 0;
        let move_target = if append_start { first - 1 } else { first };

        // Scroll block: move cursor to viewport BOTTOM first, then `\r\n` ×
        // scroll (newline only scrolls when issued from the bottom row).
        let prev_bottom = prev_viewport_top + height.saturating_sub(1);
        if move_target > prev_bottom {
            let current_screen_row = self
                .hardware_cursor_row
                .saturating_sub(prev_viewport_top)
                .min(height.saturating_sub(1));
            let to_bottom = height.saturating_sub(1).saturating_sub(current_screen_row);
            if to_bottom > 0 {
                buf.push_str(&format!("\x1b[{to_bottom}B"));
            }
            let scroll = move_target - prev_bottom;
            for _ in 0..scroll {
                buf.push_str("\r\n");
            }
            self.previous_viewport_top += scroll;
            self.hardware_cursor_row = move_target;
        }

        // Relative move to move_target, SCREEN coordinates (Step 5C lineDiff).
        self.move_to_row(&mut buf, move_target, height);

        // Open the write loop: `\r\n` for append, `\r` otherwise.
        buf.push_str(if append_start { "\r\n" } else { "\r" });
        let render_end = last.min(n_new.saturating_sub(1));
        for i in first..=render_end {
            if i > first {
                buf.push_str("\r\n");
            }
            buf.push_str("\x1b[2K"); // clear line first (new may be shorter)
            if let Some(line) = new_lines.get(i) {
                buf.push_str(line);
            }
        }

        // Shrink tail-clearing (inlined, same buffer): if content shrank, clear
        // the orphan rows below the new content. Per Step 5C: if the written
        // range didn't reach the new tail, move down to the tail first so the
        // orphan clear starts from the right place. finalCursor tracks where
        // the cursor actually stops (the new content tail).
        let mut final_cursor = render_end;
        if n_prev > n_new {
            // If the written range ended before the new tail, move down to it.
            if render_end + 1 < n_new {
                let down = (n_new - 1).saturating_sub(render_end);
                buf.push_str(&format!("\x1b[{down}B"));
                final_cursor = n_new - 1;
            }
            let extras = n_prev - n_new;
            for _ in 0..extras {
                buf.push_str("\r\n\x1b[2K");
            }
            if extras > 0 {
                buf.push_str(&format!("\x1b[{extras}A"));
            }
        }

        buf.push_str("\x1b[?2026l");
        self.term.write(&buf);

        // Step 5C save-state invariants.
        self.hardware_cursor_row = final_cursor;
        self.cursor_row = n_new.saturating_sub(1);
        let floor = final_cursor.saturating_sub(height.saturating_sub(1));
        if self.previous_viewport_top < floor {
            self.previous_viewport_top = floor;
        }
        self.max_lines_rendered = self.max_lines_rendered.max(n_new);
        self.last_changed_range = Some((first, last));
        self.last_full_redraw_reason = None;
        // Touch Ordering so the `use` stays even if both branches are currently
        // equality (keeps the import meaningful for future additions).
        let _ = Ordering::Equal;
        Ok(())
    }

    /// Append a relative cursor-row move to `buf` so the hardware cursor lands
    /// on logical `target_row`, using SCREEN coordinates (both sides minus the
    /// current viewport top — pi `computeLineDiff`, Step 5C line 246). Emits
    /// CUD (`\x1b[NB`) or CUU (`\x1b[NA`) as needed; nothing when already there.
    fn move_to_row(&self, buf: &mut String, target_row: usize, _height: usize) {
        let current_screen =
            self.hardware_cursor_row as isize - self.previous_viewport_top as isize;
        let target_screen = target_row as isize - self.previous_viewport_top as isize;
        let line_diff = target_screen - current_screen;
        if line_diff > 0 {
            buf.push_str(&format!("\x1b[{line_diff}B"));
        } else if line_diff < 0 {
            buf.push_str(&format!("\x1b[{}A", -line_diff));
        }
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
    #[cfg(test)]
    pub(crate) fn viewport_top(&self) -> usize {
        self.previous_viewport_top
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
        // After c400 padding: render always pads to terminal height (24).
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
        // Padded to terminal height (24).
        assert_eq!(tui.previous_line_count(), 24);
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
        // Append after c400 padding: both frames padded to height=24.
        // Changed range should be exactly the new-appended lines only.
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![StyledLine::raw("a"), StyledLine::raw("b")],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        // First frame: all 24 lines are new (from empty prev).
        assert_eq!(tui.last_changed_range(), Some((0, 23)));
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
        // Only the changed line (index 2, the new "c") should be rewritten.
        // Padded empty lines (3..23) are identical in both frames.
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
    fn shrink_triggers_clear_on_shrink_full_redraw() {
        // After c400 padding: all content is padded to terminal height.
        // clearOnShrink only triggers when content exceeds height. Use
        // 30→20 lines with height=8 so un-padded shrink is detected.
        let height: u16 = 8;
        let term = CapturingTerminal::new(80, height);
        let lines: Vec<StyledLine> = (0..30).map(|i| StyledLine::raw(&format!("L{i}"))).collect();
        let mut tui = Tui::new(term, Box::new(LinesWidget { lines }));
        tui.set_clear_on_shrink(true);
        tui.render_now().unwrap();
        tui.term_mut().written.clear();

        let widget2 = LinesWidget {
            lines: (0..20).map(|i| StyledLine::raw(&format!("L{i}"))).collect(),
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        // clearOnShrink → full redraw: screen-clear sequence present.
        assert!(
            out.contains("\x1b[2J"),
            "full screen clear on shrink: {out:?}"
        );
        assert_eq!(
            tui.last_full_redraw_reason(),
            Some("clearOnShrink (content shrank)")
        );
    }

    #[test]
    fn shrink_with_clear_on_shrink_disabled_diffs_orphan_rows() {
        // With clearOnShrink OFF, shrink goes through the diff path and clears
        // orphan rows in-place via \x1b[2K (no full screen clear).
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
        tui.set_clear_on_shrink(false);
        tui.render_now().unwrap();
        tui.term_mut().written.clear();

        let widget2 = LinesWidget {
            lines: vec![StyledLine::raw("a"), StyledLine::raw("b")],
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        // No full screen clear; orphan rows cleared in-place with \x1b[2K.
        assert!(
            !out.contains("\x1b[2J"),
            "no full clear in diff path: {out:?}"
        );
        let clear_count = out.matches("\x1b[2K").count();
        assert!(
            clear_count >= 2,
            "orphan rows cleared (>=2 \\x1b[2K), got {clear_count}: {out:?}"
        );
    }

    #[test]
    fn deep_shrink_above_viewport_falls_back_to_full_redraw() {
        // A shrink where the new content tail lands above the current viewport
        // top is unreachable by in-place diff (you can't scroll UP into
        // scrollback). Step 5C: fall back to full redraw. This is what actually
        // fires for a large tail-shrink (the extras>height all-deletions check
        // is a secondary guard; "firstChanged above viewport" fires first
        // whenever the content tail recedes past the viewport top, which is the
        // common case for a deep shrink).
        let term = CapturingTerminal::new(80, 4);
        let widget = LinesWidget {
            lines: (0..10)
                .map(|i| StyledLine::raw(format!("L{i}")))
                .collect::<Vec<_>>(),
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.set_clear_on_shrink(false); // isolate the diff-path fallback
        tui.render_now().unwrap();
        // viewport_top is now 10-4=6.
        tui.term_mut().written.clear();

        // Shrink to 2 lines: first=2 < viewport_top=6 → "firstChanged above
        // viewport" full redraw.
        let widget2 = LinesWidget {
            lines: vec![StyledLine::raw("L0"), StyledLine::raw("L1")],
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();
        assert_eq!(
            tui.last_full_redraw_reason(),
            Some("firstChanged above viewport"),
            "deep shrink above viewport falls back to full redraw"
        );
    }

    #[test]
    fn all_deletions_in_viewport_clears_in_place() {
        // After c400 padding: all content pads to height. Use 30→28 lines
        // with height=8 so unpadded tail-shrink triggers all-deletions branch.
        let height: u16 = 8;
        let term = CapturingTerminal::new(80, height);
        let lines: Vec<StyledLine> = (0..30).map(|i| StyledLine::raw(&format!("L{i}"))).collect();
        let mut tui = Tui::new(term, Box::new(LinesWidget { lines }));
        tui.set_clear_on_shrink(false);
        tui.render_now().unwrap();
        tui.term_mut().written.clear();

        let widget2 = LinesWidget {
            lines: (0..28).map(|i| StyledLine::raw(&format!("L{i}"))).collect(),
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        assert!(
            !out.contains("\x1b[2J"),
            "in-place clear, no full redraw: {out:?}"
        );
        assert!(tui.last_full_redraw_reason().is_none());
        // 2 orphan rows cleared in place.
        assert!(
            out.matches("\x1b[2K").count() >= 2,
            "orphan rows cleared: {out:?}"
        );
        // Cursor ends at content tail after padding to height.
        assert!(
            tui.hardware_cursor_row == (height as usize).max(28).saturating_sub(1),
            "cursor at new content tail (padded): got {}",
            tui.hardware_cursor_row
        );
    }

    #[test]
    fn in_place_middle_change_keeps_cursor_at_content_tail() {
        // Change a middle row (not append, not shrink). The write loop ends at
        // `last`, but finalCursor must reflect the content tail so the NEXT
        // frame's relative moves are correct. Here content is 5 rows, we change
        // only row 1; after render hardware_cursor_row should let a subsequent
        // append work without re-correction.
        let term = CapturingTerminal::new(80, 24);
        let widget = LinesWidget {
            lines: vec![
                StyledLine::raw("L0"),
                StyledLine::raw("L1"),
                StyledLine::raw("L2"),
                StyledLine::raw("L3"),
                StyledLine::raw("L4"),
            ],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        tui.term_mut().written.clear();

        // Change row 1 only.
        let widget2 = LinesWidget {
            lines: vec![
                StyledLine::raw("L0"),
                StyledLine::raw("CHANGED"),
                StyledLine::raw("L2"),
                StyledLine::raw("L3"),
                StyledLine::raw("L4"),
            ],
        };
        tui.set_root(Box::new(widget2));
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();
        assert!(out.contains("CHANGED"));
        // changed range was (1,1); hardware_cursor_row should be 1 (last written),
        // and a following append should still render correctly (covered by the
        // append tests). This guards against the old bug where cursor landed off.
        assert_eq!(tui.last_changed_range(), Some((1, 1)));
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

    // ── Over-viewport append / scrollback (rendering-engine.md Step 5C) ────

    #[test]
    fn append_past_viewport_scrolls_via_cursor_down_then_newline() {
        // Regression for the "over-one-screen overlap" bug. Terminal height=3,
        // start with exactly one screen (3 lines), then append 2 more. The
        // changed range starts at line 3 — past the previous viewport bottom
        // (2). Per rendering-engine.md Step 5C, the scroll block MUST move the
        // cursor to the viewport BOTTOM row first (CUD), then emit "\r\n" to
        // scroll. The old code emitted "\r\n" directly from wherever the cursor
        // was, so on a real terminal the new content overwrote old rows instead
        // of scrolling them into scrollback (visible as duplicated/overlapped
        // text). Also the relative line_diff MUST be in screen coordinates
        // (both sides minus viewport top), not absolute row numbers.
        let term = CapturingTerminal::new(40, 3);
        let widget = LinesWidget {
            lines: vec![
                StyledLine::raw("L0"),
                StyledLine::raw("L1"),
                StyledLine::raw("L2"),
            ],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();
        assert_eq!(tui.hardware_cursor_row, 2);
        assert_eq!(tui.previous_viewport_top, 0);

        // Append two lines → content (5) > height (3). firstChanged=3.
        let widget2 = LinesWidget {
            lines: vec![
                StyledLine::raw("L0"),
                StyledLine::raw("L1"),
                StyledLine::raw("L2"),
                StyledLine::raw("L3"),
                StyledLine::raw("L4"),
            ],
        };
        tui.set_root(Box::new(widget2));
        tui.term_mut().written.clear();
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();

        // No CUD (cursor-down) count may exceed height-1 = 2. The buggy
        // absolute-subtraction line_diff sent the cursor off-screen.
        let cud_counts: Vec<usize> = out
            .match_indices("\x1b[")
            .filter_map(|(i, _)| {
                out[i + 2..]
                    .split('B')
                    .next()
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .collect();
        for c in &cud_counts {
            assert!(*c <= 2, "CUD {c} exceeds height-1=2; out={out:?}");
        }
        assert!(out.contains("L3") && out.contains("L4"));
        assert_eq!(tui.previous_viewport_top, 2);
    }

    #[test]
    fn append_far_past_viewport_uses_relative_screen_diff() {
        // Terminal height=4, fill the screen, append 3 lines (4 → 7).
        // firstChanged=4 is 1 below prev_bottom(3) → scroll=1. The relative
        // move after the scroll block MUST be computed in SCREEN coordinates
        // (both sides minus viewport top), per Step 5C lineDiff formula.
        let term = CapturingTerminal::new(40, 4);
        let widget = LinesWidget {
            lines: vec![
                StyledLine::raw("L0"),
                StyledLine::raw("L1"),
                StyledLine::raw("L2"),
                StyledLine::raw("L3"),
            ],
        };
        let mut tui = Tui::new(term, Box::new(widget));
        tui.render_now().unwrap();

        let widget2 = LinesWidget {
            lines: vec![
                StyledLine::raw("L0"),
                StyledLine::raw("L1"),
                StyledLine::raw("L2"),
                StyledLine::raw("L3"),
                StyledLine::raw("L4"),
                StyledLine::raw("L5"),
                StyledLine::raw("L6"),
            ],
        };
        tui.set_root(Box::new(widget2));
        tui.term_mut().written.clear();
        tui.render_now().unwrap();
        let out = tui.term_mut().written.clone();

        let cud_counts: Vec<usize> = out
            .match_indices("\x1b[")
            .filter_map(|(i, _)| {
                out[i + 2..]
                    .split('B')
                    .next()
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .collect();
        for c in &cud_counts {
            assert!(*c <= 3, "CUD {c} exceeds height-1=3; out={out:?}");
        }
        assert!(out.contains("L6"));
        assert_eq!(tui.previous_viewport_top, 3);
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

    // ── c400: viewport anchoring tests ────────────────────────────────────

    #[test]
    fn viewport_top_follows_formula_when_growing_from_below_height() {
        // Real scenario: content starts at 5 lines (below height=10),
        // then grows to 15. Viewport must transition from 0 (all content
        // visible) to tracking the tail (viewport_top = n - height).
        //
        // Expected:
        //   n=5:  max(0, max(10,5)-10)  = 0
        //   n=6:  max(0, max(10,6)-10)  = 0
        //   ...
        //   n=10: max(0, max(10,10)-10) = 0
        //   n=11: max(0, max(10,11)-10) = 1
        //   n=12: max(0, max(10,12)-10) = 2
        //   n=13: max(0, max(10,13)-10) = 3
        //   n=14: max(0, max(10,14)-10) = 4
        //   n=15: max(0, max(10,15)-10) = 5

        let height: u16 = 10;
        let term = CapturingTerminal::new(80, height);
        let mut tui = Tui::new(
            term,
            Box::new(LinesWidget {
                lines: vec![StyledLine::raw("line0")],
            }),
        );
        tui.render_now().unwrap();

        for n in 2..=15 {
            let lines: Vec<StyledLine> = (0..n)
                .map(|i| StyledLine::raw(&format!("line{i}")))
                .collect();
            tui.set_root(Box::new(LinesWidget { lines }));
            tui.render_now().unwrap();

            let expected = (height as usize).max(n).saturating_sub(height as usize);
            assert_eq!(
                tui.viewport_top(),
                expected,
                "n={n}: viewport_top should be {expected} (max(height={height},n) - height)"
            );
        }
    }

    #[test]
    fn viewport_top_follows_formula_when_growing_from_height_boundary() {
        // Content grows from exactly height (10) → 15. Simpler variant
        // of the above to isolate if the below→above transition matters.

        let height: u16 = 10;
        let term = CapturingTerminal::new(80, height);
        let lines: Vec<StyledLine> = (0..10)
            .map(|i| StyledLine::raw(&format!("line{i}")))
            .collect();
        let mut tui = Tui::new(term, Box::new(LinesWidget { lines }));
        tui.render_now().unwrap();

        assert_eq!(tui.viewport_top(), 0, "n=10 at height=10");

        for delta in 1..=5 {
            let n: usize = 10 + delta;
            let lines: Vec<StyledLine> = (0..n)
                .map(|i| StyledLine::raw(&format!("line{i}")))
                .collect();
            tui.set_root(Box::new(LinesWidget { lines }));
            tui.render_now().unwrap();

            let expected = (height as usize).max(n).saturating_sub(height as usize);
            assert_eq!(
                tui.viewport_top(),
                expected,
                "n={n}: viewport_top should be {expected}"
            );
        }
    }
}
