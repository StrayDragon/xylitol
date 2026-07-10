//! Truecolor SGR paint helpers (39 / 49 reset), shared by palette factories.

use crate::terminal_colors::RgbColor;

/// Foreground truecolor + reset to default fg (`39`).
pub fn fg_rgb(rgb: RgbColor, s: &str) -> String {
    format!("\x1b[38;2;{};{};{}m{s}\x1b[39m", rgb.r, rgb.g, rgb.b)
}

/// Background truecolor + reset to default bg (`49`).
pub fn bg_rgb(rgb: RgbColor, s: &str) -> String {
    format!("\x1b[48;2;{};{};{}m{s}\x1b[49m", rgb.r, rgb.g, rgb.b)
}

/// Word / span tint: set word bg + fg, then restore row fg + row bg (not `49m`).
///
/// Diff word highlights need this so the line wash stays continuous.
pub fn fg_bg_rgb(fg: RgbColor, word_bg: RgbColor, row_bg: RgbColor, s: &str) -> String {
    format!(
        "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{s}\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m",
        word_bg.r,
        word_bg.g,
        word_bg.b,
        fg.r,
        fg.g,
        fg.b,
        fg.r,
        fg.g,
        fg.b,
        row_bg.r,
        row_bg.g,
        row_bg.b,
    )
}

pub fn bold(s: &str) -> String {
    format!("\x1b[1m{s}\x1b[22m")
}

pub fn dim(s: &str) -> String {
    format!("\x1b[2m{s}\x1b[22m")
}

pub fn italic(s: &str) -> String {
    format!("\x1b[3m{s}\x1b[23m")
}

pub fn underline(s: &str) -> String {
    format!("\x1b[4m{s}\x1b[24m")
}

pub fn strikethrough(s: &str) -> String {
    format!("\x1b[9m{s}\x1b[29m")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fg_contains_truecolor_and_reset() {
        let s = fg_rgb(
            RgbColor {
                r: 137,
                g: 180,
                b: 250,
            },
            "hi",
        );
        assert!(s.contains("38;2;137;180;250"));
        assert!(s.contains("\x1b[39m"));
        assert!(s.contains("hi"));
    }

    #[test]
    fn bg_resets_with_49() {
        let s = bg_rgb(
            RgbColor {
                r: 0x31,
                g: 0x32,
                b: 0x44,
            },
            "x",
        );
        assert!(s.contains("48;2;49;50;68"));
        assert!(s.contains("\x1b[49m"));
    }
}
