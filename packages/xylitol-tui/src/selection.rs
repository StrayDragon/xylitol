//! Mode B application text selection (c2070).
//!
//! Drag-select over transcript content coordinates, edge auto-scroll via
//! [`ScrollView`], optional copy-on-release, and dock/input exclusion.

use crate::scroll_view::ScrollView;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

/// Content-space cell (row = content line index, col = display column).
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

/// Optional click consumer (e.g. future fold hit). Return true to swallow the press.
pub type HitPriorityFn = Box<dyn FnMut(u16, u16) -> bool + Send>;

/// Mode B selection controller over a [`ScrollView`] transcript pane.
pub struct SelectionController {
    anchor: Option<CellPoint>,
    focus: Option<CellPoint>,
    dragging: bool,
    granularity: SelectionGranularity,
    /// Default **on** (ptim05).
    pub copy_on_release: bool,
    auto_scroll_dir: i8,
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

    pub fn clear(&mut self) {
        self.anchor = None;
        self.focus = None;
        self.dragging = false;
        self.auto_scroll_dir = 0;
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
                    let (a, f) = if self.granularity == SelectionGranularity::Character {
                        (self.anchor.unwrap_or(cell), cell)
                    } else {
                        let base = self.anchor.unwrap_or(cell);
                        expand_range(base, cell, self.granularity, scroll.lines())
                    };
                    self.anchor = Some(a);
                    self.focus = Some(f);
                    return true;
                }
                false
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if !self.dragging {
                    return false;
                }
                self.dragging = false;
                self.auto_scroll_dir = 0;
                if let Some(cell) = self
                    .screen_to_content(col, row, scroll, transcript)
                    .or_else(|| self.clamp_to_transcript_edge(col, row, scroll, transcript))
                {
                    self.focus = Some(cell);
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
                // Multi-line steps match typical terminal wheel feel.
                scroll.scroll_by(-3);
                true
            }
            MouseEventKind::ScrollDown => {
                if !transcript.contains(col, row) {
                    return false;
                }
                scroll.scroll_by(3);
                true
            }
            _ => false,
        }
    }

    /// Edge auto-scroll tick (call from idle / timer). Returns true if scrolled.
    pub fn tick_autoscroll(&mut self, scroll: &mut ScrollView, transcript: ScreenRect) -> bool {
        if !self.dragging || self.auto_scroll_dir == 0 {
            return false;
        }
        let delta = self.auto_scroll_dir as isize;
        if !scroll.scroll_by(delta) {
            self.auto_scroll_dir = 0;
            return false;
        }
        if let Some((col, row)) = self.last_pointer
            && let Some(cell) = self
                .screen_to_content(col, row, scroll, transcript)
                .or_else(|| self.clamp_to_transcript_edge(col, row, scroll, transcript))
        {
            self.focus = Some(cell);
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
                self.auto_scroll_dir = 0;
                if self.copy_on_release {
                    self.last_event_copied = self.maybe_copy(scroll, sink);
                }
                if !self.has_selection() {
                    self.clear();
                }
                true
            }
            // Wheel over dock: do not scroll transcript here (Mode B may route later).
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
            self.auto_scroll_dir = 1;
        }
        let Some(cell) = self
            .screen_to_content(col, clamp_row, scroll, transcript)
            .or_else(|| self.clamp_to_transcript_edge(col, row, scroll, transcript))
        else {
            return false;
        };
        let (a, f) = if self.granularity == SelectionGranularity::Character {
            (self.anchor.unwrap_or(cell), cell)
        } else {
            let base = self.anchor.unwrap_or(cell);
            expand_range(base, cell, self.granularity, scroll.lines())
        };
        self.anchor = Some(a);
        self.focus = Some(f);
        true
    }

    fn update_edge_dir(&mut self, row: u16, transcript: ScreenRect) {
        let top = transcript.row;
        let bottom = transcript
            .row
            .saturating_add(transcript.height.saturating_sub(1));
        self.auto_scroll_dir = if row <= top {
            -1
        } else if row >= bottom {
            1
        } else {
            0
        };
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

fn visible_len(line: &str) -> usize {
    // Selection columns are byte-index approximations for ASCII-heavy transcripts;
    // ANSI is stripped roughly by ignoring ESC sequences for length.
    strip_ansi_approx(line).chars().count()
}

fn strip_ansi_approx(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == '\u{7}' {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn extract_range(lines: &[String], start: CellPoint, end: CellPoint) -> String {
    // Drop fit/pad trailing spaces so copy char-count matches visible text
    // (demo `fit` pads every transcript line to terminal width).
    fn trim_copy_piece(s: String) -> String {
        s.trim_end_matches([' ', '\t']).to_string()
    }

    if start.row == end.row {
        let plain = strip_ansi_approx(lines.get(start.row).map(|s| s.as_str()).unwrap_or(""));
        return trim_copy_piece(
            plain
                .chars()
                .skip(start.col)
                .take(end.col.saturating_sub(start.col))
                .collect(),
        );
    }
    let mut parts: Vec<String> = Vec::new();
    for row in start.row..=end.row {
        let plain = strip_ansi_approx(lines.get(row).map(|s| s.as_str()).unwrap_or(""));
        let piece: String = if row == start.row {
            plain.chars().skip(start.col).collect()
        } else if row == end.row {
            plain.chars().take(end.col).collect()
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
            let plain = strip_ansi_approx(lines.get(start.row).map(|s| s.as_str()).unwrap_or(""));
            let chars: Vec<char> = plain.chars().collect();
            let mut l = start.col.min(chars.len());
            let mut r = end.col.min(chars.len());
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
            start.col = l;
            end.col = r;
        }
        SelectionGranularity::Line => {
            start.col = 0;
            end.col = lines.get(end.row).map(|l| visible_len(l)).unwrap_or(0);
        }
    }
    (start, end)
}

fn invert_columns(line: &str, from: usize, to: usize) -> String {
    // Apply reverse SGR around the approximate char range on the plain projection,
    // then splice back — for tests/ASCII this is enough; full ANSI-aware invert
    // can be refined later without changing the selection model.
    let plain = strip_ansi_approx(line);
    let chars: Vec<char> = plain.chars().collect();
    if from >= chars.len() || from >= to {
        return line.to_string();
    }
    let to = to.min(chars.len());
    let mut out = String::new();
    out.extend(chars[..from].iter());
    out.push_str("\x1b[7m");
    out.extend(chars[from..to].iter());
    out.push_str("\x1b[27m");
    out.extend(chars[to..].iter());
    out
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
        scroll.set_lines(vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
            "e".into(),
        ]);
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
        assert!(sel.tick_autoscroll(&mut scroll, tr));
        assert!(scroll.scroll_top() > 0);
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
}
