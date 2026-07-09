//! Product TUI root layout — transcript / editor / footer placeholders.
//!
//! Named `UiRoot` (not `shell`/`scene`) to avoid clashing with bash /
//! `infra::process::shell` and to read as the product component tree root.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use xylitol_tui::components::editor::{Editor, EditorOptions, EditorTheme};
use xylitol_tui::components::text::Text;
use xylitol_tui::{
    Component, Focusable, InputEvent, InputListenerResult, SystemClock, TUI, Terminal,
    matches_key_event,
};

use super::host::{LayoutMode, TOO_SMALL_HINT};

/// Root UI: transcript placeholder + bordered editor + footer.
pub struct UiRoot {
    transcript: Text,
    editor: Editor,
    footer: Text,
}

impl UiRoot {
    pub fn new() -> Self {
        let mut editor = Editor::new(
            EditorTheme::default(),
            EditorOptions::default(),
            Box::new(SystemClock),
        );
        editor.set_focused(true);
        Self {
            transcript: Text::new(
                "(empty transcript — bridge lands in a later change)".into(),
                0,
                0,
            ),
            editor,
            footer: Text::new("esc abort · ctrl+c clear/quit · /exit".into(), 0, 0),
        }
    }

    /// Current editor text (harness / listeners).
    pub fn editor_text(&self) -> String {
        self.editor.get_text()
    }

    /// Replace editor text (harness).
    pub fn set_editor_text(&mut self, text: impl Into<String>) {
        self.editor.set_text(text.into());
    }

    /// Ctrl+C: clear editor when non-empty; otherwise signal quit via `quit_flag`.
    pub fn on_ctrl_c(&mut self, quit_flag: &AtomicBool) {
        if !self.editor.get_text().is_empty() {
            self.editor.set_text(String::new());
            return;
        }
        quit_flag.store(true, Ordering::SeqCst);
    }
}

impl Default for UiRoot {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for UiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.extend(self.transcript.render(width));
        lines.push(String::new());
        let border = "─".repeat(width.clamp(1, 80));
        lines.push(border.clone());
        for line in self.editor.render(width.max(1)) {
            lines.push(line);
        }
        lines.push(border);
        lines.extend(self.footer.render(width));
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.editor.handle_input(event);
    }

    fn invalidate(&mut self) {
        self.transcript.invalidate();
        self.editor.invalidate();
        self.footer.invalidate();
    }

    fn tick(&mut self) -> bool {
        self.editor.tick()
    }
}

/// Shared root so InputListeners and the focused Component see the same state.
pub struct SharedUiRoot(pub Rc<RefCell<UiRoot>>);

impl Component for SharedUiRoot {
    fn render(&mut self, width: usize) -> Vec<String> {
        self.0.borrow_mut().render(width)
    }

    fn handle_input(&mut self, event: InputEvent) {
        self.0.borrow_mut().handle_input(event);
    }

    fn invalidate(&mut self) {
        self.0.borrow_mut().invalidate();
    }

    fn tick(&mut self) -> bool {
        self.0.borrow_mut().tick()
    }
}

/// Full-screen hint when the terminal is too small.
pub struct TooSmallHint;

impl Component for TooSmallHint {
    fn render(&mut self, width: usize) -> Vec<String> {
        let msg = TOO_SMALL_HINT;
        if width == 0 {
            return vec![msg.into()];
        }
        let pad = width.saturating_sub(msg.chars().count()) / 2;
        vec![format!("{}{msg}", " ".repeat(pad))]
    }

    fn handle_input(&mut self, _event: InputEvent) {}

    fn invalidate(&mut self) {}
}

/// Build root children for a layout mode (no shared state — simple tests).
#[cfg(test)]
pub fn build_root(mode: LayoutMode) -> Vec<Box<dyn Component>> {
    match mode {
        LayoutMode::Ready => vec![Box::new(UiRoot::new())],
        LayoutMode::TooSmall => vec![Box::new(TooSmallHint)],
    }
}

/// Shared root + rebuild closure that keeps the same `UiRoot` across min-size flips.
pub fn shared_ui_root_rebuild(
    root: Rc<RefCell<UiRoot>>,
) -> impl FnMut(LayoutMode) -> Vec<Box<dyn Component>> + 'static {
    move |mode| match mode {
        LayoutMode::Ready => vec![Box::new(SharedUiRoot(root.clone()))],
        LayoutMode::TooSmall => vec![Box::new(TooSmallHint)],
    }
}

/// Register pre-focus Ctrl+C (clear / quit). Esc abort waits for streaming (c480).
pub fn install_ui_root_key_listeners<T: Terminal>(
    root: &Rc<RefCell<UiRoot>>,
    quit_flag: &Arc<AtomicBool>,
    tui: &mut TUI<T>,
) {
    let root = root.clone();
    let quit_flag = quit_flag.clone();
    tui.add_input_listener(move |event| {
        let InputEvent::Key(key) = &event else {
            return InputListenerResult::Continue;
        };
        if matches_key_event(key, "ctrl+c") {
            root.borrow_mut().on_ctrl_c(&quit_flag);
            return InputListenerResult::Consumed;
        }
        InputListenerResult::Continue
    });
}
