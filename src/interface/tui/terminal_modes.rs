//! Small terminal mode toggles not covered by crossterm helpers.
//!
//! Codex-style TUI behavior:
//! - In inline viewport mode, we prefer the terminal's native scrollback.
//! - Some environments can leave "alternate scroll" enabled, which translates mouse wheel
//!   events into Up/Down keypresses (breaking normal scrolling and sometimes triggering history
//!   navigation). We explicitly disable it while the app runs.

use std::fmt;

use crossterm::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DisableAlternateScroll;

impl Command for DisableAlternateScroll {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        // xterm "Alternate Scroll Mode" (1007): disable.
        write!(f, "\x1b[?1007l")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        Err(std::io::Error::other(
            "tried to execute DisableAlternateScroll using WinAPI; use ANSI instead",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        true
    }
}
