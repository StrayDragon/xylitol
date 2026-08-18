//! Product theme name → `Palette` resolution and host reload (c1095).

use xylitol_tui::Palette;

use super::error::TuiSurfaceError;
use super::layout::LayoutTheme;

/// Resolve a built-in theme name to a [`Palette`].
///
/// Only `dark` / `light` (case-insensitive) are supported in this wave.
pub fn resolve_builtin_palette(name: &str) -> Result<Palette, TuiSurfaceError> {
    match name.trim().to_ascii_lowercase().as_str() {
        "dark" => Ok(Palette::dark()),
        "light" => Ok(Palette::light()),
        other => Err(TuiSurfaceError::invalid(format!(
            "unknown theme `{other}` (built-in: dark, light); custom JSON themes not yet applied"
        ))),
    }
}

/// Build a [`LayoutTheme`] from a theme name.
pub fn layout_theme_from_name(name: &str) -> Result<LayoutTheme, TuiSurfaceError> {
    Ok(LayoutTheme::from_palette(resolve_builtin_palette(name)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::bridge::{UiEntry, UiModel};
    use crate::app::tui::layout::UiRoot;

    #[test]
    fn resolve_dark_light() {
        assert_eq!(resolve_builtin_palette("dark").unwrap(), Palette::dark());
        assert_eq!(resolve_builtin_palette("LIGHT").unwrap(), Palette::light());
        assert!(resolve_builtin_palette("nope").is_err());
    }

    #[test]
    fn set_layout_theme_light_keeps_transcript() {
        let mut root = UiRoot::new();
        let mut model = UiModel::new();
        model.entries.push(UiEntry::User {
            text: "keep me".into(),
        });
        root.apply_ui_model(&model);
        assert_eq!(root.layout_theme().palette(), Palette::dark());

        root.set_layout_theme(layout_theme_from_name("light").unwrap());
        assert_eq!(root.layout_theme().palette(), Palette::light());
        assert_eq!(root.ui_model_entries_len_for_test(), 1);
    }

    #[test]
    fn bad_name_does_not_change_theme_when_caller_guards() {
        let mut root = UiRoot::new();
        let before = root.layout_theme().palette();
        assert!(layout_theme_from_name("missing-theme").is_err());
        if let Ok(t) = layout_theme_from_name("missing-theme") {
            root.set_layout_theme(t);
        }
        assert_eq!(root.layout_theme().palette(), before);
    }
}
