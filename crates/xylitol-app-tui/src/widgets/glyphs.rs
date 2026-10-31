#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlyphSet {
    #[default]
    Unicode,
    Ascii,
}

impl GlyphSet {
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

    pub fn mode(self) -> &'static str {
        match self {
            Self::Unicode => "◆",
            Self::Ascii => "*",
        }
    }

    pub fn todo_summary(self, expanded: bool) -> &'static str {
        if expanded { self.unfold() } else { self.fold() }
    }

    pub fn todo_doing(self) -> &'static str {
        match self {
            Self::Unicode => "→",
            Self::Ascii => ">",
        }
    }

    pub fn todo_pending(self) -> &'static str {
        match self {
            Self::Unicode => "·",
            Self::Ascii => ".",
        }
    }

    pub fn todo_done(self) -> &'static str {
        match self {
            Self::Unicode => "✓",
            Self::Ascii => "x",
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

    #[test]
    fn todo_glyphs_visible_width_one() {
        for set in [GlyphSet::Unicode, GlyphSet::Ascii] {
            assert_eq!(visible_width(set.todo_doing()), 1);
            assert_eq!(visible_width(set.todo_pending()), 1);
            assert_eq!(visible_width(set.todo_done()), 1);
            assert_eq!(visible_width(set.todo_summary(true)), 1);
            assert_eq!(visible_width(set.todo_summary(false)), 1);
        }
    }

    #[test]
    fn mode_glyph_visible_width_one() {
        assert_eq!(GlyphSet::Unicode.mode(), "◆");
        assert_eq!(GlyphSet::Ascii.mode(), "*");
        assert_eq!(visible_width(GlyphSet::Unicode.mode()), 1);
        assert_eq!(visible_width(GlyphSet::Ascii.mode()), 1);
    }
}
