//! Multi-line editor component with kill-ring, undo, history, VisualLine
//! vertical cursor movement, PasteBurst integration, and autocomplete popup.
//! Ported from pi's `components/editor.ts` (c425 VL+sticky+PasteBurst + c430 autocomplete).

#![allow(
    clippy::type_complexity,
    clippy::needless_range_loop,
    clippy::module_inception
)]

mod autocomplete;
mod edit;
mod layout;
mod selection;
mod types;

#[cfg(test)]
mod tests;

pub use types::{AutocompleteMode, VisualLine};

use crate::clock::Clock;
use crate::completion::CompletionRegistry;
use crate::components::select_list::{SelectList, SelectListTheme};
use crate::keybindings::with_keybindings;
use crate::keys::{matches_key_event, printable_from_key_event};
use crate::kill_ring::KillRing;
use crate::paste_burst::PasteBurst;
use crate::selection::CellPoint;
use crate::tui::{Component, Focusable, InputEvent};
use crate::undo_stack::UndoStack;
use selection::{EditorSelection, extract_editor_range};
use std::collections::HashMap;
use types::{CursorPlacement, EditorState, expand_paste_markers};

pub struct EditorTheme {
    pub border_color: Box<dyn Fn(&str) -> String>,
    pub select_list_theme: SelectListTheme,
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self {
            border_color: Box::new(|s| s.to_string()),
            select_list_theme: SelectListTheme::default(),
        }
    }
}

pub struct EditorOptions {
    pub padding_x: usize,
    pub terminal_rows: usize,
}

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            padding_x: 0,
            terminal_rows: 24,
        }
    }
}

// ── Editor ──────────────────────────────────────────────────────────────────

pub struct Editor {
    state: EditorState,
    focused: bool,
    theme: EditorTheme,
    padding_x: usize,
    terminal_rows: usize,
    last_width: usize,
    /// Visible content rows painted on the last render (excludes borders).
    last_content_rows: usize,
    /// Full width passed to the last [`Self::render`].
    last_paint_width: usize,
    scroll_offset: usize,
    history: Vec<String>,
    history_index: isize,
    history_draft: Option<EditorState>,
    kill_ring: KillRing,
    last_action: Option<String>,
    undo_stack: UndoStack<EditorState>,
    jump_mode: Option<bool>,
    pastes: HashMap<usize, String>,
    paste_counter: usize,
    // c425 new fields
    preferred_visual_col: Option<usize>,
    snapped_from_cursor_col: Option<usize>,
    paste_burst: PasteBurst,
    /// After a paste-burst paint-suppress window ends, force one catch-up frame.
    paste_burst_needs_paint: bool,
    clock: Box<dyn Clock>,
    // c430 autocomplete integration (CompletionSource registry)
    completion: CompletionRegistry,
    autocomplete_list: Option<SelectList>,
    autocomplete_state: Option<AutocompleteMode>,
    autocomplete_prefix: String,
    autocomplete_max_visible: usize,
    autocomplete_start_token: usize,
    /// ApplicationOwned editor-owned selection (independent of transcript SelectionController).
    selection: EditorSelection,
    /// Default on — OSC52 sequences land in [`Self::take_pending_clipboard`].
    pub copy_on_release: bool,
    pending_clipboard: Vec<String>,
    /// Absolute screen origin of the editor's top-left paint cell (ApplicationOwned hit-test).
    screen_origin_row: u16,
    screen_origin_col: u16,
    /// Last mouse handler dirtied selection / clipboard (for rerender policy).
    mouse_dirty: bool,
    pub on_submit: Option<Box<dyn FnMut(String)>>,
    pub on_change: Option<Box<dyn FnMut(&str)>>,
    pub disable_submit: bool,
}

impl Editor {
    pub fn new(theme: EditorTheme, opts: EditorOptions, clock: Box<dyn Clock>) -> Self {
        Self {
            state: EditorState::default(),
            focused: false,
            theme,
            padding_x: opts.padding_x,
            terminal_rows: opts.terminal_rows,
            last_width: 80,
            last_content_rows: 1,
            last_paint_width: 80,
            scroll_offset: 0,
            history: Vec::new(),
            history_index: -1,
            history_draft: None,
            kill_ring: KillRing::new(),
            last_action: None,
            undo_stack: UndoStack::new(),
            jump_mode: None,
            pastes: HashMap::new(),
            paste_counter: 0,
            preferred_visual_col: None,
            snapped_from_cursor_col: None,
            paste_burst: PasteBurst::new(),
            paste_burst_needs_paint: false,
            clock,
            completion: CompletionRegistry::new(),
            autocomplete_list: None,
            autocomplete_state: None,
            autocomplete_prefix: String::new(),
            autocomplete_max_visible: 5,
            autocomplete_start_token: 0,
            selection: EditorSelection::default(),
            copy_on_release: true,
            pending_clipboard: Vec::new(),
            screen_origin_row: 0,
            screen_origin_col: 0,
            mouse_dirty: false,
            on_submit: None,
            on_change: None,
            disable_submit: false,
        }
    }

    pub fn get_text(&self) -> String {
        self.state.lines.join("\n")
    }

    /// Cursor as `(line, col)` — after [`Self::set_text`], line/col are at buffer end (pi parity).
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.state.cursor_line, self.state.cursor_col)
    }

    /// Replace the border ANSI wrapper used when painting editor chrome.
    pub fn set_border_color(&mut self, border_color: Box<dyn Fn(&str) -> String>) {
        self.theme.border_color = border_color;
    }

    pub fn set_text(&mut self, text: String) {
        self.exit_history_browsing();
        self.cancel_autocomplete();
        self.last_action = None;
        self.pastes.clear();
        self.paste_counter = 0;
        self.selection.clear();
        let n = text
            .replace('\t', "    ")
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        self.push_undo();
        // pi `setText` → `setTextInternal(..., "end")` — cursor at end of buffer.
        self.set_text_internal(&n, CursorPlacement::End);
    }

    /// Whether the editor currently has a non-empty multi-cell selection.
    pub fn has_selection(&self) -> bool {
        self.selection.has_selection()
    }

    /// True while an unmodified left-drag selection is in progress.
    pub fn is_selection_dragging(&self) -> bool {
        self.selection.dragging
    }

    /// Ordered `(start, end)` in logical buffer coordinates, if non-empty.
    pub fn selection_bounds(&self) -> Option<(CellPoint, CellPoint)> {
        self.selection.bounds().filter(|(a, b)| a != b)
    }

    /// Selected plain text from the editor buffer (not transcript).
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_bounds()?;
        Some(extract_editor_range(&self.state.lines, start, end))
    }

    /// Drain OSC52 (or empty) sequences produced by copy-on-release.
    pub fn take_pending_clipboard(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_clipboard)
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Set absolute screen origin of this editor's top-left paint cell.
    ///
    /// ApplicationOwned hosts MUST update this when the dock moves so mouse hit-testing
    /// maps screen coordinates into editor-local space for [`Self::handle_mouse_local`].
    pub fn set_screen_origin(&mut self, row: u16, col: u16) {
        self.screen_origin_row = row;
        self.screen_origin_col = col;
    }

    pub fn screen_origin(&self) -> (u16, u16) {
        (self.screen_origin_row, self.screen_origin_col)
    }

    pub fn add_to_history(&mut self, text: String) {
        let t = text.trim().to_string();
        if t.is_empty() || self.history.first() == Some(&t) {
            return;
        }
        self.history.insert(0, t);
        if self.history.len() > 100 {
            self.history.pop();
        }
    }

    /// Replace ↑/↓ send history (c1560 session seed). Texts oldest→newest.
    pub fn replace_history(&mut self, texts: impl IntoIterator<Item = String>) {
        self.exit_history_browsing();
        self.history.clear();
        self.history_draft = None;
        for text in texts {
            self.add_to_history(text);
        }
    }
    pub fn get_expanded_text(&self) -> String {
        expand_paste_markers(&self.state.lines.join("\n"), &self.pastes)
    }
    fn handle_key(&mut self, key: &crossterm::event::KeyEvent) {
        if let Some(dir) = self.jump_mode.take() {
            if let Some(s) = printable_from_key_event(key).and_then(|s| s.chars().next()) {
                self.jump_to(s, dir);
            }
            return;
        }

        macro_rules! k {
            ($n:expr) => {
                with_keybindings(|kb| kb.matches_event(key, $n))
            };
        }

        // Undo
        if k!("tui.editor.undo") {
            self.undo();
            return;
        }

        // ── c430: autocomplete active routing (align pi: Tab=apply, Enter on `/`=apply+submit)
        if self.autocomplete_state.is_some() && self.autocomplete_list.is_some() {
            if k!("tui.select.cancel") {
                self.cancel_autocomplete();
                return;
            }
            if k!("tui.select.up") || k!("tui.select.down") {
                if let Some(ref mut list) = self.autocomplete_list {
                    list.handle_input(InputEvent::Key(*key));
                }
                return;
            }
            if k!("tui.input.tab") {
                // Refresh prefix against the live buffer before apply (stale
                // prefix can survive paste-burst char floods).
                self.handle_autocomplete_on_edit();
                self.apply_selected_autocomplete(/* chain_next */ true);
                return;
            }
            if k!("tui.select.confirm") {
                let prefix = self.autocomplete_prefix.clone();
                let applied = self.apply_selected_autocomplete(/* chain_next */ false);
                if applied && prefix.starts_with('/') {
                    // pi: slash confirm falls through to submit (no Tab required).
                    if !self.disable_submit {
                        let now = self.clock.now();
                        if self
                            .paste_burst
                            .should_insert_newline_instead_of_submit(now)
                        {
                            self.paste_burst.extend_window(now);
                            self.newline();
                        } else {
                            self.submit();
                        }
                    }
                }
                return;
            }
        }
        // ── end c430 autocomplete routing ────────────────────────────

        // Deletion / kill ring
        if k!("tui.editor.yankPop") {
            self.yank_pop();
        } else if k!("tui.editor.yank") {
            self.yank();
        } else if k!("tui.editor.deleteToLineStart") {
            self.del_to_start();
        } else if k!("tui.editor.deleteToLineEnd") {
            self.del_to_end();
        } else if k!("tui.editor.deleteWordBackward") {
            self.del_word_back();
        } else if k!("tui.editor.deleteWordForward") {
            self.del_word_fwd();
        } else if k!("tui.editor.deleteCharBackward") || matches_key_event(key, "shift+backspace") {
            self.backspace();
            self.handle_autocomplete_on_edit();
        } else if k!("tui.editor.deleteCharForward") || matches_key_event(key, "shift+delete") {
            self.fwd_delete();
            self.handle_autocomplete_on_edit();
        }
        // Cursor movement
        else if k!("tui.editor.cursorLineStart") {
            self.line_start();
        } else if k!("tui.editor.cursorLineEnd") {
            self.line_end();
        } else if k!("tui.editor.cursorWordLeft") {
            self.word_left();
            self.handle_autocomplete_on_edit();
        } else if k!("tui.editor.cursorWordRight") {
            self.word_right();
            self.handle_autocomplete_on_edit();
        } else if k!("tui.editor.jumpForward") {
            self.jump_mode = Some(true);
        } else if k!("tui.editor.jumpBackward") {
            self.jump_mode = Some(false);
        } else if k!("tui.editor.pageUp") {
            self.page_scroll(-1);
        } else if k!("tui.editor.pageDown") {
            self.page_scroll(1);
        }
        // Tab — trigger completion (c430)
        else if k!("tui.input.tab") && self.autocomplete_state.is_none() {
            self.handle_tab_completion();
        }
        // Newline / Submit with PasteBurst
        else if k!("tui.input.newLine") {
            let now = self.clock.now();
            if self
                .paste_burst
                .should_insert_newline_instead_of_submit(now)
            {
                self.paste_burst.extend_window(now);
                self.newline();
            } else {
                self.newline();
            }
        } else if k!("tui.input.submit") {
            if self.disable_submit {
                return;
            }
            let now = self.clock.now();
            let line = &self.state.lines[self.state.cursor_line];
            // Backslash-Escape: insert newline instead of submit
            if self.state.cursor_col > 0
                && line.as_bytes().get(self.state.cursor_col - 1) == Some(&b'\\')
            {
                self.backspace();
                self.newline();
            }
            // PasteBurst check: rapid paste → insert newline instead of submit
            else if self
                .paste_burst
                .should_insert_newline_instead_of_submit(now)
            {
                self.paste_burst.extend_window(now);
                self.newline();
            } else {
                self.submit();
            }
        }
        // Arrow navigation with VisualLine+history
        else if k!("tui.editor.cursorUp") {
            if (self.is_on_first_visual_line() || self.state.cursor_col == 0)
                && (self.is_editor_empty() || self.history_index > -1)
            {
                self.navigate_history(-1);
            } else if self.is_on_first_visual_line() {
                self.line_start();
            } else {
                self.move_cursor(-1, 0);
                self.handle_autocomplete_on_edit();
            }
        } else if k!("tui.editor.cursorDown") {
            if self.history_index > -1 && self.is_on_last_visual_line() {
                self.navigate_history(1);
            } else if self.is_on_last_visual_line() {
                self.line_end();
            } else {
                self.move_cursor(1, 0);
                self.handle_autocomplete_on_edit();
            }
        } else if k!("tui.editor.cursorLeft") {
            self.move_cursor(0, -1);
            self.handle_autocomplete_on_edit();
        } else if k!("tui.editor.cursorRight") {
            self.move_cursor(0, 1);
            self.handle_autocomplete_on_edit();
        }
        // Printable chars
        else if matches_key_event(key, "shift+space") {
            self.insert_ch(" ");
        } else if let Some(p) = printable_from_key_event(key) {
            self.insert_ch(&p);
        }
        // Everything else → reset paste burst
        else {
            self.paste_burst.reset();
        }
    }
}

impl Focusable for Editor {
    fn set_focused(&mut self, f: bool) {
        self.focused = f;
    }
    fn is_focused(&self) -> bool {
        self.focused
    }
}
