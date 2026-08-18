use super::types::LayoutLine;
use crate::selection::{CellPoint, ClipboardSink, SelectionGranularity, format_osc52};
use crate::utils::visible_width;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use unicode_segmentation::UnicodeSegmentation;

/// Independent multi-line selection inside the Editor buffer (ptim13).
///
/// Coordinates are **logical buffer** space (`row` = line index, `col` = byte
/// offset), not screen cells and not transcript [`crate::SelectionController`].
#[derive(Debug, Clone)]
pub(super) struct EditorSelection {
    pub(super) anchor: Option<CellPoint>,
    pub(super) focus: Option<CellPoint>,
    pub(super) dragging: bool,
    pub(super) granularity: SelectionGranularity,
    pub(super) click_count: u8,
    pub(super) last_click_at: Option<std::time::Instant>,
    /// Editor-local `(col, row)` of the last Down (for double/triple click).
    pub(super) last_click_cell: Option<(usize, usize)>,
}

impl Default for EditorSelection {
    fn default() -> Self {
        Self {
            anchor: None,
            focus: None,
            dragging: false,
            granularity: SelectionGranularity::Character,
            click_count: 0,
            last_click_at: None,
            last_click_cell: None,
        }
    }
}

impl EditorSelection {
    pub(super) fn clear(&mut self) {
        self.anchor = None;
        self.focus = None;
        self.dragging = false;
        self.granularity = SelectionGranularity::Character;
        // Keep click_count / last_click_* so a second Down can still double-click.
    }

    pub(super) fn has_selection(&self) -> bool {
        match (self.anchor, self.focus) {
            (Some(a), Some(f)) => a != f,
            _ => false,
        }
    }

    pub(super) fn bounds(&self) -> Option<(CellPoint, CellPoint)> {
        let a = self.anchor?;
        let f = self.focus?;
        Some(ordered_cell(a, f))
    }

    pub(super) fn update_click_count(&mut self, col: usize, row: usize) {
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
}

fn ordered_cell(a: CellPoint, b: CellPoint) -> (CellPoint, CellPoint) {
    if (a.row, a.col) <= (b.row, b.col) {
        (a, b)
    } else {
        (b, a)
    }
}

/// Expand selection endpoints for editor buffer coords (byte offsets).
fn expand_editor_range(
    a: CellPoint,
    b: CellPoint,
    g: SelectionGranularity,
    lines: &[String],
) -> (CellPoint, CellPoint) {
    let (mut start, mut end) = ordered_cell(a, b);
    match g {
        SelectionGranularity::Character => {}
        SelectionGranularity::Word => {
            if start.row == end.row {
                let line = lines.get(start.row).map(|s| s.as_str()).unwrap_or("");
                let (l, r) = word_byte_bounds_containing(line, start.col.min(end.col));
                start.col = l;
                end.col = r;
            } else {
                let start_line = lines.get(start.row).map(|s| s.as_str()).unwrap_or("");
                let end_line = lines.get(end.row).map(|s| s.as_str()).unwrap_or("");
                let (l, _) = word_byte_bounds_containing(start_line, start.col);
                let (_, r) = word_byte_bounds_containing(end_line, end.col);
                start.col = l;
                end.col = r;
            }
        }
        SelectionGranularity::Line => {
            start.col = 0;
            end.col = lines.get(end.row).map(|l| l.len()).unwrap_or(0);
        }
    }
    (start, end)
}

fn word_byte_bounds_containing(line: &str, byte_at: usize) -> (usize, usize) {
    if line.is_empty() {
        return (0, 0);
    }
    let mut byte_at = byte_at.min(line.len());
    while byte_at > 0 && !line.is_char_boundary(byte_at) {
        byte_at -= 1;
    }
    let indices: Vec<(usize, char)> = line.char_indices().collect();
    if indices.is_empty() {
        return (0, 0);
    }
    let mut i = indices.partition_point(|(off, _)| *off < byte_at);
    if i >= indices.len() {
        i = indices.len() - 1;
    } else if indices[i].0 > byte_at && i > 0 {
        i -= 1;
    } else if indices[i].0 > byte_at {
        // byte_at before first char
        i = 0;
    }
    let word_char = |c: char| c.is_alphanumeric() || c == '_';
    if !word_char(indices[i].1) {
        let off = indices[i].0;
        return (off, off + indices[i].1.len_utf8());
    }
    let mut l = i;
    let mut r = i + 1;
    while l > 0 && word_char(indices[l - 1].1) {
        l -= 1;
    }
    while r < indices.len() && word_char(indices[r].1) {
        r += 1;
    }
    let start = indices[l].0;
    let end = if r < indices.len() {
        indices[r].0
    } else {
        line.len()
    };
    (start, end)
}

impl super::Editor {
    // ── ApplicationOwned editor selection (ptim13) ─────────────────────────

    /// Handle a mouse event in **editor-local** coordinates: `(0,0)` is the
    /// top-left of the editor's last render (top border row). Hosts that receive
    /// absolute screen coords MUST remap into this space before dispatch.
    ///
    /// Returns whether presentation changed (caller should rerender).
    /// Double-click selects the word under the pointer; triple-click selects the
    /// logical line (same habit as transcript ApplicationOwned selection).
    pub fn handle_mouse_local(&mut self, event: &MouseEvent, sink: &mut dyn ClipboardSink) -> bool {
        let (col, row) = (event.column as usize, event.row as usize);
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let Some(cell) = self.local_to_buffer(col, row) else {
                    self.selection.clear();
                    return false;
                };
                self.selection.update_click_count(col, row);
                self.selection.granularity = match self.selection.click_count {
                    2 => SelectionGranularity::Word,
                    3.. => SelectionGranularity::Line,
                    _ => SelectionGranularity::Character,
                };
                let (a, f) =
                    expand_editor_range(cell, cell, self.selection.granularity, &self.state.lines);
                self.selection.anchor = Some(a);
                self.selection.focus = Some(f);
                self.selection.dragging = true;
                true
            }
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
                if !self.selection.dragging {
                    return false;
                }
                let cell = self
                    .local_to_buffer(col, row)
                    .or_else(|| self.clamp_local_to_buffer(col, row));
                if let Some(cell) = cell {
                    self.apply_editor_focus_cell(cell);
                    return true;
                }
                false
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if !self.selection.dragging {
                    return false;
                }
                self.selection.dragging = false;
                if let Some(cell) = self
                    .local_to_buffer(col, row)
                    .or_else(|| self.clamp_local_to_buffer(col, row))
                {
                    self.apply_editor_focus_cell(cell);
                }
                // Character click (no span): clear ghost highlight; do not copy.
                // Word/line multi-click keeps the expanded range and may copy.
                if !self.selection.has_selection() {
                    self.selection.clear();
                    return true;
                }
                if self.copy_on_release {
                    self.maybe_copy_selection(sink);
                }
                true
            }
            _ => false,
        }
    }

    pub(super) fn apply_editor_focus_cell(&mut self, cell: CellPoint) {
        let (a, f) = if self.selection.granularity == SelectionGranularity::Character {
            (self.selection.anchor.unwrap_or(cell), cell)
        } else {
            let base = self.selection.anchor.unwrap_or(cell);
            expand_editor_range(base, cell, self.selection.granularity, &self.state.lines)
        };
        self.selection.anchor = Some(a);
        self.selection.focus = Some(f);
    }

    pub(super) fn maybe_copy_selection(&mut self, sink: &mut dyn ClipboardSink) {
        if let Some(text) = self.selected_text()
            && !text.is_empty()
        {
            sink.copy_text(&text);
            if let Some(seq) = format_osc52(&text) {
                self.pending_clipboard.push(seq);
            }
        }
    }

    /// Map editor-local `(col, row)` onto a logical buffer cell.
    /// Content rows are `1..=last_content_rows` (row 0 = top border).
    pub(super) fn local_to_buffer(&self, col: usize, row: usize) -> Option<CellPoint> {
        if self.last_content_rows == 0 || row == 0 || row > self.last_content_rows {
            return None;
        }
        let px = self
            .padding_x
            .min(self.last_paint_width.saturating_sub(1) / 2);
        if col < px {
            return None;
        }
        let layout = self.layout_text(self.last_width);
        let layout_idx = self.scroll_offset + (row - 1);
        let ll = layout.get(layout_idx)?;
        let content_col = col - px;
        let byte_in_chunk = col_to_byte_index(&ll.text, content_col);
        let byte_in_chunk = byte_in_chunk.min(ll.text.len());
        Some(CellPoint::new(
            ll.logical_line,
            ll.start_index + byte_in_chunk,
        ))
    }

    pub(super) fn clamp_local_to_buffer(&self, _col: usize, row: usize) -> Option<CellPoint> {
        if self.last_content_rows == 0 {
            return None;
        }
        let layout = self.layout_text(self.last_width);
        if layout.is_empty() {
            return None;
        }
        // Above top border → start of first visible content row.
        // Below last content row → end of last visible content row.
        let content_row = if row == 0 {
            0
        } else if row > self.last_content_rows {
            self.last_content_rows - 1
        } else {
            row - 1
        };
        let layout_idx = (self.scroll_offset + content_row).min(layout.len() - 1);
        let ll = &layout[layout_idx];
        let byte_in_chunk = if row == 0 { 0 } else { ll.text.len() };
        Some(CellPoint::new(
            ll.logical_line,
            ll.start_index + byte_in_chunk,
        ))
    }

    pub(super) fn apply_selection_highlight(&self, ll: &LayoutLine, display: &str) -> String {
        let Some((start, end)) = self.selection.bounds() else {
            return display.to_string();
        };
        if start == end {
            return display.to_string();
        }
        let line_start = CellPoint::new(ll.logical_line, ll.start_index);
        let line_end = CellPoint::new(ll.logical_line, ll.start_index + ll.text.len());
        // No overlap with this visual chunk.
        if (ll.logical_line < start.row || ll.logical_line > end.row)
            || (ll.logical_line == start.row && line_end.col <= start.col)
            || (ll.logical_line == end.row && line_start.col >= end.col)
        {
            return display.to_string();
        }
        let from = if ll.logical_line == start.row {
            start.col.saturating_sub(ll.start_index)
        } else {
            0
        };
        let to = if ll.logical_line == end.row {
            end.col.saturating_sub(ll.start_index).min(ll.text.len())
        } else {
            ll.text.len()
        };
        invert_byte_range(display, from, to)
    }
}

fn col_to_byte_index(text: &str, col: usize) -> usize {
    let mut w = 0usize;
    for (i, g) in UnicodeSegmentation::grapheme_indices(text, true) {
        let gw = visible_width(g);
        if w + gw > col {
            return i;
        }
        w += gw;
    }
    text.len()
}

fn invert_byte_range(s: &str, from: usize, to: usize) -> String {
    if from >= to || from >= s.len() {
        return s.to_string();
    }
    let mut from = from.min(s.len());
    let mut to = to.min(s.len());
    while from > 0 && !s.is_char_boundary(from) {
        from -= 1;
    }
    while to > 0 && !s.is_char_boundary(to) {
        to -= 1;
    }
    if from >= to {
        return s.to_string();
    }
    format!("{}\x1b[7m{}\x1b[27m{}", &s[..from], &s[from..to], &s[to..])
}

pub(super) fn extract_editor_range(lines: &[String], start: CellPoint, end: CellPoint) -> String {
    if start.row == end.row {
        let line = lines.get(start.row).map(|s| s.as_str()).unwrap_or("");
        let from = start.col.min(line.len());
        let to = end.col.min(line.len());
        if from > to || !line.is_char_boundary(from) || !line.is_char_boundary(to) {
            return String::new();
        }
        return line[from..to].to_string();
    }
    let mut out = String::new();
    for row in start.row..=end.row {
        let line = lines.get(row).map(|s| s.as_str()).unwrap_or("");
        let piece = if row == start.row {
            let from = start.col.min(line.len());
            if line.is_char_boundary(from) {
                &line[from..]
            } else {
                line
            }
        } else if row == end.row {
            let to = end.col.min(line.len());
            if line.is_char_boundary(to) {
                &line[..to]
            } else {
                line
            }
        } else {
            line
        };
        if row > start.row {
            out.push('\n');
        }
        out.push_str(piece);
    }
    out
}
