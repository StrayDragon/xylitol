//! Injectable time source (c405 layer 3, spec tt04).
//!
//! Timing-dependent TUI logic (paste-burst detection, autocomplete debounce,
//! loader spinner frame scheduling) MUST accept a `Clock` so tests advance
//! time deterministically without `thread::sleep`. Synchronous logic takes a
//! `Clock`; async logic uses `#[tokio::test(start_paused = true)]` instead.
//!
//! The paste-burst detector (to be ported in a later change) will be
//! constructed as `PasteBurst::new(clock)`; production passes `SystemClock`,
//! tests pass `MockClock`.

use std::time::{Duration, Instant};

/// A source of monotonic time. Implementations:
/// - [`SystemClock`]: reads real wall-clock time (production).
/// - [`MockClock`]: a test double whose time is advanced manually.
pub trait Clock {
    fn now(&self) -> Instant;
}

/// Real wall-clock clock. Used in production.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A test clock whose time advances only when explicitly told to. Lets
/// timing-window tests (paste-burst's 8ms inter-char, 120ms enter-suppress)
/// run deterministically — the just-inside vs just-outside boundary is
/// asserted by advancing exactly N milliseconds, with no real waiting.
#[derive(Debug, Clone)]
pub struct MockClock {
    /// The current "now"; starts at the epoch of construction.
    t: Instant,
}

impl MockClock {
    /// Create a clock frozen at construction time.
    pub fn new() -> Self {
        Self { t: Instant::now() }
    }

    /// Advance the clock by `dur`. Subsequent `now()` calls reflect the new time.
    pub fn advance(&mut self, dur: Duration) {
        self.t += dur;
    }

    /// Read the current frozen time (for tests that want to inspect it).
    pub fn peek(&self) -> Instant {
        self.t
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        self.t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_clock_advances_deterministically() {
        // Covers the precondition for scenario tt04: the clock is a
        // controllable time source. Two 5ms advances yield exactly 10ms,
        // deterministically — no thread::sleep drift.
        let mut clock = MockClock::new();
        let start = clock.now();
        clock.advance(Duration::from_millis(5));
        clock.advance(Duration::from_millis(5));
        let elapsed = clock.now().duration_since(start);
        assert_eq!(elapsed, Duration::from_millis(10));
    }

    #[test]
    fn mock_clock_window_boundary_exact() {
        // The paste-burst boundary test pattern: a threshold of 8ms. A pair
        // 7ms apart is INSIDE the window; 9ms is OUTSIDE. The mock clock
        // makes both branches deterministic.
        let mut clock = MockClock::new();
        let t0 = clock.now();
        clock.advance(Duration::from_millis(7));
        let inside = clock.now().duration_since(t0);
        assert!(inside < Duration::from_millis(8), "7ms < 8ms threshold");

        clock.advance(Duration::from_millis(2)); // now 9ms total
        let outside = clock.now().duration_since(t0);
        assert!(outside >= Duration::from_millis(8), "9ms >= 8ms threshold");
    }

    #[tokio::test(start_paused = true)]
    async fn debounce_fires_after_window_under_paused_time() {
        // Covers the async-time pattern (scenario tt04): a debounce window
        // elapses under tokio's paused clock, firing a deferred callback.
        // This is a skeleton — the real debounce logic lands with the
        // autocomplete port; here we prove the mechanism works.
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(151)).await;
            // In the real debounce this callback would be the provider call.
            true
        });
        // Advance virtual time past the 150ms debounce window.
        tokio::time::advance(Duration::from_millis(151)).await;
        let fired = handle.await.unwrap();
        assert!(fired, "debounce callback should fire after the window");
    }
}
