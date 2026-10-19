//! Product layout theme — fixed dark `Palette` → component closures (c475).

use xylitol_tui::components::editor::EditorTheme;
use xylitol_tui::components::select_list::{
    SelectItem, SelectList, SelectListLayoutOptions, SelectListTheme,
};
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

    pub fn from_palette(palette: Palette) -> Self {
        Self { palette }
    }

    pub fn palette(self) -> Palette {
        self.palette
    }

    /// Shared SelectList assembly path for all product slots (c2500): uniform
    /// layout options (`truncate_primary: None`, explicit primary width bounds).
    pub fn select_list(
        self,
        items: Vec<SelectItem>,
        max_visible: usize,
        primary_width: (usize, usize),
    ) -> SelectList {
        SelectList::new(
            items,
            max_visible,
            self.select_list_theme(),
            SelectListLayoutOptions {
                min_primary_column_width: Some(primary_width.0),
                max_primary_column_width: Some(primary_width.1),
                truncate_primary: None,
            },
        )
    }

    /// SelectList theme for models picker (DESIGN models-picker.md).
    pub fn select_list_theme(self) -> SelectListTheme {
        let muted = self.palette.muted;
        let accent = self.palette.accent;
        let selected_bg = self.palette.user_message_bg;
        SelectListTheme {
            selected_prefix: Box::new(move |s| fg_rgb(accent, s)),
            // Truecolor wash (pi selectedBg) — reverse video flickers under differential paint.
            selected_text: Box::new(move |s| {
                use xylitol_tui::bg_rgb;
                bg_rgb(selected_bg, s)
            }),
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
        let selected_bg = self.palette.user_message_bg;
        TreeSelectorTheme {
            cursor: Box::new(move |s| fg_rgb(accent, s)),
            prefix: Box::new(move |s| fg_rgb(muted, s)),
            label: Box::new(move |s| fg_rgb(on_surface, s)),
            selected_row: Box::new(move |s| {
                use xylitol_tui::bg_rgb;
                bg_rgb(selected_bg, s)
            }),
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

    /// Tool name in header: bold + accent (DESIGN `tool-name` experiment → accent until settled).
    pub fn paint_tool_name(self, s: &str) -> String {
        use xylitol_tui::bold;
        bold(&fg_rgb(self.palette.accent, s))
    }

    /// Tool path / location: on-surface highlight (playground may retarget `tool-path`).
    pub fn paint_tool_path(self, s: &str) -> String {
        fg_rgb(self.palette.on_surface, s)
    }

    /// Line-range suffix (`:12-40`) — mauve `skill_ref`, fits the Mocha frame (not warning yellow).
    pub fn paint_tool_range(self, s: &str) -> String {
        fg_rgb(self.palette.skill_ref, s)
    }

    pub fn paint_error(self, s: &str) -> String {
        fg_rgb(self.palette.error, s)
    }

    /// Full-width list selection wash (pi `selectedBg`; avoids reverse-video flicker).
    pub fn paint_selected_row(self, s: &str, width: usize) -> String {
        use xylitol_tui::{apply_background_to_line, bg_rgb, visible_width};
        let w = width.max(1);
        let mut line = s.to_string();
        let pad = w.saturating_sub(visible_width(&line));
        if pad > 0 {
            line.push_str(&" ".repeat(pad));
        }
        let bg = self.palette.user_message_bg;
        apply_background_to_line(&line, w, &move |t| bg_rgb(bg, t))
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
}

impl Default for LayoutTheme {
    fn default() -> Self {
        Self::product_dark()
    }
}
