//! Wheel-event → line-count conversion.
//!
//! 两家对标（2026-10-12 复核）：
//! - **opencode**：每事件恒定行数（`CustomSpeedScroll`，默认 3）——可预测零意外，
//!   **产品默认**。
//! - **pi**（`wheel-scroll.ts`）：`"auto"` 速度自适应 1..6 行/事件 + 5ms burst
//！   合并 + macOS 本地（OS 已加速）恒 1 —— opt-in。
//!
//! 无设备探测：粒度差异（hi-res 多事件/notch）交用户 `XYLITOL_WHEEL_SPEED` 自调，
//! 同两家「settings 开放行数」哲学。

use std::time::Instant;

/// Events closer than this belong to one physical notch (hi-res wheels).
const BURST_GAP_MS: u64 = 5;
/// A pause longer than this ends a scroll gesture.
const GESTURE_GAP_MS: u64 = 200;
/// Event gap mapping to one line per event; faster gaps scale up.
const REFERENCE_GAP_MS: f32 = 100.0;
const MAX_AUTO_LINES: isize = 6;

/// Constant lines-per-event or pi-style velocity-adaptive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelMode {
    /// pi `"auto"`: velocity-adaptive 1..[`MAX_AUTO_LINES`] lines/event.
    Auto,
    /// Constant lines per event (opencode `speed`; product default = 3).
    Fixed(isize),
}

/// Converts consecutive wheel events into line counts.
#[derive(Debug)]
pub struct WheelAccelerator {
    mode: WheelMode,
    /// pi: local macOS terminals receive OS-accelerated deltas; no extra mult.
    accelerate: bool,
    last_time: Option<Instant>,
    last_direction: isize,
    average_gap_ms: Option<f32>,
    carry: f32,
    /// Observability (host `xylitol::wheel` debug span).
    pub last_gap_ms: Option<f32>,
    pub last_lines: isize,
}

impl WheelAccelerator {
    pub fn new(mode: WheelMode, accelerate: bool) -> Self {
        Self {
            mode,
            accelerate,
            last_time: None,
            last_direction: 0,
            average_gap_ms: None,
            carry: 0.0,
            last_gap_ms: None,
            last_lines: 0,
        }
    }

    /// Product default: constant 3 lines/event (old `WHEEL_NOTCH` feel).
    /// `XYLITOL_WHEEL_SPEED=auto` → pi curve; `=<n>` → constant n.
    pub fn from_env() -> Self {
        let mode = match std::env::var("XYLITOL_WHEEL_SPEED").ok().as_deref() {
            Some("auto") => Some(WheelMode::Auto),
            Some(v) => v.parse::<isize>().ok().map(WheelMode::Fixed),
            None => None,
        }
        .unwrap_or(WheelMode::Fixed(3));
        Self::new(mode, platform_already_accelerated())
    }

    /// Positive line count for this event; sign applied by the caller.
    pub fn next(&mut self, direction: isize, now: Instant) -> isize {
        let WheelMode::Fixed(lines) = self.mode else {
            return self.next_auto(direction, now);
        };
        let _ = direction;
        self.last_lines = lines.max(1);
        self.last_lines
    }

    fn next_auto(&mut self, direction: isize, now: Instant) -> isize {
        if !self.accelerate {
            self.last_lines = 1;
            return 1;
        }
        let gap_ms = self.last_time.map_or(f32::INFINITY, |last| {
            now.duration_since(last).as_secs_f32() * 1000.0
        });
        self.last_gap_ms = Some(gap_ms);
        let same_gesture = direction == self.last_direction && gap_ms <= GESTURE_GAP_MS as f32;
        self.last_time = Some(now);
        self.last_direction = direction;
        if !same_gesture {
            self.average_gap_ms = None;
            self.carry = 0.0;
            self.last_lines = 1;
            return 1;
        }
        if (gap_ms as u64) < BURST_GAP_MS {
            return 1;
        }
        self.average_gap_ms = Some(match self.average_gap_ms {
            Some(avg) => (avg + gap_ms) / 2.0,
            None => gap_ms,
        });
        let lines = self.average_gap_ms.unwrap_or(REFERENCE_GAP_MS);
        let lines = (REFERENCE_GAP_MS / lines).clamp(1.0, MAX_AUTO_LINES as f32) + self.carry;
        let whole = lines.floor();
        self.carry = lines - whole;
        let out = whole.max(1.0) as isize;
        self.last_lines = out;
        out
    }

    /// Reset gesture state.
    pub fn reset(&mut self) {
        self.last_time = None;
        self.last_direction = 0;
        self.average_gap_ms = None;
        self.carry = 0.0;
    }
}

/// Local macOS terminals receive OS-accelerated wheel deltas (one event per
/// line); remotes and other platforms emit one event per notch.
fn platform_already_accelerated() -> bool {
    let ssh = std::env::var_os("SSH_CONNECTION").is_some()
        || std::env::var_os("SSH_CLIENT").is_some()
        || std::env::var_os("SSH_TTY").is_some();
    !ssh && cfg!(target_os = "macos")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn default_fixed_is_constant_three() {
        let mut a = WheelAccelerator::new(WheelMode::Fixed(3), true);
        let t0 = Instant::now();
        assert_eq!(a.next(1, t0), 3);
        assert_eq!(
            a.next(1, t0 + Duration::from_millis(5)),
            3,
            "hi-res burst too"
        );
        assert_eq!(a.next(-1, t0 + Duration::from_millis(50)), 3);
    }

    #[test]
    fn auto_slow_scroll_is_one_line_per_notch() {
        let mut a = WheelAccelerator::new(WheelMode::Auto, true);
        let t0 = Instant::now();
        assert_eq!(a.next(1, t0), 1);
        assert_eq!(a.next(1, t0 + Duration::from_millis(300)), 1, "new gesture");
        assert_eq!(a.next(1, t0 + Duration::from_millis(400)), 1);
    }

    #[test]
    fn auto_fast_spin_scales_up() {
        let mut a = WheelAccelerator::new(WheelMode::Auto, true);
        let t0 = Instant::now();
        a.next(1, t0);
        let mut t = t0;
        let mut last = 0;
        for _ in 0..30 {
            t += Duration::from_millis(20);
            last = a.next(1, t);
        }
        assert_eq!(last, 5, "20ms cadence settles at REFERENCE_GAP/20ms");
    }

    #[test]
    fn auto_burst_events_move_one_line() {
        let mut a = WheelAccelerator::new(WheelMode::Auto, true);
        let t0 = Instant::now();
        a.next(1, t0);
        assert_eq!(a.next(1, t0 + Duration::from_millis(2)), 1);
        assert_eq!(a.next(1, t0 + Duration::from_millis(4)), 1);
    }

    #[test]
    fn auto_no_acceleration_is_always_one() {
        let mut a = WheelAccelerator::new(WheelMode::Auto, false);
        let t0 = Instant::now();
        a.next(1, t0);
        for i in 1..10 {
            assert_eq!(a.next(1, t0 + Duration::from_millis(i * 5)), 1);
        }
    }

    #[test]
    fn auto_direction_flip_restarts_gesture() {
        let mut a = WheelAccelerator::new(WheelMode::Auto, true);
        let t0 = Instant::now();
        a.next(1, t0);
        a.next(1, t0 + Duration::from_millis(10));
        assert_eq!(a.next(-1, t0 + Duration::from_millis(20)), 1);
    }
}
