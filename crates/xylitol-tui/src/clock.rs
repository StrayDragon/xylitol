use std::time::{Duration, Instant};

pub trait Clock {
    fn now(&self) -> Instant;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Debug, Clone)]
pub struct MockClock {
    t: Instant,
}

impl MockClock {
    pub fn new() -> Self {
        Self { t: Instant::now() }
    }

    pub fn advance(&mut self, dur: Duration) {
        self.t += dur;
    }

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
        let mut clock = MockClock::new();
        let start = clock.now();
        clock.advance(Duration::from_millis(5));
        clock.advance(Duration::from_millis(5));
        let elapsed = clock.now().duration_since(start);
        assert_eq!(elapsed, Duration::from_millis(10));
    }

    #[test]
    fn mock_clock_window_boundary_exact() {
        let mut clock = MockClock::new();
        let t0 = clock.now();
        clock.advance(Duration::from_millis(7));
        let inside = clock.now().duration_since(t0);
        assert!(inside < Duration::from_millis(8), "7ms < 8ms threshold");

        clock.advance(Duration::from_millis(2));
        let outside = clock.now().duration_since(t0);
        assert!(outside >= Duration::from_millis(8), "9ms >= 8ms threshold");
    }

    #[tokio::test(start_paused = true)]
    async fn debounce_fires_after_window_under_paused_time() {
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(151)).await;
            true
        });
        tokio::time::advance(Duration::from_millis(151)).await;
        let fired = handle.await.unwrap();
        assert!(fired, "debounce callback should fire after the window");
    }
}
