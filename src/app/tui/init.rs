//! Terminal initialization — local replacement for the umbrella ratatui::init.
//!
//! We depend on `ratatui-core` + `ratatui-crossterm` directly (c341), so the
//! umbrella convenience helpers (`try_init_with_options` / `restore` /
//! `DefaultTerminal`) are not available. This module re-implements the inline
//! subset we need (~15 lines of logic). The umbrella's panic hook
//! (`set_panic_hook`) is intentionally NOT included here — that is c355's job.
//!
//! Raw-mode lifecycle stays explicit: the caller enables raw mode before
//! [`try_init_with_options`] and disables it (via [`restore`] or
//! `InlineTerminal::drop`) on exit. This mirrors the pre-c341 code path and
//! keeps the enable/disable pair visible in one place.

use std::io::{self, Stdout};

use ratatui_core::terminal::{Terminal, TerminalOptions};
use ratatui_crossterm::CrosstermBackend;

/// The inline terminal type (crossterm backend over stdout).
pub type DefaultTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Create an inline terminal with the given options.
///
/// The caller MUST have enabled raw mode beforehand (we do NOT enable it here,
/// to keep the enable/disable pair colocated in `InlineTerminal`).
pub fn try_init_with_options(options: TerminalOptions) -> io::Result<DefaultTerminal> {
    let backend = CrosstermBackend::new(io::stdout());
    Terminal::with_options(backend, options)
}

/// Restore the terminal: disable raw mode (best-effort).
///
/// Inline mode never enters the alternate screen, so we do NOT emit
/// `LeaveAlternateScreen` here (it would be a harmless no-op, but omitting it
/// keeps the intent clear). The umbrella's `restore` emits both; we drop the
/// alt-screen half because spec tui1 forbids alt-screen entirely.
pub fn restore() {
    let _ = crossterm::terminal::disable_raw_mode();
}
