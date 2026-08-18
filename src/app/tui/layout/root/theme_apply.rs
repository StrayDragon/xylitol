//! Theme apply helpers for [`UiRoot`] (c1095).

use xylitol_tui::components::loader::{Loader, LoaderIndicatorOptions};
use xylitol_tui::fg_rgb;

use super::super::slots::EditorSlot;
use super::UiRoot;
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
        self.sync_editor_border();
        if matches!(&self.slot, EditorSlot::SessionResume(_)) {
            self.slot = EditorSlot::SessionResume(
                crate::app::tui::session_resume::SessionResumePanel::new(theme),
            );
        } else {
            match &mut self.slot {
                EditorSlot::Tree(tree) => tree.wipe_themed(theme),
                EditorSlot::Models(models) => models.retheme(theme),
                EditorSlot::Themes(themes) => themes.retheme(theme),
                EditorSlot::Mcp(mcp) => mcp.retheme(theme),
                EditorSlot::ImportConfirm(imp) => imp.retheme(theme),
                EditorSlot::Editor
                | EditorSlot::Plate
                | EditorSlot::Settings
                | EditorSlot::Choice(_)
                | EditorSlot::SessionResume(_) => {}
            }
        }
        self.refresh_footer();
    }
}
