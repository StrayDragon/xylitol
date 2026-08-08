//! Thinking-level editor border colors (c1140) — package-local, no domain dependency.
//!
//! Color ramp synced from pi `coding-agent` themes:
//! `packages/coding-agent/src/modes/interactive/theme/{dark,light}.json`
//! (`thinkingOff` … `thinkingMax`).

use crate::components::editor::Editor;
use crate::terminal_colors::RgbColor;
use crate::theme::paint::fg_rgb;
use crate::theme::palette::Palette;

/// Package-local thinking border palette level, independent of product config strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThinkingBorderLevel {
    Off,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    Xhigh,
    Max,
}

impl ThinkingBorderLevel {
    pub const ALL: [Self; 7] = [
        Self::Off,
        Self::Minimal,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::Xhigh,
        Self::Max,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "minimal" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" => Some(Self::Xhigh),
            "max" => Some(Self::Max),
            _ => None,
        }
    }

    pub fn cycle_next(self) -> Self {
        match self {
            Self::Off => Self::Minimal,
            Self::Minimal => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Xhigh,
            Self::Xhigh => Self::Max,
            Self::Max => Self::Off,
        }
    }
}

/// pi dark.json thinking* hex (resolved vars).
const DARK_THINKING: [RgbColor; 7] = [
    rgb(0x50, 0x50, 0x50), // off ← darkGray
    rgb(0x6e, 0x6e, 0x6e), // minimal
    rgb(0x5f, 0x87, 0xaf), // low
    rgb(0x81, 0xa2, 0xbe), // medium
    rgb(0xb2, 0x94, 0xbb), // high
    rgb(0xd1, 0x83, 0xe8), // xhigh
    rgb(0xff, 0x5f, 0xff), // max
];

/// pi light.json thinking* hex (resolved vars).
const LIGHT_THINKING: [RgbColor; 7] = [
    rgb(0xb0, 0xb0, 0xb0), // off ← lightGray
    rgb(0x76, 0x76, 0x76), // minimal
    rgb(0x54, 0x7d, 0xa7), // low ← blue
    rgb(0x5a, 0x80, 0x80), // medium ← teal
    rgb(0x87, 0x5f, 0x87), // high
    rgb(0x8b, 0x00, 0x8b), // xhigh
    rgb(0xaf, 0x00, 0x5f), // max
];

const fn rgb(r: u8, g: u8, b: u8) -> RgbColor {
    RgbColor { r, g, b }
}

fn thinking_ramp_for(palette: Palette) -> &'static [RgbColor; 7] {
    if palette == Palette::light() {
        &LIGHT_THINKING
    } else {
        // Dark MVP + any custom that is not exactly light().
        &DARK_THINKING
    }
}

fn level_index(level: ThinkingBorderLevel) -> usize {
    match level {
        ThinkingBorderLevel::Off => 0,
        ThinkingBorderLevel::Minimal => 1,
        ThinkingBorderLevel::Low => 2,
        ThinkingBorderLevel::Medium => 3,
        ThinkingBorderLevel::High => 4,
        ThinkingBorderLevel::Xhigh => 5,
        ThinkingBorderLevel::Max => 6,
    }
}

impl Palette {
    /// Border color for a thinking level (pi thinkingOff…thinkingMax ramp).
    pub fn thinking_border_rgb(self, level: ThinkingBorderLevel) -> RgbColor {
        thinking_ramp_for(self)[level_index(level)]
    }

    /// Truecolor paint closure for editor border chrome.
    pub fn thinking_border_paint(
        self,
        level: ThinkingBorderLevel,
    ) -> Box<dyn Fn(&str) -> String + Send> {
        let rgb = self.thinking_border_rgb(level);
        Box::new(move |s| fg_rgb(rgb, s))
    }
}

/// Apply thinking border via [`Editor::set_border_color`] without rebuilding the editor.
pub fn apply_thinking_border(editor: &mut Editor, palette: &Palette, level: ThinkingBorderLevel) {
    editor.set_border_color(palette.thinking_border_paint(level));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::SystemClock;
    use crate::components::editor::{Editor, EditorOptions, EditorTheme};
    use crate::tui::Component;

    #[test]
    fn cycle_returns_to_off_via_max() {
        let mut level = ThinkingBorderLevel::Off;
        let mut saw_max = false;
        for _ in 0..7 {
            level = level.cycle_next();
            if level == ThinkingBorderLevel::Max {
                saw_max = true;
            }
        }
        assert!(saw_max);
        assert_eq!(level, ThinkingBorderLevel::Off);
    }

    #[test]
    fn parse_round_trips() {
        for level in ThinkingBorderLevel::ALL {
            assert_eq!(ThinkingBorderLevel::parse(level.as_str()), Some(level));
        }
        assert_eq!(
            ThinkingBorderLevel::parse("XHIGH"),
            Some(ThinkingBorderLevel::Xhigh)
        );
        assert!(ThinkingBorderLevel::parse("ultra").is_none());
    }

    #[test]
    fn pi_dark_ramp_matches_json() {
        let p = Palette::dark();
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Off),
            rgb(0x50, 0x50, 0x50)
        );
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Minimal),
            rgb(0x6e, 0x6e, 0x6e)
        );
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Low),
            rgb(0x5f, 0x87, 0xaf)
        );
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Medium),
            rgb(0x81, 0xa2, 0xbe)
        );
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::High),
            rgb(0xb2, 0x94, 0xbb)
        );
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Xhigh),
            rgb(0xd1, 0x83, 0xe8)
        );
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Max),
            rgb(0xff, 0x5f, 0xff)
        );
    }

    #[test]
    fn pi_light_ramp_matches_json() {
        let p = Palette::light();
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Off),
            rgb(0xb0, 0xb0, 0xb0)
        );
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Low),
            rgb(0x54, 0x7d, 0xa7)
        );
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Medium),
            rgb(0x5a, 0x80, 0x80)
        );
        assert_eq!(
            p.thinking_border_rgb(ThinkingBorderLevel::Max),
            rgb(0xaf, 0x00, 0x5f)
        );
    }

    #[test]
    fn adjacent_rgb_distinct_on_dark() {
        let p = Palette::dark();
        let colors: Vec<_> = ThinkingBorderLevel::ALL
            .into_iter()
            .map(|l| p.thinking_border_rgb(l))
            .collect();
        for w in colors.windows(2) {
            assert_ne!(w[0], w[1], "adjacent thinking border colors must differ");
        }
    }

    #[test]
    fn paint_is_truecolor_pi_high() {
        let paint = Palette::dark().thinking_border_paint(ThinkingBorderLevel::High);
        let out = paint("x");
        assert!(out.contains("38;2"), "expected truecolor SGR: {out}");
        assert!(
            out.contains("178;148;187"),
            "expected pi high #b294bb in {out}"
        );
    }

    #[test]
    fn apply_thinking_border_swaps_without_rebuild() {
        let mut editor = Editor::new(
            EditorTheme::default(),
            EditorOptions::default(),
            Box::new(SystemClock),
        );
        let p = Palette::dark();
        apply_thinking_border(&mut editor, &p, ThinkingBorderLevel::Medium);
        let mid = editor.render(40);
        apply_thinking_border(&mut editor, &p, ThinkingBorderLevel::High);
        let high = editor.render(40);
        let mid_s = mid.join("\n");
        let high_s = high.join("\n");
        assert_ne!(mid_s, high_s, "border paint should change between levels");
        assert!(
            high_s.contains("38;2"),
            "high border render should include truecolor"
        );
    }
}
