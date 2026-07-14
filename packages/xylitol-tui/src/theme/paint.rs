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

/// Linear blend `from → to` by `amount` (0..=1).
pub fn mix_rgb(from: RgbColor, to: RgbColor, amount: f32) -> RgbColor {
    let t = amount.clamp(0.0, 1.0);
    let lerp =
        |a: u8, b: u8| -> u8 { (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8 };
    RgbColor {
        r: lerp(from.r, to.r),
        g: lerp(from.g, to.g),
        b: lerp(from.b, to.b),
    }
}

/// Slightly lighten toward white.
pub fn shade_toward_white(base: RgbColor, amount: f32) -> RgbColor {
    mix_rgb(
        base,
        RgbColor {
            r: 255,
            g: 255,
            b: 255,
        },
        amount,
    )
}

/// Slightly darken `base` toward black.
///
/// Kept for general tinting; word-diff prefers [`word_wash_bg`].
pub fn shade_toward_black(base: RgbColor, amount: f32) -> RgbColor {
    let t = amount.clamp(0.0, 1.0);
    let scale = 1.0 - t;
    RgbColor {
        r: (f32::from(base.r) * scale).round() as u8,
        g: (f32::from(base.g) * scale).round() as u8,
        b: (f32::from(base.b) * scale).round() as u8,
    }
}

/// Word-diff wash: **bright polarity tint** on the current block/row bg.
///
/// - `polarity`: added → green (`diff_added`), removed → red (`diff_removed`)
/// - Mixes block bg toward a lightly lifted polarity color so `+`/`-` words read
///   as soft green/red highlights without reverse-video white flash.
/// - Mix amount ~**0.32** (subtle); raise toward 0.42 if you need more pop.
pub fn word_wash_bg(block_bg: RgbColor, polarity: RgbColor) -> RgbColor {
    let bright = shade_toward_white(polarity, 0.22);
    mix_rgb(block_bg, bright, 0.32)
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

    #[test]
    fn word_wash_follows_polarity() {
        let block = RgbColor {
            r: 0x24,
            g: 0x35,
            b: 0x2a,
        };
        let green = RgbColor {
            r: 0xa6,
            g: 0xe3,
            b: 0xa1,
        };
        let red = RgbColor {
            r: 0xf3,
            g: 0x8b,
            b: 0xa8,
        };
        let add = word_wash_bg(block, green);
        let rem = word_wash_bg(block, red);
        // Added wash pulls greener; removed pulls redder than the tool block.
        assert!(add.g > block.g, "added wash should lift green: {add:?}");
        assert!(rem.r > block.r, "removed wash should lift red: {rem:?}");
        assert_ne!(add, rem);
    }
}
