//! Wheel-batch → line-count acceleration.
//!
//! 口径（v3，实测校准 2026-10-12）：coalesce 后的**事件批**为单元——高分辨率
//! 滚轮一格发 4-6 事件（实测同毫秒 5 连发），批内事件数不放大行数；批计 1 notch
//! 基线 [`BASE_LINES_PER_NOTCH`]（旧 `WHEEL_NOTCH=3` 手感），批间 cadence 加速
//! 快甩至基线×[`MAX_SPEED_MULT`]。`XYLITOL_WHEEL_SPEED=<n>` 钉固定行数。

use std::time::Instant;

/// A pause longer than this ends a scroll gesture (new gesture ⇒ base lines).
const GESTURE_GAP_MS: f32 = 200.0;
/// Batch gap mapping to 1× base (slower than this never accelerates).
const REFERENCE_GAP_MS: f32 = 100.0;
/// Base lines per notch — the historical `WHEEL_NOTCH` feel.
const BASE_LINES_PER_NOTCH: f32 = 3.0;
const MAX_SPEED_MULT: f32 = 3.0;

/// Fixed lines-per-batch or velocity-adaptive mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelMode {
    /// Velocity-adaptive mult over [`BASE_LINES_PER_NOTCH`].
    Auto,
    /// Constant lines per batch.
    Fixed(isize),
}

/// Converts coalesced wheel batches into line counts.
#[derive(Debug)]
pub struct WheelAccelerator {
    mode: WheelMode,
    last_time: Option<Instant>,
    last_direction: isize,
    average_gap_ms: Option<f32>,
    carry: f32,
    /// Observability: last gap / lines (host `xylitol::wheel` debug span).
    pub last_gap_ms: Option<f32>,
    pub last_lines: isize,
}

impl WheelAccelerator {
    pub fn new(mode: WheelMode) -> Self {
        Self {
            mode,
            last_time: None,
            last_direction: 0,
            average_gap_ms: None,
            carry: 0.0,
            last_gap_ms: None,
            last_lines: 0,
        }
    }

    /// Default product policy; `XYLITOL_WHEEL_SPEED=<n>` pins a constant.
    pub fn from_env() -> Self {
        let mode = std::env::var("XYLITOL_WHEEL_SPEED")
            .ok()
            .and_then(|v| v.parse::<isize>().ok().map(WheelMode::Fixed))
            .unwrap_or(WheelMode::Auto);
        Self::new(mode)
    }

    /// Lines for one coalesced batch of `events` wheel events (same direction).
    /// Sign is applied by the caller.
    pub fn next_batch(&mut self, events: isize, now: Instant) -> isize {
        let WheelMode::Fixed(lines) = self.mode else {
            return self.next_auto(now);
        };
        let _ = events;
        self.last_lines = lines.max(1);
        self.last_lines
    }

    fn next_auto(&mut self, now: Instant) -> isize {
        let gap_ms = self.last_time.map_or(f32::INFINITY, |last| {
            now.duration_since(last).as_secs_f32() * 1000.0
        });
        self.last_time = Some(now);
        self.last_gap_ms = Some(gap_ms);
        let same_gesture = self.last_direction > 0 && gap_ms <= GESTURE_GAP_MS;
        if !same_gesture {
            self.last_direction = 1;
            self.average_gap_ms = None;
            self.carry = 0.0;
            self.last_lines = BASE_LINES_PER_NOTCH as isize;
            return self.last_lines;
        }
        self.average_gap_ms = Some(match self.average_gap_ms {
            Some(avg) => (avg + gap_ms) / 2.0,
            None => gap_ms,
        });
        let mult = (REFERENCE_GAP_MS / self.average_gap_ms.unwrap_or(REFERENCE_GAP_MS))
            .clamp(1.0, MAX_SPEED_MULT);
        let lines = BASE_LINES_PER_NOTCH * mult + self.carry;
        let whole = lines.floor();
        self.carry = lines - whole;
        let out = whole.max(1.0) as isize;
        self.last_lines = out;
        out
    }

    /// Reset gesture state (direction change handled by the caller re-invoking
    /// after a `scroll_to` / jump; gaps > `GESTURE_GAP_MS` self-reset).
    pub fn reset(&mut self) {
        self.last_time = None;
        self.last_direction = 0;
        self.average_gap_ms = None;
        self.carry = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn slow_batches_stay_at_base() {
        let mut a = WheelAccelerator::new(WheelMode::Auto);
        let t0 = Instant::now();
        assert_eq!(a.next_batch(1, t0), 3);
        assert_eq!(
            a.next_batch(1, t0 + Duration::from_millis(300)),
            3,
            "new gesture"
        );
        assert_eq!(a.next_batch(1, t0 + Duration::from_millis(400)), 3);
    }

    #[test]
    fn fast_cadence_scales_to_cap() {
        let mut a = WheelAccelerator::new(WheelMode::Auto);
        let t0 = Instant::now();
        a.next_batch(1, t0);
        let mut t = t0;
        let mut last = 0;
        for _ in 0..30 {
            t += Duration::from_millis(20);
            last = a.next_batch(1, t);
        }
        assert_eq!(last, 9, "20ms cadence ⇒ 3×cap 3 = 9");
    }

    #[test]
    fn hires_batch_counts_once() {
        let mut a = WheelAccelerator::new(WheelMode::Auto);
        let t0 = Instant::now();
        assert_eq!(
            a.next_batch(5, t0),
            3,
            "5-event hi-res notch = 1 notch of lines"
        );
    }

    #[test]
    fn fixed_mode_is_constant() {
        let mut a = WheelAccelerator::new(WheelMode::Fixed(3));
        let t0 = Instant::now();
        assert_eq!(a.next_batch(1, t0), 3);
        assert_eq!(a.next_batch(7, t0 + Duration::from_millis(5)), 3);
    }

    #[test]
    fn direction_flip_via_gap_reset() {
        let mut a = WheelAccelerator::new(WheelMode::Auto);
        let t0 = Instant::now();
        a.next_batch(1, t0);
        a.next_batch(1, t0 + Duration::from_millis(10));
        // Caller models a flip as a long gap (scroll_to / pause).
        assert_eq!(a.next_batch(1, t0 + Duration::from_millis(400)), 3);
    }
}
