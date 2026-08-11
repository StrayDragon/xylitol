//! Dual interaction modes for xylitol-tui (c2070 / `package-tui-interaction-modes`).
//!
//! - [`InteractionMode::Inline`] (Mode A): differential paint into the main buffer /
//!   terminal scrollback; selection is emulator-owned.
//! - [`InteractionMode::ApplicationOwned`] (Mode B): application viewport + app
//!   selection; enter path SHOULD use the terminal alternate buffer.

/// Session interaction ownership (one primary mode per session).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractionMode {
    /// Mode A — inline / emulator-owned selection + terminal scrollback.
    #[default]
    Inline,
    /// Mode B — application-owned viewport + in-app selection (alt-screen preferred).
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
    fn default_is_inline_mode_a() {
        assert_eq!(InteractionMode::default(), InteractionMode::Inline);
        assert!(InteractionMode::Inline.is_inline());
        assert!(!InteractionMode::ApplicationOwned.is_inline());
    }
}
