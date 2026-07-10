//! Product TUI host — event step machine (testable without a real TTY).

use xylitol_tui::{InputEvent, RenderError, TUI, Terminal};

/// Minimum usable terminal size (ath4).
pub const MIN_COLS: u16 = 40;
pub const MIN_ROWS: u16 = 6;

/// Friendly prompt when the terminal is too small.
pub const TOO_SMALL_HINT: &str = "请放大终端";

/// Whether the terminal is below the product minimum.
pub fn is_too_small(cols: u16, rows: u16) -> bool {
    cols < MIN_COLS || rows < MIN_ROWS
}

/// Events the host loop can process. Production maps crossterm → these;
/// harnesses inject them directly (ath5).
#[derive(Debug, Clone)]
pub enum HostEvent {
    Input(InputEvent),
    Resize { cols: u16, rows: u16 },
    Tick,
    Quit,
}

/// Layout mode after applying size policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Shell,
    TooSmall,
}

/// One host-driven TUI session. Does **not** call [`TUI::start`](xylitol_tui::TUI::start).
pub struct HostSession<T: Terminal> {
    pub tui: TUI<T>,
    mode: LayoutMode,
    quit: bool,
    /// Rebuild root children when mode flips.
    rebuild: Box<dyn FnMut(LayoutMode) -> Vec<Box<dyn xylitol_tui::Component>>>,
}

impl<T: Terminal> HostSession<T> {
    /// Create a session. `rebuild(mode)` supplies the root component tree.
    pub fn new(
        terminal: T,
        mut rebuild: impl FnMut(LayoutMode) -> Vec<Box<dyn xylitol_tui::Component>> + 'static,
    ) -> Self {
        let cols = terminal.columns();
        let rows = terminal.rows();
        let mode = if is_too_small(cols, rows) {
            LayoutMode::TooSmall
        } else {
            LayoutMode::Shell
        };
        let mut tui = TUI::new(terminal);
        for child in rebuild(mode) {
            tui.add_child(child);
        }
        tui.set_focus(Some(0));
        tui.request_render(true);
        Self {
            tui,
            mode,
            quit: false,
            rebuild: Box::new(rebuild),
        }
    }

    pub fn mode(&self) -> LayoutMode {
        self.mode
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn request_quit(&mut self) {
        self.quit = true;
    }

    /// Apply one host event and attempt a throttled render.
    pub fn step(&mut self, event: HostEvent) -> Result<(), String> {
        match event {
            HostEvent::Quit => {
                self.quit = true;
            }
            HostEvent::Tick => {
                if self.mode == LayoutMode::Shell {
                    let _ = self.tui.idle_tick();
                }
                self.tui.request_render(false);
            }
            HostEvent::Resize { cols, rows } => {
                self.tui.terminal.set_size_hint(cols, rows);
                self.tui.terminal.refresh_size();
                self.sync_layout_from_terminal();
                self.tui.request_render(true);
            }
            HostEvent::Input(input) => {
                if self.mode == LayoutMode::Shell {
                    self.tui.dispatch_event(input);
                }
                self.tui.request_render(false);
            }
        }

        match self.tui.try_render() {
            Ok(_) => Ok(()),
            Err(RenderError { .. }) => {
                // Extreme width invariant failure: signal quit so the outer
                // loop can restore the terminal and exit cleanly (ath4).
                self.quit = true;
                Err("render failed: terminal too extreme; restoring and exiting".into())
            }
        }
    }

    /// Force an immediate frame (bypasses throttle) — useful after mount.
    pub fn render_now(&mut self) -> Result<(), String> {
        self.tui.render_now().map(|_| ()).map_err(|e| e.to_string())
    }

    /// Re-evaluate layout from the terminal's current size (after an external
    /// size mutation on a test double).
    pub fn sync_layout_from_terminal(&mut self) {
        let cols = self.tui.terminal.columns();
        let rows = self.tui.terminal.rows();
        let next = if is_too_small(cols, rows) {
            LayoutMode::TooSmall
        } else {
            LayoutMode::Shell
        };
        if next != self.mode {
            self.mode = next;
            self.tui.clear_children();
            for child in (self.rebuild)(self.mode) {
                self.tui.add_child(child);
            }
            self.tui.set_focus(Some(0));
        }
    }
}
