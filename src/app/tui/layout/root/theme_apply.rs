//! Theme apply helpers for [`UiRoot`] (c1095).

use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::fg_rgb;

use super::UiRoot;
use super::empty_widgets::{
    empty_mcp_list, empty_models_list, empty_session_resume_panel, empty_themes_list,
    empty_tree_selector, import_confirm_list,
};
use crate::app::tui::layout::LayoutTheme;

impl UiRoot {
    pub fn layout_theme(&self) -> LayoutTheme {
        self.theme
    }

    /// Replace the layout theme and rebuild theme-dependent chrome (c1095).
    /// Does not clear transcript / `ui_model` entries.
    pub fn set_layout_theme(&mut self, theme: LayoutTheme) {
        self.theme = theme;
        self.scrollback_paint.invalidate();
        self.bump_upper_gen();
        let accent = theme.palette().accent;
        let muted = theme.palette().muted;
        self.status_loader = Loader::new(
            Box::new(move |s| fg_rgb(accent, s)),
            Box::new(move |s| fg_rgb(muted, s)),
            String::new(),
            Some(LoaderIndicatorOptions::default()),
        );
        // Bash accent or thinking border under the new Palette (c1150).
        self.sync_editor_border();
        // Rebuild themed shells; tree/resume content is host-refreshed on next open.
        let selected = self.tree.selected_id().map(str::to_string);
        self.tree = empty_tree_selector(theme);
        if let Some(id) = selected {
            let _ = self.tree.select_id(&id);
        }
        self.models_list = empty_models_list(theme);
        self.apply_models_filter();
        self.themes_list = empty_themes_list(theme);
        self.mcp_list = empty_mcp_list(theme);
        self.import_confirm_list = import_confirm_list(theme);
        self.session_resume = empty_session_resume_panel(theme);
        self.refresh_footer();
    }
}
