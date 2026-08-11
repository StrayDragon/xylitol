//! PasteBurst — detects non-bracketed paste bursts (c415, port of pi
//! `paste-burst.ts`).
//!
//! A heuristic fallback for terminals that do not surface bracketed-paste
//! markers: a rapid stream of plain characters followed by Enter, where a
//! bare Enter would otherwise submit the draft. The detector does NOT buffer
//! characters (the editor inserts text normally); it only decides whether an
//! imminent Enter should insert a newline instead of submitting.
//!
//! ## Time injection
//!
//! All methods take an `Instant` (`now`) parameter rather than reading
//! wall-clock time, so tests advance time deterministically without
//! `thread::sleep` (design decision: parameter injection over holding a
//! `Clock`). The editor passes `clock.now()` at each call site.

use std::time::{Duration, Instant};

/// Minimum consecutive fast chars to recognize as a paste burst (pi: 8).
pub const PASTE_BURST_MIN_CHARS: u32 = 8;
/// Max inter-char gap (in ms) still counted as consecutive (pi: 8ms).
pub const PASTE_BURST_CHAR_INTERVAL_MS: u64 = 8;
/// How long the detector stays active between chars during a burst (pi: 30ms).
pub const PASTE_BURST_ACTIVE_IDLE_TIMEOUT_MS: u64 = 30;
/// Window after a burst during which Enter inserts a newline (pi: 120ms).
pub const PASTE_BURST_ENTER_SUPPRESS_WINDOW_MS: u64 = 120;

const CHAR_INTERVAL: Duration = Duration::from_millis(PASTE_BURST_CHAR_INTERVAL_MS);
const ACTIVE_IDLE_TIMEOUT: Duration = Duration::from_millis(PASTE_BURST_ACTIVE_IDLE_TIMEOUT_MS);
const ENTER_SUPPRESS_WINDOW: Duration = Duration::from_millis(PASTE_BURST_ENTER_SUPPRESS_WINDOW_MS);

/// Detects non-bracketed paste bursts. Pure state machine; methods accept an
/// explicit `now` so tests are deterministic (c405 layer 3 pattern).
#[derive(Default)]
pub struct PasteBurst {
    /// Timestamp of the last `on_plain_char`; None until the first char.
    last_plain_char_at: Option<Instant>,
    /// Current run of consecutive fast chars (reset when a gap exceeds the
    /// inter-char threshold).
    consecutive_plain_chars: u32,
    /// Detector is active (a burst was recognized) until this instant.
    active_until: Option<Instant>,
    /// Enter should insert a newline (not submit) until this instant.
    enter_suppress_until: Option<Instant>,
}

impl PasteBurst {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a plain-character arrival. Call this for every printable char the
    /// editor receives (the editor decides what counts as printable).
    pub fn on_plain_char(&mut self, now: Instant) {
        let is_consecutive = self
            .last_plain_char_at
            .is_some_and(|last| now.duration_since(last) <= CHAR_INTERVAL);

        if is_consecutive {
            self.consecutive_plain_chars += 1;
        } else {
            self.consecutive_plain_chars = 1;
        }
        self.last_plain_char_at = Some(now);

        if self.consecutive_plain_chars >= PASTE_BURST_MIN_CHARS {
            self.extend_window(now);
        }
    }

    /// Should an Enter at `now` insert a newline rather than submit? Returns
    /// true during the active or enter-suppress windows, or when the last run
    /// of chars was a burst still within the inter-char threshold.
    pub fn should_insert_newline_instead_of_submit(&self, now: Instant) -> bool {
        // Explicit open windows take precedence.
        if self.active_until.is_some_and(|t| now <= t)
            || self.enter_suppress_until.is_some_and(|t| now <= t)
        {
            return true;
        }
        // Otherwise: a burst that just ended (last char within the threshold
        // and enough consecutive chars) still suppresses one final Enter.
        self.last_plain_char_at.is_some_and(|last| {
            self.consecutive_plain_chars >= PASTE_BURST_MIN_CHARS
                && now.duration_since(last) <= CHAR_INTERVAL
        })
    }

    /// True while a non-bracketed paste burst window is open (active or suppress).
    pub fn is_coalescing(&self, now: Instant) -> bool {
        self.active_until.is_some_and(|t| now <= t)
            || self.enter_suppress_until.is_some_and(|t| now <= t)
            || self.consecutive_plain_chars >= PASTE_BURST_MIN_CHARS
    }

    /// How many consecutive fast chars are currently tracked.
    pub fn consecutive_plain_chars(&self) -> u32 {
        self.consecutive_plain_chars
    }

    /// Open/extend the active + enter-suppress windows. Called internally on
    /// burst detection; also public so the editor can force-extend (pi does
    /// this right before inserting the newline).
    pub fn extend_window(&mut self, now: Instant) {
        self.active_until = Some(now + ACTIVE_IDLE_TIMEOUT);
        self.enter_suppress_until = Some(now + ENTER_SUPPRESS_WINDOW);
    }

    /// Clear all state. Call on non-printable keys, bracketed paste, or when
    /// paste-burst detection is disabled (pi: editor calls reset in
    /// handleInput when the key is neither a plain char nor Enter).
    pub fn reset(&mut self) {
        self.last_plain_char_at = None;
        self.consecutive_plain_chars = 0;
        self.active_until = None;
        self.enter_suppress_until = None;
    }
}
