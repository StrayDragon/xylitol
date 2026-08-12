//! Application-owned host helpers.
//!
//! Prefer this module's layout helpers + [`crate::TUI`] ApplicationOwned lifecycle over
//! copying demo-private mouse arithmetic. Product hosts and demos SHOULD share
//! one path: measure dock → [`editor_screen_origin`] → [`Editor::set_screen_origin`]
//! → absolute [`InputEvent::Mouse`] through normal focus routing.
//!
//! # Minimal ApplicationOwned host loop
//!
//! ```ignore
//! use xylitol_tui::{ApplicationOwnedTui, InputEvent, InteractionMode, TUI};
//!
//! let mut tui = ApplicationOwnedTui::new(terminal);
//! tui.terminal_mut().start();
//! tui.begin();
//! // … add children, set focus …
//! loop {
//!     // read InputEvent …
//!     tui.dispatch_event(event);
//!     if tui.idle_tick() { let _ = tui.try_render(); }
//!     // after paint / dock measure:
//!     // editor.set_screen_origin(editor_screen_origin(rows, dock, above).0, 0);
//!     if quit { break; }
//! }
//! tui.finish(); // leave alt; append session to main scrollback by default
//! ```
//!
//! See package `AGENTS.md` § ApplicationOwned host checklist (ptim14).

use crate::interaction_mode::InteractionMode;
use crate::terminal::Terminal;
use crate::tui::TUI;

/// Absolute screen origin `(row, col)` of the editor's top-left paint cell.
///
/// `rows_above_editor_in_dock` counts dock lines above the editor (toast +
/// status, etc.). Footer / chrome below the editor is already inside
/// `dock_rows` and must **not** be added here.
///
/// # Examples
///
/// ```
/// use xylitol_tui::editor_screen_origin;
/// assert_eq!(editor_screen_origin(24, 5, 2), (21, 0)); // dock_top=19 + 2
/// ```
#[must_use]
pub fn editor_screen_origin(
    term_rows: u16,
    dock_rows: usize,
    rows_above_editor_in_dock: usize,
) -> (u16, u16) {
    let dock = dock_rows.max(1).min(term_rows as usize) as u16;
    let dock_top = term_rows.saturating_sub(dock);
    let above = rows_above_editor_in_dock.min(dock.saturating_sub(1) as usize) as u16;
    (dock_top.saturating_add(above), 0)
}

/// Whether `mouse_row` falls in the bottom dock band (inclusive of dock top).
#[must_use]
pub fn mouse_in_dock(mouse_row: u16, term_rows: u16, dock_rows: usize) -> bool {
    if term_rows == 0 {
        return false;
    }
    let dock = dock_rows.max(1).min(term_rows as usize) as u16;
    let dock_top = term_rows.saturating_sub(dock);
    mouse_row >= dock_top
}

/// Thin facade: [`TUI`] constructed as [`InteractionMode::ApplicationOwned`].
///
/// Owns the ApplicationOwned **entry narrative** (`begin` / `finish`) without forking the
/// differential engine. Deref to [`TUI`] for the full host surface.
pub struct ApplicationOwnedTui<T: Terminal> {
    inner: TUI<T>,
}

impl<T: Terminal> ApplicationOwnedTui<T> {
    /// Construct an ApplicationOwned TUI (does not enter alt-buffer yet).
    #[must_use]
    pub fn new(terminal: T) -> Self {
        Self {
            inner: TUI::with_interaction_mode(terminal, InteractionMode::ApplicationOwned),
        }
    }

    /// Enter alt-buffer + mouse + [`crate::application_owned_runtime::ApplicationOwnedRuntime`] (idempotent).
    pub fn begin(&mut self) {
        self.inner.begin_application_owned_session();
    }

    /// Leave ApplicationOwned: leave alt-screen, optionally append session to main
    /// scrollback, then stop the terminal.
    pub fn finish(&mut self) {
        self.inner.finish_application_owned();
    }

    /// Borrow the inner [`TUI`].
    #[must_use]
    pub fn tui(&self) -> &TUI<T> {
        &self.inner
    }

    /// Mutably borrow the inner [`TUI`].
    pub fn tui_mut(&mut self) -> &mut TUI<T> {
        &mut self.inner
    }
}

impl<T: Terminal> std::ops::Deref for ApplicationOwnedTui<T> {
    type Target = TUI<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Terminal> std::ops::DerefMut for ApplicationOwnedTui<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_origin_accounts_for_status_above_editor() {
        // term=20, dock=6 → dock_top=14; status=2 → editor at row 16
        assert_eq!(editor_screen_origin(20, 6, 2), (16, 0));
    }

    #[test]
    fn mouse_in_dock_boundary() {
        assert!(!mouse_in_dock(13, 20, 6));
        assert!(mouse_in_dock(14, 20, 6));
        assert!(mouse_in_dock(19, 20, 6));
    }

    #[test]
    fn oversized_dock_and_offset_stay_inside_terminal() {
        assert_eq!(editor_screen_origin(4, usize::MAX, usize::MAX), (3, 0));
        assert!(mouse_in_dock(0, 4, usize::MAX));
        assert!(!mouse_in_dock(0, 0, usize::MAX));
    }
}
