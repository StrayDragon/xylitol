//! Terminal abstraction + crossterm-backed implementation with keyboard
//! protocol negotiation (c410).
//!
//! Aligns with pi-tui's `terminal.ts` (531 lines): Kitty keyboard protocol
//! probe at start, modifyOtherKeys fallback, OSC title/progress, and
//! pre-exit stdin drain. The key architectural difference (pi owns the raw
//! stdin byte stream; xy delegates input to crossterm's `event::read`) is
//! resolved by route A (design.md): xy pushes Kitty enhancement flags via
//! crossterm at start and lets keys.rs's incremental matcher handle both
//! Kitty and legacy sequences, rather than intercepting the Kitty response.

use crossterm::terminal;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::keys::set_kitty_protocol_active;

// ── Keyboard protocol constants (pi terminal.ts:11-17) ─────────────────────

/// Kitty flags to request: disambiguate escape codes (1) + report event types
/// (2) + report alternate keys (4) = 7. Matches pi DESIRED_KITTY_KEYBOARD_PROTOCOL_FLAGS.
const KITTY_FLAGS_REQUEST: u8 = 7;

/// `CSI >{flags}u` — push Kitty enhancement flags (pi sends `>7u`).
#[allow(dead_code)] // documented for clarity; inlined in KITTY_KEYBOARD_PROTOCOL_QUERY
const KITTY_PUSH_SEQUENCE: &str = "\x1b[>7u";
/// `CSI ?u` — pop current Kitty flags (pi sends as part of the query so a
/// terminal that doesn't know `>Nu` still answers the trailing DA).
#[allow(dead_code)] // documented for clarity; inlined in KITTY_KEYBOARD_PROTOCOL_QUERY
const KITTY_POP_QUERY_SEQUENCE: &str = "\x1b[?u";
/// `CSI <u` — pop Kitty enhancement flags (restore prior state).
const KITTY_POP_SEQUENCE: &str = "\x1b[<u";
/// `CSI c` — device attributes sentinel (terminals respond with `CSI ?..c`).
#[allow(dead_code)] // documented for clarity; inlined in KITTY_KEYBOARD_PROTOCOL_QUERY
const DA_QUERY_SEQUENCE: &str = "\x1b[c";
/// The combined query pi sends at start: push flags, pop-query, then DA.
const KITTY_KEYBOARD_PROTOCOL_QUERY: &str = "\x1b[>7u\x1b[?u\x1b[c";

/// modifyOtherKeys mode 2 enable / reset (pi terminal.ts:322/328). Retained for
/// parity with pi and future route-B negotiation; route A pushes Kitty
/// unconditionally so these are not emitted at runtime today.
#[allow(dead_code)]
const MODIFY_OTHER_KEYS_ENABLE: &str = "\x1b[>4;2m";
const MODIFY_OTHER_KEYS_DISABLE: &str = "\x1b[>4;0m";

/// Bracketed paste enable/disable (pi terminal.ts:147/412).
const BRACKETED_PASTE_ENABLE: &str = "\x1b[?2004h";
const BRACKETED_PASTE_DISABLE: &str = "\x1b[?2004l";

/// Terminal trait - abstract output/lifecycle interface for the TUI.
///
/// `start`/`stop` own the raw-mode + protocol negotiation lifecycle; the
/// output methods mirror pi's Terminal interface. Test doubles (VirtualTerminal)
/// implement these as no-ops.
pub trait Terminal {
    fn write(&mut self, data: &str);
    fn columns(&self) -> u16;
    fn rows(&self) -> u16;
    fn hide_cursor(&mut self);
    fn show_cursor(&mut self);
    fn clear_line(&mut self);
    fn clear_from_cursor(&mut self);
    fn clear_screen(&mut self);
    fn flush(&mut self);
    /// Re-query the terminal size from the underlying backend. Called after a
    /// resize event so the cached `columns`/`rows` stay in sync.
    fn refresh_size(&mut self) {}

    // ── c410: lifecycle + protocol (defaults no-op for test doubles) ──

    /// Enter raw mode, enable bracketed paste, and negotiate keyboard
    /// enhancement protocols (Kitty push + modifyOtherKeys fallback). Called
    /// once at TUI start, before the first render.
    fn start(&mut self) {}
    /// Tear down: drain residual input, disable protocols, restore non-raw
    /// mode. Called once at TUI exit. MUST be safe to call without a prior
    /// `start` (no-op then).
    fn stop(&mut self) {}

    // ── c410: OSC / cursor helpers (defaults no-op for test doubles) ──

    /// Set the terminal window title via OSC 0;... BEL.
    fn set_title(&mut self, _title: &str) {}
    /// Toggle the taskbar progress indicator via OSC 9;4 (xterm/conpty).
    fn set_progress(&mut self, _active: bool) {}
    /// Move the cursor by `lines` rows (negative = up, positive = down).
    fn move_by(&mut self, _lines: i32) {}
}

/// Parse a Kitty keyboard-protocol response `CSI ?Nu` into the negotiated
/// flags `N`. Returns `None` for non-matching sequences (e.g. a
/// device-attributes response `CSI ?62;4;52c`). A zero-flags response
/// (`CSI ?0u`) returns `Some(0)`, distinct from no match.
///
/// Mirrors pi `parseKeyboardProtocolNegotiationSequence`. Pure / unit-testable.
pub fn parse_kitty_flags(seq: &str) -> Option<u32> {
    // Match `\x1b[?<digits>u` exactly.
    let rest = seq.strip_prefix("\x1b[?")?;
    let digits = rest.strip_suffix('u')?;
    digits.parse::<u32>().ok()
}

// ── CrosstermTerminal ──────────────────────────────────────────────────────

/// Crossterm-backed terminal. Owns raw-mode + keyboard-protocol negotiation
/// state (c410): `start()` pushes Kitty flags (or falls back to
/// modifyOtherKeys) and `stop()` reverses whichever was enabled.
pub struct CrosstermTerminal {
    columns: u16,
    rows: u16,
    /// Whether `start()` successfully pushed Kitty enhancement flags. Drives
    /// the pop on `stop()`.
    kitty_pushed: bool,
    /// Whether modifyOtherKeys mode 2 is currently enabled.
    modify_other_keys_active: bool,
    /// Whether `start()` has run (so `stop()` knows to tear down).
    started: bool,
}

impl Default for CrosstermTerminal {
    fn default() -> Self {
        Self::new().expect("CrosstermTerminal::new requires a tty")
    }
}

impl CrosstermTerminal {
    pub fn new() -> io::Result<Self> {
        let (cols, rows) = terminal::size()?;
        Ok(Self {
            columns: cols,
            rows,
            kitty_pushed: false,
            modify_other_keys_active: false,
            started: false,
        })
    }

    pub fn refresh_size_impl(&mut self) {
        if let Ok((cols, rows)) = terminal::size() {
            self.columns = cols;
            self.rows = rows;
        }
    }

    /// Probe for Kitty keyboard protocol support and enable it (or fall back
    /// to modifyOtherKeys). Route A (design.md): xy pushes Kitty enhancement
    /// flags via crossterm unconditionally and sets the keys.rs global active,
    /// because keys.rs's matcher handles both Kitty and legacy sequences — a
    /// terminal that ignores the push simply keeps emitting legacy sequences.
    fn negotiate_keyboard_protocol(&mut self) {
        // Emit pi's combined query so the terminal enters enhancement mode if
        // it understands Kitty (push flags, pop-query, then DA sentinel).
        self.write_raw(KITTY_KEYBOARD_PROTOCOL_QUERY);
        // Ask crossterm to push the flags too (it emits the same CSI >Nu on
        // unix; on Windows it may translate differently). We do NOT rely on
        // crossterm's return value to detect support — execute! only fails on
        // I/O errors, not "terminal doesn't understand".
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::event::PushKeyboardEnhancementFlags(
                crossterm::event::KeyboardEnhancementFlags::from_bits_truncate(KITTY_FLAGS_REQUEST),
            )
        );
        // keys.rs matches legacy sequences even when the kitty flag is set, so
        // setting this active unconditionally is safe: terminals that ignore
        // the push keep working via the legacy branches.
        set_kitty_protocol_active(true);
        self.kitty_pushed = true;
    }

    #[allow(dead_code)] // parity with pi; route A does not invoke this today
    fn enable_modify_other_keys(&mut self) {
        if self.kitty_pushed || self.modify_other_keys_active {
            return;
        }
        self.write_raw(MODIFY_OTHER_KEYS_ENABLE);
        self.modify_other_keys_active = true;
    }

    fn disable_modify_other_keys(&mut self) {
        if !self.modify_other_keys_active {
            return;
        }
        self.write_raw(MODIFY_OTHER_KEYS_DISABLE);
        self.modify_other_keys_active = false;
    }

    /// Write raw bytes directly to stdout (bypasses crossterm's Command
    /// machinery for sequences crossterm doesn't expose, e.g. OSC 9;4).
    fn write_raw(&mut self, data: &str) {
        let _ = io::stdout().write_all(data.as_bytes());
    }

    /// Drain residual stdin events for up to `max_ms`, exiting early after
    /// `idle_ms` of no input. Prevents Kitty key-release events from leaking
    /// to the parent shell over slow SSH (pi terminal.ts:368). Called by
    /// `stop()` AFTER the Kitty pop so late releases do not generate new
    /// escape sequences.
    fn drain_input(&mut self, max_ms: u64, idle_ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(max_ms);
        let mut last_seen = Instant::now();
        while Instant::now() < deadline {
            let idle_deadline = last_seen + Duration::from_millis(idle_ms);
            if Instant::now() >= idle_deadline {
                break;
            }
            // poll for a short slice; consume any waiting event.
            if crossterm::event::poll(Duration::from_millis(idle_ms)).unwrap_or(false) {
                let _ = crossterm::event::read();
                last_seen = Instant::now();
            }
        }
    }
}

impl Terminal for CrosstermTerminal {
    fn write(&mut self, data: &str) {
        self.write_raw(data);
    }

    fn columns(&self) -> u16 {
        self.columns
    }

    fn rows(&self) -> u16 {
        self.rows
    }

    fn hide_cursor(&mut self) {
        let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Hide);
    }

    fn show_cursor(&mut self) {
        let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Show);
    }

    fn clear_line(&mut self) {
        self.write_raw("\x1b[K");
    }

    fn clear_from_cursor(&mut self) {
        self.write_raw("\x1b[J");
    }

    fn clear_screen(&mut self) {
        self.write_raw("\x1b[2J\x1b[H");
    }

    fn flush(&mut self) {
        let _ = io::stdout().flush();
    }

    fn refresh_size(&mut self) {
        self.refresh_size_impl();
    }

    fn start(&mut self) {
        if self.started {
            return;
        }
        self.started = true;
        let _ = terminal::enable_raw_mode();
        self.write_raw(BRACKETED_PASTE_ENABLE);
        self.negotiate_keyboard_protocol();
    }

    fn stop(&mut self) {
        if !self.started {
            return;
        }
        self.started = false;

        // Disable bracketed paste first.
        self.write_raw(BRACKETED_PASTE_DISABLE);

        // Pop Kitty enhancement flags BEFORE draining, so late key releases
        // do not generate new Kitty escape sequences.
        if self.kitty_pushed {
            self.write_raw(KITTY_POP_SEQUENCE);
            let _ =
                crossterm::execute!(io::stdout(), crossterm::event::PopKeyboardEnhancementFlags);
            set_kitty_protocol_active(false);
            self.kitty_pushed = false;
        }
        self.disable_modify_other_keys();

        // Drain residual input (slow-SSH key-release leak guard).
        self.drain_input(200, 30);

        let _ = terminal::disable_raw_mode();
        self.show_cursor();
        self.flush();
    }

    fn set_title(&mut self, title: &str) {
        // OSC 0;title BEL — set both icon and window title (OSC 2 is equivalent
        // on xterm; OSC 0 is the broader convention pi uses).
        let _ = write!(io::stdout(), "\x1b]0;{title}\x07");
        let _ = io::stdout().flush();
    }

    fn set_progress(&mut self, active: bool) {
        // OSC 9;4;3 = indeterminate active, OSC 9;4;0 = clear. Single emission
        // per state change (no 1s keepalive like pi — the TUI clears on stop).
        if active {
            self.write_raw("\x1b]9;4;3\x07");
        } else {
            self.write_raw("\x1b]9;4;0\x07");
        }
    }

    fn move_by(&mut self, lines: i32) {
        if lines > 0 {
            let _ = write!(io::stdout(), "\x1b[{}B", lines);
        } else if lines < 0 {
            let _ = write!(io::stdout(), "\x1b[{}A", -lines);
        }
        let _ = io::stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_kitty_flags (spec tp02) ──

    #[test]
    fn parse_nonzero_flags() {
        assert_eq!(parse_kitty_flags("\x1b[?7u"), Some(7));
        assert_eq!(parse_kitty_flags("\x1b[?1u"), Some(1));
    }

    #[test]
    fn parse_zero_flags() {
        // Zero is distinct from None: the terminal understood the query but
        // supports no Kitty flags (drives the modifyOtherKeys fallback).
        assert_eq!(parse_kitty_flags("\x1b[?0u"), Some(0));
    }

    #[test]
    fn parse_nonmatch_device_attributes() {
        // A DA response like CSI ?62;4;52c is NOT a Kitty flags response.
        assert_eq!(parse_kitty_flags("\x1b[?62;4;52c"), None);
    }

    #[test]
    fn parse_garbage() {
        assert_eq!(parse_kitty_flags("abc"), None);
        assert_eq!(parse_kitty_flags(""), None);
        assert_eq!(parse_kitty_flags("\x1b[7u"), None); // missing '?'
        assert_eq!(parse_kitty_flags("\x1b[?7"), None); // missing 'u'
    }
}
