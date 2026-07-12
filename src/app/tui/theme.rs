//! Product chrome theme — fixed dark `Palette` → component closures (c475).

use xylitol_tui::components::editor::EditorTheme;
use xylitol_tui::components::select_list::SelectListTheme;
use xylitol_tui::{Palette, fg_rgb};

/// Product MVP chrome: always `Palette::dark()` (no theme auto).
#[derive(Clone, Copy)]
pub struct ChromeTheme {
    palette: Palette,
}

impl ChromeTheme {
    pub fn product_dark() -> Self {
        Self {
            palette: Palette::dark(),
        }
    }

    pub fn palette(self) -> Palette {
        self.palette
    }

    pub fn editor_theme(self) -> EditorTheme {
        let muted = self.palette.muted;
        EditorTheme {
            border_color: Box::new(move |s| fg_rgb(muted, s)),
            select_list_theme: SelectListTheme::default(),
        }
    }

    pub fn paint_muted(self, s: &str) -> String {
        fg_rgb(self.palette.muted, s)
    }

    pub fn paint_status(self, s: &str) -> String {
        fg_rgb(self.palette.muted, s)
    }

    pub fn paint_user(self, s: &str) -> String {
        fg_rgb(self.palette.user, s)
    }

    pub fn paint_assistant(self, s: &str) -> String {
        fg_rgb(self.palette.assistant, s)
    }

    pub fn paint_tool(self, s: &str) -> String {
        fg_rgb(self.palette.tool, s)
    }

    pub fn paint_error(self, s: &str) -> String {
        fg_rgb(self.palette.error, s)
    }

    pub fn paint_success(self, s: &str) -> String {
        fg_rgb(self.palette.success, s)
    }

    pub fn paint_warning(self, s: &str) -> String {
        fg_rgb(self.palette.warning, s)
    }

    /// Editor border painter for bash (`!` / `!!`) mode.
    pub fn bash_border_color(self) -> Box<dyn Fn(&str) -> String> {
        let success = self.palette.success;
        Box::new(move |s| fg_rgb(success, s))
    }

    /// Default muted editor border painter.
    pub fn muted_border_color(self) -> Box<dyn Fn(&str) -> String> {
        let muted = self.palette.muted;
        Box::new(move |s| fg_rgb(muted, s))
    }

    pub fn paint_border(self, width: usize) -> String {
        let raw = "─".repeat(width.clamp(1, 80));
        self.paint_muted(&raw)
    }
}

impl Default for ChromeTheme {
    fn default() -> Self {
        Self::product_dark()
    }
}
