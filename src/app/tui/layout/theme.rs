//! Product layout theme — fixed dark `Palette` → component closures (c475).

use xylitol_tui::components::editor::EditorTheme;
use xylitol_tui::components::select_list::SelectListTheme;
use xylitol_tui::components::tree_selector::TreeSelectorTheme;
use xylitol_tui::{Palette, fg_rgb};

/// Product MVP layout theme: always `Palette::dark()` (no theme auto).
#[derive(Clone, Copy)]
pub struct LayoutTheme {
    palette: Palette,
}

impl LayoutTheme {
    pub fn product_dark() -> Self {
        Self::from_palette(Palette::dark())
    }

    pub fn product_light() -> Self {
        Self::from_palette(Palette::light())
    }

    pub fn from_palette(palette: Palette) -> Self {
        Self { palette }
    }

    pub fn palette(self) -> Palette {
        self.palette
    }

    /// SelectList theme for models picker (DESIGN models-picker.md).
    pub fn select_list_theme(self) -> SelectListTheme {
        let muted = self.palette.muted;
        let accent = self.palette.accent;
        SelectListTheme {
            selected_prefix: Box::new(move |s| fg_rgb(accent, s)),
            selected_text: Box::new(|s| format!("\x1b[7m{s}\x1b[27m")),
            description: Box::new(move |s| fg_rgb(muted, s)),
            scroll_info: Box::new(move |s| fg_rgb(muted, s)),
            no_match: Box::new(move |s| fg_rgb(muted, s)),
        }
    }

    pub fn editor_theme(self) -> EditorTheme {
        let muted = self.palette.muted;
        EditorTheme {
            border_color: Box::new(move |s| fg_rgb(muted, s)),
            select_list_theme: SelectListTheme::default(),
        }
    }

    /// Session-tree theme: kind prefixes use DESIGN tokens (user / success / tool).
    pub fn tree_selector_theme(self) -> TreeSelectorTheme {
        let user = self.palette.user;
        let success = self.palette.success;
        let tool = self.palette.tool;
        let muted = self.palette.muted;
        let on_surface = self.palette.on_surface;
        let warning = self.palette.warning;
        let accent = self.palette.accent;
        TreeSelectorTheme {
            cursor: Box::new(move |s| fg_rgb(accent, s)),
            prefix: Box::new(move |s| fg_rgb(muted, s)),
            label: Box::new(move |s| fg_rgb(on_surface, s)),
            selected_row: Box::new(|s| format!("\x1b[7m{s}\x1b[27m")),
            active_marker: Box::new(move |s| fg_rgb(accent, s)),
            scroll_info: Box::new(move |s| fg_rgb(muted, s)),
            empty: Box::new(move |s| fg_rgb(muted, s)),
            annotation: Box::new(move |s| fg_rgb(warning, s)),
            annotation_time: Box::new(move |s| fg_rgb(muted, s)),
            kind_prefix: Box::new(move |kind| match kind {
                "user" => fg_rgb(user, "user: "),
                "assistant" => fg_rgb(success, "assistant: "),
                "tool" => fg_rgb(tool, "tool: "),
                other => fg_rgb(muted, &format!("[{other}]: ")),
            }),
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

impl Default for LayoutTheme {
    fn default() -> Self {
        Self::product_dark()
    }
}
