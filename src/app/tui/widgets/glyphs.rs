//! Product glyph sets — unicode / ascii; no font probing (c475).

/// Configurable short prefixes for scrollback lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlyphSet {
    #[default]
    Unicode,
    Ascii,
}

impl GlyphSet {
    /// Read `XYLITOL_TUI_GLYPH_SET` (`ascii` → Ascii; otherwise Unicode).
    pub fn from_env() -> Self {
        match std::env::var("XYLITOL_TUI_GLYPH_SET").ok().as_deref() {
            Some("ascii") | Some("ASCII") => Self::Ascii,
            _ => Self::Unicode,
        }
    }

    pub fn user(self) -> &'static str {
        match self {
            Self::Unicode => "❯",
            Self::Ascii => ">",
        }
    }

    pub fn system(self) -> &'static str {
        match self {
            Self::Unicode => "·",
            Self::Ascii => ".",
        }
    }

    pub fn fold(self) -> &'static str {
        match self {
            Self::Unicode => "▸",
            Self::Ascii => ">",
        }
    }

    pub fn unfold(self) -> &'static str {
        match self {
            Self::Unicode => "▾",
            Self::Ascii => "v",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_tui::visible_width;

    #[test]
    fn fold_glyphs_visible_width_one() {
        assert_eq!(GlyphSet::Unicode.fold(), "▸");
        assert_eq!(GlyphSet::Unicode.unfold(), "▾");
        assert_eq!(visible_width(GlyphSet::Unicode.fold()), 1);
        assert_eq!(visible_width(GlyphSet::Unicode.unfold()), 1);
        assert_eq!(GlyphSet::Ascii.fold(), ">");
        assert_eq!(GlyphSet::Ascii.unfold(), "v");
        assert_eq!(visible_width(GlyphSet::Ascii.fold()), 1);
        assert_eq!(visible_width(GlyphSet::Ascii.unfold()), 1);
    }
}
