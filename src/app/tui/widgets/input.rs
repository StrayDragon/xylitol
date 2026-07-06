//! Input widget — single-line Focusable text input (c399 stage 2.3).
//!
//! Port of pi's `components/input.ts` to the c399 line-array engine. Differences
//! from the pi original:
//!
//! - **Outcome channel**: pi stores `onSubmit?: (value) => void` / `onEscape?`
//!   callbacks on the widget and invokes them from inside `handleInput`. Rust
//!   forbids that pattern cleanly (host ↔ widget callback cycle), so we record
//!   `pending_outcome: Option<UxOutcome>` and the host polls
//!   [`Input::take_outcome`] after [`Component::handle_input`]. Same semantics
//!   ("after handleInput, drain the submit/abort/quit intent"), different shape.
//! - **Deferred features** (design.md "不在范围"): kill-ring, undo, word
//!   navigation, bracketed paste, Kitty CSI-u decoding. The grapheme cursor +
//!   basic edit set is enough for the chat input; Emacs editing is additive.
//!
//! The horizontal-scroll window algorithm mirrors pi `input.ts:378-445`
//! (half-width bias so the cursor stays centered when scrolling). Cursor
//! movement is **grapheme-level** via `unicode-segmentation` (correct for
//! emoji+ZWJ sequences like 👨‍👩‍👧), not char-level like the legacy `TuiApp`
//! cursor (which is CJK-safe but splits ZWJ clusters).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::app::tui::engine::component::{Component, Focusable, InputResult};
use crate::app::tui::engine::keybindings::KeybindingsManager;
use crate::app::tui::engine::outcome::UxOutcome;
use crate::app::tui::engine::style::{CURSOR_MARKER, CellStyle};
use crate::app::tui::engine::width::marker_aware_width;

/// Prompt prefix shown before the input text (pi uses `"> "`).
const PROMPT: &str = "> ";

/// Single-line Focusable text input with horizontal scrolling and a grapheme
/// cursor. Owns its buffer; the host reads outcomes via [`take_outcome`].
///
/// Render emits one line of exactly `width` display columns: prompt + visible
/// window (with `CURSOR_MARKER` + reverse-video glyph at the cursor when
/// focused) + right padding. Width contract honored — the engine's hard-width
/// check never trips.
pub struct Input {
    value: String,
    /// Byte offset of the insert cursor within `value`. Always lands on a
    /// grapheme boundary (which is also a char boundary, so `&value[..cursor]`
    /// is sound). `cursor == value.len()` is the end-of-input position.
    cursor: usize,
    focused: bool,
    /// High-level outcome surfaced to the host (Submit/Slash/Abort/Quit). Set
    /// inside `handle_input`, drained by `take_outcome`. At most one pending —
    /// a second submit before the host drains overwrites (the host drains every
    /// key, so this never happens in practice).
    pending_outcome: Option<UxOutcome>,
    /// Keybinding registry — widgets query this instead of hardcoding keys
    /// (stage 3). Defaults to [`KeybindingsManager::default`]; tests inject a
    /// custom manager via [`with_keybindings`].
    keybindings: KeybindingsManager,
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            focused: false,
            pending_outcome: None,
            keybindings: KeybindingsManager::default(),
        }
    }

    /// Construct with a custom keybinding manager (tests / host override).
    pub fn with_keybindings(keybindings: KeybindingsManager) -> Self {
        Self {
            keybindings,
            ..Self::new()
        }
    }

    /// Current buffer contents.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Replace the buffer; clamps cursor to the end if it's now past the value.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        if self.cursor > self.value.len() {
            self.cursor = self.value.len();
        }
    }

    /// Current cursor as a byte offset into `value()`.
    pub fn cursor_byte(&self) -> usize {
        self.cursor
    }

    /// Clear the buffer and reset cursor (e.g. after the host consumes a submit).
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    /// Drain the pending high-level outcome (Submit/Slash/Abort/Quit). The host
    /// calls this after [`Component::handle_input`] returns `Handled`.
    pub fn take_outcome(&mut self) -> Option<UxOutcome> {
        self.pending_outcome.take()
    }

    // ── buffer edits (grapheme-aware) ───────────────────────────────────────

    /// Insert `c` at the cursor; advance cursor by its UTF-8 length. Rejects
    /// control chars (C0 0x00-0x1F, DEL 0x7F, C1 0x80-0x9F) — they would corrupt
    /// the rendered line or act as stray escapes.
    fn insert_char(&mut self, c: char) {
        let code = c as u32;
        if code < 0x20 || code == 0x7f || (0x80..=0x9f).contains(&code) {
            return;
        }
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Delete the grapheme cluster immediately before the cursor (Backspace).
    /// Uses `graphemes(true)` so a ZWJ emoji sequence deletes as one unit.
    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let before = &self.value[..self.cursor];
        let last_len = before
            .graphemes(true)
            .next_back()
            .map(str::len)
            .unwrap_or(0);
        if last_len == 0 {
            return;
        }
        let cut = self.cursor - last_len;
        self.value.replace_range(cut..self.cursor, "");
        self.cursor = cut;
    }

    /// Move cursor left by one grapheme cluster.
    fn cursor_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let before = &self.value[..self.cursor];
        let last_len = before
            .graphemes(true)
            .next_back()
            .map(str::len)
            .unwrap_or(0);
        if last_len == 0 {
            return;
        }
        self.cursor -= last_len;
    }

    /// Move cursor right by one grapheme cluster.
    fn cursor_right(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        let after = &self.value[self.cursor..];
        let first_len = after.graphemes(true).next().map(str::len).unwrap_or(0);
        if first_len == 0 {
            return;
        }
        self.cursor += first_len;
    }

    fn cursor_home(&mut self) {
        self.cursor = 0;
    }

    fn cursor_end(&mut self) {
        self.cursor = self.value.len();
    }
}

impl Component for Input {
    fn render(&self, width: usize) -> Vec<crate::app::tui::engine::style::StyledLine> {
        // Build the visible window: pi input.ts:378-445. `available` is the
        // column budget left after the prompt. The rendered line is always:
        //   PROMPT + visible_text_with_cursor_glyph + right-padding
        // where visible_text_with_cursor_glyph ≤ available (so prompt + content
        // ≤ width). The cursor glyph is the grapheme under the cursor (or a
        // space when the cursor is at end-of-input), shown in reverse video;
        // CURSOR_MARKER is emitted just before it when focused so the engine
        // can position the IME hardware cursor.
        use crate::app::tui::engine::style::{Span, StyledLine};

        let prompt_w = marker_aware_width(PROMPT);

        // Degenerate: no room even for a single content column → prompt only,
        // truncated to width. Width invariant still holds.
        if width <= prompt_w {
            let line = StyledLine::raw(PROMPT.chars().take(width).collect::<String>());
            return vec![line];
        }

        let available = width - prompt_w;
        let total_w = UnicodeWidthStr::width(self.value.as_str());
        let cursor_col = UnicodeWidthStr::width(&self.value[..self.cursor]);

        // Decide the window: a slice of the value (by display column) plus the
        // cursor's column within that slice. The slice width budget is
        // `available`; we reserve 1 column for the end-of-input cursor glyph
        // when the cursor sits at the end (pi input.ts:397).
        let scroll_w = if self.cursor == self.value.len() {
            available.saturating_sub(1)
        } else {
            available
        };

        let (visible_text, cursor_col_in_visible): (String, usize) = if total_w <= scroll_w {
            // Fits entirely; no scroll.
            (self.value.clone(), cursor_col)
        } else if scroll_w == 0 {
            (String::new(), 0)
        } else {
            // Horizontal scroll with half-width bias (pi input.ts:401-413).
            let half = scroll_w / 2;
            let start_col = if cursor_col < half {
                0
            } else if cursor_col > total_w.saturating_sub(half) {
                total_w.saturating_sub(scroll_w)
            } else {
                cursor_col.saturating_sub(half)
            };
            slice_by_column(&self.value, start_col, scroll_w, cursor_col)
        };

        // Split the visible text at the cursor: before + glyph-at + after.
        let (before, at, after) = split_at_grapheme_col(&visible_text, cursor_col_in_visible);
        let marker = if self.focused { CURSOR_MARKER } else { "" };
        let cursor_glyph = at.unwrap_or(' ');
        let reverse = CellStyle::default().reverse();

        // Assemble spans: prompt + before + (marker) + reverse-glyph + after.
        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::raw(PROMPT));
        if !before.is_empty() {
            spans.push(Span::raw(before));
        }
        if !marker.is_empty() {
            spans.push(Span::raw(marker));
        }
        spans.push(Span::styled(cursor_glyph.to_string(), reverse));
        if !after.is_empty() {
            spans.push(Span::raw(after));
        }

        // Right-pad to width so the line fills the terminal column. Width
        // invariant: content (prompt + visible) ≤ prompt + available = width,
        // so `used ≤ width` always; padding is ≥ 0.
        let used = spans_width(&spans);
        if used < width {
            spans.push(Span::raw(" ".repeat(width - used)));
        }
        vec![StyledLine::from_spans(spans)]
    }

    fn handle_input(&mut self, key: &KeyEvent) -> InputResult {
        let kb = &self.keybindings;
        // App-level intents (pi model: the focused widget decides ctrl+c/d
        // semantics — see ux.md Step 4 + tui.ts:825 comment). These stay on the
        // widget path, not in a global input listener; keybindings make them
        // user-configurable instead of hardcoded.
        if kb.matches(key, "app.clear") {
            self.pending_outcome = Some(UxOutcome::Abort);
            return InputResult::Handled;
        }
        if kb.matches(key, "app.exit") {
            self.pending_outcome = Some(UxOutcome::Quit);
            return InputResult::Handled;
        }
        if kb.matches(key, "tui.input.submit") {
            let text = self.value.trim().to_string();
            if text.is_empty() {
                return InputResult::Handled;
            }
            if let Some(stripped) = text.strip_prefix('/') {
                self.pending_outcome = Some(UxOutcome::Slash(stripped.to_string()));
            } else {
                self.pending_outcome = Some(UxOutcome::Submit(text));
            }
            return InputResult::Handled;
        }
        if kb.matches(key, "tui.editor.deleteCharBackward") {
            self.backspace();
            return InputResult::Handled;
        }
        if kb.matches(key, "tui.editor.cursorLeft") {
            self.cursor_left();
            return InputResult::Handled;
        }
        if kb.matches(key, "tui.editor.cursorRight") {
            self.cursor_right();
            return InputResult::Handled;
        }
        if kb.matches(key, "tui.editor.cursorLineStart") {
            self.cursor_home();
            return InputResult::Handled;
        }
        if kb.matches(key, "tui.editor.cursorLineEnd") {
            self.cursor_end();
            return InputResult::Handled;
        }
        // Printable char insert: a Char with no ctrl/alt (shift is OK — crossterm
        // delivers shift+letter as the already-shifted glyph, e.g. 'A' not
        // shift+'a'). control/alt combos are reserved for keybindings above.
        let reserved = KeyModifiers::CONTROL.union(KeyModifiers::ALT);
        match key.code {
            KeyCode::Char(c) if !key.modifiers.intersects(reserved) => {
                self.insert_char(c);
                InputResult::Handled
            }
            _ => InputResult::NotHandled,
        }
    }

    fn invalidate(&mut self) {
        // No render cache (computed fresh each frame, like pi input.ts:374).
    }

    fn focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn take_outcome(&mut self) -> Option<UxOutcome> {
        // Delegate to the inherent method so tests + host can use either path.
        Input::take_outcome(self)
    }
}

impl Focusable for Input {
    // Marker only — set_focused/focused live on Component (see above) so they
    // dispatch through `Box<dyn Component>` in a Container.
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// Sum of display widths of a span vector's texts (prompt + content + padding),
/// ANSI/APC-aware so the zero-width `CURSOR_MARKER` contributes 0.
fn spans_width(spans: &[crate::app::tui::engine::style::Span]) -> usize {
    spans
        .iter()
        .map(|s| marker_aware_width(s.text.as_str()))
        .sum()
}

/// Slice `value` by display column: keep the graphemes whose cumulative width
/// lands in `[start_col, start_col + max_width)`. Returns the visible text and
/// the cursor's column within that visible text. Mirrors pi `utils.ts
/// sliceByColumn` (strict=true: never split a wide char at the boundary).
fn slice_by_column(
    value: &str,
    start_col: usize,
    max_width: usize,
    cursor_col: usize,
) -> (String, usize) {
    let mut out = String::new();
    let mut out_w = 0usize;
    let mut col = 0usize;
    let mut cursor_in_visible = 0usize;
    let mut cursor_settled = cursor_col < start_col; // cursor before window → clamp 0
    for g in value.graphemes(true) {
        let gw = UnicodeWidthStr::width(g);
        if col + gw <= start_col {
            // entirely before the window
            col += gw;
            continue;
        }
        if col >= start_col + max_width {
            break;
        }
        // Wide char straddling the start boundary: skip (strict) — col < start_col
        // but col + gw > start_col. Advance col without emitting.
        if col < start_col {
            col += gw;
            continue;
        }
        if out_w + gw > max_width {
            break;
        }
        if !cursor_settled && cursor_col <= col {
            cursor_in_visible = out_w;
            cursor_settled = true;
        }
        out.push_str(g);
        out_w += gw;
        col += gw;
    }
    if !cursor_settled {
        // cursor at or past the window end → clamp to the visible width
        cursor_in_visible = out_w.min(max_width);
    }
    (out, cursor_in_visible)
}

/// Split `text` at a display column: return `(before, glyph_at, after)`.
/// `glyph_at` is the grapheme cluster whose cumulative width covers `col` (or
/// the grapheme right after the end if `col == total_width`). Returns None for
/// `glyph_at` when `col` is at the end (so the caller can render a space cursor).
fn split_at_grapheme_col(text: &str, col: usize) -> (String, Option<char>, String) {
    let mut before = String::new();
    let mut cumulative = 0usize;
    let mut graphemes = text.graphemes(true);
    // Walk forward; when cumulative reaches `col`, the next grapheme is "at".
    for g in graphemes.by_ref() {
        if cumulative == col {
            // `g` is the glyph under the cursor.
            let after: String = graphemes.collect();
            let at = g.chars().next();
            return (before, at, after);
        }
        let gw = UnicodeWidthStr::width(g);
        if cumulative + gw > col {
            // Wide char covers `col` — treat it as the at-glyph (cursor sits on it).
            let after: String = graphemes.collect();
            let at = g.chars().next();
            return (before, at, after);
        }
        cumulative += gw;
        before.push_str(g);
    }
    // `col` is at or past the end: cursor is end-of-input, no glyph under it.
    (before, None, String::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::engine::style::StyledLine;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }
    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    // ── grapheme cursor movement ────────────────────────────────────────────

    #[test]
    fn cursor_left_right_ascii() {
        let mut input = Input::new();
        for c in "hello".chars() {
            input.insert_char(c);
        }
        // cursor at end (5); move left twice → byte 3
        input.cursor_left();
        input.cursor_left();
        assert_eq!(input.cursor_byte(), 3);
        input.cursor_right();
        assert_eq!(input.cursor_byte(), 4);
    }

    #[test]
    fn cursor_left_right_cjk_by_grapheme() {
        // 你好 = 2 graphemes, 6 bytes. Moving left from end steps by grapheme.
        let mut input = Input::new();
        for c in "你好".chars() {
            input.insert_char(c);
        }
        assert_eq!(input.cursor_byte(), 6);
        input.cursor_left();
        assert_eq!(input.cursor_byte(), 3, "left by one grapheme (3 bytes)");
        input.cursor_left();
        assert_eq!(input.cursor_byte(), 0);
        // clamped at 0
        input.cursor_left();
        assert_eq!(input.cursor_byte(), 0);
        input.cursor_right();
        assert_eq!(input.cursor_byte(), 3);
    }

    #[test]
    fn cursor_left_right_emoji_zwj_sequence() {
        // 👨‍👩‍👧 is ONE grapheme cluster (man + ZWJ + woman + ZWJ + girl),
        // 11 code points / 17 bytes. Cursor must step the whole cluster, not
        // each char.
        let s = "a👨‍👩‍👧b";
        let mut input = Input::new();
        input.set_value(s.to_string());
        input.cursor_end();
        assert_eq!(input.cursor_byte(), s.len());
        input.cursor_left(); // back to before 'b'
        assert_eq!(
            input.cursor_byte(),
            s.len() - 1,
            "left from end lands on 'b' (1 byte)"
        );
        input.cursor_left(); // skip the entire ZWJ family emoji
        assert_eq!(
            input.cursor_byte(),
            1,
            "left skips the whole grapheme cluster to 'a' (byte 1)"
        );
        input.cursor_left();
        assert_eq!(input.cursor_byte(), 0);
    }

    // ── insert / backspace ───────────────────────────────────────────────────

    #[test]
    fn insert_at_cursor_middle() {
        let mut input = Input::new();
        input.set_value("ac");
        input.cursor_home();
        input.cursor_right(); // between a and c
        input.insert_char('b');
        assert_eq!(input.value(), "abc");
        assert_eq!(input.cursor_byte(), 2);
    }

    #[test]
    fn backspace_deletes_grapheme_cluster() {
        let mut input = Input::new();
        input.set_value("x👨‍👩‍👧");
        input.cursor_end();
        input.backspace();
        assert_eq!(input.value(), "x", "backspace removes the whole emoji");
        assert_eq!(input.cursor_byte(), 1);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut input = Input::new();
        input.set_value("abc");
        input.cursor_home();
        input.backspace();
        assert_eq!(input.value(), "abc");
        assert_eq!(input.cursor_byte(), 0);
    }

    #[test]
    fn insert_rejects_control_chars() {
        let mut input = Input::new();
        input.insert_char('\x01'); // C0 control
        input.insert_char('\x7f'); // DEL
        assert!(input.value().is_empty());
        input.insert_char('a');
        assert_eq!(input.value(), "a");
    }

    #[test]
    fn home_and_end() {
        let mut input = Input::new();
        input.set_value("hello");
        input.cursor_home();
        assert_eq!(input.cursor_byte(), 0);
        input.cursor_end();
        assert_eq!(input.cursor_byte(), 5);
    }

    // ── outcomes (Submit / Slash / Abort / Quit) ─────────────────────────────

    #[test]
    fn enter_plain_submits() {
        let mut input = Input::new();
        for c in "hello".chars() {
            input.handle_input(&char_key(c));
        }
        assert_eq!(
            input.handle_input(&key(KeyCode::Enter, KeyModifiers::NONE)),
            InputResult::Handled
        );
        match input.take_outcome() {
            Some(UxOutcome::Submit(s)) => assert_eq!(s, "hello"),
            other => panic!("expected Submit, got {:?}", other_is(other)),
        }
        assert!(input.take_outcome().is_none(), "outcome drained");
    }

    #[test]
    fn enter_slash_routes_to_slash() {
        let mut input = Input::new();
        for c in "/model".chars() {
            input.handle_input(&char_key(c));
        }
        input.handle_input(&key(KeyCode::Enter, KeyModifiers::NONE));
        match input.take_outcome() {
            Some(UxOutcome::Slash(s)) => assert_eq!(s, "model"),
            other => panic!("expected Slash, got {:?}", other_is(other)),
        }
    }

    #[test]
    fn enter_whitespace_only_no_outcome() {
        let mut input = Input::new();
        for c in "   ".chars() {
            input.handle_input(&char_key(c));
        }
        input.handle_input(&key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(
            input.take_outcome().is_none(),
            "trim-empty enter yields no outcome"
        );
    }

    #[test]
    fn ctrl_c_aborts() {
        let mut input = Input::new();
        input.handle_input(&key(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(matches!(input.take_outcome(), Some(UxOutcome::Abort)));
    }

    #[test]
    fn ctrl_d_quits() {
        let mut input = Input::new();
        input.handle_input(&key(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(matches!(input.take_outcome(), Some(UxOutcome::Quit)));
    }

    #[test]
    fn ctrl_other_is_not_handled() {
        let mut input = Input::new();
        assert!(matches!(
            input.handle_input(&key(KeyCode::Char('z'), KeyModifiers::CONTROL)),
            InputResult::NotHandled
        ));
        assert!(input.take_outcome().is_none());
    }

    #[test]
    fn unknown_key_is_not_handled() {
        let mut input = Input::new();
        assert!(matches!(
            input.handle_input(&key(KeyCode::F(1), KeyModifiers::NONE)),
            InputResult::NotHandled
        ));
    }

    // ── render ───────────────────────────────────────────────────────────────

    #[test]
    fn render_short_fits_with_prompt() {
        let mut input = Input::new();
        input.set_focused(true);
        input.set_value("hi");
        let lines = input.render(20);
        assert_eq!(lines.len(), 1);
        let ansi = lines[0].to_ansi();
        // Prompt present, both value chars present (the cursor glyph inserts
        // marker + SGR between '>' and 'h', so we check chars individually).
        assert!(ansi.starts_with("> "), "prompt present");
        assert!(
            ansi.contains('h') && ansi.contains('i'),
            "value chars present"
        );
        assert!(ansi.contains(CURSOR_MARKER), "marker emitted when focused");
    }

    #[test]
    fn render_unfocused_omits_cursor_marker() {
        let mut input = Input::new();
        input.set_focused(false);
        input.set_value("hi");
        let ansi = input.render(20)[0].to_ansi();
        assert!(!ansi.contains(CURSOR_MARKER));
    }

    #[test]
    fn render_honors_width_invariant() {
        // Long value + narrow width → must still produce a line ≤ width.
        let mut input = Input::new();
        input.set_focused(true);
        input.set_value("abcdefghijklmnopqrstuvwxyz0123456789");
        for w in [5usize, 10, 20, 40] {
            let line = &input.render(w)[0];
            assert!(
                line.width() <= w,
                "width {} produced line of width {}",
                w,
                line.width()
            );
        }
    }

    #[test]
    fn render_honors_width_invariant_cjk() {
        let mut input = Input::new();
        input.set_focused(true);
        input.set_value("你好世界再见你好世界再见");
        for w in [4usize, 6, 10, 20] {
            let line = &input.render(w)[0];
            assert!(
                line.width() <= w,
                "width {} produced CJK line of width {}",
                w,
                line.width()
            );
        }
    }

    #[test]
    fn render_scrolls_long_content_cursor_at_end() {
        // Value longer than width; cursor at the end → window shows the tail.
        let mut input = Input::new();
        input.set_focused(true);
        input.set_value("abcdefghij"); // 10 chars, width 10
        input.cursor_end();
        let line = &input.render(6)[0]; // width 6: "> " + 4 cols
        let ansi = line.to_ansi();
        // The visible window should include chars near the end (cursor in view).
        assert!(ansi.contains('h') || ansi.contains('i') || ansi.contains('j'));
        assert!(ansi.contains(CURSOR_MARKER));
        assert!(line.width() <= 6);
    }

    #[test]
    fn render_scrolls_cursor_at_start() {
        let mut input = Input::new();
        input.set_focused(true);
        input.set_value("abcdefghij");
        input.cursor_home();
        let line = &input.render(6)[0];
        let ansi = line.to_ansi();
        // Cursor at start → window begins at col 0 → 'a' visible.
        assert!(ansi.contains('a'));
        assert!(ansi.contains(CURSOR_MARKER));
    }

    #[test]
    fn render_scrolls_cursor_in_middle() {
        let mut input = Input::new();
        input.set_focused(true);
        input.set_value("abcdefghij");
        // move cursor to the middle (byte 5, between 'e' and 'f')
        input.set_value("abcdefghij");
        // cursor at byte 5
        for _ in 0..5 {
            input.cursor_right();
        }
        let line = &input.render(6)[0];
        // half-width bias keeps cursor in view; marker present somewhere.
        assert!(line.to_ansi().contains(CURSOR_MARKER));
        assert!(line.width() <= 6);
    }

    #[test]
    fn render_cjk_cursor_column_correct() {
        // 你好 at width 10: cursor between 你 (2 cols) and 好.
        let mut input = Input::new();
        input.set_focused(true);
        input.set_value("你好");
        input.cursor_home();
        input.cursor_right(); // after 你 (byte 3, col 2)
        let line: &StyledLine = &input.render(10)[0];
        let ansi = line.to_ansi();
        // Reverse-video should land on 好 (the grapheme at col 2).
        assert!(ansi.contains("\x1b[7m"));
        assert!(ansi.contains('好'));
    }

    #[test]
    fn render_zero_available_width_just_prompt() {
        let mut input = Input::new();
        input.set_focused(true);
        input.set_value("hi");
        // width 1: "> " truncated to just ">"
        let line = &input.render(1)[0];
        assert!(line.width() <= 1);
    }

    #[test]
    fn render_empty_input_shows_cursor_glyph_at_start() {
        let mut input = Input::new();
        input.set_focused(true);
        let line = &input.render(10)[0];
        let ansi = line.to_ansi();
        // Empty value → cursor glyph is a space (reverse-video space).
        assert!(ansi.contains(CURSOR_MARKER));
        assert!(ansi.contains("\x1b[7m"));
    }

    #[test]
    fn invalidate_is_noop_and_render_still_works() {
        let mut input = Input::new();
        input.set_value("hi");
        input.set_focused(true);
        input.invalidate();
        assert_eq!(input.render(10).len(), 1);
    }

    // Helper so panic! messages compile (UxOutcome has no Debug derive).
    fn other_is(_: Option<UxOutcome>) -> &'static str {
        "other"
    }
}
