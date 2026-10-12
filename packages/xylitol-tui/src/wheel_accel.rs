//! Wheel-event → line-count acceleration (pi `wheel-scroll.ts` semantics).
//!
//! Slow scrolling stays at one line per event; fast spins scale up to
//! [`MAX_AUTO_LINES`] by event cadence. Bursts closer than
//! [`BURST_GAP_MS`] belong to one physical notch and move a single line.

use std::time::Instant;

/// Events closer than this belong to one physical notch (high-res wheels).
const BURST_GAP_MS: u64 = 5;
/// A pause longer than this ends a scroll gesture.
const GESTURE_GAP_MS: u64 = 200;
/// Event gap that maps to one line per event; faster gaps scale up.
const REFERENCE_GAP_MS: f32 = 100.0;
const MAX_AUTO_LINES: isize = 6;

/// Fixed lines-per-event or velocity-adaptive mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelMode {
    /// Velocity-adaptive (pi `lines: "auto"`).
    Auto,
    /// Constant lines per event.
    Fixed(isize),
}

/// Converts consecutive wheel events into line counts.
#[derive(Debug)]
pub struct WheelAccelerator {
    mode: WheelMode,
    accelerate: bool,
    last_time: Option<Instant>,
    last_direction: isize,
    average_gap_ms: Option<f32>,
    carry: f32,
    /// Lines emitted by the last non-burst event; bursts inherit it (our
    /// coalesce drain collapses same-batch events to ~0ms gap, which is a
    /// processing artifact — pi sees real hi-res sub-notches there).
    last_lines: isize,
    last_gap_ms: f32,
}

impl WheelAccelerator {
    /// `accelerate=false` keeps one line per event (for sources whose OS
    /// already applies acceleration, e.g. local macOS terminals).
    pub fn new(mode: WheelMode, accelerate: bool) -> Self {
        Self {
            mode,
            accelerate,
            last_time: None,
            last_direction: 0,
            average_gap_ms: None,
            carry: 0.0,
            last_lines: 1,
            last_gap_ms: 0.0,
        }
    }

    /// Observability: (last inter-event gap ms, lines emitted by last event).
    pub fn last_debug(&self) -> (Option<f32>, isize) {
        let gap = self.last_time.map(|_| self.last_gap_ms);
        (gap, self.last_lines)
    }

    /// Default product policy: adaptive, with the platform dedup respect.
    pub fn from_env() -> Self {
        let mode = match std::env::var("XYLITOL_WHEEL_SPEED").ok().as_deref() {
            Some(v) => v.parse::<isize>().ok().map(WheelMode::Fixed),
            _ => None,
        }
        .unwrap_or(WheelMode::Auto);
        Self::new(mode, platform_already_accelerates())
    }

    /// Positive line count for this event; sign comes from the caller.
    pub fn next(&mut self, direction: isize, now: Instant) -> isize {
        let WheelMode::Fixed(lines) = self.mode else {
            return self.next_auto(direction, now);
        };
        let _ = direction;
        lines.max(1)
    }

    fn next_auto(&mut self, direction: isize, now: Instant) -> isize {
        if !self.accelerate {
            self.last_lines = 1;
            return 1;
        }
        let gap_ms = self.last_time.map_or(f32::INFINITY, |last| {
            now.duration_since(last).as_secs_f32() * 1000.0
        });
        let same_gesture = direction == self.last_direction && gap_ms <= GESTURE_GAP_MS as f32;
        self.last_time = Some(now);
        self.last_direction = direction;
        if !same_gesture {
            self.average_gap_ms = None;
            self.carry = 0.0;
            self.last_gap_ms = gap_ms;
            self.last_lines = 1;
            return 1;
        }
        if (gap_ms as u64) < BURST_GAP_MS {
            // Same-batch / hi-res sub-event: inherit the notch's line count
            // instead of collapsing to 1 (coalesce makes these gaps ~0).
            self.last_gap_ms = gap_ms;
            return self.last_lines.max(1);
        }
        self.average_gap_ms = Some(match self.average_gap_ms {
            Some(avg) => (avg + gap_ms) / 2.0,
            None => gap_ms,
        });
        self.last_gap_ms = gap_ms;
        let avg = self.average_gap_ms.unwrap_or(REFERENCE_GAP_MS);
        let lines = (REFERENCE_GAP_MS / avg).clamp(1.0, MAX_AUTO_LINES as f32) + self.carry;
        let whole = lines.floor();
        self.carry = lines - whole;
        // Carry is fractional by construction; whole is at least 1 here.
        let out = whole.max(1.0) as isize;
        self.last_lines = out;
        out
    }

    /// Reset gesture state (direction change / long pause handled in [`Self::next`]).
    pub fn reset(&mut self) {
        self.last_time = None;
        self.last_gap_ms = 0.0;
        self.last_lines = 1;
        self.last_direction = 0;
        self.average_gap_ms = None;
        self.carry = 0.0;
    }
}

/// Local macOS terminals receive OS-accelerated wheel deltas (one event per
/// line); remotes and other platforms emit one event per notch.
fn platform_already_accelerates() -> bool {
    let ssh = std::env::var_os("SSH_CONNECTION").is_some()
        || std::env::var_os("SSH_CLIENT").is_some()
        || std::env::var_os("SSH_TTY").is_some();
    !ssh && cfg!(target_os = "macos")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn acc() -> WheelAccelerator {
        WheelAccelerator::new(WheelMode::Auto, true)
    }

    #[test]
    fn slow_scroll_is_one_line_per_notch() {
        let mut a = acc();
        let t0 = Instant::now();
        assert_eq!(a.next(1, t0), 1);
        assert_eq!(a.next(1, t0 + Duration::from_millis(300)), 1, "new gesture");
        assert_eq!(a.next(1, t0 + Duration::from_millis(400)), 1);
    }

    #[test]
    fn fast_spin_scales_up() {
        let mut a = acc();
        let t0 = Instant::now();
        a.next(1, t0);
        // 20ms cadence ⇒ REFERENCE/avg grows towards the cap.
        let mut t = t0;
        let mut last = 0;
        for _ in 0..30 {
            t += Duration::from_millis(20);
            last = a.next(1, t);
        }
        assert_eq!(
            last, 5,
            "sustained 20ms cadence settles at REFERENCE_GAP/20ms"
        );
    }

    #[test]
    fn burst_events_move_one_line() {
        let mut a = acc();
        let t0 = Instant::now();
        a.next(1, t0);
        assert_eq!(a.next(1, t0 + Duration::from_millis(2)), 1);
        assert_eq!(a.next(1, t0 + Duration::from_millis(4)), 1);
    }

    #[test]
    fn direction_flip_restarts_gesture() {
        let mut a = acc();
        let t0 = Instant::now();
        a.next(1, t0);
        a.next(1, t0 + Duration::from_millis(10));
        assert_eq!(a.next(-1, t0 + Duration::from_millis(20)), 1);
    }

    #[test]
    fn fixed_mode_is_constant() {
        let mut a = WheelAccelerator::new(WheelMode::Fixed(3), true);
        let t0 = Instant::now();
        assert_eq!(a.next(1, t0), 3);
        assert_eq!(a.next(1, t0 + Duration::from_millis(5)), 3);
    }

    #[test]
    fn no_acceleration_is_always_one() {
        let mut a = WheelAccelerator::new(WheelMode::Auto, false);
        let t0 = Instant::now();
        a.next(1, t0);
        for i in 1..10 {
            assert_eq!(a.next(1, t0 + Duration::from_millis(i * 5)), 1);
        }
    }
}
