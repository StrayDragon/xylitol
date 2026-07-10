//! Empty product shell layout (transcript / editor / footer placeholders).

use xylitol_tui::components::editor::{Editor, EditorOptions, EditorTheme};
use xylitol_tui::components::text::Text;
use xylitol_tui::{Component, Focusable, InputEvent, SystemClock};

use super::host::{LayoutMode, TOO_SMALL_HINT};

/// Root shell: transcript placeholder + bordered editor + footer.
pub struct Shell {
    transcript: Text,
    editor: Editor,
    footer: Text,
}

impl Shell {
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
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Shell {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.extend(self.transcript.render(width));
        lines.push(String::new());
        // Operation zone borders (DESIGN.md).
        let border = "─".repeat(width.clamp(1, 80));
        lines.push(border.clone());
        for line in self.editor.render(width.saturating_sub(0).max(1)) {
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

/// Build the root child list for a layout mode.
pub fn build_root(mode: LayoutMode) -> Vec<Box<dyn Component>> {
    match mode {
        LayoutMode::Shell => vec![Box::new(Shell::new())],
        LayoutMode::TooSmall => vec![Box::new(TooSmallHint)],
    }
}
