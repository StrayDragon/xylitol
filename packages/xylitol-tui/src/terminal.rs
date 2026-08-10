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

use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
};
use crossterm::{cursor, execute};
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

    /// Host hint: set cached size from a resize event without querying the OS.
    /// Real terminals SHOULD apply this — `refresh_size` alone can lag.
    fn set_size_hint(&mut self, _cols: u16, _rows: u16) {}

    // ── c410: lifecycle + protocol (defaults no-op for test doubles) ──

    /// Enter raw mode, enable bracketed paste, and negotiate keyboard
    /// enhancement protocols (Kitty push + modifyOtherKeys fallback). Called
    /// once at TUI start, before the first render.
    fn start(&mut self) {}
    /// Tear down: drain residual input, disable protocols, restore non-raw
    /// mode. Called once at TUI exit. MUST be safe to call without a prior
    /// `start` (no-op then).
    fn stop(&mut self) {}

    /// Opt in to mouse reporting (`EnableMouseCapture`). Default off.
    /// Implementations SHOULD remember desire across `stop`/`start` (suspend).
    fn enable_mouse_capture(&mut self) {}
    /// Opt out of mouse reporting. `stop` MUST disable if currently active.
    fn disable_mouse_capture(&mut self) {}
    /// Whether mouse capture is currently active on the TTY (not merely desired).
    fn mouse_capture_active(&self) -> bool {
        false
    }

    /// Enter the terminal alternate buffer (`CSI ?1049h` / crossterm EnterAlternateScreen).
    /// Used by Mode B ([`crate::InteractionMode::ApplicationOwned`]). Default no-op.
    fn enter_alternate_screen(&mut self) {}
    /// Leave the alternate buffer. `stop` MUST leave if currently active.
    fn leave_alternate_screen(&mut self) {}
    /// Whether the alternate buffer is currently entered.
    fn alternate_screen_active(&self) -> bool {
        false
    }

    // ── c410: OSC / cursor helpers (defaults no-op for test doubles) ──

    /// Set the terminal window title via OSC 0;... BEL.
    fn set_title(&mut self, _title: &str) {}
    /// Toggle the taskbar progress indicator via OSC 9;4 (xterm/conpty).
    fn set_progress(&mut self, _active: bool) {}
    /// Move the cursor by `lines` rows (negative = up, positive = down).
    fn move_by(&mut self, _lines: i32) {}
}

/// True when `XYLITOL_TUI_MOUSE` is a truthy opt-in (`1` / `true` / `yes`).
///
/// **Default off.** This is a **lab / PTY e2e** hook for `agent_demo` and harnesses
/// that call [`Terminal::enable_mouse_capture`] explicitly — **not** a product
/// setting for the inline TUI app.
///
/// Enabling capture on inline (emulator-owned / Mode A) sessions trades away
/// unmodified terminal selection/scroll. Official mouse UX (application
/// selection, click-fold) belongs on Mode B ([`crate::InteractionMode::ApplicationOwned`])
/// — see change `c2070` / capability `package-tui-interaction-modes`.
/// The product `TerminalGuard` deliberately does **not** read this env.
pub fn env_requests_mouse_capture() -> bool {
    match std::env::var("XYLITOL_TUI_MOUSE") {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
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
    /// Host/API asked for mouse capture (survives suspend `stop`/`start`).
    mouse_capture_desired: bool,
    /// Mouse capture is currently enabled on the TTY.
    mouse_capture_active: bool,
    /// Alternate screen buffer currently entered (Mode B).
    alternate_screen_active: bool,
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
            mouse_capture_desired: false,
            mouse_capture_active: false,
            alternate_screen_active: false,
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
        // Prefer crossterm's push (same CSI >Nu on unix). Optionally emit pi's
        // combined query for terminals that answer DA before Kitty flags —
        // modifyOtherKeys fallback remains available but is not auto-armed
        // on route A (keys.rs still matches legacy when the push is ignored).
        let _ = execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::from_bits_truncate(
                KITTY_FLAGS_REQUEST,
            ))
        );
        // Best-effort DA/Kitty probe for parity with pi; response is not
        // consumed here (crossterm owns the input layer).
        self.write_raw(KITTY_KEYBOARD_PROTOCOL_QUERY);
        // keys.rs matches legacy sequences even when the kitty flag is set, so
        // setting this active is safe: terminals that ignore the push keep
        // working via the legacy branches.
        set_kitty_protocol_active(true);
        self.kitty_pushed = true;
    }

    #[allow(dead_code)] // 预留：route-B modifyOtherKeys 协商；Kitty 路径今日不调用
    fn enable_modify_other_keys(&mut self) {
        if self.kitty_pushed || self.modify_other_keys_active {
            return;
        }
        self.write_raw(MODIFY_OTHER_KEYS_ENABLE);
        self.modify_other_keys_active = true;
    }

    /// After `EnterAlternateScreen`, re-push Kitty flags and arm modifyOtherKeys.
    ///
    /// Pi negotiates keyboard **after** alt-buffer is already active
    /// (`tui-alt-screen` enter → `ProcessTerminal.start`). xylitol does
    /// `start()` first then Mode B `enter_alternate_screen`, so this rearm is
    /// the parity path (research: reapply after alt-screen / suspend restore).
    /// Dual-arm is intentional under crossterm: we cannot consume Kitty DA
    /// responses the way pi's stdin owner does; write both and let the
    /// emulator honor what it supports.
    fn rearm_keyboard_after_alt_screen(&mut self) {
        let _ = execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::from_bits_truncate(
                KITTY_FLAGS_REQUEST,
            ))
        );
        set_kitty_protocol_active(true);
        self.kitty_pushed = true;
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

    /// Apply desired mouse capture to the live TTY when `started`.
    fn sync_mouse_capture(&mut self) {
        if !self.started {
            return;
        }
        if self.mouse_capture_desired && !self.mouse_capture_active {
            let _ = execute!(io::stdout(), EnableMouseCapture);
            self.mouse_capture_active = true;
        } else if !self.mouse_capture_desired && self.mouse_capture_active {
            let _ = execute!(io::stdout(), DisableMouseCapture);
            self.mouse_capture_active = false;
        }
    }

    /// Release capture on the TTY without clearing desire (for `stop`/suspend).
    fn release_mouse_capture_active(&mut self) {
        if self.mouse_capture_active {
            let _ = execute!(io::stdout(), DisableMouseCapture);
            self.mouse_capture_active = false;
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
        let _ = execute!(io::stdout(), cursor::Hide);
    }

    fn show_cursor(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show);
    }

    fn clear_line(&mut self) {
        let _ = execute!(io::stdout(), Clear(ClearType::UntilNewLine));
    }

    fn clear_from_cursor(&mut self) {
        let _ = execute!(io::stdout(), Clear(ClearType::FromCursorDown));
    }

    fn clear_screen(&mut self) {
        let _ = execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0));
    }

    fn flush(&mut self) {
        let _ = io::stdout().flush();
    }

    fn refresh_size(&mut self) {
        self.refresh_size_impl();
    }

    fn set_size_hint(&mut self, cols: u16, rows: u16) {
        if cols > 0 {
            self.columns = cols;
        }
        if rows > 0 {
            self.rows = rows;
        }
    }

    fn start(&mut self) {
        if self.started {
            return;
        }
        self.started = true;
        let _ = terminal::enable_raw_mode();
        let _ = execute!(io::stdout(), EnableBracketedPaste);
        self.negotiate_keyboard_protocol();
        self.sync_mouse_capture();
    }

    fn stop(&mut self) {
        if !self.started {
            return;
        }
        self.started = false;

        // Drop mouse reporting before paste/Kitty teardown (keeps exit order
        // aligned with crossterm event-read example: disable what we enabled).
        self.release_mouse_capture_active();

        // Leave alt-buffer before restoring main-screen protocols.
        if self.alternate_screen_active {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            self.alternate_screen_active = false;
        }

        // Disable bracketed paste first.
        let _ = execute!(io::stdout(), DisableBracketedPaste);

        // Pop Kitty enhancement flags BEFORE draining, so late key releases
        // do not generate new Kitty escape sequences.
        if self.kitty_pushed {
            self.write_raw(KITTY_POP_SEQUENCE);
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
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

    fn enable_mouse_capture(&mut self) {
        self.mouse_capture_desired = true;
        self.sync_mouse_capture();
    }

    fn disable_mouse_capture(&mut self) {
        self.mouse_capture_desired = false;
        self.sync_mouse_capture();
    }

    fn mouse_capture_active(&self) -> bool {
        self.mouse_capture_active
    }

    fn enter_alternate_screen(&mut self) {
        if self.alternate_screen_active {
            return;
        }
        let _ = execute!(io::stdout(), EnterAlternateScreen);
        self.alternate_screen_active = true;
        // Some emulators reset bracketed paste / keyboard enhancement when
        // entering the alt buffer. Without re-arm:
        // - pastes arrive as per-char keys (typewriter feel)
        // - Shift+Enter may degrade to plain Enter (pi: re-query Kitty +
        //   modifyOtherKeys CSI 27;2;13~ / Ghostty `\n` when kitty active).
        if self.started {
            let _ = execute!(io::stdout(), EnableBracketedPaste);
            self.rearm_keyboard_after_alt_screen();
        }
    }

    fn leave_alternate_screen(&mut self) {
        if !self.alternate_screen_active {
            return;
        }
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        self.alternate_screen_active = false;
    }

    fn alternate_screen_active(&self) -> bool {
        self.alternate_screen_active
    }

    fn set_title(&mut self, title: &str) {
        let _ = execute!(io::stdout(), SetTitle(title));
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
            let _ = execute!(io::stdout(), cursor::MoveDown(lines as u16));
        } else if lines < 0 {
            let _ = execute!(io::stdout(), cursor::MoveUp((-lines) as u16));
        }
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
