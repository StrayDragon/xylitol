use std::collections::HashMap;

/// Expand `[paste #N …]` markers using the editor's paste table (pi `expandPasteMarkers`).
pub(super) fn expand_paste_markers(text: &str, pastes: &HashMap<usize, String>) -> String {
    let mut result = text.to_string();
    for (&id, content) in pastes {
        let prefix = format!("[paste #{id}");
        let mut search_from = 0;
        while let Some(rel) = result[search_from..].find(&prefix) {
            let start = search_from + rel;
            let after_prefix = start + prefix.len();
            let Some(end) = paste_marker_end(&result, after_prefix) else {
                search_from = after_prefix;
                continue;
            };
            result.replace_range(start..end, content);
            search_from = start + content.len();
        }
    }
    result
}

/// End index (exclusive) of a paste marker that starts with `[paste #N` ending at `after_prefix`.
fn paste_marker_end(text: &str, after_prefix: usize) -> Option<usize> {
    let rest = text.get(after_prefix..)?;
    if rest.starts_with(']') {
        return Some(after_prefix + 1);
    }
    let stripped = rest.strip_prefix(' ')?;
    let close = stripped.find(']')?;
    Some(after_prefix + 1 + close + 1)
}

// ── types ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub(super) struct EditorState {
    pub(super) lines: Vec<String>,
    pub(super) cursor_line: usize,
    pub(super) cursor_col: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
        }
    }
}

/// A visual line — one row on screen after word-wrap.
pub struct VisualLine {
    pub logical_line: usize,
    pub start_col: usize,
    pub len: usize,
}

/// Where to place cursor after set_text_internal.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CursorPlacement {
    Start,
    End,
}

/// Autocomplete popup mode.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AutocompleteMode {
    Regular,
    Force,
}

pub(super) struct LayoutLine {
    pub(super) text: String,
    pub(super) has_cursor: bool,
    pub(super) cursor_pos: Option<usize>,
    /// Logical buffer line this visual row belongs to.
    pub(super) logical_line: usize,
    /// Byte offset into the logical line where `text` begins.
    pub(super) start_index: usize,
}

pub(super) struct TextChunk {
    pub(super) text: String,
    pub(super) start_index: usize,
    pub(super) end_index: usize,
}
