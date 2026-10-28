use xylitol_tui::truncate_to_width;

use crate::bridge::ModeStripState;
use crate::layout::LayoutTheme;
use crate::widgets::GlyphSet;

pub fn render_mode_strip(
    state: &ModeStripState,
    glyphs: GlyphSet,
    width: usize,
    theme: &LayoutTheme,
) -> String {
    let line = theme.paint_muted(&format!("{} {}", glyphs.mode(), state.label));
    if width == 0 {
        line
    } else {
        truncate_to_width(&line, width, "…", false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_with_glyph_and_label() {
        let state = ModeStripState {
            label: "plan".into(),
        };
        let theme = LayoutTheme::product_dark();
        let line = render_mode_strip(&state, GlyphSet::Unicode, 80, &theme);
        assert!(line.contains("◆ plan"));
        assert!(!line.contains('\n'));
    }

    #[test]
    fn ascii_fallback_and_truncation() {
        let state = ModeStripState {
            label: "execute · a-very-long-plan-id".into(),
        };
        let theme = LayoutTheme::product_dark();
        let line = render_mode_strip(&state, GlyphSet::Ascii, 12, &theme);
        assert!(line.contains('*'));
        assert!(line.contains('…'));
        assert_eq!(xylitol_tui::visible_width(&line), 12);
    }
}
