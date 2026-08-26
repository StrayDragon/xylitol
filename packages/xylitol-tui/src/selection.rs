//! ApplicationOwned text selection (c2070).
//!
//! Drag-select over transcript content coordinates, edge auto-scroll via
//! [`ScrollView`], optional copy-on-release, and dock/input exclusion.
//!
//! Coordinate model: [`CellPoint::col`] is a **display column** (terminal
//! cells, ANSI-stripped; wide chars occupy 2), not a char index. All
//! extraction / inversion convert between columns and char indices via the
//! width-aware helpers below, so CJK / emoji-bearing lines select and copy
//! exactly what is visually covered.

use crate::scroll_view::ScrollView;
use crate::utils::ansi_escape_len;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use unicode_width::UnicodeWidthChar;

// UX tuning knobs: keep the first tier at one row for precise selection. These
// hold thresholds and later multipliers may be adjusted together after manual
// terminal feel testing; they are not protocol or persistence contracts.
const EDGE_AUTOSCROLL_MEDIUM_AFTER: std::time::Duration = std::time::Duration::from_millis(400);
const EDGE_AUTOSCROLL_FAST_AFTER: std::time::Duration = std::time::Duration::from_secs(1);
const EDGE_AUTOSCROLL_MEDIUM_MULTIPLIER: isize = 2;
const EDGE_AUTOSCROLL_FAST_MULTIPLIER: isize = 4;

/// Content-space cell (row = content line index, col = display column).
///
/// `col` counts terminal cells of the ANSI-stripped text — one per narrow
/// char, two per wide char — matching what the screen and mouse report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPoint {
    pub row: usize,
    pub col: usize,
}

impl CellPoint {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Selection granularity after multi-click.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionGranularity {
    #[default]
    Character,
    Word,
    Line,
}

/// Layout rectangles in **screen** coordinates (origin top-left of the TUI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub row: u16,
    pub col: u16,
    pub height: u16,
    pub width: u16,
}

impl ScreenRect {
    pub fn contains(self, column: u16, row: u16) -> bool {
        column >= self.col
            && column < self.col.saturating_add(self.width)
            && row >= self.row
            && row < self.row.saturating_add(self.height)
    }
}

/// Sink for copy-on-release (OSC52 and/or local tools — backend is host-owned).
pub trait ClipboardSink {
    fn copy_text(&mut self, text: &str);
}

/// Records copied strings for tests / in-memory sinks.
#[derive(Debug, Default)]
pub struct RecordingClipboardSink {
    pub copies: Vec<String>,
}

impl ClipboardSink for RecordingClipboardSink {
    fn copy_text(&mut self, text: &str) {
        self.copies.push(text.to_string());
    }
}

/// Build an OSC 52 clipboard write sequence (package-local; no infra dependency).
pub fn format_osc52(text: &str) -> Option<String> {
    const MAX: usize = 100_000;
    let encoded = base64_encode(text.as_bytes());
    if encoded.len() > MAX {
        return None;
    }
    Some(format!("\x1b]52;c;{encoded}\x07"))
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Optional click consumer (e.g. fold-triangle hit for c2040).
/// Return true to swallow the press (clear selection; do not start drag).
///
/// Not `Send`: ApplicationOwned hosts are single-threaded and typically capture
/// `Rc<RefCell<_>>` fold tables (c2040).
pub type HitPriorityFn = Box<dyn FnMut(u16, u16) -> bool>;

/// ApplicationOwned selection controller over a [`ScrollView`] transcript pane.
pub struct SelectionController {
    anchor: Option<CellPoint>,
    focus: Option<CellPoint>,
    dragging: bool,
    granularity: SelectionGranularity,
    /// Default **on** (ptim05).
    pub copy_on_release: bool,
    auto_scroll_dir: i8,
    auto_scroll_started_at: Option<std::time::Instant>,
    last_pointer: Option<(u16, u16)>,
    click_count: u8,
    last_click_at: Option<std::time::Instant>,
    last_click_cell: Option<(u16, u16)>,
    hit_priority: Option<HitPriorityFn>,
    /// Set when the last [`Self::handle_mouse`] issued a non-empty copy (ptim15).
    last_event_copied: bool,
}

impl Default for SelectionController {
    fn default() -> Self {
        Self {
            anchor: None,
            focus: None,
            dragging: false,
            granularity: SelectionGranularity::Character,
            copy_on_release: true,
            auto_scroll_dir: 0,
            auto_scroll_started_at: None,
            last_pointer: None,
            click_count: 0,
            last_click_at: None,
            last_click_cell: None,
            hit_priority: None,
            last_event_copied: false,
        }
    }
}

impl SelectionController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_hit_priority(&mut self, hit: Option<HitPriorityFn>) {
        self.hit_priority = hit;
    }

    /// Take the hit-priority hook (e.g. TUI park/restore across a mouse dispatch).
    pub fn take_hit_priority(&mut self) -> Option<HitPriorityFn> {
        self.hit_priority.take()
    }

    pub fn clear(&mut self) {
        self.anchor = None;
        self.focus = None;
        self.dragging = false;
        self.stop_auto_scroll();
        self.granularity = SelectionGranularity::Character;
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn has_selection(&self) -> bool {
        match (self.anchor, self.focus) {
            (Some(a), Some(f)) => a != f,
            _ => false,
        }
    }

    /// True if the most recent [`Self::handle_mouse`] copied non-empty text.
    pub fn last_event_copied(&self) -> bool {
        self.last_event_copied
    }

    pub fn bounds(&self) -> Option<(CellPoint, CellPoint)> {
        let a = self.anchor?;
        let f = self.focus?;
        Some(ordered(a, f))
    }

    /// Handle a mouse event.
    ///
    /// - `transcript`: screen rect of the scrollable content pane
    /// - `dock`: screen rect of the input dock (excluded from transcript selection)
    ///
    /// Returns whether presentation changed (caller should rerender).
    pub fn handle_mouse(
        &mut self,
        event: &MouseEvent,
        scroll: &mut ScrollView,
        transcript: ScreenRect,
        dock: ScreenRect,
        sink: &mut dyn ClipboardSink,
    ) -> bool {
        self.last_event_copied = false;
        let (col, row) = (event.column, event.row);

        // Dock / input: never *start* transcript selection on dock text (ptim06).
        // While already dragging, clamp to transcript bottom and keep extending
        // (Pi-style) — MUST NOT clear / copy-cancel mid-drag (ptim12).
        if dock.contains(col, row) {
            return self.handle_dock_mouse(event, scroll, transcript, sink);
        }

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(ref mut hit) = self.hit_priority
                    && hit(col, row)
                {
                    self.clear();
                    return true;
                }
                if !transcript.contains(col, row) {
                    return false;
                }
                self.update_click_count(col, row);
                let Some(cell) = self.screen_to_content(col, row, scroll, transcript) else {
                    return false;
                };
                self.granularity = match self.click_count {
                    2 => SelectionGranularity::Word,
                    3.. => SelectionGranularity::Line,
                    _ => SelectionGranularity::Character,
                };
                let (a, f) = expand_for_granularity(cell, self.granularity, scroll.lines());
                self.anchor = Some(a);
                self.focus = Some(f);
                self.dragging = true;
                self.last_pointer = Some((col, row));
                self.update_edge_dir(row, transcript);
                true
            }
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
                if !self.dragging {
                    return false;
                }
                self.last_pointer = Some((col, row));
                self.update_edge_dir(row, transcript);
                let cell = self
                    .screen_to_content(col, row, scroll, transcript)
                    .or_else(|| self.clamp_to_transcript_edge(col, row, scroll, transcript));
                if let Some(cell) = cell {
                    self.apply_focus_cell(cell, scroll.lines());
                    return true;
                }
                false
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if !self.dragging {
                    return false;
                }
                self.dragging = false;
                self.stop_auto_scroll();
                if let Some(cell) = self
                    .screen_to_content(col, row, scroll, transcript)
                    .or_else(|| self.clamp_to_transcript_edge(col, row, scroll, transcript))
                {
                    // Keep word/line expansion on release — raw click cell would
                    // collapse a double-click `apple` selection to `appl`.
                    self.apply_focus_cell(cell, scroll.lines());
                }
                if self.copy_on_release {
                    self.last_event_copied = self.maybe_copy(scroll, sink);
                }
                // Empty selection (click without drag) clears.
                if !self.has_selection() {
                    self.clear();
                }
                true
            }
            MouseEventKind::ScrollUp => {
                if !transcript.contains(col, row) {
                    return false;
                }
                // Wheel notch (edge-drag still uses [`ScrollView::motion_step`]).
                scroll.scroll_by(-ScrollView::wheel_notch());
                true
            }
            MouseEventKind::ScrollDown => {
                if !transcript.contains(col, row) {
                    return false;
                }
                scroll.scroll_by(ScrollView::wheel_notch());
                true
            }
            _ => false,
        }
    }

    /// Edge auto-scroll tick (call from idle / timer). Returns true if scrolled.
    pub fn tick_autoscroll(&mut self, scroll: &mut ScrollView, transcript: ScreenRect) -> bool {
        self.tick_autoscroll_at(scroll, transcript, std::time::Instant::now())
    }

    fn tick_autoscroll_at(
        &mut self,
        scroll: &mut ScrollView,
        transcript: ScreenRect,
        now: std::time::Instant,
    ) -> bool {
        if !self.dragging || self.auto_scroll_dir == 0 {
            return false;
        }
        // Start precise, then accelerate a sustained edge hold symmetrically.
        let held = self
            .auto_scroll_started_at
            .map(|started| now.saturating_duration_since(started))
            .unwrap_or_default();
        let multiplier = if held >= EDGE_AUTOSCROLL_FAST_AFTER {
            EDGE_AUTOSCROLL_FAST_MULTIPLIER
        } else if held >= EDGE_AUTOSCROLL_MEDIUM_AFTER {
            EDGE_AUTOSCROLL_MEDIUM_MULTIPLIER
        } else {
            1
        };
        let step = ScrollView::motion_step(scroll.viewport_height()) * multiplier;
        let delta = self.auto_scroll_dir as isize * step;
        if !scroll.scroll_by(delta) {
            self.stop_auto_scroll();
            return false;
        }
        if let Some((col, row)) = self.last_pointer
            && let Some(cell) = self
                .screen_to_content(col, row, scroll, transcript)
                .or_else(|| self.clamp_to_transcript_edge(col, row, scroll, transcript))
        {
            self.apply_focus_cell(cell, scroll.lines());
        }
        true
    }

    /// Extract selected plain text from content lines.
    pub fn selected_text(&self, lines: &[String]) -> Option<String> {
        let (start, end) = self.bounds()?;
        if start == end {
            return None;
        }
        Some(extract_range(lines, start, end))
    }

    /// Apply reverse-video highlight to visible lines (content coords via scroll_top).
    pub fn apply_highlight(&self, visible: &mut [String], scroll_top: usize) {
        let Some((start, end)) = self.bounds() else {
            return;
        };
        for (i, line) in visible.iter_mut().enumerate() {
            let content_row = scroll_top + i;
            if content_row < start.row || content_row > end.row {
                continue;
            }
            let from = if content_row == start.row {
                start.col
            } else {
                0
            };
            let to = if content_row == end.row {
                end.col
            } else {
                visible_len(line)
            };
            if from < to {
                *line = invert_columns(line, from, to);
            }
        }
    }

    /// Copy non-empty selection to `sink`. Returns true when text was issued.
    fn maybe_copy(&self, scroll: &ScrollView, sink: &mut dyn ClipboardSink) -> bool {
        if let Some(text) = self.selected_text(scroll.lines())
            && !text.is_empty()
        {
            sink.copy_text(&text);
            return true;
        }
        false
    }

    /// Pointer in dock rectangle.
    ///
    /// - Idle / completed selection: ignore motion; Down clears transcript
    ///   selection and returns false so Editor can take the press (ptim13).
    /// - Active drag: clamp to transcript bottom edge, keep selection, edge
    ///   auto-scroll downward; Up finishes copy like a release at the edge.
    fn handle_dock_mouse(
        &mut self,
        event: &MouseEvent,
        scroll: &mut ScrollView,
        transcript: ScreenRect,
        sink: &mut dyn ClipboardSink,
    ) -> bool {
        let (col, row) = (event.column, event.row);
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Do not start transcript selection from dock text (ptim06).
                // Clear any prior transcript selection, but return false so the
                // event falls through to Editor independent selection (ptim13).
                if self.dragging || self.has_selection() {
                    self.clear();
                }
                false
            }
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
                if !self.dragging {
                    // Completed selection: moving over dock MUST NOT clear it.
                    return false;
                }
                self.extend_drag_clamped(col, row, scroll, transcript)
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if !self.dragging {
                    return false;
                }
                self.extend_drag_clamped(col, row, scroll, transcript);
                self.dragging = false;
                self.stop_auto_scroll();
                if self.copy_on_release {
                    self.last_event_copied = self.maybe_copy(scroll, sink);
                }
                if !self.has_selection() {
                    self.clear();
                }
                true
            }
            // Wheel over dock: do not scroll transcript here (ApplicationOwned may route later).
            _ => false,
        }
    }

    /// Continue an in-progress drag as if the pointer were on the transcript
    /// edge nearest to `(col, row)` (typically the bottom when in the dock).
    fn extend_drag_clamped(
        &mut self,
        col: u16,
        row: u16,
        scroll: &ScrollView,
        transcript: ScreenRect,
    ) -> bool {
        // Treat dock as past the bottom edge → downward autoscroll while held.
        let edge_row = transcript
            .row
            .saturating_add(transcript.height.saturating_sub(1));
        let clamp_row = if row > edge_row { edge_row } else { row };
        self.last_pointer = Some((col, clamp_row));
        self.update_edge_dir(if row > edge_row { edge_row } else { row }, transcript);
        // If pointer is in dock (below transcript), force bottom-edge scroll.
        if row > edge_row {
            self.set_auto_scroll_dir(1);
        }
        let Some(cell) = self
            .screen_to_content(col, clamp_row, scroll, transcript)
            .or_else(|| self.clamp_to_transcript_edge(col, row, scroll, transcript))
        else {
            return false;
        };
        self.apply_focus_cell(cell, scroll.lines());
        true
    }

    fn update_edge_dir(&mut self, row: u16, transcript: ScreenRect) {
        let top = transcript.row;
        let bottom = transcript
            .row
            .saturating_add(transcript.height.saturating_sub(1));
        let next = if row <= top {
            -1
        } else if row >= bottom {
            1
        } else {
            0
        };
        self.set_auto_scroll_dir(next);
    }

    fn set_auto_scroll_dir(&mut self, next: i8) {
        if next == self.auto_scroll_dir {
            return;
        }
        self.auto_scroll_dir = next;
        self.auto_scroll_started_at = (next != 0).then(std::time::Instant::now);
    }

    fn stop_auto_scroll(&mut self) {
        self.auto_scroll_dir = 0;
        self.auto_scroll_started_at = None;
    }

    fn update_click_count(&mut self, col: u16, row: u16) {
        let now = std::time::Instant::now();
        let same = self.last_click_cell == Some((col, row));
        let quick = self
            .last_click_at
            .is_some_and(|t| now.duration_since(t).as_millis() < 400);
        if same && quick {
            self.click_count = self.click_count.saturating_add(1).min(3);
        } else {
            self.click_count = 1;
        }
        self.last_click_at = Some(now);
        self.last_click_cell = Some((col, row));
    }

    /// Update focus (and re-expand anchor/focus for word/line granularity).
    fn apply_focus_cell(&mut self, cell: CellPoint, lines: &[String]) {
        let (a, f) = if self.granularity == SelectionGranularity::Character {
            (self.anchor.unwrap_or(cell), cell)
        } else {
            let base = self.anchor.unwrap_or(cell);
            expand_range(base, cell, self.granularity, lines)
        };
        self.anchor = Some(a);
        self.focus = Some(f);
    }

    fn screen_to_content(
        &self,
        col: u16,
        row: u16,
        scroll: &ScrollView,
        transcript: ScreenRect,
    ) -> Option<CellPoint> {
        if !transcript.contains(col, row) {
            return None;
        }
        let screen_row = (row - transcript.row) as usize;
        let content_row = scroll.content_row_at_screen(screen_row)?;
        let col_in = (col - transcript.col) as usize;
        let max_col = scroll
            .lines()
            .get(content_row)
            .map(|l| visible_len(l))
            .unwrap_or(0);
        Some(CellPoint::new(content_row, col_in.min(max_col)))
    }

    fn clamp_to_transcript_edge(
        &self,
        _col: u16,
        row: u16,
        scroll: &ScrollView,
        transcript: ScreenRect,
    ) -> Option<CellPoint> {
        if scroll.content_len() == 0 {
            return None;
        }
        let screen_row = if row < transcript.row {
            0
        } else if row >= transcript.row.saturating_add(transcript.height) {
            transcript.height.saturating_sub(1) as usize
        } else {
            (row - transcript.row) as usize
        };
        let content_row = scroll.content_row_at_screen(screen_row).or_else(|| {
            if scroll.scroll_top() + screen_row >= scroll.content_len() {
                Some(scroll.content_len().saturating_sub(1))
            } else {
                None
            }
        })?;
        let max_col = scroll
            .lines()
            .get(content_row)
            .map(|l| visible_len(l))
            .unwrap_or(0);
        Some(CellPoint::new(content_row, max_col))
    }
}

fn ordered(a: CellPoint, b: CellPoint) -> (CellPoint, CellPoint) {
    if (a.row, a.col) <= (b.row, b.col) {
        (a, b)
    } else {
        (b, a)
    }
}

/// Display width (terminal cells) of a line's visible content.
fn visible_len(line: &str) -> usize {
    strip_ansi(line).chars().map(char_cols).sum()
}

/// Terminal columns for one scalar, matching `utils::grapheme_width`
/// semantics (tab = 3, control/zero-width = 0).
fn char_cols(c: char) -> usize {
    if c == '\t' { 3 } else { c.width().unwrap_or(0) }
}

/// Strip ANSI escapes (CSI any final, OSC/APC BEL or ST terminated) leaving
/// only visible text. Shares [`ansi_escape_len`] with the engine width model.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(len) = ansi_escape_len(bytes, i) {
            i += len;
            continue;
        }
        let c = line[i..].chars().next().expect("char boundary");
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Map the **start** boundary display column to a char index.
///
/// A column falling anywhere inside a wide char snaps onto that char
/// (dragging from either half of `你` anchors at `你`).
fn col_to_idx_from(plain: &str, col: usize) -> usize {
    let mut start = 0usize;
    for (idx, c) in plain.chars().enumerate() {
        let cw = char_cols(c);
        if col < start + cw {
            return idx;
        }
        start += cw;
        if cw == 0 && start > col {
            return idx;
        }
    }
    plain.chars().count()
}

/// Map the **end** boundary display column to an exclusive char index.
///
/// A column falling inside a wide char resolves past it, so a drag ending on
/// either half of `你` includes `你`.
fn col_to_idx_to(plain: &str, col: usize) -> usize {
    let mut w = 0usize;
    for (idx, c) in plain.chars().enumerate() {
        if w >= col {
            return idx;
        }
        w += char_cols(c);
    }
    plain.chars().count()
}

/// Map a char index in the stripped projection back to a display column.
fn idx_to_col(plain: &str, idx: usize) -> usize {
    plain.chars().take(idx).map(char_cols).sum()
}

fn extract_range(lines: &[String], start: CellPoint, end: CellPoint) -> String {
    // Drop fit/pad trailing spaces so copy char-count matches visible text
    // (demo `fit` pads every transcript line to terminal width).
    fn trim_copy_piece(s: String) -> String {
        s.trim_end_matches([' ', '\t']).to_string()
    }

    /// Slice one row's plain projection by display-column bounds.
    fn slice_cols(plain: &str, from_col: usize, to_col: Option<usize>) -> String {
        let chars: Vec<char> = plain.chars().collect();
        let s = col_to_idx_from(plain, from_col).min(chars.len());
        let e = match to_col {
            Some(c) => col_to_idx_to(plain, c).max(s).min(chars.len()),
            None => chars.len(),
        };
        chars[s..e].iter().collect()
    }

    if start.row == end.row {
        let plain = strip_ansi(lines.get(start.row).map(|s| s.as_str()).unwrap_or(""));
        return trim_copy_piece(slice_cols(&plain, start.col, Some(end.col)));
    }
    let mut parts: Vec<String> = Vec::new();
    for row in start.row..=end.row {
        let plain = strip_ansi(lines.get(row).map(|s| s.as_str()).unwrap_or(""));
        let piece: String = if row == start.row {
            slice_cols(&plain, start.col, None)
        } else if row == end.row {
            slice_cols(&plain, 0, Some(end.col))
        } else {
            plain
        };
        let piece = trim_copy_piece(piece);
        // Fully-padded trailing rows become empty after trim — drop them.
        if piece.is_empty() && row == end.row && !parts.is_empty() {
            continue;
        }
        parts.push(piece);
    }
    parts.join("\n")
}

fn expand_for_granularity(
    cell: CellPoint,
    g: SelectionGranularity,
    lines: &[String],
) -> (CellPoint, CellPoint) {
    expand_range(cell, cell, g, lines)
}

fn expand_range(
    a: CellPoint,
    b: CellPoint,
    g: SelectionGranularity,
    lines: &[String],
) -> (CellPoint, CellPoint) {
    let (mut start, mut end) = ordered(a, b);
    match g {
        SelectionGranularity::Character => {}
        SelectionGranularity::Word => {
            let plain = strip_ansi(lines.get(start.row).map(|s| s.as_str()).unwrap_or(""));
            let chars: Vec<char> = plain.chars().collect();
            // Bounds arrive as display columns; expand in char space, report back columns.
            let mut l = col_to_idx_from(&plain, start.col).min(chars.len());
            let mut r = col_to_idx_to(&plain, end.col).max(l).min(chars.len());
            while l > 0
                && chars
                    .get(l - 1)
                    .is_some_and(|c| c.is_alphanumeric() || *c == '_')
            {
                l -= 1;
            }
            while r < chars.len() && (chars[r].is_alphanumeric() || chars[r] == '_') {
                r += 1;
            }
            start.col = idx_to_col(&plain, l);
            end.col = idx_to_col(&plain, r);
        }
        SelectionGranularity::Line => {
            start.col = 0;
            end.col = lines.get(end.row).map(|l| visible_len(l)).unwrap_or(0);
        }
    }
    (start, end)
}

/// Apply reverse-video to the display-column range `[from, to)` **in place**,
/// preserving the line's original SGR styling (syntax colors survive).
///
/// Strategy: splice `\x1b[7m` before the first visible char at/after `from`
/// and `\x1b[27m` before the first visible char at/after `to`; rewrite every
/// SGR sequence inside the span by appending `;7` (`\x1b[0m` → `\x1b[0;7m`,
/// `\x1b[31m` → `\x1b[31;7m`) so inner resets cannot wipe the reversal
/// mid-span. Wide chars straddling a boundary are included whole.
fn invert_columns(line: &str, from: usize, to: usize) -> String {
    if to <= from || line.is_empty() {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len() + 16);
    let bytes = line.as_bytes();
    let mut col = 0usize;
    let mut started = false;
    let mut finished = false;
    let mut i = 0usize;
    while i < line.len() {
        if let Some(len) = ansi_escape_len(bytes, i) {
            let seq = &line[i..i + len];
            if started && !finished && is_sgr(seq) {
                // Keep the sequence's own parameters, force reverse back on.
                out.push_str(&seq[..seq.len() - 1]);
                out.push_str(";7m");
            } else {
                out.push_str(seq);
            }
            i += len;
            continue;
        }
        let c = line[i..].chars().next().expect("char boundary");
        let cw = char_cols(c);
        if started && !finished && col >= to {
            out.push_str("\x1b[27m");
            finished = true;
        }
        if !started && col < to && col + cw.max(1) > from {
            out.push_str("\x1b[7m");
            started = true;
        }
        out.push(c);
        col += cw;
        i += c.len_utf8();
    }
    if started && !finished {
        out.push_str("\x1b[27m");
    }
    out
}

/// True for CSI SGR sequences (`ESC[…m`).
fn is_sgr(seq: &str) -> bool {
    seq.starts_with("\x1b[") && seq.ends_with('m')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn layout() -> (ScreenRect, ScreenRect) {
        let transcript = ScreenRect {
            row: 0,
            col: 0,
            height: 4,
            width: 20,
        };
        let dock = ScreenRect {
            row: 4,
            col: 0,
            height: 2,
            width: 20,
        };
        (transcript, dock)
    }

    #[test]
    fn drag_select_and_copy_on_release() {
        let mut scroll = ScrollView::new(4);
        scroll.set_lines(vec!["hello world".into(), "second".into()]);
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();

        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 5, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), 5, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert_eq!(sink.copies, vec!["hello".to_string()]);
        assert!(sel.last_event_copied());
    }

    #[test]
    fn double_click_selects_whole_word_on_release() {
        // Regression: Up used to set focus to the raw click cell and collapse
        // a word selection (`apple` → `appl` when clicking on `l`).
        let mut scroll = ScrollView::new(4);
        scroll.set_lines(vec!["say apple pie".into()]);
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();
        // "say apple pie" — 'l' of apple is column 7.
        let col = 7u16;
        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), col, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), col, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        sink.copies.clear();
        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), col, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert_eq!(sel.selected_text(scroll.lines()).as_deref(), Some("apple"));
        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), col, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert_eq!(sink.copies, vec!["apple".to_string()]);
        assert_eq!(sel.selected_text(scroll.lines()).as_deref(), Some("apple"));
    }

    #[test]
    fn empty_click_does_not_copy() {
        let mut scroll = ScrollView::new(4);
        scroll.set_lines(vec!["hello".into()]);
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();
        sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 1, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), 1, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        assert!(sink.copies.is_empty());
        assert!(!sel.has_selection());
        assert!(!sel.last_event_copied());
    }

    #[test]
    fn dock_excludes_selection_start() {
        let mut scroll = ScrollView::new(4);
        scroll.set_lines(vec!["hello".into()]);
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();
        assert!(!sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 4),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert!(!sel.is_dragging());
    }

    #[test]
    fn hit_priority_runs_only_on_left_down_not_drag() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let calls = std::sync::Arc::new(AtomicUsize::new(0));
        let calls_hook = calls.clone();
        let mut scroll = ScrollView::new(4);
        scroll.set_lines(vec!["hello world".into()]);
        let mut sel = SelectionController::new();
        sel.set_hit_priority(Some(Box::new(move |_col, _row| {
            calls_hook.fetch_add(1, Ordering::SeqCst);
            // Miss: allow selection to start so Drag path is exercised.
            false
        })));
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();

        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(sel.is_dragging());

        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 5, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "Drag must not re-invoke hit_priority (fold latch)"
        );

        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), 5, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "Up must not invoke hit_priority"
        );
    }

    #[test]
    fn dock_down_clears_transcript_selection_and_falls_through() {
        let mut scroll = ScrollView::new(4);
        scroll.set_lines(vec!["hello".into()]);
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();
        sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        sel.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 4, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        assert!(sel.has_selection() || sel.is_dragging());
        // Dock Down: clear, but return false so Editor can receive the press (ptim13).
        assert!(!sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 4),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert!(!sel.has_selection());
        assert!(!sel.is_dragging());
    }

    #[test]
    fn drag_into_dock_keeps_selection_and_clamps() {
        let mut scroll = ScrollView::new(3);
        scroll.set_lines(vec![
            "one".into(),
            "two".into(),
            "three".into(),
            "four".into(),
        ]);
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let tr = ScreenRect {
            row: 0,
            col: 0,
            height: 3,
            width: 20,
        };
        let dock = ScreenRect {
            row: 3,
            col: 0,
            height: 2,
            width: 20,
        };
        sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        assert!(sel.is_dragging());
        // Drag into dock — must NOT clear; focus clamps to transcript edge.
        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 2, 4),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        ));
        assert!(sel.is_dragging());
        assert!(sel.has_selection() || sel.is_dragging());
        // Completed selection: moving over dock must not clear.
        sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), 2, 4),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        assert!(!sel.is_dragging());
        let had = sel.has_selection() || !sink.copies.is_empty();
        assert!(had, "release in dock should keep/copy selection");
        let still = sel.has_selection();
        assert!(!sel.handle_mouse(
            &mouse(MouseEventKind::Moved, 1, 4),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        ));
        assert_eq!(sel.has_selection(), still);
    }

    #[test]
    fn edge_autoscroll_extends_selection() {
        let mut scroll = ScrollView::new(2);
        scroll.set_lines((0..20).map(|i| format!("L{i}")).collect());
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let tr = ScreenRect {
            row: 0,
            col: 0,
            height: 2,
            width: 10,
        };
        let dock = ScreenRect {
            row: 10,
            col: 0,
            height: 1,
            width: 10,
        };
        sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        // Drag to bottom edge → auto_scroll_dir = +1
        sel.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 0, 1),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        assert_eq!(sel.auto_scroll_dir, 1);
        let started = sel
            .auto_scroll_started_at
            .expect("edge hold start timestamp");
        let top_before = scroll.scroll_top();
        assert!(sel.tick_autoscroll_at(&mut scroll, tr, started));
        assert_eq!(
            scroll.scroll_top() - top_before,
            1,
            "a fresh edge hold must advance exactly one row"
        );

        let top_before = scroll.scroll_top();
        assert!(sel.tick_autoscroll_at(&mut scroll, tr, started + EDGE_AUTOSCROLL_MEDIUM_AFTER));
        assert_eq!(
            scroll.scroll_top() - top_before,
            2,
            "a medium edge hold should advance two rows"
        );

        let top_before = scroll.scroll_top();
        assert!(sel.tick_autoscroll_at(&mut scroll, tr, started + EDGE_AUTOSCROLL_FAST_AFTER));
        assert_eq!(
            scroll.scroll_top() - top_before,
            4,
            "a sustained edge hold should advance four rows"
        );
    }

    #[test]
    fn copy_on_release_can_disable() {
        let mut scroll = ScrollView::new(4);
        scroll.set_lines(vec!["abcd".into()]);
        let mut sel = SelectionController::new();
        sel.copy_on_release = false;
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();
        sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), 3, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        assert!(sink.copies.is_empty());
        assert!(sel.has_selection());
        assert!(!sel.last_event_copied());
    }

    #[test]
    fn copy_trims_fit_padding_spaces() {
        let mut scroll = ScrollView::new(4);
        // Simulate demo `fit`: content + trailing pad to transcript width (20).
        scroll.set_lines(vec![
            format!("hello{}", " ".repeat(15)),
            format!("world{}", " ".repeat(15)),
        ]);
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();
        sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        sel.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 19, 1),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), 19, 1),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        assert_eq!(sink.copies, vec!["hello\nworld".to_string()]);
    }

    #[test]
    fn format_osc52_shape() {
        let seq = format_osc52("hi").expect("fits");
        assert!(seq.starts_with("\x1b]52;c;"));
        assert!(seq.ends_with('\u{7}'));
    }

    #[test]
    fn wheel_scroll_uses_fine_notch() {
        let mut scroll = ScrollView::new(40);
        scroll.set_lines((0..200).map(|i| format!("L{i}")).collect());
        scroll.scroll_to_end();
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();
        let top0 = scroll.scroll_top();
        sel.handle_mouse(
            &mouse(MouseEventKind::ScrollUp, 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        assert_eq!(
            top0 - scroll.scroll_top(),
            ScrollView::wheel_notch() as usize
        );
    }

    // ── display-column coordinate model regressions ────────────────────────

    /// What `n` display columns of visible text actually cover.
    fn cols_prefix(plain: &str, n: usize) -> String {
        let mut w = 0usize;
        plain
            .chars()
            .take_while(|c| {
                let cw = char_cols(*c);
                let fits = w + cw <= n;
                if fits {
                    w += cw;
                }
                fits
            })
            .collect()
    }

    #[test]
    fn column_to_char_index_snaps_inside_wide_char() {
        // cells: a@0 b@1 你@2-3 好@4-5 c@6 d@7
        let plain = "ab你好cd";
        assert_eq!(col_to_idx_from(plain, 0), 0);
        assert_eq!(col_to_idx_from(plain, 1), 1);
        assert_eq!(col_to_idx_from(plain, 2), 2, "left half of 你");
        assert_eq!(col_to_idx_from(plain, 3), 2, "right half still anchors 你");
        assert_eq!(col_to_idx_from(plain, 4), 3);
        assert_eq!(col_to_idx_from(plain, 99), plain.chars().count());
        assert_eq!(col_to_idx_to(plain, 2), 2, "end at col 2 excludes 你");
        assert_eq!(col_to_idx_to(plain, 3), 3, "end inside 你 includes it");
        assert_eq!(idx_to_col(plain, 2), 2);
        assert_eq!(idx_to_col(plain, 4), 6);
    }

    #[test]
    fn highlighted_cjk_drag_copies_exactly_visible_columns() {
        // Streamed rust-ish line with a CJK comment in fake syntect SGRs — the
        // historical bug: char-index selection drifted by one cell per wide char.
        let line = "\x1b[38;2;166;227;161mlet\x1b[38;2;205;214;244m total = \
                    \x1b[38;2;137;220;235m42\x1b[38;2;205;214;244m; // 累计订单金额\
                    \x1b[38;2;137;180;255m（含税）\x1b[0m";
        let plain = strip_ansi(line);
        // Select up through `累计`: 19 ASCII cols + 累计 = 23 columns.
        let expected = cols_prefix(&plain, 23);
        assert_eq!(expected, "let total = 42; // 累计");

        let mut scroll = ScrollView::new(4);
        scroll.set_lines(vec![line.to_string()]);
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        // Wide transcript rect — the shared `layout()` is only 20 cols wide
        // and would clamp the drag to its right edge.
        let tr = ScreenRect {
            row: 0,
            col: 0,
            height: 4,
            width: 60,
        };
        let dock = ScreenRect {
            row: 4,
            col: 0,
            height: 2,
            width: 60,
        };

        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 23, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert!(sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), 23, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink
        ));
        assert_eq!(sink.copies, vec![expected], "copy must match visible span");
    }

    #[test]
    fn reverse_video_preserves_syntax_and_survives_inner_resets() {
        let line = "A\x1b[31mBC\x1b[0mDE";
        let inverted = invert_columns(line, 1, 4);
        assert_eq!(
            inverted, "A\x1b[31m\x1b[7mBC\x1b[0;7mD\x1b[27mE",
            "original SGR kept; inner reset rewritten to keep reversal on"
        );
        assert_eq!(strip_ansi(&inverted), strip_ansi(line));
    }

    #[test]
    fn reverse_video_snaps_wide_char_boundaries() {
        // 你 spans columns 2-3; from=3 (right half) must include all of 你,
        // and the span closes before `d` at column 4.
        let inverted = invert_columns("ab你de", 3, 4);
        assert_eq!(inverted, "ab\x1b[7m你\x1b[27mde");
    }

    #[test]
    fn st_terminated_osc_does_not_inflate_length() {
        let line = crate::hyperlink("text", "https://example.test");
        assert_eq!(visible_len(&line), 4);

        let mut scroll = ScrollView::new(4);
        scroll.set_lines(vec![line]);
        let mut sel = SelectionController::new();
        let mut sink = RecordingClipboardSink::default();
        let (tr, dock) = layout();
        sel.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        sel.handle_mouse(
            &mouse(MouseEventKind::Drag(MouseButton::Left), 4, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        sel.handle_mouse(
            &mouse(MouseEventKind::Up(MouseButton::Left), 4, 0),
            &mut scroll,
            tr,
            dock,
            &mut sink,
        );
        assert_eq!(sink.copies, vec!["text".to_string()]);
    }
}
