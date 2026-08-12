//! Dual interaction modes for xylitol-tui (c2070 / `package-tui-interaction-modes`).
//!
//! Prefer these **type names** in code and host docs (not informal "Mode A/B"):
//!
//! - [`InteractionMode::Inline`]: differential paint into the main buffer /
//!   terminal scrollback; selection is emulator-owned.
//! - [`InteractionMode::ApplicationOwned`]: application viewport + app
//!   selection; enter path SHOULD use the terminal **alternate screen**.
//!
//! Informal shorthand "Mode A" / "Mode B" in older notes maps to Inline /
//! ApplicationOwned respectively — new code and comments SHOULD use the enum
//! variants (or "inline" / "alt-screen") so the meaning stays obvious.

/// Session interaction ownership (one primary mode per session).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractionMode {
    /// Inline TUI — main buffer / emulator-owned selection + terminal scrollback.
    #[default]
    Inline,
    /// Application-owned TUI — app viewport + in-app selection (alt-screen preferred).
    ApplicationOwned,
}

impl InteractionMode {
    pub const fn is_application_owned(self) -> bool {
        matches!(self, Self::ApplicationOwned)
    }

    pub const fn is_inline(self) -> bool {
        matches!(self, Self::Inline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_inline() {
        assert_eq!(InteractionMode::default(), InteractionMode::Inline);
        assert!(InteractionMode::Inline.is_inline());
        assert!(!InteractionMode::ApplicationOwned.is_inline());
    }
}
