use std::time::Instant;

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
        let pulse_start = Instant::now();
        self.status_loader = Loader::new(
            Box::new(move |s| {
                let phase =
                    (pulse_start.elapsed().as_secs_f32() * std::f32::consts::TAU / 1.2).sin() * 0.5
                        + 0.5;
                fg_rgb(mix_rgb(dim, accent, phase), s)
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
