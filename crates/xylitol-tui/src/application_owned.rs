use crate::interaction_mode::InteractionMode;
use crate::terminal::Terminal;
use crate::tui::TUI;

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

#[must_use]
pub fn mouse_in_dock(mouse_row: u16, term_rows: u16, dock_rows: usize) -> bool {
    if term_rows == 0 {
        return false;
    }
    let dock = dock_rows.max(1).min(term_rows as usize) as u16;
    let dock_top = term_rows.saturating_sub(dock);
    mouse_row >= dock_top
}

pub struct ApplicationOwnedTui<T: Terminal> {
    inner: TUI<T>,
}

impl<T: Terminal> ApplicationOwnedTui<T> {
    #[must_use]
    pub fn new(terminal: T) -> Self {
        Self {
            inner: TUI::with_interaction_mode(terminal, InteractionMode::ApplicationOwned),
        }
    }

    pub fn begin(&mut self) {
        self.inner.begin_application_owned_session();
    }

    pub fn finish(&mut self) {
        self.inner.finish_application_owned();
    }

    #[must_use]
    pub fn tui(&self) -> &TUI<T> {
        &self.inner
    }

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
