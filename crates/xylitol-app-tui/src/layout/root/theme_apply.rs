use std::sync::atomic::{AtomicBool, Ordering};

use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::{fg_rgb, mix_rgb};

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
        let dim = mix_rgb(accent, theme.palette().surface, 0.55);
        let muted = theme.palette().muted;
        let pulse = std::sync::Arc::new(AtomicBool::new(false));
        self.status_loader = Loader::new(
            Box::new(move |s| {
                let on = pulse.fetch_xor(true, Ordering::SeqCst);
                fg_rgb(if on { accent } else { dim }, s)
            }),
            Box::new(move |s| fg_rgb(muted, s)),
            String::new(),
            Some(LoaderIndicatorOptions {
                frames: vec!["⬢".into()],
                interval_ms: 100,
            }),
        );
        self.sync_editor_border();
        self.slot.retheme(theme);
        self.refresh_footer();
    }
}
