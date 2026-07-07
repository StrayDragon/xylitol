//! Panic-restore hook for the TUI (c355).
//!
//! c399 stage 4 removed the ratatui terminal-init helpers that used to live
//! here (`DefaultTerminal`, `try_init_with_options`, `restore`): raw-mode
//! lifecycle is now owned by
//! [`crate::app::tui::engine::terminal::ProcessTerminal`] (start/stop). What
//! remains is the panic-restore hook, which runs on every panic regardless of
//! unwind/abort and is global (not tied to any stack frame).

use std::sync::Once;

static HOOK_INSTALLED: Once = Once::new();

/// Install a panic hook that restores the terminal before the default hook
/// runs (c355: 补齐 spec tui15 的 panic 安全缺口).
///
/// `ProcessTerminal::Drop` restores the terminal on normal exit, but Drop has
/// two blind spots:
/// 1. `panic = "abort"` — Drop does not run; the process terminates with raw
///    mode still on, leaving the terminal broken.
/// 2. A panic in a spawned task (`spawn_blocking` keyboard reader, `tokio::spawn`
///    event drain) unwinds that child's stack, which has no `ProcessTerminal` —
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
