//! Terminal lifecycle — panic hook + signal exit flag (ath2).

use std::sync::atomic::{AtomicBool, Ordering};

use super::error::TuiSurfaceError;
use xylitol_tui::Terminal;

static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);

/// True after a panic hook has restored the terminal once.
static PANIC_RESTORED: AtomicBool = AtomicBool::new(false);

/// Hooks installed at most once per process.
static HOOKS_INSTALLED: std::sync::Once = std::sync::Once::new();

pub fn exit_requested() -> bool {
    EXIT_REQUESTED.load(Ordering::SeqCst)
}

pub fn request_exit() {
    EXIT_REQUESTED.store(true, Ordering::SeqCst);
}

/// Install panic restore + Unix signal listeners (idempotent).
pub fn install_lifecycle_hooks() {
    HOOKS_INSTALLED.call_once(|| {
        install_panic_hook();
        install_signal_handlers();
    });
}

/// Best-effort restore when no `CrosstermTerminal` handle is available.
pub fn emergency_restore() {
    use std::io::Write;
    // Deep-pop Kitty keyboard stacks (main + whatever screen is active).
    // Without this, Ghostty keeps emitting CSI-u (`c9;1:3u…`) into the shell.
    let _ = std::io::stdout().write_all(b"\x1b[<8u");
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::event::PopKeyboardEnhancementFlags,
        crossterm::event::PopKeyboardEnhancementFlags,
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableBracketedPaste,
        crossterm::cursor::Show
    );
    let _ = std::io::stdout().write_all(b"\x1b[<8u\x1b[>4;0m");
    let _ = std::io::stdout().flush();
}

fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if !PANIC_RESTORED.swap(true, Ordering::SeqCst) {
            emergency_restore();
        }
        prev(info);
    }));
}

fn install_signal_handlers() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        tokio::spawn(async move {
            let Ok(mut sigterm) = signal(SignalKind::terminate()) else {
                return;
            };
            let Ok(mut sighup) = signal(SignalKind::hangup()) else {
                return;
            };
            tokio::select! {
                _ = sigterm.recv() => {}
                _ = sighup.recv() => {}
            }
            request_exit();
            emergency_restore();
        });
    }
}

/// RAII: `terminal.start()` on create, `stop()` on Drop.
pub struct TerminalGuard {
    terminal: Option<xylitol_tui::CrosstermTerminal>,
}

impl TerminalGuard {
    pub fn enter() -> Result<Self, TuiSurfaceError> {
        install_lifecycle_hooks();
        let mut terminal = xylitol_tui::CrosstermTerminal::new()
            .map_err(|e| TuiSurfaceError::io(format!("open terminal: {e}")))?;
        terminal.hide_cursor();
        terminal.start();
        // Product TUI does not enable mouse via TerminalGuard / XYLITOL_TUI_MOUSE.
        // ApplicationOwned sessions enable capture when the host is constructed
        // (`HostSession::new_product_ui_with_meta_mode`) — see c2070 / ath30.
        // Keep DisableMouseCapture in `emergency_restore` for leaked sessions.
        Ok(Self {
            terminal: Some(terminal),
        })
    }

    pub fn take(mut self) -> xylitol_tui::CrosstermTerminal {
        self.terminal
            .take()
            .expect("TerminalGuard::take called twice")
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if let Some(mut term) = self.terminal.take() {
            term.stop();
        }
    }
}
