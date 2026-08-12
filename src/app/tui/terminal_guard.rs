//! Terminal lifecycle — panic hook + signal exit flag (ath2).

use std::sync::atomic::{AtomicBool, Ordering};

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
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableBracketedPaste,
        crossterm::cursor::Show
    );
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
    pub fn enter() -> Result<Self, String> {
        install_lifecycle_hooks();
        let mut terminal =
            xylitol_tui::CrosstermTerminal::new().map_err(|e| format!("open terminal: {e}"))?;
        terminal.hide_cursor();
        terminal.start();
        // Product TUI MUST NOT enable mouse capture here by default (Inline).
        // `XYLITOL_TUI_MOUSE` remains package lab/e2e only. ApplicationOwned
        // enters alt-buffer + mouse when the host is constructed with that mode
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
