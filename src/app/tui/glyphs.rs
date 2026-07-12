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

    pub fn tool(self) -> &'static str {
        match self {
            Self::Unicode => "⚙",
            Self::Ascii => "*",
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
            Self::Unicode => "▶",
            Self::Ascii => ">",
        }
    }

    pub fn unfold(self) -> &'static str {
        match self {
            Self::Unicode => "▼",
            Self::Ascii => "v",
        }
    }
}
