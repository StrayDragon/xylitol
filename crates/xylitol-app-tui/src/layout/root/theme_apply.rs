use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::fg_rgb;

use super::UiRoot;
use crate::layout::LayoutTheme;

impl UiRoot {
    pub fn layout_theme(&self) -> LayoutTheme {
        self.theme
    }

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
        self.sync_editor_border();
        self.slot.retheme(theme);
        self.refresh_footer();
    }
}
