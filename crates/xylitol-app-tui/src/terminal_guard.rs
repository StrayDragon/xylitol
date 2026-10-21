use std::sync::atomic::{AtomicBool, Ordering};

use super::error::TuiSurfaceError;
use xylitol_tui::Terminal;

static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);

static PANIC_RESTORED: AtomicBool = AtomicBool::new(false);

static HOOKS_INSTALLED: std::sync::Once = std::sync::Once::new();

pub fn exit_requested() -> bool {
    EXIT_REQUESTED.load(Ordering::SeqCst)
}

pub fn request_exit() {
    EXIT_REQUESTED.store(true, Ordering::SeqCst);
}

pub fn install_lifecycle_hooks() {
    HOOKS_INSTALLED.call_once(|| {
        install_panic_hook();
        install_signal_handlers();
    });
}

pub fn emergency_restore() {
    use std::io::Write;
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

pub struct TerminalGuard {
    terminal: Option<xylitol_tui::CrosstermTerminal>,
}

impl TerminalGuard {
    pub fn enter() -> Result<Self, TuiSurfaceError> {
        install_lifecycle_hooks();
        let mut terminal = xylitol_tui::CrosstermTerminal::new()
            .map_err(|e| TuiSurfaceError::io("open terminal", e))?;
        terminal.hide_cursor();
        terminal.start();
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
