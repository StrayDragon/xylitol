//! Minimal terminal-image helpers kept for the width invariant and OSC 8.
//!
//! Full Kitty/iTerm encode + `Image` component were trimmed in c445 — primary
//! agent_demo / app-shell paths do not need them. Reintroduce behind a feature
//! if a product surface needs inline images.

const KITTY_PREFIX: &str = "\x1b_G";
const ITERM2_PREFIX: &str = "\x1b]1337;File=";

/// Heuristic for Kitty / iTerm2 inline-image lines. Their visible width is 0
/// (all escape bytes), so the render engine exempts them from the width check.
pub fn is_image_line(line: &str) -> bool {
    line.starts_with(KITTY_PREFIX)
        || line.starts_with(ITERM2_PREFIX)
        || line.contains(KITTY_PREFIX)
        || line.contains(ITERM2_PREFIX)
}

/// OSC 8 hyperlink wrapper (kept — tiny and useful for markdown/link styling).
pub fn hyperlink(text: &str, url: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_kitty_and_iterm_payloads() {
        assert!(is_image_line("\x1b_Ga=T,f=100;abc\x1b\\"));
        assert!(is_image_line("\x1b]1337;File=inline=1:abc\x07"));
        assert!(!is_image_line("plain text"));
    }
}
