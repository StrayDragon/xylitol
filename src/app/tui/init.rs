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

// ── panic safety (c355) ─────────────────────────────────────────────

use std::sync::Once;

static HOOK_INSTALLED: Once = Once::new();

/// Install a panic hook that restores the terminal before the default hook
/// runs (c355: 补齐 spec tui15 的 panic 安全缺口).
///
/// `InlineTerminal::Drop` restores the terminal on normal exit, but Drop has
/// two blind spots:
/// 1. `panic = "abort"` — Drop does not run; the process terminates with raw
///    mode still on, leaving the terminal broken.
/// 2. A panic in a spawned task (`spawn_blocking` keyboard reader, `tokio::spawn`
///    event drain) unwinds that child's stack, which has no `InlineTerminal` —
///    so its Drop never fires.
///
/// `std::panic::set_hook` covers both: the hook runs on every panic regardless
/// of unwind/abort, and is global (not tied to any stack frame). Idempotent via
/// `Once` — safe to call multiple times (e.g. across test runs or re-entry).
pub fn install_terminal_restore_hook() {
    HOOK_INSTALLED.call_once(|| {
        let original = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Restore the terminal FIRST (best-effort), then chain to the
            // original hook so the panic message prints to a usable terminal.
            let _ = crossterm::terminal::disable_raw_mode();
            original(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::install_terminal_restore_hook;

    #[test]
    fn hook_install_is_callable_and_idempotent() {
        // Calling multiple times must not panic (Once guards against stacking).
        install_terminal_restore_hook();
        install_terminal_restore_hook();
    }
}
