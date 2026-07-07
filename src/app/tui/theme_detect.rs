//! Terminal theme detection (dark/light) — the shared entry the whole TUI uses.
//!
//! c399 had this logic ([`syntect_highlight::pick_theme_name`]) but it was
//! trapped in the syntect module, feeding ONLY code-block highlighting. The
//! rest of the UI (Palette, cursor) ran on a fixed color set with no theme
//! awareness, which is why a raw `reverse` cursor on a transparent dark
//! terminal became a glaring white block (reverse swaps the terminal's default
//! fg/bg, both uncontrolled).
//!
//! This module lifts the detection to a shared, dependency-free function
//! returning a domain enum ([`TerminalTheme`]). `syntect_highlight` now maps
//! from it; `theme::palette` branches on it; the Input widget's cursor uses
//! the resulting explicit fg/bg pair instead of raw `reverse`.
//!
//! ## Detection source (v1: COLORFGBG only)
//!
//! Reads `$COLORFGBG` (the xterm-canonical spelling iTerm2/Alacritty/Tmux/
//! Kitty/GNOME Terminal set). Format `"fg;bg"`; background code `>= 7`
//! indicates a light background. Unset or unparseable → dark (the safe
//! default — most developer terminals are dark).
//!
//! OSC 11 / 996-997 background querying is deliberately NOT here: it needs a
//! stdin reply-interceptor in the main loop (pi's `consumeOsc11BackgroundResponse`
//! pattern), which c399 design D4 omitted. That is tracked as a separate
//! enhancement (c400); adding it later means this function grows a richer
//! probe order while its callers stay unchanged.

/// The terminal's resolved background theme. Drives Palette color selection.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TerminalTheme {
    /// Dark background (default — the safe fallback when detection fails).
    #[default]
    Dark,
    /// Light background.
    Light,
}

impl TerminalTheme {
    /// `true` when the terminal background is light.
    pub fn is_light(self) -> bool {
        matches!(self, Self::Light)
    }
}

/// Detect the terminal theme from `$COLORFGBG`.
///
/// Format `"fg;bg"` where the background code `>= 7` → light; else dark.
/// Falls back to dark when unset or unparseable. Pure + dependency-free:
/// safe to call from anywhere in the TUI layer (no agent/infra reach, so
/// arch_guard stays green).
pub fn detect_from_colorfgbg() -> TerminalTheme {
    classify_colorfgbg(std::env::var("COLORFGBG").ok().as_deref())
}

/// Classify a raw `COLORFGBG` value (the `"fg;bg"` string, or `None` when
/// unset). Pure function — separated from [`detect_from_colorfgbg`] so tests
/// don't race on the process environment.
pub fn classify_colorfgbg(colorfgbg: Option<&str>) -> TerminalTheme {
    let Some(val) = colorfgbg else {
        return TerminalTheme::Dark;
    };
    let parts: Vec<&str> = val.split(';').collect();
    if parts.len() >= 2
        && let Ok(bg) = parts[1].trim().parse::<u8>()
    {
        return if bg >= 7 {
            TerminalTheme::Light
        } else {
            TerminalTheme::Dark
        };
    }
    TerminalTheme::Dark
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_when_bg_code_high() {
        // bg code 15 (white) / 7 (default-bg in some schemes) → light.
        assert_eq!(classify_colorfgbg(Some("0;15")), TerminalTheme::Light);
        assert_eq!(classify_colorfgbg(Some("7;7")), TerminalTheme::Light);
    }

    #[test]
    fn dark_when_bg_code_low() {
        // bg code 0 (black) → dark; non-numeric fg field still parses bg.
        assert_eq!(classify_colorfgbg(Some("15;0")), TerminalTheme::Dark);
        assert_eq!(classify_colorfgbg(Some("default;0")), TerminalTheme::Dark);
    }

    #[test]
    fn dark_when_unset() {
        assert_eq!(classify_colorfgbg(None), TerminalTheme::Dark);
    }

    #[test]
    fn dark_when_unparseable() {
        assert_eq!(classify_colorfgbg(Some("garbage")), TerminalTheme::Dark);
        assert_eq!(
            classify_colorfgbg(Some("onlyonefield")),
            TerminalTheme::Dark
        );
        assert_eq!(classify_colorfgbg(Some(";")), TerminalTheme::Dark);
    }

    #[test]
    fn dark_when_bg_not_a_number() {
        // `fg;bg` with a non-numeric bg → can't classify → safe dark default.
        assert_eq!(classify_colorfgbg(Some("0;white")), TerminalTheme::Dark);
    }

    #[test]
    fn boundary_7_is_light() {
        // The c396/c399 threshold: bg >= 7 → light. Exact boundary must hold
        // (pi uses the same cutoff; matches xterm's default-bg color slots).
        assert_eq!(classify_colorfgbg(Some("0;6")), TerminalTheme::Dark);
        assert_eq!(classify_colorfgbg(Some("0;7")), TerminalTheme::Light);
    }
}
