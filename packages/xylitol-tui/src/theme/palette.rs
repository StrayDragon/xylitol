//! Typed Dark / Light palettes + inherent component-theme builders.

use crate::components::choice_prompt::ChoicePromptTheme;
use crate::components::diff::DiffTheme;
use crate::components::markdown::MarkdownTheme;
use crate::highlight::highlight_code;
use crate::terminal_colors::{RgbColor, TerminalColorScheme};
use crate::theme::paint::{
    bg_rgb, bold, fg_bg_rgb, fg_rgb, italic, strikethrough, underline, word_wash_bg,
};

/// Semantic colors aligned with `src/app/tui/DESIGN.md` (+ Latte light companion).
///
/// Only two built-in flavors. Construct custom values freely; builders are inherent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub on_surface: RgbColor,
    pub muted: RgbColor,
    pub accent: RgbColor,
    pub user: RgbColor,
    pub assistant: RgbColor,
    pub tool: RgbColor,
    pub error: RgbColor,
    pub warning: RgbColor,
    pub success: RgbColor,
    pub diff_added: RgbColor,
    pub diff_removed: RgbColor,
    pub diff_context: RgbColor,
    pub diff_added_bg: RgbColor,
    pub diff_removed_bg: RgbColor,
    pub diff_added_word_bg: RgbColor,
    pub diff_removed_word_bg: RgbColor,
    pub surface: RgbColor,
    pub tool_pending_bg: RgbColor,
    pub tool_success_bg: RgbColor,
    pub tool_error_bg: RgbColor,
    pub user_message_bg: RgbColor,
    /// Inline `$skill` highlight in user messages (`DESIGN.md` `skill-ref`, A10).
    pub skill_ref: RgbColor,
}

impl Palette {
    /// Catppuccin Mocha — `DESIGN.md` product dark MVP.
    pub const fn dark() -> Self {
        Self {
            on_surface: rgb(0xcd, 0xd6, 0xf4),
            muted: rgb(0x6c, 0x70, 0x86),
            accent: rgb(0x89, 0xb4, 0xfa),
            user: rgb(0xcb, 0xa6, 0xf7),
            assistant: rgb(0xcd, 0xd6, 0xf4),
            tool: rgb(0x6c, 0x70, 0x86),
            error: rgb(0xf3, 0x8b, 0xa8),
            warning: rgb(0xf9, 0xe2, 0xaf),
            success: rgb(0xa6, 0xe3, 0xa1),
            diff_added: rgb(0xa6, 0xe3, 0xa1),
            diff_removed: rgb(0xf3, 0x8b, 0xa8),
            diff_context: rgb(0x6c, 0x70, 0x86),
            diff_added_bg: rgb(0x1e, 0x2b, 0x22),
            diff_removed_bg: rgb(0x2b, 0x1e, 0x24),
            diff_added_word_bg: rgb(0x2d, 0x4a, 0x35),
            diff_removed_word_bg: rgb(0x4a, 0x2d, 0x35),
            surface: rgb(0x1e, 0x1e, 0x2e),
            tool_pending_bg: rgb(0x31, 0x32, 0x44),
            tool_success_bg: rgb(0x24, 0x35, 0x2a),
            tool_error_bg: rgb(0x35, 0x24, 0x28),
            user_message_bg: rgb(0x31, 0x32, 0x44),
            skill_ref: rgb(0xcb, 0xa6, 0xf7),
        }
    }

    /// Catppuccin Latte — light companion (`DESIGN.md` `colors_light`).
    pub const fn light() -> Self {
        Self {
            on_surface: rgb(0x4c, 0x4f, 0x69),
            muted: rgb(0x9c, 0xa0, 0xb0),
            accent: rgb(0x1e, 0x66, 0xf5),
            user: rgb(0x88, 0x39, 0xef),
            assistant: rgb(0x4c, 0x4f, 0x69),
            tool: rgb(0x9c, 0xa0, 0xb0),
            error: rgb(0xd2, 0x0f, 0x39),
            warning: rgb(0xdf, 0x8e, 0x1d),
            success: rgb(0x40, 0xa0, 0x2b),
            diff_added: rgb(0x40, 0xa0, 0x2b),
            diff_removed: rgb(0xd2, 0x0f, 0x39),
            diff_context: rgb(0x9c, 0xa0, 0xb0),
            diff_added_bg: rgb(0xdd, 0xe8, 0xdc),
            diff_removed_bg: rgb(0xe8, 0xdc, 0xe0),
            diff_added_word_bg: rgb(0xc5, 0xdb, 0xc4),
            diff_removed_word_bg: rgb(0xdb, 0xc5, 0xca),
            surface: rgb(0xef, 0xf1, 0xf5),
            tool_pending_bg: rgb(0xcc, 0xd0, 0xda),
            tool_success_bg: rgb(0xdc, 0xe8, 0xd8),
            tool_error_bg: rgb(0xe8, 0xdc, 0xe0),
            user_message_bg: rgb(0xcc, 0xd0, 0xda),
            skill_ref: rgb(0x88, 0x39, 0xef),
        }
    }

    pub fn fg(&self, color: RgbColor, s: &str) -> String {
        fg_rgb(color, s)
    }

    pub fn bg(&self, color: RgbColor, s: &str) -> String {
        bg_rgb(color, s)
    }

    /// Markdown theme (heading grades + emphasis).
    pub fn markdown_theme(&self) -> MarkdownTheme {
        let accent = self.accent;
        let on_surface = self.on_surface;
        let muted = self.muted;
        let success = self.success;
        let warning = self.warning;

        MarkdownTheme {
            heading: Box::new(move |level, s| match level {
                1 | 2 => fg_rgb(accent, &bold(&underline(s))),
                3 | 4 => fg_rgb(on_surface, &bold(s)),
                _ => fg_rgb(muted, s),
            }),
            link: Box::new(move |s| fg_rgb(accent, s)),
            link_url: Box::new(move |s| underline(&fg_rgb(accent, s))),
            code: Box::new(move |s| fg_rgb(success, s)),
            code_block: Box::new(|s| s.to_string()),
            code_block_border: Box::new(|_| String::new()),
            quote: Box::new(move |s| fg_rgb(muted, &italic(s))),
            quote_border: Box::new(move |s| fg_rgb(muted, s)),
            hr: Box::new(move |s| fg_rgb(muted, s)),
            list_bullet: Box::new(|s| s.to_string()),
            bold: Box::new(move |s| bold(&fg_rgb(accent, s))),
            italic: Box::new(move |s| italic(&fg_rgb(warning, s))),
            strikethrough: Box::new(move |s| strikethrough(&fg_rgb(muted, s))),
            underline: Box::new(underline),
            highlight_code: Some(Box::new(highlight_code)),
            code_block_indent: Some("  ".into()),
        }
    }

    /// Diff theme: row tint + stronger word tint (restore row bg, not `49m`).
    pub fn diff_theme(&self) -> DiffTheme {
        let added = self.diff_added;
        let removed = self.diff_removed;
        let context = self.diff_context;
        let added_bg = self.diff_added_bg;
        let removed_bg = self.diff_removed_bg;
        let added_word = self.diff_added_word_bg;
        let removed_word = self.diff_removed_word_bg;

        DiffTheme {
            added: Box::new(move |s| fg_rgb(added, s)),
            removed: Box::new(move |s| fg_rgb(removed, s)),
            context: Box::new(move |s| fg_rgb(context, s)),
            gutter: Box::new(move |s| fg_rgb(context, s)),
            meta: Box::new(move |s| fg_rgb(context, s)),
            word_change_added: Box::new(move |s| fg_bg_rgb(added, added_word, added_bg, s)),
            word_change_removed: Box::new(move |s| fg_bg_rgb(removed, removed_word, removed_bg, s)),
            added_line_bg: Box::new(move |s| bg_rgb(added_bg, s)),
            removed_line_bg: Box::new(move |s| bg_rgb(removed_bg, s)),
            highlight_line: Box::new(|s| s.to_string()),
        }
    }

    /// Diff inside a `tool-*-bg` wash (pi edit path / `design/diff-block.md` §Edit 一体块).
    ///
    /// Polarity stays on fg; word spans use [`word_wash_bg`] toward red/green and restore
    /// to `block_bg`. **No** `diff-*-bg` row tints — the expandable shell owns the wash.
    pub fn diff_theme_on_block(&self, block_bg: RgbColor) -> DiffTheme {
        let added = self.diff_added;
        let removed = self.diff_removed;
        let context = self.diff_context;
        let word_added_bg = word_wash_bg(block_bg, added);
        let word_removed_bg = word_wash_bg(block_bg, removed);
        DiffTheme {
            added: Box::new(move |s| fg_rgb(added, s)),
            removed: Box::new(move |s| fg_rgb(removed, s)),
            context: Box::new(move |s| fg_rgb(context, s)),
            gutter: Box::new(move |s| fg_rgb(context, s)),
            meta: Box::new(move |s| fg_rgb(context, s)),
            word_change_added: Box::new(move |s| fg_bg_rgb(added, word_added_bg, block_bg, s)),
            word_change_removed: Box::new(move |s| {
                fg_bg_rgb(removed, word_removed_bg, block_bg, s)
            }),
            added_line_bg: Box::new(|s| s.to_string()),
            removed_line_bg: Box::new(|s| s.to_string()),
            highlight_line: Box::new(|s| s.to_string()),
        }
    }

    /// ChoicePrompt chrome — accent select (not reverse); product Ask uses fixed left rail.
    /// Demo MAY clear `rail` for wash/pi contrast via `/entry-style`.
    pub fn choice_prompt_theme(&self) -> ChoicePromptTheme {
        let accent = self.accent;
        let on_surface = self.on_surface;
        let muted = self.muted;

        ChoicePromptTheme {
            title: Box::new(move |s| bold(&fg_rgb(on_surface, s))),
            prompt: Box::new(move |s| fg_rgb(on_surface, s)),
            selected: Box::new(move |s| bold(&fg_rgb(accent, s))),
            normal: Box::new(move |s| fg_rgb(on_surface, s)),
            muted: Box::new(move |s| fg_rgb(muted, s)),
            tab_active: Box::new(move |s| bold(&fg_rgb(accent, s))),
            tab_idle: Box::new(move |s| fg_rgb(muted, s)),
            hint: Box::new(move |s| fg_rgb(muted, s)),
            rail: Some(accent),
        }
    }
}

impl From<TerminalColorScheme> for Palette {
    fn from(scheme: TerminalColorScheme) -> Self {
        match scheme {
            TerminalColorScheme::Dark => Self::dark(),
            TerminalColorScheme::Light => Self::light(),
        }
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> RgbColor {
    RgbColor { r, g, b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_differ() {
        let d = Palette::dark();
        let l = Palette::light();
        assert_ne!(d.on_surface, l.on_surface);
        assert_ne!(d.accent, l.accent);
        assert_ne!(d.tool_pending_bg, l.tool_pending_bg);
    }

    #[test]
    fn from_scheme() {
        assert_eq!(Palette::from(TerminalColorScheme::Dark), Palette::dark());
        assert_eq!(Palette::from(TerminalColorScheme::Light), Palette::light());
    }

    #[test]
    fn markdown_uses_scheme_accent() {
        let dark = Palette::dark().markdown_theme();
        assert!((dark.heading)(1, "T").contains("38;2;137;180;250"));
        let light = Palette::light().markdown_theme();
        assert!((light.heading)(1, "T").contains("38;2;30;102;245"));
    }

    #[test]
    fn diff_line_bg_resets_49() {
        let line = (Palette::dark().diff_theme().added_line_bg)("x");
        assert!(line.contains("48;2;30;43;34"));
        assert!(line.contains("\x1b[49m"));
    }
}
